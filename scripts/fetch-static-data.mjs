// Genera la semilla estática de campeones, descarga sus retratos y sincroniza
// los nombres e iconos de objetos y runas del catálogo de builds.
// Uso: node scripts/fetch-static-data.mjs
//
// Fuentes:
//  - Data Dragon (Riot): identidad, tags de clase, retratos, objetos (es_ES),
//    runas (es_ES) e iconos.
//  - Meraki Analytics (MIT, atribución solicitada): posiciones jugables.
// No se generan estadísticas ni perfiles estratégicos.

import { mkdir, readFile, writeFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";

const ROOT = process.cwd();
const SEED_PATH = join(ROOT, "crates", "loldraft-data", "seeds", "champions.json");
const IMAGE_DIR = join(ROOT, "apps", "desktop", "public", "champions");
const ITEM_CATALOG_PATH = join(
  ROOT,
  "crates",
  "loldraft-engine",
  "data",
  "items-v1.json",
);
const ITEM_IMAGE_DIR = join(ROOT, "apps", "desktop", "public", "items");
const RUNE_IMAGE_DIR = join(ROOT, "apps", "desktop", "public", "runes");

const POSITION_MAP = {
  TOP: "top",
  JUNGLE: "jungle",
  MIDDLE: "mid",
  BOTTOM: "adc",
  SUPPORT: "support",
};

async function getJson(url) {
  const res = await fetch(url, { headers: { "user-agent": "LolDraft/0.1 static-data seed" } });
  if (!res.ok) throw new Error(`${url} -> HTTP ${res.status}`);
  return res.json();
}

const versions = await getJson("https://ddragon.leagueoflegends.com/api/versions.json");
const version = versions.find((v) => /^\d+\.\d+\.\d+$/.test(v)) ?? versions[0];
const ddragon = await getJson(
  `https://ddragon.leagueoflegends.com/cdn/${version}/data/en_US/champion.json`,
);
const meraki = await getJson(
  "https://cdn.merakianalytics.com/riot/lol/resources/latest/en-US/champions.json",
);

const merakiById = new Map(Object.values(meraki).map((c) => [c.id, c]));
const champions = Object.values(ddragon.data)
  .map((c) => {
    const played = merakiById.get(Number(c.key));
    const positions = (played?.positions ?? []).filter((p) => POSITION_MAP[p]);
    const roles = positions.map((position, index) => ({
      role: POSITION_MAP[position],
      proficiency: index === 0 ? 1.0 : index === 1 ? 0.75 : 0.6,
      confidence: 0.8,
      source: "meraki",
    }));

    return {
      riot_key: Number(c.key),
      slug: c.id,
      name: c.name,
      title: c.title,
      image: c.image.full,
      tags: c.tags ?? [],
      roles,
    };
  })
  .sort((a, b) => a.name.localeCompare(b.name, "en"));

await mkdir(dirname(SEED_PATH), { recursive: true });
await mkdir(IMAGE_DIR, { recursive: true });

const dataset = {
  dataset: {
    kind: "champion_identity",
    source: "ddragon+meraki",
    ddragon_version: version,
    ddragon_license: "Riot Games. Data Dragon assets, ver aviso legal.",
    roles_source: "Meraki Analytics (MIT); atribución solicitada.",
    roles_confidence: 0.8,
    fetched_at: new Date().toISOString(),
    note: "Los roles provienen de posiciones publicadas; los campeones sin posición quedan sin rol.",
  },
  champions,
};

await writeFile(SEED_PATH, JSON.stringify(dataset, null, 2) + "\n", "utf8");
console.log(`Seed: ${champions.length} campeones -> ${SEED_PATH}`);

async function download(slug, file) {
  const dest = join(IMAGE_DIR, `${slug}.png`);
  try {
    const s = await stat(dest);
    if (s.size > 0) return "cached";
  } catch {}
  const url = `https://ddragon.leagueoflegends.com/cdn/${version}/img/champion/${file}`;
  const res = await fetch(url, { headers: { "user-agent": "LolDraft/0.1 static-data seed" } });
  if (!res.ok) return `missing ${res.status}`;
  const buf = Buffer.from(await res.arrayBuffer());
  await writeFile(dest, buf);
  return "ok";
}

const queue = [...champions];
const results = { ok: 0, cached: 0, failed: 0 };
async function worker() {
  while (queue.length) {
    const c = queue.shift();
    const result = await download(c.slug, c.image);
    if (result === "ok") results.ok++;
    else if (result === "cached") results.cached++;
    else {
      results.failed++;
      console.warn(`${c.slug}: ${result}`);
    }
  }
}

await Promise.all(Array.from({ length: 8 }, worker));
console.log(`Retratos: ${JSON.stringify(results)} -> ${IMAGE_DIR}`);

async function downloadAsset(url, dest) {
  try {
    const s = await stat(dest);
    if (s.size > 0) return "cached";
  } catch {}
  const res = await fetch(url, { headers: { "user-agent": "LolDraft/0.1 static-data seed" } });
  if (!res.ok) return `missing ${res.status}`;
  await writeFile(dest, Buffer.from(await res.arrayBuffer()));
  return "ok";
}

async function downloadQueue(queue, label) {
  const tally = { ok: 0, cached: 0, failed: 0 };
  async function run() {
    while (queue.length) {
      const job = queue.shift();
      const result = await downloadAsset(job.url, job.dest);
      if (result === "ok") tally.ok++;
      else if (result === "cached") tally.cached++;
      else {
        tally.failed++;
        console.warn(`${label} ${job.name}: ${result}`);
      }
    }
  }
  await Promise.all(Array.from({ length: 8 }, run));
  console.log(`${label}: ${JSON.stringify(tally)}`);
}

// Nombres oficiales en español e iconos locales de objetos y runas.
const catalog = JSON.parse(await readFile(ITEM_CATALOG_PATH, "utf8"));
const esItems = await getJson(
  `https://ddragon.leagueoflegends.com/cdn/${version}/data/es_ES/item.json`,
);
let renamed = 0;
for (const item of catalog.items) {
  const official = esItems.data?.[String(item.icon)];
  if (official?.name && official.name !== item.name) {
    item.name = official.name;
    renamed++;
  }
}

const runesReforged = await getJson(
  `https://ddragon.leagueoflegends.com/cdn/${version}/data/es_ES/runesReforged.json`,
);
const keystoneIcons = new Map();
const keystoneNames = new Map();
for (const path of runesReforged) {
  for (const slot of path.slots) {
    for (const rune of slot.runes) {
      keystoneIcons.set(rune.id, rune.icon);
      keystoneNames.set(rune.id, rune.name);
    }
  }
}
let runesRenamed = 0;
for (const page of catalog.rune_pages) {
  const official = keystoneNames.get(page.keystone_id);
  if (official && official !== page.keystone) {
    page.keystone = official;
    runesRenamed++;
  }
}

await mkdir(ITEM_IMAGE_DIR, { recursive: true });
await mkdir(RUNE_IMAGE_DIR, { recursive: true });

await downloadQueue(
  catalog.items.map((item) => ({
    name: item.name,
    url: `https://ddragon.leagueoflegends.com/cdn/${version}/img/item/${item.icon}.png`,
    dest: join(ITEM_IMAGE_DIR, `${item.icon}.png`),
  })),
  "Iconos de objetos",
);

await downloadQueue(
  catalog.rune_pages
    .map((page) => {
      const icon = keystoneIcons.get(page.keystone_id);
      return icon
        ? {
            name: page.keystone,
            url: `https://ddragon.leagueoflegends.com/cdn/img/${icon}`,
            dest: join(RUNE_IMAGE_DIR, `${page.keystone_id}.png`),
          }
        : null;
    })
    .filter(Boolean),
  "Iconos de runas",
);

await writeFile(ITEM_CATALOG_PATH, JSON.stringify(catalog, null, 2) + "\n", "utf8");
console.log(
  `Catálogo: ${catalog.items.length} objetos (${renamed} nombres sincronizados), ` +
    `${catalog.rune_pages.length} páginas (${runesRenamed} runas clave) -> ${ITEM_CATALOG_PATH}`,
);
