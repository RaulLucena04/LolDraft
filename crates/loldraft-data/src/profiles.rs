//! Persistencia de conjuntos de perfiles estratégicos.
//!
//! Un conjunto se guarda entero dentro de una transacción. Las celdas
//! desconocidas no se rellenan: la ausencia de fila también significa `None`.

use crate::{DataError, Database};
use loldraft_domain::{ChampionKey, Role};
use loldraft_engine::{ArchetypeId, ProfileSet, Provenance, Taxonomy, TraitId};
use rusqlite::{params, OptionalExtension, Transaction};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSetSummary {
    pub id: String,
    pub version: String,
    pub taxonomy_version: String,
    pub patch: Option<String>,
    pub champion_count: usize,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveProfileSet {
    pub id: String,
    pub version: String,
    pub champion_count: usize,
}

impl Database {
    /// Refleja la taxonomía incluida en el binario en las tablas de referencia.
    /// Es idempotente y no borra filas históricas.
    pub(crate) fn sync_taxonomy(&mut self) -> Result<(), DataError> {
        let taxonomy = Taxonomy::seed()
            .map_err(|error| DataError::Invalid(format!("Taxonomía inválida: {error}")))?;
        let now = crate::now_ms();
        let tx = self.conn.transaction()?;

        for entry in &taxonomy.archetypes {
            tx.execute(
                r#"
                INSERT INTO archetypes (id, name, category, description)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    category = excluded.category,
                    description = excluded.description
                "#,
                params![
                    entry.id.as_str(),
                    entry.label_es,
                    entry.category.as_str(),
                    entry.definition
                ],
            )?;
        }

        for entry in &taxonomy.traits {
            tx.execute(
                r#"
                INSERT INTO traits (id, kind, name, description)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(id) DO UPDATE SET
                    kind = excluded.kind,
                    name = excluded.name,
                    description = excluded.description
                "#,
                params![
                    entry.id.as_str(),
                    entry.kind.as_str(),
                    entry.label_es,
                    entry.definition
                ],
            )?;
        }

        put_setting(
            &tx,
            "taxonomy.version",
            &serde_json::to_string(&taxonomy.version)?,
            now,
        )?;
        put_setting(
            &tx,
            "taxonomy.rubric_json",
            &serde_json::to_string(&taxonomy.rubric)?,
            now,
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Guarda o reemplaza un conjunto completo. Las publicaciones definitivas
    /// deben usar un id nuevo; reutilizar el id equivale a editar el conjunto.
    pub fn save_profile_set(&mut self, set: &ProfileSet) -> Result<(), DataError> {
        let taxonomy = Taxonomy::seed()
            .map_err(|error| DataError::Invalid(format!("Taxonomía inválida: {error}")))?;
        set.validate(&taxonomy).map_err(|error| {
            DataError::Invalid(format!("Conjunto de perfiles inválido: {error}"))
        })?;

        let now = crate::now_ms();
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO profile_sets
                (id, version, taxonomy_version, patch, is_active, created_at)
            VALUES (?1, ?2, ?3, ?4, 0, ?5)
            ON CONFLICT(id) DO UPDATE SET
                version = excluded.version,
                taxonomy_version = excluded.taxonomy_version,
                patch = excluded.patch
            "#,
            params![
                set.id,
                set.version,
                set.taxonomy_version,
                set.patch.as_deref(),
                now
            ],
        )?;

        tx.execute(
            "DELETE FROM profile_set_roles WHERE profile_set_id = ?1",
            params![set.id],
        )?;
        tx.execute(
            "DELETE FROM champion_archetypes WHERE profile_set_id = ?1",
            params![set.id],
        )?;
        tx.execute(
            "DELETE FROM champion_traits WHERE profile_set_id = ?1",
            params![set.id],
        )?;

        for profile in set.champions.values() {
            for role in &profile.roles {
                tx.execute(
                    r#"
                    INSERT INTO profile_set_roles
                        (profile_set_id, champion_key, role_id, proficiency, confidence, source_json)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                    params![
                        set.id,
                        profile.key.0,
                        role.role.as_str(),
                        role.proficiency,
                        role.confidence,
                        serde_json::to_string(&role.source)?
                    ],
                )?;
            }

            for (id, rating) in &profile.archetypes {
                tx.execute(
                    r#"
                    INSERT INTO champion_archetypes
                        (profile_set_id, champion_key, archetype_id, intensity, confidence, source_json, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    "#,
                    params![
                        set.id,
                        profile.key.0,
                        id.as_str(),
                        rating.intensity,
                        rating.confidence,
                        serde_json::to_string(&rating.source)?,
                        now
                    ],
                )?;
            }

            for (id, rating) in &profile.traits {
                tx.execute(
                    r#"
                    INSERT INTO champion_traits
                        (profile_set_id, champion_key, trait_id, value, confidence, source_json, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    "#,
                    params![
                        set.id,
                        profile.key.0,
                        id.as_str(),
                        rating.value,
                        rating.confidence,
                        serde_json::to_string(&rating.source)?,
                        now
                    ],
                )?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn load_profile_set(&self, id: &str) -> Result<ProfileSet, DataError> {
        let meta = self
            .conn
            .query_row(
                "SELECT version, taxonomy_version, patch FROM profile_sets WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DataError::NotFound(format!("conjunto de perfiles {id}")))?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT champion_key FROM profile_set_roles WHERE profile_set_id = ?1
            UNION
            SELECT champion_key FROM champion_archetypes WHERE profile_set_id = ?1
            UNION
            SELECT champion_key FROM champion_traits WHERE profile_set_id = ?1
            ORDER BY champion_key
            "#,
        )?;
        let keys: Vec<u16> = stmt
            .query_map(params![id], |row| row.get::<_, u16>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut champions = BTreeMap::new();
        for key in keys {
            let champion_key = ChampionKey(key);
            let identity = self
                .conn
                .query_row(
                    "SELECT slug, name FROM champions WHERE riot_key = ?1",
                    params![key],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .ok_or_else(|| {
                    DataError::Invalid(format!(
                        "el perfil referencia al campeón {key}, que no existe en el catálogo"
                    ))
                })?;

            let roles = self.load_profile_roles(id, key)?;
            let archetypes = self.load_profile_archetypes(id, key)?;
            let traits = self.load_profile_traits(id, key)?;

            champions.insert(
                champion_key,
                loldraft_engine::ChampionProfile {
                    key: champion_key,
                    slug: identity.0,
                    name: identity.1,
                    roles,
                    archetypes,
                    traits,
                },
            );
        }

        let set = ProfileSet {
            id: id.to_string(),
            version: meta.0,
            taxonomy_version: meta.1,
            patch: meta.2,
            champions,
        };
        let taxonomy = Taxonomy::seed()
            .map_err(|error| DataError::Invalid(format!("Taxonomía inválida: {error}")))?;
        set.validate(&taxonomy).map_err(|error| {
            DataError::Invalid(format!("Conjunto persistido inválido: {error}"))
        })?;
        Ok(set)
    }

    fn load_profile_roles(
        &self,
        set_id: &str,
        champion_key: u16,
    ) -> Result<Vec<loldraft_engine::ProfileRole>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT role_id, proficiency, confidence, source_json
            FROM profile_set_roles
            WHERE profile_set_id = ?1 AND champion_key = ?2
            ORDER BY role_id
            "#,
        )?;
        let rows = stmt.query_map(params![set_id, champion_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut roles = Vec::new();
        for row in rows {
            let (role, proficiency, confidence, source_json) = row?;
            let role = Role::from_code(&role)
                .ok_or_else(|| DataError::Invalid(format!("rol persistido inválido: {role}")))?;
            roles.push(loldraft_engine::ProfileRole {
                role,
                proficiency,
                confidence,
                source: parse_provenance(&source_json)?,
            });
        }
        Ok(roles)
    }

    fn load_profile_archetypes(
        &self,
        set_id: &str,
        champion_key: u16,
    ) -> Result<BTreeMap<ArchetypeId, loldraft_engine::ArchetypeRating>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT archetype_id, intensity, confidence, source_json
            FROM champion_archetypes
            WHERE profile_set_id = ?1 AND champion_key = ?2
            "#,
        )?;
        let rows = stmt.query_map(params![set_id, champion_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f32>>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut archetypes = BTreeMap::new();
        for row in rows {
            let (id, intensity, confidence, source_json) = row?;
            let id = ArchetypeId::from_code(&id).ok_or_else(|| {
                DataError::Invalid(format!("arquetipo persistido inválido: {id}"))
            })?;
            let inserted = archetypes.insert(
                id,
                loldraft_engine::ArchetypeRating {
                    intensity,
                    confidence,
                    source: parse_provenance(&source_json)?,
                },
            );
            if inserted.is_some() {
                return Err(DataError::Invalid(format!(
                    "arquetipo duplicado en el conjunto: {id}"
                )));
            }
        }
        Ok(archetypes)
    }

    fn load_profile_traits(
        &self,
        set_id: &str,
        champion_key: u16,
    ) -> Result<BTreeMap<TraitId, loldraft_engine::TraitRating>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT trait_id, value, confidence, source_json
            FROM champion_traits
            WHERE profile_set_id = ?1 AND champion_key = ?2
            "#,
        )?;
        let rows = stmt.query_map(params![set_id, champion_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f32>>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut traits = BTreeMap::new();
        for row in rows {
            let (id, value, confidence, source_json) = row?;
            let id = TraitId::from_code(&id)
                .ok_or_else(|| DataError::Invalid(format!("trait persistido inválido: {id}")))?;
            let inserted = traits.insert(
                id,
                loldraft_engine::TraitRating {
                    value,
                    confidence,
                    source: parse_provenance(&source_json)?,
                },
            );
            if inserted.is_some() {
                return Err(DataError::Invalid(format!(
                    "trait duplicado en el conjunto: {id}"
                )));
            }
        }
        Ok(traits)
    }

    pub fn list_profile_sets(&self) -> Result<Vec<ProfileSetSummary>, DataError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                ps.id, ps.version, ps.taxonomy_version, ps.patch, ps.is_active, ps.created_at,
                (SELECT COUNT(*) FROM (
                    SELECT champion_key FROM profile_set_roles WHERE profile_set_id = ps.id
                    UNION
                    SELECT champion_key FROM champion_archetypes WHERE profile_set_id = ps.id
                    UNION
                    SELECT champion_key FROM champion_traits WHERE profile_set_id = ps.id
                )) AS champion_count
            FROM profile_sets ps
            ORDER BY ps.created_at DESC, ps.id
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut sets = Vec::new();
        for row in rows {
            let (id, version, taxonomy_version, patch, is_active, created_at, champion_count) =
                row?;
            sets.push(ProfileSetSummary {
                id,
                version,
                taxonomy_version,
                patch,
                champion_count: champion_count as usize,
                is_active: is_active != 0,
                created_at,
            });
        }
        Ok(sets)
    }

    pub fn profile_set_count(&self) -> Result<usize, DataError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM profile_sets", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn active_profile_set(&self) -> Result<Option<ActiveProfileSet>, DataError> {
        let row = self
            .conn
            .query_row(
                r#"
                SELECT
                    ps.id, ps.version,
                    (SELECT COUNT(*) FROM (
                        SELECT champion_key FROM profile_set_roles WHERE profile_set_id = ps.id
                        UNION
                        SELECT champion_key FROM champion_archetypes WHERE profile_set_id = ps.id
                        UNION
                        SELECT champion_key FROM champion_traits WHERE profile_set_id = ps.id
                    )) AS champion_count
                FROM profile_sets ps
                WHERE ps.is_active = 1
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?;

        Ok(row.map(|(id, version, champion_count)| ActiveProfileSet {
            id,
            version,
            champion_count: champion_count as usize,
        }))
    }

    /// Activa un conjunto y desactiva el anterior en la misma transacción.
    pub fn set_active_profile_set(&mut self, id: &str) -> Result<(), DataError> {
        let tx = self.conn.transaction()?;
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM profile_sets WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(DataError::NotFound(format!("conjunto de perfiles {id}")));
        }
        tx.execute(
            "UPDATE profile_sets SET is_active = 0 WHERE is_active = 1",
            [],
        )?;
        tx.execute(
            "UPDATE profile_sets SET is_active = 1 WHERE id = ?1",
            params![id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_profile_set(&self, id: &str) -> Result<bool, DataError> {
        let changed = self
            .conn
            .execute("DELETE FROM profile_sets WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }
}

fn parse_provenance(json: &str) -> Result<Provenance, DataError> {
    Ok(serde_json::from_str(json)?)
}

fn put_setting(
    tx: &Transaction<'_>,
    key: &str,
    value_json: &str,
    updated_at: i64,
) -> Result<(), DataError> {
    tx.execute(
        r#"
        INSERT INTO settings (key, value_json, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value_json = excluded.value_json,
            updated_at = excluded.updated_at
        "#,
        params![key, value_json, updated_at],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::parse_seed;
    use loldraft_engine::{
        ArchetypeRating, ChampionProfile, ProfileRole, TraitRating, TAXONOMY_VERSION,
    };

    const TEST_SEED: &str = r#"{
      "dataset": {"kind": "test"},
      "champions": [{
        "riot_key": 266,
        "slug": "Aatrox",
        "name": "Aatrox",
        "title": "the Darkin Blade",
        "image": "Aatrox.png",
        "tags": ["Fighter"],
        "roles": [{"role":"top","proficiency":0.95,"confidence":0.8,"source":"test"}]
      }]
    }"#;

    fn curated() -> Provenance {
        Provenance::Curated {
            editor: "test".to_string(),
            review: "review".to_string(),
        }
    }

    fn sample_set(id: &str, version: &str) -> ProfileSet {
        let mut archetypes = BTreeMap::new();
        archetypes.insert(
            ArchetypeId::Engage,
            ArchetypeRating {
                intensity: Some(0.75),
                confidence: 0.8,
                source: curated(),
            },
        );
        archetypes.insert(
            ArchetypeId::Frontline,
            ArchetypeRating {
                intensity: Some(0.5),
                confidence: 0.8,
                source: curated(),
            },
        );

        let mut traits = BTreeMap::new();
        for (id, value) in [
            (TraitId::DamagePhysical, 0.6),
            (TraitId::DamageMagic, 0.4),
            (TraitId::DamageTrue, 0.0),
        ] {
            traits.insert(
                id,
                TraitRating {
                    value: Some(value),
                    confidence: 0.8,
                    source: curated(),
                },
            );
        }
        traits.insert(
            TraitId::Range,
            TraitRating {
                value: None,
                confidence: 0.0,
                source: curated(),
            },
        );

        let profile = ChampionProfile {
            key: ChampionKey(266),
            slug: "Aatrox".to_string(),
            name: "Aatrox".to_string(),
            roles: vec![ProfileRole {
                role: Role::Top,
                proficiency: 0.95,
                confidence: 0.8,
                source: curated(),
            }],
            archetypes,
            traits,
        };

        ProfileSet {
            id: id.to_string(),
            version: version.to_string(),
            taxonomy_version: TAXONOMY_VERSION.to_string(),
            patch: Some("16.22".to_string()),
            champions: BTreeMap::from([(ChampionKey(266), profile)]),
        }
    }

    fn database_with_champion() -> Database {
        let mut db = Database::open_in_memory().expect("db");
        let seed = parse_seed(TEST_SEED).expect("seed");
        db.seed_champions(&seed).expect("campeón");
        db
    }

    #[test]
    fn taxonomy_is_seeded_and_synced_idempotently() {
        let mut db = Database::open_in_memory().expect("db");
        db.sync_taxonomy().expect("segunda sincronización");

        let archetypes: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM archetypes", [], |row| row.get(0))
            .unwrap();
        let traits: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM traits", [], |row| row.get(0))
            .unwrap();
        assert_eq!(archetypes, 20);
        assert_eq!(traits, 18);

        let version = db.get_setting("taxonomy.version").unwrap().unwrap();
        assert_eq!(
            serde_json::from_str::<String>(&version).unwrap(),
            TAXONOMY_VERSION
        );
        assert!(db.get_setting("taxonomy.rubric_json").unwrap().is_some());
    }

    #[test]
    fn a_profile_set_roundtrips_through_sqlite() {
        let mut db = database_with_champion();
        let set = sample_set("set-1", "2026.11.1");
        db.save_profile_set(&set).expect("guarda");

        let loaded = db.load_profile_set("set-1").expect("carga");
        assert_eq!(loaded, set);
        assert_eq!(loaded.get(ChampionKey(266)).unwrap().roles.len(), 1);
        assert_eq!(
            loaded
                .get(ChampionKey(266))
                .unwrap()
                .trait_value(TraitId::Range),
            None
        );

        let summaries = db.list_profile_sets().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].champion_count, 1);
        assert_eq!(summaries[0].patch.as_deref(), Some("16.22"));
        assert!(!summaries[0].is_active);
        assert_eq!(db.profile_set_count().unwrap(), 1);
        assert!(db.active_profile_set().unwrap().is_none());
    }

    #[test]
    fn saving_the_same_id_replaces_cells() {
        let mut db = database_with_champion();
        let mut set = sample_set("set-1", "2026.11.1");
        db.save_profile_set(&set).unwrap();

        set.get_mut(ChampionKey(266))
            .unwrap()
            .archetypes
            .get_mut(&ArchetypeId::Engage)
            .unwrap()
            .intensity = Some(1.0);
        set.version = "2026.11.2".to_string();
        db.save_profile_set(&set).unwrap();

        let loaded = db.load_profile_set("set-1").unwrap();
        assert_eq!(
            loaded.get(ChampionKey(266)).unwrap().archetypes[&ArchetypeId::Engage].intensity,
            Some(1.0)
        );
        assert_eq!(loaded.version, "2026.11.2");
        let cells: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM champion_archetypes WHERE profile_set_id = 'set-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cells, 2);
    }

    #[test]
    fn only_one_profile_set_can_be_active() {
        let mut db = database_with_champion();
        db.save_profile_set(&sample_set("set-1", "1")).unwrap();
        db.save_profile_set(&sample_set("set-2", "2")).unwrap();

        db.set_active_profile_set("set-1").unwrap();
        assert_eq!(db.active_profile_set().unwrap().unwrap().id, "set-1");

        db.set_active_profile_set("set-2").unwrap();
        let active = db.active_profile_set().unwrap().unwrap();
        assert_eq!(active.id, "set-2");
        assert_eq!(active.version, "2");
        assert_eq!(active.champion_count, 1);

        let summaries = db.list_profile_sets().unwrap();
        assert_eq!(summaries.iter().filter(|set| set.is_active).count(), 1);
        assert!(db.set_active_profile_set("missing").is_err());
    }

    #[test]
    fn deleting_a_set_cascades_its_cells() {
        let mut db = database_with_champion();
        db.save_profile_set(&sample_set("set-1", "1")).unwrap();
        assert!(db.delete_profile_set("set-1").unwrap());
        assert!(db.load_profile_set("set-1").is_err());

        let cells: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM champion_archetypes", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cells, 0);
    }

    #[test]
    fn a_failed_import_does_not_leave_a_partial_set() {
        let mut db = database_with_champion();
        let mut set = sample_set("set-bad", "1");
        let mut profile = set.get(ChampionKey(266)).unwrap().clone();
        profile.key = ChampionKey(999);
        profile.slug = "Missing".to_string();
        profile.name = "Missing".to_string();
        set.champions = BTreeMap::from([(ChampionKey(999), profile)]);

        assert!(db.save_profile_set(&set).is_err());
        assert_eq!(db.profile_set_count().unwrap(), 0);
    }
}
