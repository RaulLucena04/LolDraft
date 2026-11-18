// Compila el instalador y lo publica en el canal local de actualizaciones.
// Uso:
//   npm run release                 sube la versión de parche (0.1.0 -> 0.1.1)
//   npm run release -- --version 0.2.0
//   npm run release -- --no-bump    recompila sin cambiar la versión
//   npm run release -- --skip-build publica el último bundle ya compilado
//   npm run release -- --out "D:\\updates"   publica en otra carpeta
//
// El canal es `%APPDATA%\<identifier>\updates` por defecto: la misma carpeta
// que lee la app cuando no se configura `updates.source`. No hay firma ni
// servidor: el `sha256` se publica solo para verificación manual.

import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const ROOT = process.cwd();
const ROOT_PACKAGE = join(ROOT, "package.json");
const DESKTOP_PACKAGE = join(ROOT, "apps", "desktop", "package.json");
const PACKAGE_LOCK = join(ROOT, "package-lock.json");
const TAURI_CONF = join(ROOT, "src-tauri", "tauri.conf.json");
const WORKSPACE_CARGO = join(ROOT, "Cargo.toml");
const BUNDLE_DIRS = [
  join(ROOT, "target", "release", "bundle", "nsis"),
  join(ROOT, "src-tauri", "target", "release", "bundle", "nsis"),
];

function parseArgs(argv) {
  const options = { bump: true, build: true, version: null, notes: null, out: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--version") {
      options.version = argv[++index];
      options.bump = false;
      if (!options.version) throw new Error("--version necesita un valor, por ejemplo 0.2.0");
    } else if (arg === "--notes") {
      options.notes = argv[++index];
      if (!options.notes) throw new Error("--notes necesita un texto");
    } else if (arg === "--out") {
      options.out = argv[++index];
      if (!options.out) throw new Error("--out necesita una carpeta");
    } else if (arg === "--no-bump") {
      options.bump = false;
    } else if (arg === "--skip-build") {
      options.build = false;
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Uso: npm run release -- [--version X.Y.Z] [--no-bump] [--skip-build] [--notes TEXTO] [--out CARPETA]",
      );
      process.exit(0);
    } else {
      throw new Error(`Argumento no reconocido: ${arg}`);
    }
  }
  return options;
}

function bumpPatch(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/.exec(version.trim());
  if (!match) throw new Error(`La versión actual no es válida: ${version}`);
  return `${match[1]}.${match[2]}.${Number(match[3]) + 1}`;
}

function assertVersion(version) {
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Versión no válida: ${version}. Usa un formato como 0.2.0`);
  }
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

/** Cambia solo el primer `"version"` del fichero, sin reformatear el JSON. */
async function writeVersion(path, version) {
  const text = await readFile(path, "utf8");
  const next = text.replace(/("version"\s*:\s*")[^"]*(")/, `$1${version}$2`);
  if (next === text && !text.includes(`"version": "${version}"`)) {
    throw new Error(`No encontré la versión en ${path}`);
  }
  await writeFile(path, next, "utf8");
}

/** Cambia `version` dentro de `[workspace.package]` de Cargo.toml. */
async function writeCargoVersion(path, version) {
  const text = await readFile(path, "utf8");
  const next = text.replace(
    /(\[workspace\.package\][^[]*?\nversion\s*=\s*")[^"]*(")/,
    `$1${version}$2`,
  );
  if (next === text && !text.includes(`[workspace.package]`)) {
    throw new Error(`No encontré [workspace.package] en ${path}`);
  }
  await writeFile(path, next, "utf8");
}

/** Sincroniza la versión de los paquetes del workspace en `package-lock.json`. */
async function writeLockVersion(path, version) {
  const lock = await readJson(path);
  lock.version = version;
  if (lock.packages?.[""]) lock.packages[""].version = version;
  if (lock.packages?.["apps/desktop"]) {
    lock.packages["apps/desktop"].version = version;
  }
  await writeFile(path, `${JSON.stringify(lock, null, 2)}\n`, "utf8");
}

function runNpm() {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  const result = spawnSync(npm, ["run", "app:build"], {
    cwd: ROOT,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`La compilación falló con código ${result.status}`);
  }
}

async function findInstaller(version) {
  const candidates = [];
  for (const dir of BUNDLE_DIRS) {
    let files;
    try {
      files = await readdir(dir);
    } catch {
      continue;
    }
    for (const name of files) {
      if (name.toLowerCase().endsWith("-setup.exe")) {
        candidates.push(join(dir, name));
      }
    }
  }
  if (candidates.length === 0) {
    throw new Error(
      "No encontré ningún instalador NSIS. Compila con `npm run app:build` o revisa target/release/bundle/nsis.",
    );
  }
  const exact = candidates.find((path) => basename(path).includes(`_${version}_`));
  if (exact) return exact;
  const stats = await Promise.all(
    candidates.map(async (path) => ({ path, modified: (await stat(path)).mtimeMs })),
  );
  stats.sort((left, right) => right.modified - left.modified);
  return stats[0].path;
}

async function sha256(path) {
  const hash = createHash("sha256");
  hash.update(await readFile(path));
  return hash.digest("hex");
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const rootPackage = await readJson(ROOT_PACKAGE);
  const tauriConf = await readJson(TAURI_CONF);
  const current = rootPackage.version;

  let version = options.version;
  if (!version) version = options.bump ? bumpPatch(current) : current;
  assertVersion(version);

  if (version !== current) {
    console.log(`\nVersión ${current} → ${version}`);
    await writeVersion(ROOT_PACKAGE, version);
    await writeVersion(DESKTOP_PACKAGE, version);
    await writeVersion(TAURI_CONF, version);
    await writeCargoVersion(WORKSPACE_CARGO, version);
    await writeLockVersion(PACKAGE_LOCK, version);
  } else {
    console.log(`\nSe mantiene la versión ${version}`);
  }

  if (options.build) {
    console.log("\nCompilando el instalador (tauri build)…");
    runNpm();
  }

  const installer = await findInstaller(version);
  const installerName = basename(installer);
  const identifier = tauriConf.identifier;
  const appData =
    process.platform === "win32"
      ? process.env.APPDATA ?? join(homedir(), "AppData", "Roaming")
      : join(homedir(), ".local", "share");
  const outDir = options.out ? resolve(options.out) : join(appData, identifier, "updates");

  await mkdir(outDir, { recursive: true });
  const published = join(outDir, installerName);
  if (resolve(published) !== resolve(installer)) {
    await copyFile(installer, published);
  }

  const manifest = {
    version,
    installer: installerName,
    notes: options.notes ?? `Compilación local de LolDraft ${version}.`,
    sha256: await sha256(published),
    generated_at: new Date().toISOString(),
  };
  const manifestPath = join(outDir, "latest.json");
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  console.log("\nCanal actualizado:");
  console.log(`  versión    ${version}`);
  console.log(`  instalador ${published}`);
  console.log(`  sha256     ${manifest.sha256}`);
  console.log(`  manifiesto ${manifestPath}`);
  console.log("\nAbre LolDraft → Inicio → Actualizaciones → Buscar actualizaciones.");
}

main().catch((error) => {
  console.error(`\nError: ${error.message}`);
  process.exit(1);
});
