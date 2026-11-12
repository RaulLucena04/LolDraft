// Genera la semilla estática de campeones y descarga sus retratos.
// Uso: node scripts/fetch-static-data.mjs
//
// Fuentes:
//  - Data Dragon (Riot): identidad, tags de clase y retratos.
//  - Meraki Analytics (MIT, atribución solicitada): posiciones jugables.
// No se generan estadísticas ni perfiles estratégicos.

import { mkdir, writeFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";

const ROOT = process.cwd();
const SEED_PATH = join(ROOT, "crates", "loldraft-data", "seeds", "champions.json");
const IMAGE_DIR = join(ROOT, "apps", "desktop", "public", "champions");

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
