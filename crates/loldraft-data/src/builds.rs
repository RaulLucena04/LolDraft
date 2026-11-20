//! Persistencia de builds y runas observadas.
//!
//! Reutiliza `datasets` para metadatos y procedencia y guarda cada build en
//! `champion_builds` dentro de una transacción. Un dataset se escribe entero:
//! reimportar el mismo id reemplaza sus filas o no deja nada a medias.

use crate::{DataError, Database};
use loldraft_domain::{ChampionKey, Role};
use loldraft_engine::{
    ChampionBuildRow, ObservedBuilds, RunePage, StatsQueue, BUILDS_SCHEMA_VERSION,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Resumen de un dataset de builds, sin cargar todas sus filas.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildDatasetSummary {
    pub id: String,
    pub version: String,
    pub queue: StatsQueue,
    pub patch: String,
    pub source: String,
    pub license: String,
    pub row_count: usize,
    pub created_at: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct BuildsMetadata {
    schema: String,
    version: String,
    queue: StatsQueue,
}

/// Clave del dataset activo por cola. SoloQ y pro nunca comparten puntero.
fn active_key(queue: StatsQueue) -> &'static str {
    match queue {
        StatsQueue::Soloq => "builds.active.soloq",
        StatsQueue::Pro => "builds.active.pro",
    }
}

fn active_id(db: &Database, queue: StatsQueue) -> Result<Option<String>, DataError> {
    let Some(value) = db.get_setting(active_key(queue))? else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&value).ok())
}

fn parse_role(code: &str) -> Result<Role, DataError> {
    Role::from_code(code)
        .ok_or_else(|| DataError::Invalid(format!("Rol desconocido en builds: {code}")))
}

/// FNV-1a de 64 bits: huella de integridad, no firma criptográfica.
fn checksum(builds: &ObservedBuilds) -> Result<String, DataError> {
    let text = serde_json::to_string(builds)?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("{hash:016x}"))
}

