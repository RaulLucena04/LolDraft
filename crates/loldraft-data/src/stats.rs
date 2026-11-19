//! Persistencia de estadísticas observadas.
//!
//! Reutiliza la tabla `datasets` (metadatos y procedencia) y guarda las filas
//! en `observed_stats`. Un dataset se escribe entero dentro de una transacción:
//! reimportar el mismo id reemplaza sus filas o no deja nada a medias.

use crate::{DataError, Database};
use loldraft_domain::{ChampionKey, Role};
use loldraft_engine::{ObservedStats, StatsQueue, STATS_SCHEMA_VERSION};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Resumen de un dataset observado, sin cargar todas sus filas.
#[derive(Debug, Clone, PartialEq)]
pub struct StatsDatasetSummary {
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
struct StatsMetadata {
    schema: String,
    version: String,
    queue: StatsQueue,
}

/// Clave de ajuste del dataset activo por cola. SoloQ y pro nunca comparten
/// puntero.
fn active_key(queue: StatsQueue) -> &'static str {
    match queue {
        StatsQueue::Soloq => "stats.active.soloq",
        StatsQueue::Pro => "stats.active.pro",
    }
}

/// Id activo de una cola, deserializado del ajuste. Un valor corrupto se
/// trata como ausente, no como un dataset válido.
fn active_id(db: &Database, queue: StatsQueue) -> Result<Option<String>, DataError> {
    let Some(value) = db.get_setting(active_key(queue))? else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&value).ok())
}

fn role_code(role: Role) -> &'static str {
    role.as_str()
}

fn parse_role(code: &str) -> Result<Role, DataError> {
    Role::from_code(code)
        .ok_or_else(|| DataError::Invalid(format!("Rol desconocido en estadísticas: {code}")))
}

