use crate::dto::{DataStatusDto, SessionSummaryDto};
use crate::error::CommandError;
use loldraft_data::{default_seed, Database, SessionRecord, SessionSummary};
use loldraft_domain::{
    ChampionCatalog, ChampionKey, DraftFormat, DraftSession, DraftState, Role, Side,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

pub struct ActiveSession {
    pub id: String,
    pub session: DraftSession,
    /// Rol fijado manualmente para un pick concreto de un lado.
    pub overrides: HashMap<(Side, ChampionKey), Role>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    version: u8,
    state: DraftState,
    #[serde(default)]
    overrides: Vec<StoredRoleOverride>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredRoleOverride {
    side: Side,
    champion_key: ChampionKey,
    role: Role,
}

impl ActiveSession {
    pub fn new(id: String, side: Side) -> Self {
        Self {
            id,
            session: DraftSession::standard(side),
            overrides: HashMap::new(),
        }
    }

    pub fn restore(record: &SessionRecord) -> Result<Self, CommandError> {
        let stored = match serde_json::from_str::<StoredSession>(&record.state_json) {
            Ok(stored) => stored,
            Err(_) => StoredSession {
                version: 1,
                state: serde_json::from_str::<DraftState>(&record.state_json)?,
                overrides: Vec::new(),
            },
        };

        let session = match DraftSession::replay(
            stored.state.format.clone(),
            record.side,
            stored.state.actions(),
        ) {
            Ok(session) => session,
            Err(_) => DraftSession::from_state(stored.state, record.side),
        };
        let overrides = stored
            .overrides
            .into_iter()
            .map(|item| ((item.side, item.champion_key), item.role))
            .collect();

        Ok(Self {
            id: record.id.clone(),
            session,
            overrides,
        })
    }

    pub fn state(&self) -> &DraftState {
        self.session.state()
    }

    pub fn stored_json(&self) -> Result<String, CommandError> {
        let stored = StoredSession {
            version: 1,
            state: self.session.state().clone(),
            overrides: self
                .overrides
                .iter()
                .map(|((side, champion_key), role)| StoredRoleOverride {
                    side: *side,
                    champion_key: *champion_key,
                    role: *role,
                })
                .collect(),
        };
        Ok(serde_json::to_string(&stored)?)
    }
}

pub struct AppState {
    pub db: Mutex<Database>,
    pub catalog: RwLock<ChampionCatalog>,
    pub active: Mutex<Option<ActiveSession>>,
}

impl AppState {
    pub fn new(mut db: Database) -> Result<Self, CommandError> {
        if db.champion_count()? == 0 {
            let seed = default_seed()?;
            db.seed_champions(&seed)?;

            if let Some(version) = seed.dataset.ddragon_version.as_deref() {
                set_json_string(&db, "ddragon.version", version)?;
            }
            if let Some(source) = seed.dataset.source.as_deref() {
                set_json_string(&db, "ddragon.source", source)?;
            }
            if let Some(roles_source) = seed.dataset.roles_source.as_deref() {
                set_json_string(&db, "ddragon.roles_source", roles_source)?;
            }
            if let Some(fetched_at) = seed.dataset.fetched_at.as_deref() {
                set_json_string(&db, "ddragon.seed_fetched_at", fetched_at)?;
            }
        }

        let catalog = db.load_catalog()?;
        Ok(Self {
            db: Mutex::new(db),
            catalog: RwLock::new(catalog),
            active: Mutex::new(None),
        })
    }

    pub fn reload_catalog(&self) -> Result<(), CommandError> {
        let catalog = {
            let db = self
                .db
                .lock()
                .map_err(|_| CommandError::internal("la base de datos está bloqueada"))?;
            db.load_catalog()?
        };
        let mut guard = self
            .catalog
            .write()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        *guard = catalog;
        Ok(())
    }

    pub fn data_status(&self) -> Result<DataStatusDto, CommandError> {
        let (patch, last_sync_ms, catalog_source, roles_source, champion_count) = {
            let db = self
                .db
                .lock()
                .map_err(|_| CommandError::internal("la base de datos está bloqueada"))?;
            (
                get_json_string(&db, "ddragon.version")?,
                get_json_i64(&db, "ddragon.last_sync_ms")?,
                get_json_string(&db, "ddragon.source")?,
                get_json_string(&db, "ddragon.roles_source")?,
                db.champion_count()?,
            )
        };

        let (with_roles, without_roles) = {
            let catalog = self
                .catalog
                .read()
                .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
            let with_roles = catalog
                .iter()
                .filter(|champ| !champ.roles.is_empty())
                .count();
            (with_roles, catalog.len().saturating_sub(with_roles))
        };

        let mut warnings = Vec::new();
        if champion_count == 0 {
            warnings.push(
                "El catálogo está vacío. Sincroniza Data Dragon o revisa la instalación."
                    .to_string(),
            );
        }
        if without_roles > 0 {
            warnings.push(format!(
                "{without_roles} campeones aún no tienen rol importado; se muestran como rol pendiente."
            ));
        }
        if patch.is_none() {
            warnings.push(
                "No hay parche registrado. Sincroniza Data Dragon para fijar la versión."
                    .to_string(),
            );
        }

        Ok(DataStatusDto {
            patch,
            last_sync_ms,
            catalog_source,
            identity_source: "Data Dragon (Riot Games)".to_string(),
            roles_source: roles_source
                .map(|source| {
                    if source.to_ascii_lowercase().contains("meraki") {
                        "Meraki Analytics (positions)".to_string()
                    } else {
                        source
                    }
                })
                .unwrap_or_else(|| "Sin roles importados".to_string()),
            champion_count,
            champions_with_roles: with_roles,
            champions_without_roles: without_roles,
            curated_profiles: 0,
            strategic_profile_status: "pending",
            warnings,
        })
    }
}

pub fn new_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("draft-{}-{seq}", loldraft_data::now_ms())
}

pub fn summary_from_record(record: SessionSummary) -> SessionSummaryDto {
    SessionSummaryDto {
        id: record.id,
        format_id: record.format_id,
        side: record.side,
        action_count: record.action_count,
        total_actions: DraftFormat::standard().len(),
        updated_at: record.updated_at,
    }
}

fn set_json_string(db: &Database, key: &str, value: &str) -> Result<(), CommandError> {
    db.set_setting(key, &serde_json::to_string(value)?)?;
    Ok(())
}

fn get_json_string(db: &Database, key: &str) -> Result<Option<String>, CommandError> {
    let Some(raw) = db.get_setting(key)? else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<String>(&raw).unwrap_or_else(|_| raw.trim_matches('"').to_string()),
    ))
}

