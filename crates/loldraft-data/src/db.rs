use crate::error::DataError;
use crate::seed::{ChampionIdentity, ChampionSeedFile};
use loldraft_domain::{Champion, ChampionCatalog, ChampionKey, Role, RoleFit, Side};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS champions (
    riot_key    INTEGER PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    image       TEXT NOT NULL DEFAULT '',
    tags_json   TEXT NOT NULL DEFAULT '[]',
    updated_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_champions_name ON champions(name);

CREATE TABLE IF NOT EXISTS champion_roles (
    champion_key INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
    role_id      TEXT NOT NULL CHECK (role_id IN ('top','jungle','mid','adc','support')),
    proficiency  REAL NOT NULL CHECK (proficiency BETWEEN 0 AND 1),
    confidence   REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    source       TEXT NOT NULL,
    PRIMARY KEY (champion_key, role_id)
);

CREATE TABLE IF NOT EXISTS draft_sessions (
    id            TEXT PRIMARY KEY,
    format_id     TEXT NOT NULL,
    side          TEXT NOT NULL CHECK (side IN ('blue','red')),
    state_json    TEXT NOT NULL,
    action_count  INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_draft_sessions_updated ON draft_sessions(updated_at DESC);

CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value_json  TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- Preparado para la Fase 4; todavía no se escribe desde la UI.
CREATE TABLE IF NOT EXISTS datasets (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    source        TEXT NOT NULL,
    patch         TEXT,
    fetched_at    INTEGER NOT NULL,
    checksum      TEXT NOT NULL,
    license       TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL
);
"#;

const MIGRATION_2: &str = r#"
CREATE TABLE IF NOT EXISTS profile_sets (
    id                TEXT PRIMARY KEY,
    version           TEXT NOT NULL,
    taxonomy_version  TEXT NOT NULL,
    patch             TEXT,
    notes             TEXT,
    is_active         INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0,1)),
    created_at        INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_profile_sets_active
    ON profile_sets(is_active) WHERE is_active = 1;

CREATE TABLE IF NOT EXISTS archetypes (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS traits (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('damage_share','scalar')),
    name        TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS profile_set_roles (
    profile_set_id TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
    champion_key   INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
    role_id        TEXT NOT NULL CHECK (role_id IN ('top','jungle','mid','adc','support')),
    proficiency    REAL NOT NULL CHECK (proficiency BETWEEN 0 AND 1),
    confidence     REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    source_json    TEXT NOT NULL,
    PRIMARY KEY (profile_set_id, champion_key, role_id)
);

CREATE TABLE IF NOT EXISTS champion_archetypes (
    profile_set_id TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
    champion_key   INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
    archetype_id   TEXT NOT NULL REFERENCES archetypes(id),
    intensity      REAL CHECK (intensity BETWEEN 0 AND 1),
    confidence     REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    source_json    TEXT NOT NULL,
    updated_at     INTEGER NOT NULL,
    PRIMARY KEY (profile_set_id, champion_key, archetype_id)
);

CREATE TABLE IF NOT EXISTS champion_traits (
    profile_set_id TEXT NOT NULL REFERENCES profile_sets(id) ON DELETE CASCADE,
    champion_key   INTEGER NOT NULL REFERENCES champions(riot_key) ON DELETE CASCADE,
    trait_id       TEXT NOT NULL REFERENCES traits(id),
    value          REAL CHECK (value BETWEEN 0 AND 1),
    confidence     REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    source_json    TEXT NOT NULL,
    updated_at     INTEGER NOT NULL,
    PRIMARY KEY (profile_set_id, champion_key, trait_id)
);

CREATE INDEX IF NOT EXISTS ix_champion_archetypes_set
    ON champion_archetypes(profile_set_id);
CREATE INDEX IF NOT EXISTS ix_champion_traits_set
    ON champion_traits(profile_set_id);
"#;

const MIGRATION_3: &str = r#"
CREATE TABLE IF NOT EXISTS observed_stats (
    dataset_id    TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    champion_key  INTEGER NOT NULL,
    role_id       TEXT NOT NULL CHECK (role_id IN ('top','jungle','mid','adc','support')),
    games         INTEGER NOT NULL CHECK (games > 0),
    wins          INTEGER NOT NULL CHECK (wins >= 0 AND wins <= games),
    pick_rate     REAL CHECK (pick_rate BETWEEN 0 AND 1),
    ban_rate      REAL CHECK (ban_rate BETWEEN 0 AND 1),
    PRIMARY KEY (dataset_id, champion_key, role_id)
);

CREATE INDEX IF NOT EXISTS ix_observed_stats_role
    ON observed_stats(dataset_id, role_id);
"#;

const MIGRATION_4: &str = r#"
CREATE TABLE IF NOT EXISTS champion_builds (
    dataset_id        TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    position          INTEGER NOT NULL,
    champion_key      INTEGER NOT NULL,
    role_id           TEXT NOT NULL CHECK (role_id IN ('top','jungle','mid','adc','support')),
    name              TEXT NOT NULL,
    starter_items     TEXT NOT NULL,
    core_items        TEXT NOT NULL,
    situational_items TEXT NOT NULL,
    runes_json        TEXT NOT NULL,
    games             INTEGER NOT NULL CHECK (games >= 0),
    wins              INTEGER NOT NULL CHECK (wins >= 0 AND wins <= games),
    PRIMARY KEY (dataset_id, position)
);

CREATE INDEX IF NOT EXISTS ix_champion_builds_champion
    ON champion_builds(dataset_id, champion_key, role_id);
"#;

const MIGRATIONS: &[&str] = &[MIGRATION_1, MIGRATION_2, MIGRATION_3, MIGRATION_4];

pub struct Database {
    pub(crate) conn: Connection,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub format_id: String,
    pub side: Side,
    pub state_json: String,
    pub action_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub format_id: String,
    pub side: Side,
    pub action_count: usize,
    pub updated_at: i64,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DataError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                DataError::Invalid(format!("No se pudo crear el directorio de datos: {error}"))
            })?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, DataError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, DataError> {
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        let mut database = Self { conn };
        database.migrate()?;
        database.sync_taxonomy()?;
        Ok(database)
    }

    fn migrate(&mut self) -> Result<(), DataError> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if current as usize >= MIGRATIONS.len() {
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
            tx.execute_batch(sql)?;
            let version = (index + 1) as i64;
            tx.execute(
                "INSERT OR REPLACE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![version, now_ms()],
            )?;
            tx.pragma_update(None, "user_version", version)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn seed_champions(&mut self, seed: &ChampionSeedFile) -> Result<usize, DataError> {
        let tx = self.conn.transaction()?;
        let now = now_ms();
        let mut count = 0usize;

        for champion in &seed.champions {
            let tags_json = serde_json::to_string(&champion.tags)?;
            tx.execute(
                r#"
                INSERT INTO champions (riot_key, slug, name, title, image, tags_json, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(riot_key) DO UPDATE SET
                    slug = excluded.slug,
                    name = excluded.name,
                    title = excluded.title,
                    image = excluded.image,
                    tags_json = excluded.tags_json,
                    updated_at = excluded.updated_at
                "#,
                params![
                    champion.riot_key,
                    champion.slug,
                    champion.name,
                    champion.title,
                    champion.image,
                    tags_json,
                    now
                ],
            )?;

            tx.execute(
                "DELETE FROM champion_roles WHERE champion_key = ?1",
                params![champion.riot_key],
            )?;

            for fit in &champion.roles {
                tx.execute(
                    r#"
                    INSERT OR REPLACE INTO champion_roles
                        (champion_key, role_id, proficiency, confidence, source)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    "#,
                    params![
                        champion.riot_key,
                        fit.role.as_str(),
                        fit.proficiency,
                        fit.confidence,
                        fit.source
                    ],
                )?;
            }

            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Actualiza solo la identidad de los campeones. No borra sus roles ni
    /// perfiles: una sincronización de Data Dragon no debe destruir datos
    /// estratégicos ya curados.
    pub fn upsert_identities(
        &mut self,
        identities: &[ChampionIdentity],
    ) -> Result<usize, DataError> {
        let tx = self.conn.transaction()?;
        let now = now_ms();
        let mut count = 0usize;

        for identity in identities {
            let tags_json = serde_json::to_string(&identity.tags)?;
            tx.execute(
                r#"
                INSERT INTO champions (riot_key, slug, name, title, image, tags_json, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(riot_key) DO UPDATE SET
                    slug = excluded.slug,
                    name = excluded.name,
                    title = excluded.title,
                    image = excluded.image,
                    tags_json = excluded.tags_json,
                    updated_at = excluded.updated_at
                "#,
                params![
                    identity.riot_key,
                    identity.slug,
                    identity.name,
                    identity.title,
                    identity.image,
                    tags_json,
                    now
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    pub fn load_catalog(&self) -> Result<ChampionCatalog, DataError> {
        let mut roles_by_champion: HashMap<u16, Vec<RoleFit>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT champion_key, role_id, proficiency, confidence, source FROM champion_roles",
            )?;
            let rows = stmt.query_map([], |row| {
                let key: u16 = row.get(0)?;
                let role_id: String = row.get(1)?;
                Ok((
                    key,
                    role_id,
                    row.get::<_, f32>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            for row in rows {
                let (key, role_id, proficiency, confidence, source) = row?;
                let Some(role) = Role::from_code(&role_id) else {
                    continue;
                };
                roles_by_champion.entry(key).or_default().push(RoleFit {
                    role,
                    proficiency,
                    confidence,
                    source,
                });
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT riot_key, slug, name, title, image, tags_json FROM champions ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, u16>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut champions = Vec::new();
        for row in rows {
            let (key, slug, name, title, image, tags_json) = row?;
            let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
            let mut roles = roles_by_champion.remove(&key).unwrap_or_default();
            roles.sort_by_key(|fit| std::cmp::Reverse((fit.proficiency * 1000.0) as i32));
            champions.push(Champion {
                key: ChampionKey(key),
                slug,
                name,
                title,
                image,
                tags,
                roles,
            });
        }

        Ok(ChampionCatalog::new(champions))
    }

    pub fn champion_count(&self) -> Result<usize, DataError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM champions", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn save_session(
        &self,
        id: &str,
        format_id: &str,
        side: Side,
        state_json: &str,
        action_count: usize,
    ) -> Result<(), DataError> {
        let now = now_ms();
        self.conn.execute(
            r#"
            INSERT INTO draft_sessions
                (id, format_id, side, state_json, action_count, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(id) DO UPDATE SET
                format_id = excluded.format_id,
                side = excluded.side,
                state_json = excluded.state_json,
                action_count = excluded.action_count,
                updated_at = excluded.updated_at
            "#,
            params![
                id,
                format_id,
                side.as_str(),
                state_json,
                action_count as i64,
                now
            ],
        )?;
        Ok(())
    }

    pub fn load_session(&self, id: &str) -> Result<Option<SessionRecord>, DataError> {
        let record = self
            .conn
            .query_row(
                r#"
                SELECT id, format_id, side, state_json, action_count, created_at, updated_at
                FROM draft_sessions WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?;

        record
            .map(
                |(id, format_id, side, state_json, action_count, created_at, updated_at)| {
                    let side = parse_side(&side)?;
                    Ok(SessionRecord {
                        id,
                        format_id,
                        side,
                        state_json,
                        action_count: action_count as usize,
                        created_at,
                        updated_at,
                    })
                },
            )
            .transpose()
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, format_id, side, action_count, updated_at
            FROM draft_sessions
            ORDER BY updated_at DESC
            LIMIT 20
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let (id, format_id, side, action_count, updated_at) = row?;
            sessions.push(SessionSummary {
                id,
                format_id,
                side: parse_side(&side)?,
                action_count: action_count as usize,
                updated_at,
            });
        }
        Ok(sessions)
    }

    pub fn delete_session(&self, id: &str) -> Result<bool, DataError> {
        let changed = self
            .conn
            .execute("DELETE FROM draft_sessions WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, DataError> {
        let value = self
            .conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value_json: &str) -> Result<(), DataError> {
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value_json, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at
            "#,
            params![key, value_json, now_ms()],
        )?;
        Ok(())
    }
}

fn parse_side(value: &str) -> Result<Side, DataError> {
    match value {
        "blue" => Ok(Side::Blue),
        "red" => Ok(Side::Red),
        other => Err(DataError::Invalid(format!(
            "Lado inválido en la base: {other}"
        ))),
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::parse_seed;

    const TEST_SEED: &str = r#"{
      "dataset": {"kind": "test"},
      "champions": [
        {
          "riot_key": 1,
          "slug": "Alpha",
          "name": "Alpha",
          "title": "the First",
          "image": "Alpha.png",
          "tags": ["Tank"],
          "roles": [
            {"role":"top","proficiency":1.0,"confidence":0.8,"source":"test"},
            {"role":"support","proficiency":0.6,"confidence":0.8,"source":"test"}
          ]
        },
        {
          "riot_key": 2,
          "slug": "Beta",
          "name": "Beta",
          "title": "the Second",
          "image": "Beta.png",
          "tags": ["Mage"],
          "roles": [{"role":"mid","proficiency":1.0,"confidence":0.8,"source":"test"}]
        }
      ]
    }"#;

    #[test]
    fn seeds_and_loads_catalog() {
        let mut db = Database::open_in_memory().unwrap();
        let seed = parse_seed(TEST_SEED).unwrap();
        assert_eq!(db.seed_champions(&seed).unwrap(), 2);

        let catalog = db.load_catalog().unwrap();
        assert_eq!(catalog.len(), 2);
        let alpha = catalog.get(ChampionKey(1)).unwrap();
        assert_eq!(alpha.name, "Alpha");
        assert_eq!(alpha.roles.len(), 2);
        assert_eq!(alpha.primary_role(), Some(Role::Top));
        assert_eq!(db.champion_count().unwrap(), 2);
    }

    #[test]
    fn resending_seed_is_idempotent() {
        let mut db = Database::open_in_memory().unwrap();
        let seed = parse_seed(TEST_SEED).unwrap();
        db.seed_champions(&seed).unwrap();
        db.seed_champions(&seed).unwrap();
        assert_eq!(db.champion_count().unwrap(), 2);
        let catalog = db.load_catalog().unwrap();
        assert_eq!(catalog.get(ChampionKey(1)).unwrap().roles.len(), 2);
    }

    #[test]
    fn identity_sync_preserves_roles() {
        let mut db = Database::open_in_memory().unwrap();
        let seed = parse_seed(TEST_SEED).unwrap();
        db.seed_champions(&seed).unwrap();

        db.upsert_identities(&[ChampionIdentity {
            riot_key: 1,
            slug: "Alpha".to_string(),
            name: "Alpha Renamed".to_string(),
            title: "the First".to_string(),
            image: "Alpha.png".to_string(),
            tags: vec!["Tank".to_string()],
        }])
        .unwrap();

        let catalog = db.load_catalog().unwrap();
        let alpha = catalog.get(ChampionKey(1)).unwrap();
        assert_eq!(alpha.name, "Alpha Renamed");
        assert_eq!(alpha.roles.len(), 2);
    }

    #[test]
    fn session_crud_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        db.save_session("s1", "standard", Side::Blue, r#"{"actions":[]}"#, 0)
            .unwrap();
        db.save_session("s1", "standard", Side::Blue, r#"{"actions":[1]}"#, 1)
            .unwrap();

        let loaded = db.load_session("s1").unwrap().unwrap();
        assert_eq!(loaded.action_count, 1);
        assert_eq!(loaded.side, Side::Blue);
        assert_eq!(loaded.state_json, r#"{"actions":[1]}"#);

        let list = db.list_sessions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "s1");

        assert!(db.delete_session("s1").unwrap());
        assert!(db.load_session("s1").unwrap().is_none());
    }

    #[test]
    fn settings_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.get_setting("theme").unwrap().is_none());
        db.set_setting("theme", "\"dark\"").unwrap();
        assert_eq!(
            db.get_setting("theme").unwrap().as_deref(),
            Some("\"dark\"")
        );
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut db = Database::open_in_memory().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();
    }

    #[test]
    fn packaged_seed_imports_all_champions() {
        let mut db = Database::open_in_memory().unwrap();
        let seed = crate::seed::default_seed().unwrap();
        let expected = seed.champions.len();
        db.seed_champions(&seed).unwrap();
        assert_eq!(db.champion_count().unwrap(), expected);
        assert!(expected >= 170);
    }
}
