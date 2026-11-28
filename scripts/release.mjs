// Compila el instalador y lo publica en el canal de actualizaciones.
// Uso:
//   npm run release                 sube la versión de parche (0.1.0 -> 0.1.1)
//   npm run release -- --version 0.2.0
//   npm run release -- --no-bump    recompila sin cambiar la versión
//   npm run release -- --skip-build publica el último bundle ya compilado
//   npm run release -- --out "D:\\updates"   publica en otra carpeta
//   npm run release -- --notes-file notas.md
//   npm run release -- --github     además crea la release de GitHub con
//                                   `gh` (requiere `gh auth login`)
//   npm run release -- --github --repo usuario/otro-repo
//
// El canal local es `%APPDATA%\<identifier>\updates`, donde publica el script.
// La app ya no lo lee por defecto: sin origen configurado consulta la última
// release de GitHub. El canal local sirve para probar antes de publicar.
// No hay firma: el `sha256` se publica solo para verificación manual.

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
/** Repositorio del canal público por defecto (mismo que `DEFAULT_CHANNEL`). */
const DEFAULT_REPO = "RaulLucena04/LolDraft";
const BUNDLE_DIRS = [
  join(ROOT, "target", "release", "bundle", "nsis"),
  join(ROOT, "src-tauri", "target", "release", "bundle", "nsis"),
];

function parseArgs(argv) {
  const options = {
    bump: true,
    build: true,
    github: false,
    repo: null,
    version: null,
    notes: null,
    notesFile: null,
    out: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--version") {
      options.version = argv[++index];
      options.bump = false;
      if (!options.version) throw new Error("--version necesita un valor, por ejemplo 0.2.0");
    } else if (arg === "--notes") {
      options.notes = argv[++index];
      if (!options.notes) throw new Error("--notes necesita un texto");
    } else if (arg === "--notes-file") {
      options.notesFile = argv[++index];
      if (!options.notesFile) throw new Error("--notes-file necesita una ruta");
    } else if (arg === "--out") {
      options.out = argv[++index];
      if (!options.out) throw new Error("--out necesita una carpeta");
    } else if (arg === "--repo") {
      options.repo = argv[++index];
      if (!options.repo) throw new Error("--repo necesita owner/nombre");
    } else if (arg === "--no-bump") {
      options.bump = false;
    } else if (arg === "--skip-build") {
      options.build = false;
    } else if (arg === "--github") {
      options.github = true;
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Uso: npm run release -- [--version X.Y.Z] [--no-bump] [--skip-build] [--notes TEXTO] [--notes-file RUTA] [--out CARPETA] [--github] [--repo OWNER/NOMBRE]",
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
  const result =
    process.platform === "win32"
      ? spawnSync("cmd.exe", ["/C", "npm", "run", "app:build"], {
          cwd: ROOT,
          stdio: "inherit",
        })
      : spawnSync("npm", ["run", "app:build"], { cwd: ROOT, stdio: "inherit" });
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

/** Ejecuta `gh` capturando la salida; con `inherit` deja ver su progreso. */
function gh(args, { inherit = false } = {}) {
  const command = process.platform === "win32" ? "gh.exe" : "gh";
  const result = spawnSync(command, args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: inherit ? "inherit" : "pipe",
  });
  if (result.error) {
    if (result.error.code === "ENOENT") {
      throw new Error("No encuentro `gh` en el PATH. Instala GitHub CLI: https://cli.github.com/");
    }
    throw result.error;
  }
  return result;
}

function detectRepo() {
  const view = gh(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"]);
  const name = view.stdout?.trim();
  if (view.status !== 0 || !name) {
    throw new Error("No pude detectar el repositorio. Indícalo con --repo owner/nombre.");
  }
  return name;
}

/**
 * Crea (o actualiza) la release de GitHub con el instalador y `latest.json`.
 * `releases/latest/download/<asset>` sirve ambos por nombre, que es justo el
 * canal que consulta la app.
 */
async function publishToGithub({ version, installer, manifestPath, notes, repo }) {
  if (gh(["auth", "status"]).status !== 0) {
    throw new Error(
      "GitHub CLI no tiene sesión. Ejecuta `gh auth login` (o define GH_TOKEN) y reintenta.",
    );
  }

  const target = repo ?? detectRepo();
  const tag = `v${version}`;
  const assets = [installer, manifestPath];
  const exists = gh(["release", "view", tag, "--repo", target]).status === 0;

  if (exists) {
    console.log(`\nLa release ${tag} ya existe: subo los assets y actualizo las notas.`);
    const upload = gh(["release", "upload", tag, ...assets, "--repo", target, "--clobber"], {
      inherit: true,
    });
    if (upload.status !== 0) throw new Error(`gh release upload falló (${upload.status})`);
    const edit = gh(
      ["release", "edit", tag, "--repo", target, "--notes", notes, "--latest"],
      { inherit: true },
    );
    if (edit.status !== 0) throw new Error(`gh release edit falló (${edit.status})`);
  } else {
    const create = gh(
      [
        "release",
        "create",
        tag,
        ...assets,
        "--repo",
        target,
        "--title",
        `LolDraft ${version}`,
        "--notes",
        notes,
        "--latest",
      ],
      { inherit: true },
    );
    if (create.status !== 0) throw new Error(`gh release create falló (${create.status})`);
  }

  return target;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const rootPackage = await readJson(ROOT_PACKAGE);
  const tauriConf = await readJson(TAURI_CONF);
  const current = rootPackage.version;

  // Las notas llegan por fichero: pasar UTF-8 por argv las descuadra en Windows.
  if (options.notes && options.notesFile) {
    throw new Error("Usa --notes o --notes-file, no los dos a la vez");
  }
  const notes = options.notesFile
    ? (await readFile(resolve(options.notesFile), "utf8")).trim()
    : options.notes;

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
    notes: notes ?? `Versión ${version} de LolDraft.`,
    sha256: await sha256(published),
    generated_at: new Date().toISOString(),
  };
  const manifestPath = join(outDir, "latest.json");
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  console.log("\nCanal local actualizado:");
  console.log(`  versión    ${version}`);
  console.log(`  instalador ${published}`);
  console.log(`  sha256     ${manifest.sha256}`);
  console.log(`  manifiesto ${manifestPath}`);

  let publishedRepo = null;
  if (options.github) {
    console.log("\nPublicando en GitHub Releases…");
    publishedRepo = await publishToGithub({
      version,
      installer: published,
      manifestPath,
      notes: manifest.notes,
      repo: options.repo,
    });
  }

  if (publishedRepo) {
    const channel = `https://github.com/${publishedRepo}/releases/latest/download`;
    console.log(`  release    https://github.com/${publishedRepo}/releases/tag/v${version}`);
    console.log(`  canal      ${channel}`);
    if (publishedRepo.toLowerCase() !== DEFAULT_REPO.toLowerCase()) {
      console.log(
        `\nAviso: el canal por defecto de la app es ${DEFAULT_REPO}. ` +
          "Indica esta URL en Origen o cambia DEFAULT_CHANNEL en src-tauri/src/updates.rs.",
      );
    }
    console.log("\nLa app consulta ese canal sin configurar nada. Abre LolDraft → Inicio →");
    console.log("Actualizaciones → Buscar actualizaciones.");
  } else {
    console.log("\nCanal solo local: la app consulta GitHub por defecto. Para probar esta");
    console.log("compilación, pon la carpeta de arriba en Origen, o publica con --github.");
  }
}

main().catch((error) => {
  console.error(`\nError: ${error.message}`);
  process.exit(1);
});