impl Database {
    /// Guarda o reemplaza un dataset completo. La validación ocurre antes de
    /// abrir la transacción: un dataset inválido no toca el anterior.
    pub fn save_builds(
        &mut self,
        builds: &ObservedBuilds,
    ) -> Result<BuildDatasetSummary, DataError> {
        builds
            .validate()
            .map_err(|error| DataError::Invalid(format!("Builds inválidas: {error}")))?;

        let now = crate::now_ms();
        let metadata = BuildsMetadata {
            schema: builds.schema.clone(),
            version: builds.version.clone(),
            queue: builds.queue,
        };
        let metadata_json = serde_json::to_string(&metadata)?;
        let checksum = checksum(builds)?;
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO datasets
                (id, kind, source, patch, fetched_at, checksum, license, metadata_json, created_at)
            VALUES (?1, 'observed_builds', ?2, ?3, ?4, ?5, ?6, ?7, ?4)
            ON CONFLICT(id) DO UPDATE SET
                source = excluded.source,
                patch = excluded.patch,
                fetched_at = excluded.fetched_at,
                checksum = excluded.checksum,
                license = excluded.license,
                metadata_json = excluded.metadata_json
            "#,
            params![
                builds.id,
                builds.source,
                builds.patch,
                now,
                checksum,
                builds.license,
                metadata_json
            ],
        )?;
        tx.execute(
            "DELETE FROM champion_builds WHERE dataset_id = ?1",
            params![builds.id],
        )?;
        for (position, row) in builds.rows.iter().enumerate() {
            tx.execute(
                r#"
                INSERT INTO champion_builds
                    (dataset_id, position, champion_key, role_id, name, starter_items,
                     core_items, situational_items, runes_json, games, wins)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    builds.id,
                    i64::try_from(position).unwrap_or(i64::MAX),
                    row.champion.0,
                    row.role.as_str(),
                    row.name,
                    serde_json::to_string(&row.starter_items)?,
                    serde_json::to_string(&row.core_items)?,
                    serde_json::to_string(&row.situational_items)?,
                    serde_json::to_string(&row.runes)?,
                    row.games,
                    row.wins
                ],
            )?;
        }
        tx.commit()?;

        Ok(BuildDatasetSummary {
            id: builds.id.clone(),
            version: builds.version.clone(),
            queue: builds.queue,
            patch: builds.patch.clone(),
            source: builds.source.clone(),
            license: builds.license.clone(),
            row_count: builds.rows.len(),
            created_at: now,
            is_active: active_id(self, builds.queue)?.as_deref() == Some(builds.id.as_str()),
        })
    }

    /// Lista los datasets de builds, del más reciente al más antiguo.
    pub fn list_build_datasets(&self) -> Result<Vec<BuildDatasetSummary>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT d.id, d.source, d.patch, d.license, d.metadata_json, d.created_at,
                   (SELECT COUNT(*) FROM champion_builds b WHERE b.dataset_id = d.id)
            FROM datasets d
            WHERE d.kind = 'observed_builds'
            ORDER BY d.created_at DESC, d.id ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut datasets = Vec::new();
        for row in rows {
            let (id, source, patch, license, metadata_json, created_at, row_count) = row?;
            let metadata: BuildsMetadata = serde_json::from_str(&metadata_json)?;
            let is_active = active_id(self, metadata.queue)?.as_deref() == Some(&id);
            datasets.push(BuildDatasetSummary {
                id,
                version: metadata.version,
                queue: metadata.queue,
                patch: patch.unwrap_or_else(|| "desconocido".to_string()),
                source,
                license: license.unwrap_or_default(),
                row_count: usize::try_from(row_count).unwrap_or(0),
                created_at,
                is_active,
            });
        }
        Ok(datasets)
    }

    /// Carga un dataset por id, con todas sus builds.
    pub fn get_build_dataset(&self, id: &str) -> Result<ObservedBuilds, DataError> {
        let (source, patch, license, metadata_json, kind) = self
            .conn
            .query_row(
                r#"
                SELECT source, patch, license, metadata_json, kind
                FROM datasets
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DataError::NotFound(format!("dataset de builds {id}")))?;
        if kind != "observed_builds" {
            return Err(DataError::Invalid(format!(
                "El dataset {id} no es de builds observadas"
            )));
        }

        let metadata: BuildsMetadata = serde_json::from_str(&metadata_json)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT champion_key, role_id, name, starter_items, core_items,
                   situational_items, runes_json, games, wins
            FROM champion_builds
            WHERE dataset_id = ?1
            ORDER BY position ASC
            "#,
        )?;
        let rows = stmt.query_map(params![id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })?;

        let mut builds = ObservedBuilds {
            schema: metadata.schema,
            id: id.to_string(),
            version: metadata.version,
            source,
            license: license.unwrap_or_default(),
            queue: metadata.queue,
            patch: patch.unwrap_or_else(|| "desconocido".to_string()),
            rows: Vec::new(),
        };
        for row in rows {
            let (champion, role, name, starter, core, situational, runes, games, wins) = row?;
            builds.rows.push(ChampionBuildRow {
                champion: ChampionKey(u16::try_from(champion).map_err(|_| {
                    DataError::Invalid(format!("Clave de campeón fuera de rango: {champion}"))
                })?),
                role: parse_role(&role)?,
                name,
                starter_items: serde_json::from_str(&starter)?,
                core_items: serde_json::from_str(&core)?,
                situational_items: serde_json::from_str(&situational)?,
                runes: serde_json::from_str::<RunePage>(&runes)?,
                games: u32::try_from(games)
                    .map_err(|_| DataError::Invalid(format!("Partidas fuera de rango: {games}")))?,
                wins: u32::try_from(wins)
                    .map_err(|_| DataError::Invalid(format!("Victorias fuera de rango: {wins}")))?,
            });
        }

        if builds.schema != BUILDS_SCHEMA_VERSION {
            return Err(DataError::Invalid(format!(
                "El dataset {id} usa el esquema {} y este código entiende {BUILDS_SCHEMA_VERSION}",
                builds.schema
            )));
        }
        builds
            .validate()
            .map_err(|error| DataError::Invalid(format!("Builds inválidas: {error}")))?;
        Ok(builds)
    }

    /// Marca un dataset como activo **para su cola**.
    pub fn set_active_builds_dataset(&mut self, id: &str) -> Result<(), DataError> {
        let builds = self.get_build_dataset(id)?;
        self.set_setting(active_key(builds.queue), &serde_json::to_string(id)?)
    }

    /// Dataset activo de una cola, si existe y sigue siendo válido.
    pub fn active_builds_dataset(
        &self,
        queue: StatsQueue,
    ) -> Result<Option<ObservedBuilds>, DataError> {
        let Some(value) = self.get_setting(active_key(queue))? else {
            return Ok(None);
        };
        let id: String = serde_json::from_str(&value)?;
        match self.get_build_dataset(&id) {
            Ok(builds) if builds.queue == queue => Ok(Some(builds)),
            // Puntero huérfano o de otra cola: no se inventa un sustituto.
            Ok(_) => Ok(None),
            Err(DataError::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Borra un dataset y su puntero activo. Devuelve `false` si no existía.
    pub fn delete_builds_dataset(&mut self, id: &str) -> Result<bool, DataError> {
        let queue = match self.get_build_dataset(id) {
            Ok(builds) => Some(builds.queue),
            Err(DataError::NotFound(_)) => None,
            Err(error) => return Err(error),
        };
        let Some(queue) = queue else {
            return Ok(false);
        };

        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM datasets WHERE id = ?1", params![id])?;
        tx.execute(
            "DELETE FROM settings WHERE key = ?1 AND value_json = ?2",
            params![active_key(queue), serde_json::to_string(id)?],
        )?;
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rune_page() -> RunePage {
        RunePage {
            primary_path: "Precision".into(),
            keystone: "Conqueror".into(),
            primary_runes: vec!["Triumph".into(), "Legend: Alacrity".into()],
            secondary_path: "Inspiration".into(),
            secondary_runes: vec!["Magical Footwear".into()],
            shards: vec!["Adaptive Force".into(), "Adaptive Force".into()],
        }
    }

    fn builds(id: &str, queue: StatsQueue) -> ObservedBuilds {
        ObservedBuilds {
            schema: BUILDS_SCHEMA_VERSION.to_string(),
            id: id.to_string(),
            version: "1".to_string(),
            source: "Importación manual".to_string(),
            license: "Uso personal".to_string(),
            queue,
            patch: "16.22".to_string(),
            rows: vec![
                ChampionBuildRow {
                    champion: ChampionKey(64),
                    role: Role::Jungle,
                    name: "Conqueror bruiser".to_string(),
                    starter_items: vec!["Doran's Blade".into()],
                    core_items: vec!["Eclipse".into(), "Sundered Sky".into()],
                    situational_items: vec!["Death's Dance".into()],
                    runes: rune_page(),
                    games: 1200,
                    wins: 620,
                },
                ChampionBuildRow {
                    champion: ChampionKey(103),
                    role: Role::Mid,
                    name: "Electrocute burst".to_string(),
                    starter_items: vec!["Doran's Ring".into()],
                    core_items: vec!["Luden's Companion".into()],
                    situational_items: vec![],
                    runes: RunePage {
                        primary_path: "Domination".into(),
                        keystone: "Electrocute".into(),
                        primary_runes: vec!["Sudden Impact".into()],
                        secondary_path: "Sorcery".into(),
                        secondary_runes: vec!["Manaflow Band".into()],
                        shards: vec!["Adaptive Force".into()],
                    },
                    games: 0,
                    wins: 0,
                },
            ],
        }
    }

    #[test]
    fn a_dataset_roundtrips_through_sqlite() {
        let mut db = Database::open_in_memory().unwrap();
        let saved = db
            .save_builds(&builds("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        assert_eq!(saved.row_count, 2);
        assert!(!saved.is_active);

        let loaded = db.get_build_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded, builds("soloq-16.22", StatsQueue::Soloq));
        assert_eq!(loaded.rows[1].games, 0);

        let list = db.list_build_datasets().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].queue, StatsQueue::Soloq);
        assert_eq!(list[0].row_count, 2);
    }

    #[test]
    fn reimporting_the_same_id_replaces_rows_without_duplicates() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_builds(&builds("soloq-16.22", StatsQueue::Soloq))
            .unwrap();

        let mut updated = builds("soloq-16.22", StatsQueue::Soloq);
        updated.version = "2".to_string();
        updated.rows.truncate(1);
        db.save_builds(&updated).unwrap();

        let loaded = db.get_build_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded.version, "2");
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(db.list_build_datasets().unwrap().len(), 1);
    }

    #[test]
    fn an_invalid_dataset_does_not_touch_the_previous_one() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_builds(&builds("soloq-16.22", StatsQueue::Soloq))
            .unwrap();

        let mut invalid = builds("soloq-16.22", StatsQueue::Soloq);
        invalid.rows[0].core_items.clear();
        assert!(db.save_builds(&invalid).is_err());

        let loaded = db.get_build_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded.rows.len(), 2);
        assert_eq!(loaded.rows[0].core_items.len(), 2);
    }

    #[test]
    fn active_builds_are_tracked_per_queue() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_builds(&builds("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        db.save_builds(&builds("pro-2026", StatsQueue::Pro))
            .unwrap();

        db.set_active_builds_dataset("soloq-16.22").unwrap();
        assert_eq!(
            db.active_builds_dataset(StatsQueue::Soloq)
                .unwrap()
                .unwrap()
                .id,
            "soloq-16.22"
        );
        assert!(db.active_builds_dataset(StatsQueue::Pro).unwrap().is_none());

        db.set_active_builds_dataset("pro-2026").unwrap();
        assert_eq!(
            db.active_builds_dataset(StatsQueue::Pro)
                .unwrap()
                .unwrap()
                .id,
            "pro-2026"
        );
        assert_eq!(
            db.active_builds_dataset(StatsQueue::Soloq)
                .unwrap()
                .unwrap()
                .id,
            "soloq-16.22"
        );
    }

    #[test]
    fn deleting_a_dataset_clears_its_active_pointer() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_builds(&builds("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        db.set_active_builds_dataset("soloq-16.22").unwrap();

        assert!(db.delete_builds_dataset("soloq-16.22").unwrap());
        assert!(db
            .active_builds_dataset(StatsQueue::Soloq)
            .unwrap()
            .is_none());
        assert!(db.list_build_datasets().unwrap().is_empty());
        assert!(!db.delete_builds_dataset("soloq-16.22").unwrap());
    }

    #[test]
    fn stats_and_builds_datasets_do_not_mix() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_builds(&builds("compartido", StatsQueue::Soloq))
            .unwrap();
        assert!(
            db.get_stats_dataset("compartido").is_err(),
            "un dataset de builds no puede cargarse como estadísticas"
        );
        assert!(
            db.delete_stats_dataset("compartido").is_err(),
            "borrar con la API equivocada no debe tocar el dataset"
        );
    }
}
