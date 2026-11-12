# 02 — Modelo de datos

> Estado: diseño. El esquema se implementará con migraciones versionadas en Fase 1 y se ampliará en
> fases posteriores. Ningún campeón ni estadística de este documento es un dato real de ejemplo:
> solo se definen estructuras.

## 1. Principios

1. **Identidad estable:** `champion_key` es la clave numérica de Riot (`266` para Aatrox). El nombre
   y el slug pueden cambiar; la clave no.
2. **Separación de capas de dato:**
   - *Identidad y assets*: Data Dragon (regenerable).
   - *Perfil estratégico*: curado o inferido, versionado en `profile_sets`.
   - *Estadísticas observadas*: importadas por dataset y parche, inmutables.
   - *Drafts profesionales*: hechos históricos importados, con su dataset.
3. **Procedencia y confianza obligatorias** en todo dato estratégico o estadístico.
4. **Desconocido ≠ cero:** `intensity` y `value` admiten `NULL`. La ausencia se representa como
   ausencia, no como `0.0`.
5. **Promoción atómica:** una importación escribe en staging, valida y luego publica un
   `dataset_id`. Si falla, no deja datos a medias.
6. **Reproducibilidad:** toda recomendación guardada referencia la versión de perfiles, los
   datasets estadísticos y el hash de los pesos.

## 2. Tipos de dominio (Rust)

```rust
pub struct ChampionKey(pub u16);          // clave de Riot
pub struct PatchId(pub String);           // "16.22"
pub struct ProfileSetId(pub String);      // ULID
pub struct DatasetId(pub String);         // ULID

pub enum Role { Top, Jungle, Mid, Adc, Support }
pub enum Side { Blue, Red }
pub enum ActionKind { Ban, Pick }

pub struct ChampionProfile {
    pub key: ChampionKey,
    pub slug: String,
    pub name: String,
    pub roles: Vec<RoleFit>,
    pub archetypes: BTreeMap<ArchetypeId, ArchetypeRating>,
    pub traits: BTreeMap<TraitId, TraitRating>,
}

pub struct RoleFit {
    pub role: Role,
    pub proficiency: f32,   // 0..1
    pub confidence: f32,    // 0..1
    pub source: Provenance,
}

pub struct ArchetypeRating {
    pub intensity: Option<f32>, // None = desconocido
    pub confidence: f32,
    pub source: Provenance,
}

pub struct TraitRating {
    pub value: Option<f32>,
    pub confidence: f32,
    pub source: Provenance,
}

pub enum Provenance {
    Riot,
    Curated { editor: String, review: String },
    Imported { source: String, dataset: DatasetId },
    Inferred { method: String, version: String },
}
```

`ArchetypeRating` y `TraitRating` usan `Option` deliberadamente. El motor debe poder distinguir
"no aporta engage" de "no sabemos si aporta engage".

## 3. Taxonomía estratégica

### 3.1 Arquetipos (20 dimensiones)

Escala de intensidad:

| Valor | Significado | Ancla |
| --- | --- | --- |
| 0.00 | No aporta | No tiene la herramienta o es irrelevante |
| 0.25 | Incidental | La aporta de forma secundaria o condicionada |
| 0.50 | Sólida | La aporta de forma fiable en la mayoría de partidas |
| 0.75 | Fuerte | Es una herramienta central de su kit |
| 1.00 | Definitoria | La identidad del campeón gira alrededor de ello |

| ID | Categoría | Definición operativa |
| --- | --- | --- |
| `engage` | inicio | Inicia peleas 5v5 de forma fiable y comprometida |
| `disengage` | defensa | Detiene o evita iniciaciones enemigas |
| `frontline` | defensa | Ocupa espacio y absorbe presión delante del equipo |
| `tank` | defensa | Resistencia sostenida y capacidad de aguantar foco |
| `bruiser` | daño | Combina daño y resistencia en primera línea |
| `assassin` | daño | Amenaza letal sobre objetivos prioritarios aislados |
| `poke` | daño | Desgasta a distancia antes de la pelea |
| `burst` | daño | Concentra mucho daño en una ventana corta |
| `dps` | daño | Daño sostenido y repetible en peleas largas |
| `scaling` | tempo | Mejora de forma notable con oro/niveles/items |
| `early_game` | tempo | Fuerte antes de la primera gran pelea |
| `peel` | defensa | Protege a los carries con escudos, curas o control |
| `utility` | utilidad | Aporta herramientas no dañinas relevantes |
| `catch` | inicio | Castiga objetivos aislados sin comprometer una pelea 5v5 |
| `splitpush` | mapa | Presión lateral y duelos fuera del grupo |
| `wombo_combo` | inicio | Combina habilidades de área con aliados |
| `zone_control` | utilidad | Niega espacio o accesos con amenazas persistentes |
| `sustain` | defensa | Recupera vida o mantiene al equipo en el mapa |
| `anti_tank` | daño | Rompe tanques: daño porcentual, verdadero o penetración |
| `siege` | mapa | Presión a torres y objetivos a distancia |