/// FNV-1a de 64 bits. Es una huella de integridad para detectar ficheros
/// cambiados o truncados, no una firma criptográfica.
fn checksum(stats: &ObservedStats) -> Result<String, DataError> {
    let text = serde_json::to_string(stats)?;
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
    pub fn save_observed_stats(
        &mut self,
        stats: &ObservedStats,
    ) -> Result<StatsDatasetSummary, DataError> {
        stats
            .validate()
            .map_err(|error| DataError::Invalid(format!("Estadísticas inválidas: {error}")))?;

        let now = crate::now_ms();
        let metadata = StatsMetadata {
            schema: stats.schema.clone(),
            version: stats.version.clone(),
            queue: stats.queue,
        };
        let metadata_json = serde_json::to_string(&metadata)?;
        let checksum = checksum(stats)?;
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO datasets
                (id, kind, source, patch, fetched_at, checksum, license, metadata_json, created_at)
            VALUES (?1, 'observed_stats', ?2, ?3, ?4, ?5, ?6, ?7, ?4)
            ON CONFLICT(id) DO UPDATE SET
                source = excluded.source,
                patch = excluded.patch,
                fetched_at = excluded.fetched_at,
                checksum = excluded.checksum,
                license = excluded.license,
                metadata_json = excluded.metadata_json
            "#,
            params![
                stats.id,
                stats.source,
                stats.patch,
                now,
                checksum,
                stats.license,
                metadata_json
            ],
        )?;
        tx.execute(
            "DELETE FROM observed_stats WHERE dataset_id = ?1",
            params![stats.id],
        )?;
        for row in &stats.rows {
            tx.execute(
                r#"
                INSERT INTO observed_stats
                    (dataset_id, champion_key, role_id, games, wins, pick_rate, ban_rate)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    stats.id,
                    row.champion.0,
                    role_code(row.role),
                    row.games,
                    row.wins,
                    row.pick_rate,
                    row.ban_rate
                ],
            )?;
        }
        tx.commit()?;

        Ok(StatsDatasetSummary {
            id: stats.id.clone(),
            version: stats.version.clone(),
            queue: stats.queue,
            patch: stats.patch.clone(),
            source: stats.source.clone(),
            license: stats.license.clone(),
            row_count: stats.rows.len(),
            created_at: now,
            is_active: active_id(self, stats.queue)?.as_deref() == Some(stats.id.as_str()),
        })
    }

    /// Lista los datasets observados, del más reciente al más antiguo.
    pub fn list_stats_datasets(&self) -> Result<Vec<StatsDatasetSummary>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT d.id, d.source, d.patch, d.license, d.metadata_json, d.created_at,
                   (SELECT COUNT(*) FROM observed_stats s WHERE s.dataset_id = d.id)
            FROM datasets d
            WHERE d.kind = 'observed_stats'
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
            let metadata: StatsMetadata = serde_json::from_str(&metadata_json)?;
            let is_active = active_id(self, metadata.queue)?.as_deref() == Some(&id);
            datasets.push(StatsDatasetSummary {
                id,
                version: metadata.version,
                queue: metadata.queue,
                // Un dataset sin parche no se pudo guardar; si falta, se dice.
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

    /// Carga un dataset por id, con todas sus filas.
    pub fn get_stats_dataset(&self, id: &str) -> Result<ObservedStats, DataError> {
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
            .ok_or_else(|| DataError::NotFound(format!("dataset de estadísticas {id}")))?;
        if kind != "observed_stats" {
            return Err(DataError::Invalid(format!(
                "El dataset {id} no es de estadísticas observadas"
            )));
        }

        let metadata: StatsMetadata = serde_json::from_str(&metadata_json)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT champion_key, role_id, games, wins, pick_rate, ban_rate
            FROM observed_stats
            WHERE dataset_id = ?1
            ORDER BY role_id ASC, champion_key ASC
            "#,
        )?;
        let rows = stmt.query_map(params![id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
            ))
        })?;

        let mut stats = ObservedStats {
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
            let (champion, role, games, wins, pick_rate, ban_rate) = row?;
            stats.rows.push(loldraft_engine::ChampionStatRow {
                champion: ChampionKey(u16::try_from(champion).map_err(|_| {
                    DataError::Invalid(format!("Clave de campeón fuera de rango: {champion}"))
                })?),
                role: parse_role(&role)?,
                games: u32::try_from(games)
                    .map_err(|_| DataError::Invalid(format!("Partidas fuera de rango: {games}")))?,
                wins: u32::try_from(wins)
                    .map_err(|_| DataError::Invalid(format!("Victorias fuera de rango: {wins}")))?,
                pick_rate: pick_rate.map(|value| value as f32),
                ban_rate: ban_rate.map(|value| value as f32),
            });
        }

        // El esquema almacenado debe ser el que entiende este binario.
        if stats.schema != STATS_SCHEMA_VERSION {
            return Err(DataError::Invalid(format!(
                "El dataset {id} usa el esquema {} y este código entiende {STATS_SCHEMA_VERSION}",
                stats.schema
            )));
        }
        stats
            .validate()
            .map_err(|error| DataError::Invalid(format!("Estadísticas inválidas: {error}")))?;
        Ok(stats)
    }

    /// Marca un dataset como activo **para su cola**. Activar SoloQ no cambia
    /// el dataset profesional ni al revés.
    pub fn set_active_stats_dataset(&mut self, id: &str) -> Result<(), DataError> {
        let stats = self.get_stats_dataset(id)?;
        self.set_setting(active_key(stats.queue), &serde_json::to_string(id)?)
    }

    /// Dataset activo de una cola, si existe y sigue siendo válido.
    pub fn active_stats_dataset(
        &self,
        queue: StatsQueue,
    ) -> Result<Option<ObservedStats>, DataError> {
        let Some(value) = self.get_setting(active_key(queue))? else {
            return Ok(None);
        };
        let id: String = serde_json::from_str(&value)?;
        match self.get_stats_dataset(&id) {
            Ok(stats) if stats.queue == queue => Ok(Some(stats)),
            // Puntero huérfano o de otra cola: no se inventa un sustituto.
            Ok(_) => Ok(None),
            Err(DataError::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Borra un dataset y su puntero activo. Devuelve `false` si no existía.
    pub fn delete_stats_dataset(&mut self, id: &str) -> Result<bool, DataError> {
        let queue = match self.get_stats_dataset(id) {
            Ok(stats) => Some(stats.queue),
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
    use loldraft_engine::ChampionStatRow;

    fn stats(id: &str, queue: StatsQueue) -> ObservedStats {
        ObservedStats {
            schema: STATS_SCHEMA_VERSION.to_string(),
            id: id.to_string(),
            version: "1".to_string(),
            source: "Importación manual".to_string(),
            license: "Uso personal".to_string(),
            queue,
            patch: "16.22".to_string(),
            rows: vec![
                ChampionStatRow {
                    champion: ChampionKey(64),
                    role: Role::Jungle,
                    games: 1200,
                    wins: 620,
                    pick_rate: Some(0.08),
                    ban_rate: None,
                },
                ChampionStatRow {
                    champion: ChampionKey(103),
                    role: Role::Mid,
                    games: 900,
                    wins: 470,
                    pick_rate: None,
                    ban_rate: Some(0.12),
                },
            ],
        }
    }

    #[test]
    fn a_dataset_roundtrips_through_sqlite() {
        let mut db = Database::open_in_memory().unwrap();
        let saved = db
            .save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        assert_eq!(saved.row_count, 2);
        assert!(!saved.is_active);

        let loaded = db.get_stats_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded, stats("soloq-16.22", StatsQueue::Soloq));

        let list = db.list_stats_datasets().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].queue, StatsQueue::Soloq);
        assert_eq!(list[0].patch, "16.22");
        assert_eq!(list[0].row_count, 2);
    }

    #[test]
    fn reimporting_the_same_id_replaces_rows_without_duplicates() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();

        let mut updated = stats("soloq-16.22", StatsQueue::Soloq);
        updated.version = "2".to_string();
        updated.rows.truncate(1);
        db.save_observed_stats(&updated).unwrap();

        let loaded = db.get_stats_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded.version, "2");
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(db.list_stats_datasets().unwrap().len(), 1);
    }

    #[test]
    fn an_invalid_dataset_does_not_touch_the_previous_one() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();

        let mut invalid = stats("soloq-16.22", StatsQueue::Soloq);
        invalid.rows[0].wins = 10_000;
        assert!(db.save_observed_stats(&invalid).is_err());

        let loaded = db.get_stats_dataset("soloq-16.22").unwrap();
        assert_eq!(loaded.rows.len(), 2);
        assert_eq!(loaded.rows[0].wins, 620);
    }

    #[test]
    fn active_dataset_is_tracked_per_queue() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        db.save_observed_stats(&stats("pro-2026", StatsQueue::Pro))
            .unwrap();

        db.set_active_stats_dataset("soloq-16.22").unwrap();
        assert_eq!(
            db.active_stats_dataset(StatsQueue::Soloq)
                .unwrap()
                .unwrap()
                .id,
            "soloq-16.22"
        );
        assert!(db.active_stats_dataset(StatsQueue::Pro).unwrap().is_none());

        db.set_active_stats_dataset("pro-2026").unwrap();
        assert_eq!(
            db.active_stats_dataset(StatsQueue::Pro)
                .unwrap()
                .unwrap()
                .id,
            "pro-2026"
        );
        assert_eq!(
            db.active_stats_dataset(StatsQueue::Soloq)
                .unwrap()
                .unwrap()
                .id,
            "soloq-16.22"
        );

        let list = db.list_stats_datasets().unwrap();
        assert!(list.iter().all(|dataset| dataset.is_active));
    }

    #[test]
    fn deleting_a_dataset_clears_its_active_pointer() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        db.set_active_stats_dataset("soloq-16.22").unwrap();

        assert!(db.delete_stats_dataset("soloq-16.22").unwrap());
        assert!(db
            .active_stats_dataset(StatsQueue::Soloq)
            .unwrap()
            .is_none());
        assert!(db.list_stats_datasets().unwrap().is_empty());
        assert!(!db.delete_stats_dataset("soloq-16.22").unwrap());
    }

    #[test]
    fn an_active_pointer_to_another_queue_is_ignored() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_observed_stats(&stats("soloq-16.22", StatsQueue::Soloq))
            .unwrap();
        db.set_setting("stats.active.pro", "\"soloq-16.22\"")
            .unwrap();

        assert!(db.active_stats_dataset(StatsQueue::Pro).unwrap().is_none());
    }
}
