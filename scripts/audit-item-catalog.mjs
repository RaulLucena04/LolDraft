// Audita el catálogo curado de objetos contra los datos del parche en vivo.
// Uso: node scripts/audit-item-catalog.mjs [--strict]
//
// Qué comprueba:
//  1. Fallo: objetos del catálogo que ya no existen o no se pueden comprar y
//     objetos cuyo nombre oficial en español cambió (el catálogo debe corregirse
//     a mano; este script no reescribe datos curados).
//  2. Fallo: iconos locales que faltan en `apps/desktop/public/items`.
//  3. Aviso: objetos legendarios comprables en la Grieta que el catálogo aún
//     no recoge. El catálogo es curado, no exhaustivo: la lista sirve para
//     decidir qué añadir, nunca se importa automáticamente. Los componentes,
//     consumibles, guardianes y objetos de misión quedan fuera del aviso.
//
// Criterio de «legendario comprable en la Grieta»: CommunityDragon `inStore` y
// `displayInItemSets` con `priceTotal >= 2000`, sin campeón/aliado/moneda de
// modo requeridos, más `maps["11"]` de Data Dragon. Así quedan fuera los
// objetos de Arena, las variantes con id de seis cifras, las transformaciones
// (Muramana, Fimbulwinter), los objetos de misión de apoyo y las botas de la
// prueba de fuerza, que no se compran como objetos normales.
//
// `--strict` devuelve código 1 también cuando hay objetos nuevos sin catalogar.

import { readFile, stat } from "node:fs/promises";
import { join } from "node:path";

const STRICT = process.argv.includes("--strict");
const ROOT = process.cwd();
const CATALOG_PATH = join(ROOT, "crates", "loldraft-engine", "data", "items-v1.json");
const ICON_DIR = join(ROOT, "apps", "desktop", "public", "items");
const HEADERS = { "user-agent": "LolDraft/0.1 item catalog audit" };

async function getJson(url) {
  const res = await fetch(url, { headers: HEADERS });
  if (!res.ok) throw new Error(`${url} -> HTTP ${res.status}`);
  return res.json();
}

const versions = await getJson("https://ddragon.leagueoflegends.com/api/versions.json");
const version = versions.find((value) => /^\d+\.\d+\.\d+$/.test(value)) ?? versions[0];
const ddragon = await getJson(
  `https://ddragon.leagueoflegends.com/cdn/${version}/data/es_ES/item.json`,
);
const live = await getJson(
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/items.json",
);

const catalog = JSON.parse(await readFile(CATALOG_PATH, "utf8"));
const liveById = new Map(live.map((item) => [item.id, item]));
const catalogIcons = new Set(catalog.items.map((item) => item.icon));
const problems = [];
const missingIcons = [];
const renamed = [];

for (const item of catalog.items) {
  const official = ddragon.data[String(item.icon)];
  const entry = liveById.get(item.icon);
  if (!official || !entry) {
    problems.push(`${item.id}: el icono ${item.icon} no existe en los datos del parche`);
    continue;
  }
  if (entry.inStore !== true || entry.displayInItemSets !== true || !(entry.priceTotal > 0)) {
    problems.push(`${item.id}: «${official.name}» ya no se compra en la Grieta`);
  }
  if (official.name !== item.name) {
    renamed.push(`${item.id}: «${item.name}» -> «${official.name}»`);
  }
  try {
    await stat(join(ICON_DIR, `${item.icon}.png`));
  } catch {
    missingIcons.push(`${item.id}: falta apps/desktop/public/items/${item.icon}.png`);
  }
}

const isRiftItem = (entry) =>
  entry.inStore === true &&
  entry.displayInItemSets === true &&
  entry.priceTotal >= 2000 &&
  !entry.requiredChampion &&
  !entry.requiredAlly &&
  !entry.requiredBuffCurrencyName &&
  entry.to.length === 0 &&
  entry.id < 100000 &&
  ddragon.data[String(entry.id)]?.maps?.["11"] === true;

const missing = live
  .filter((entry) => isRiftItem(entry) && !catalogIcons.has(entry.id))
  .map((entry) => {
    const name = ddragon.data[String(entry.id)]?.name ?? entry.name;
    return `${String(entry.priceTotal).padStart(5)} ${String(entry.id).padEnd(5)} ${name}`;
  })
  .sort();

console.log(`Catálogo ${catalog.version} contra Data Dragon ${version}`);
console.log(`Objetos catalogados: ${catalog.items.length}`);

if (problems.length) {
  console.log(`\nERROR: ${problems.length} objeto(s) del catálogo ya no son válidos`);
  for (const line of problems) console.log(`  ${line}`);
}
if (renamed.length) {
  console.log(`\nERROR: ${renamed.length} nombre(s) oficiales cambiados`);
  for (const line of renamed) console.log(`  ${line}`);
}
if (missingIcons.length) {
  console.log(`\nERROR: ${missingIcons.length} icono(s) locales ausentes`);
  for (const line of missingIcons) console.log(`  ${line}`);
}
if (missing.length) {
  console.log(`\nAVISO: ${missing.length} objeto(s) comprables en la Grieta sin catalogar`);
  for (const line of missing) console.log(`  ${line}`);
}

const hardFailure = problems.length + renamed.length + missingIcons.length > 0;
if (!hardFailure && (!STRICT || missing.length === 0)) {
  console.log("\nCatálogo coherente con el parche.");
}
process.exit(hardFailure || (STRICT && missing.length > 0) ? 1 : 0);