fn get_json_i64(db: &Database, key: &str) -> Result<Option<i64>, CommandError> {
    let Some(raw) = db.get_setting(key)? else {
        return Ok(None);
    };
    Ok(serde_json::from_str::<i64>(&raw).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use loldraft_domain::{ActionKind, DraftAction};

    #[test]
    fn stored_session_roundtrips_state_overrides_and_side() {
        let mut active = ActiveSession::new("roundtrip".to_string(), Side::Red);
        let key = ChampionKey(266);
        active
            .session
            .apply(DraftAction::new(Side::Blue, ActionKind::Ban, key))
            .expect("valid ban");
        active.session.undo();
        active.session.redo();
        active.overrides.insert((Side::Blue, key), Role::Top);

        let json = active.stored_json().expect("serialize");
        let record = SessionRecord {
            id: "roundtrip".to_string(),
            format_id: "standard".to_string(),
            side: Side::Red,
            state_json: json,
            action_count: 1,
            created_at: 0,
            updated_at: 0,
        };

        let restored = ActiveSession::restore(&record).expect("restore");
        assert_eq!(restored.session.side(), Side::Red);
        assert_eq!(restored.state().actions().len(), 1);
        assert!(restored.session.can_undo());
        assert_eq!(restored.session.undo_depth(), 1);
        assert_eq!(
            restored.overrides.get(&(Side::Blue, key)).copied(),
            Some(Role::Top)
        );
    }

    #[test]
    fn restores_legacy_state_json_without_an_envelope() {
        let state = DraftState::new(DraftFormat::standard());
        let record = SessionRecord {
            id: "legacy".to_string(),
            format_id: "standard".to_string(),
            side: Side::Blue,
            state_json: serde_json::to_string(&state).expect("serialize"),
            action_count: 0,
            created_at: 0,
            updated_at: 0,
        };

        let restored = ActiveSession::restore(&record).expect("restore");
        assert!(restored.state().actions().is_empty());
        assert!(!restored.session.can_undo());
        assert!(restored.overrides.is_empty());
    }
}