Distinciones que el motor debe respetar:

- `engage` compromete al equipo; `catch` busca una pieza aislada; `pick` (fortaleza) combina
  `catch` + `burst` + `zone_control`.
- `tank` describe resistencia; `frontline` describe ocupación de espacio. Un tanque que no
  frontlinea no cubre la carencia de frontline.
- `scaling` describe la curva, no la fuerza actual. Un campeón puede escalar y ser débil en el
  parche actual.
- `early_game` y `scaling` pueden coexistir en intensidades medias; no son opuestos binarios.

### 3.2 Traits (descriptores ortogonales)

Los arquetipos describen *qué aporta*; los traits describen *cómo es* y alimentan matchups,
balance de daño y necesidades.

| Trait | Tipo | Uso |
| --- | --- | --- |
| `damage_physical` / `damage_magic` / `damage_true` | share 0..1 (suman 1) | Balance de daño |
| `range` | 0..1 (melee→ranged) | Vulnerabilidad a poke/engage |
| `mobility` | 0..1 | Disengage, dive, kiting |
| `hard_cc` | 0..1 | Iniciación y peel fiables |
| `soft_cc` | 0..1 | Zoning, ralentizaciones |
| `waveclear` | 0..1 | Defensa de torre y presión |
| `objective_dps` | 0..1 | Velocidad de objetivos neutrales |
| `durability` | 0..1 | Resistencia efectiva |
| `lane_safety` | 0..1 | Seguridad a ciegas / blind safety |
| `early_power` / `mid_power` / `late_power` | 0..1 | Curva de poder |
| `aoe` | 0..1 | Potencial de wombo y teamfight |
| `single_target` | 0..1 | Pick y amenaza a carries |
| `gold_dependency` | 0..1 | Riesgo de reparto de recursos |
| `execution_difficulty` | 0..1 | Riesgo de ejecución |

Los shares de daño se validan al importar: si la suma no está en `[0.99, 1.01]` se normaliza y se
registra una advertencia.

### 3.3 Sinergias

```rust
pub struct SynergyEdge {
    pub a: ChampionKey,
    pub b: ChampionKey,          // a < b canónico
    pub score: f32,              // -1..1 (negativo = anti-sinergia)
    pub tags: Vec<SynergyTag>,   // EngageFollowup, Wombo, PeelCarry...
    pub note: Option<String>,
    pub source: Provenance,
    pub confidence: f32,
}
```

Se combinan dos fuentes:

1. **Curadas:** relación concreta y explicable ("muro + carry inmóvil", "engage + burst de área").
2. **Derivadas:** reglas de complementariedad entre traits/arquetipos (por ejemplo, `aoe` alto +
   `wombo_combo` alto + `burst` alto). Se marcan como `Inferred` y con menor confianza.

Una sinergia no se inventa a partir de winrate de dúo sin muestra suficiente. Si solo existe la
estadística, se usa como evidencia débil y se muestra su `n`.

## 4. Esquema SQLite

Se usa SQLite embebido (bundled), `STRICT` cuando esté disponible, claves foráneas activas y WAL.
Las migraciones son incrementales y solo hacia delante.

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE schema_migrations (
  version     INTEGER PRIMARY KEY,
  applied_at  INTEGER NOT NULL
) STRICT;

-- ---------------------------------------------------------------- datasets
CREATE TABLE datasets (
  id            TEXT PRIMARY KEY,             -- ULID
  kind          TEXT NOT NULL,                -- ddragon | pro_drafts | soloq_stats | profiles | manual
  source        TEXT NOT NULL,                -- ddragon | leaguepedia | lolalytics | csv | curated
  patch         TEXT,
  fetched_at    INTEGER NOT NULL,             -- epoch ms
  checksum      TEXT NOT NULL,
  license       TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at    INTEGER NOT NULL
) STRICT;

CREATE TABLE import_runs (
  id           TEXT PRIMARY KEY,
  dataset_id   TEXT REFERENCES datasets(id),
  source       TEXT NOT NULL,
  started_at   INTEGER NOT NULL,
  finished_at  INTEGER,
  status       TEXT NOT NULL,                 -- running | success | failed | partial
  stats_json   TEXT NOT NULL DEFAULT '{}',
  error        TEXT
) STRICT;

-- ---------------------------------------------------------------- campeones
CREATE TABLE champions (
  riot_key          INTEGER PRIMARY KEY,
  slug              TEXT NOT NULL UNIQUE,
  name              TEXT NOT NULL,
  title             TEXT,
  image_path        TEXT,
  resource          TEXT,
  attack_range      INTEGER,
  riot_tags_json    TEXT NOT NULL DEFAULT '[]',
  ddragon_version   TEXT,
  updated_at        INTEGER NOT NULL
) STRICT;

CREATE TABLE roles (
  id TEXT PRIMARY KEY CHECK (id IN ('top','jungle','mid','adc','support'))
) STRICT;

CREATE TABLE champion_roles (
  champion_key  INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
  role_id       TEXT NOT NULL REFERENCES roles(id),
  proficiency   REAL NOT NULL CHECK (proficiency BETWEEN 0 AND 1),
  confidence    REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  source        TEXT NOT NULL,                -- riot | curated | inferred | imported
  PRIMARY KEY (champion_key, role_id)
) STRICT;

-- ---------------------------------------------------------------- perfiles
CREATE TABLE profile_sets (
  id          TEXT PRIMARY KEY,               -- ULID
  version     TEXT NOT NULL,                  -- "2026.11.1"
  patch       TEXT,
  notes       TEXT,
  is_active   INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0,1)),
  created_at  INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX ux_profile_sets_active
  ON profile_sets(is_active) WHERE is_active = 1;

CREATE TABLE archetypes (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  category    TEXT NOT NULL,
  description TEXT NOT NULL,
  rubric_json TEXT NOT NULL DEFAULT '{}'
) STRICT;

CREATE TABLE champion_archetypes (
  profile_set_id  TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
  champion_key    INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
  archetype_id    TEXT NOT NULL REFERENCES archetypes(id),
  intensity       REAL CHECK (intensity BETWEEN 0 AND 1),  -- NULL = desconocido
  confidence      REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  source          TEXT NOT NULL,
  notes           TEXT,
  updated_at      INTEGER NOT NULL,
  PRIMARY KEY (profile_set_id, champion_key, archetype_id)
) STRICT;

CREATE TABLE traits (
  id          TEXT PRIMARY KEY,
  kind        TEXT NOT NULL,                  -- scalar | share
  description TEXT NOT NULL
) STRICT;

CREATE TABLE champion_traits (
  profile_set_id  TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
  champion_key    INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
  trait_id        TEXT NOT NULL REFERENCES traits(id),
  value           REAL,                       -- NULL = desconocido
  confidence      REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  source          TEXT NOT NULL,
  PRIMARY KEY (profile_set_id, champion_key, trait_id)
) STRICT;

CREATE TABLE synergies (
  profile_set_id  TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
  champion_a      INTEGER NOT NULL REFERENCES champions(riot_key),
  champion_b      INTEGER NOT NULL REFERENCES champions(riot_key),
  score           REAL NOT NULL CHECK (score BETWEEN -1 AND 1),
  tags_json       TEXT NOT NULL DEFAULT '[]',
  note            TEXT,
  source          TEXT NOT NULL,
  confidence      REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  CHECK (champion_a < champion_b),
  PRIMARY KEY (profile_set_id, champion_a, champion_b)
) STRICT;

-- ---------------------------------------------------------------- estadísticas
CREATE TABLE champion_stats (
  dataset_id    TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  champion_key  INTEGER NOT NULL REFERENCES champions(riot_key),
  role_id       TEXT NOT NULL REFERENCES roles(id),
  patch         TEXT NOT NULL,
  queue         TEXT NOT NULL CHECK (queue IN ('soloq','pro')),
  tier          TEXT NOT NULL DEFAULT 'all',  -- all | emerald_plus | master_plus...
  region        TEXT NOT NULL DEFAULT 'world',
  winrate       REAL NOT NULL CHECK (winrate BETWEEN 0 AND 1),
  pickrate      REAL,
  banrate       REAL,
  games         INTEGER NOT NULL CHECK (games >= 0),
  fetched_at    INTEGER NOT NULL,
  PRIMARY KEY (dataset_id, champion_key, role_id, patch, queue, tier, region)
) STRICT;

CREATE INDEX ix_champion_stats_lookup
  ON champion_stats(patch, queue, role_id, champion_key);

CREATE TABLE matchup_stats (
  dataset_id    TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  champion_key  INTEGER NOT NULL REFERENCES champions(riot_key),
  opponent_key  INTEGER NOT NULL REFERENCES champions(riot_key),
  role_id       TEXT NOT NULL REFERENCES roles(id),
  patch         TEXT NOT NULL,
  queue         TEXT NOT NULL CHECK (queue IN ('soloq','pro')),
  tier          TEXT NOT NULL DEFAULT 'all',
  region        TEXT NOT NULL DEFAULT 'world',
  winrate       REAL NOT NULL CHECK (winrate BETWEEN 0 AND 1),
  games         INTEGER NOT NULL CHECK (games >= 0),
  fetched_at    INTEGER NOT NULL,
  CHECK (champion_key <> opponent_key),
  PRIMARY KEY (dataset_id, champion_key, opponent_key, role_id, patch, queue, tier, region)
) STRICT;

CREATE INDEX ix_matchup_lookup
  ON matchup_stats(patch, queue, role_id, champion_key, opponent_key);

-- ---------------------------------------------------------------- esports
CREATE TABLE tournaments (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  league        TEXT,
  year          INTEGER,
  split         TEXT,
  dataset_id    TEXT NOT NULL REFERENCES datasets(id)
) STRICT;

CREATE TABLE teams (
  id       TEXT PRIMARY KEY,
  name     TEXT NOT NULL,
  region   TEXT
) STRICT;

CREATE TABLE pro_games (
  id              TEXT PRIMARY KEY,
  dataset_id      TEXT NOT NULL REFERENCES datasets(id),
  source_game_id  TEXT NOT NULL,
  tournament_id   TEXT REFERENCES tournaments(id),
  patch           TEXT,
  played_at       INTEGER,
  blue_team_id    TEXT REFERENCES teams(id),
  red_team_id     TEXT REFERENCES teams(id),
  winner_side     TEXT CHECK (winner_side IN ('blue','red')),
  UNIQUE (dataset_id, source_game_id)
) STRICT;

CREATE TABLE pro_draft_actions (
  id            INTEGER PRIMARY KEY,
  game_id       TEXT NOT NULL REFERENCES pro_games(id) ON DELETE CASCADE,
  seq           INTEGER,                      -- NULL si la fuente no da orden
  side          TEXT NOT NULL CHECK (side IN ('blue','red')),
  action        TEXT NOT NULL CHECK (action IN ('ban','pick')),
  champion_key  INTEGER NOT NULL REFERENCES champions(riot_key),
  role_id       TEXT REFERENCES roles(id),    -- solo en picks, si se conoce
  UNIQUE (game_id, seq)                       -- SQLite permite varios NULL
) STRICT;

CREATE INDEX ix_pro_actions_champion ON pro_draft_actions(champion_key);
CREATE INDEX ix_pro_actions_game ON pro_draft_actions(game_id, seq);

-- ---------------------------------------------------------------- aplicación
CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value_json  TEXT NOT NULL,
  updated_at  INTEGER NOT NULL
) STRICT;

CREATE TABLE draft_sessions (
  id              TEXT PRIMARY KEY,           -- ULID
  format_id       TEXT NOT NULL,
  patch           TEXT,
  state_json      TEXT NOT NULL,
  schema_version  INTEGER NOT NULL,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL
) STRICT;

CREATE TABLE recommendation_runs (
  id               TEXT PRIMARY KEY,
  session_id       TEXT REFERENCES draft_sessions(id) ON DELETE SET NULL,
  created_at       INTEGER NOT NULL,
  engine_version   TEXT NOT NULL,
  profile_set_id   TEXT REFERENCES profile_sets(id),
  stat_dataset_id  TEXT REFERENCES datasets(id),
  weights_json     TEXT NOT NULL,
  input_hash       TEXT NOT NULL,
  output_json      TEXT NOT NULL
) STRICT;
```

### 4.1 Índices y consultas críticas

- Último dataset por `kind/patch/source`: índice implícito en `datasets`, añadir
  `(kind, patch, fetched_at DESC)` si la consulta lo pide.
- Perfil activo: índice parcial único `is_active = 1`.
- Candidatos de un draft: se resuelven en memoria desde el catálogo; no se consulta por campeón.
- Estadísticas de un campeón/rol/parche: índice compuesto ya definido.
- Matchup: índice `(patch, queue, role_id, champion_key, opponent_key)`.

### 4.2 Vistas de lectura

Se pueden añadir vistas para simplificar la UI, por ejemplo la última estadística por campeón y
parche:

```sql
CREATE VIEW latest_champion_stats AS
SELECT cs.*
FROM champion_stats cs
JOIN datasets d ON d.id = cs.dataset_id
WHERE d.fetched_at = (
  SELECT MAX(d2.fetched_at)
  FROM datasets d2
  JOIN champion_stats cs2 ON cs2.dataset_id = d2.id
  WHERE cs2.patch = cs.patch
    AND cs2.queue = cs.queue
    AND cs2.role_id = cs.role_id
    AND cs2.champion_key = cs.champion_key
);
```

Si el rendimiento de la vista se resiente, se materializa en una tabla `stats_current` durante la
promoción del dataset. No se hará antes de medirlo.

## 5. Importación y staging

Para evitar mezclar datos a medio importar:

1. Se crea un `dataset` en estado *pendiente* con checksum.
2. Se insertan filas en tablas staging (`stg_*`) o en el dataset nuevo con `dataset_id`.
3. Se validan invariantes: claves existentes, rangos, duplicados, parche, tamaño de muestra.
4. En una transacción se marca el dataset como válido y se actualiza el puntero de "dataset
   activo" por tipo/parche en `settings`.
5. Si algo falla: rollback y `import_runs.status = failed` con el detalle.

Los perfiles curados no usan staging: se cargan como un `profile_set` completo y se activan
atómicamente. Un perfil incompleto es válido si las ausencias están marcadas como desconocidas.

## 6. Versionado

| Entidad | Versión | Motivo |
| --- | --- | --- |
| Taxonomía | `taxonomy_version` en settings | Cambiar definiciones invalida comparaciones |
| Perfiles | `profile_set_id` + `version` | Reproducir recomendaciones antiguas |
| Estadísticas | `dataset_id` + parche | Resultados por parche y fuente |
| Drafts pro | `dataset_id` | Reeimportaciones idempotentes |
| Motor | `engine_version` + hash de pesos | Explicar cambios de ranking |
| Esquema | `schema_migrations.version` | Migraciones controladas |

## 7. Retención y tamaño

- Texto estructurado, sin blobs de imágenes en SQLite: las imágenes se cachean en disco.
- Un dataset de estadísticas de ~173 campeones × 5 roles × 3 tiers × 2 colas es pequeño (decenas de
  miles de filas). Los drafts profesionales son la tabla que más crece: decenas de miles de filas
  por temporada, también manejable.
- Se permite borrar datasets antiguos desde Configuración, salvo el activo y los referenciados por
  `recommendation_runs` que el usuario quiera conservar.
- `VACUUM` manual o al borrar datasets, nunca en cada arranque.

## 8. Datos que NO se guardan

- No se guardan perfiles inventados de relleno para completar los 173 campeones.
- No se guarda una única cifra de winrate sin muestra, fuente y parche.
- No se guarda el HTML crudo de Lolalytics como si fuera un dato estructurado.
- No se guardan credenciales en SQLite.
