//! Lectura del cliente de League of Legends (LCU) durante la selección de
//! campeones.
//!
//! Solo lectura: este módulo no envía acciones al cliente ni escribe datos. La
//! app la usa para rellenar el draft de SoloQ con lo que el cliente ya muestra.
//! El cliente expone un HTTP local con certificado autofirmado; las credenciales
//! salen del `lockfile` que crea mientras está abierto.

use loldraft_domain::{ChampionKey, Role, Side};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Rutas habituales del `lockfile` cuando el cliente no está en la ruta por
/// defecto. La variable de entorno `LOLDRAFT_LCU_LOCKFILE` tiene prioridad.
pub const DEFAULT_LOCKFILE_PATHS: [&str; 3] = [
    r"C:\Riot Games\League of Legends\lockfile",
    r"D:\Riot Games\League of Legends\lockfile",
    r"E:\Riot Games\League of Legends\lockfile",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcuCredentials {
    pub port: u16,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LcuError {
    #[error("el cliente de League of Legends no está abierto: no encuentro su lockfile")]
    NotRunning,
    #[error("el lockfile `{0}` no tiene el formato esperado")]
    InvalidLockfile(String),
    #[error("no se pudo conectar con el cliente: {0}")]
    Transport(String),
    #[error("el cliente no está en selección de campeones")]
    NotInChampSelect,
    #[error("el cliente respondió con estado {0}")]
    Http(u16),
    #[error("el cliente aún no ha creado tu acción de pick en esta selección")]
    NoPickAction,
    #[error("respuesta inesperada del cliente: falta `{0}`")]
    InvalidSession(&'static str),
}

/// Estado de champ select ya normalizado para el motor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampSelectDraft {
    pub side: Side,
    /// Posición de pick dentro de tu equipo (1..=5).
    pub slot: u8,
    /// Rol asignado por el cliente, si ya se conoce.
    pub role: Option<Role>,
    /// Campeón que tú ya has bloqueado, si lo hay. No se repite en `ally_picks`.
    pub own_pick: Option<ChampionKey>,
    pub ally_picks: Vec<ChampionKey>,
    pub enemy_picks: Vec<ChampionKey>,
    pub bans: Vec<ChampionKey>,
    /// Fase del temporizador del cliente (`BAN_PICK`, `FINALIZATION`, …).
    pub timer_phase: Option<String>,
}

/// Convierte el contenido del `lockfile` en credenciales.
///
/// Formato: `proceso:pid:puerto:contraseña:protocolo`.
pub fn parse_lockfile(text: &str) -> Result<LcuCredentials, LcuError> {
    let text = text.trim();
    let mut parts = text.split(':');
    let _process = parts.next();
    let _pid = parts.next();
    let port = parts.next().unwrap_or_default();
    let password = parts.next().unwrap_or_default();
    let protocol = parts.next().unwrap_or_default();

    let port = port
        .parse::<u16>()
        .map_err(|_| LcuError::InvalidLockfile(text.to_string()))?;
    if password.is_empty() || protocol != "https" {
        return Err(LcuError::InvalidLockfile(text.to_string()));
    }

    Ok(LcuCredentials {
        port,
        password: password.to_string(),
    })
}

/// Busca el `lockfile` del cliente en el entorno y en las rutas habituales.
pub fn find_lockfile() -> Result<PathBuf, LcuError> {
    if let Ok(path) = std::env::var("LOLDRAFT_LCU_LOCKFILE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(LcuError::InvalidLockfile(path.display().to_string()));
    }

    for candidate in DEFAULT_LOCKFILE_PATHS {
        let path = Path::new(candidate);
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
    }

    Err(LcuError::NotRunning)
}

pub struct LcuClient {
    http: reqwest::Client,
    credentials: LcuCredentials,
}

impl LcuClient {
    pub fn new(credentials: LcuCredentials) -> Result<Self, LcuError> {
        let http = reqwest::Client::builder()
            // El cliente usa un certificado autofirmado en 127.0.0.1.
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|error| LcuError::Transport(error.to_string()))?;
        Ok(Self { http, credentials })
    }

    /// Descubre el cliente desde su `lockfile`.
    pub fn connect() -> Result<Self, LcuError> {
        let path = find_lockfile()?;
        let text = std::fs::read_to_string(&path)
            .map_err(|error| LcuError::Transport(error.to_string()))?;
        Self::new(parse_lockfile(&text)?)
    }

    pub async fn champ_select(&self) -> Result<ChampSelectDraft, LcuError> {
        let url = format!(
            "https://127.0.0.1:{}/lol-champ-select/v1/session",
            self.credentials.port
        );
        let response = self
            .http
            .get(url)
            .basic_auth("riot", Some(&self.credentials.password))
            .send()
            .await
            .map_err(|error| LcuError::Transport(error.to_string()))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(LcuError::NotInChampSelect);
        }
        if !status.is_success() {
            return Err(LcuError::Http(status.as_u16()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|error| LcuError::Transport(error.to_string()))?;
        draft_from_session(&body)
    }
}

/// Traduce el JSON de `/lol-champ-select/v1/session` al estado que usa la app.
///
/// No inventa huecos: si un jugador no ha bloqueado campeón, su pick no aparece.
pub fn draft_from_session(session: &serde_json::Value) -> Result<ChampSelectDraft, LcuError> {
    let local_cell = session
        .get("localPlayerCellId")
        .and_then(serde_json::Value::as_i64)
        .filter(|cell| *cell >= 0)
        .ok_or(LcuError::NotInChampSelect)?;
    let local_cell = local_cell as u16;

    let my_team = session
        .get("myTeam")
        .and_then(serde_json::Value::as_array)
        .ok_or(LcuError::InvalidSession("myTeam"))?;

    let side = my_team
        .iter()
        .find(|player| {
            player.get("cellId").and_then(serde_json::Value::as_u64) == Some(local_cell as u64)
        })
        .and_then(|player| player.get("team").and_then(serde_json::Value::as_u64))
        .map(|team| if team == 2 { Side::Red } else { Side::Blue })
        .unwrap_or(if local_cell < 5 {
            Side::Blue
        } else {
            Side::Red
        });

    let role = my_team
        .iter()
        .find(|player| {
            player.get("cellId").and_then(serde_json::Value::as_u64) == Some(local_cell as u64)
        })
        .and_then(|player| {
            player
                .get("assignedPosition")
                .and_then(serde_json::Value::as_str)
        })
        .and_then(role_from_position);

    let mut picks: Vec<(i64, u16, u16)> = Vec::new();
    let actions = session
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or(LcuError::InvalidSession("actions"))?;
    for group in actions {
        let Some(group) = group.as_array() else {
            continue;
        };
        for action in group {
            if action.get("type").and_then(serde_json::Value::as_str) != Some("pick") {
                continue;
            }
            let Some(actor) = action
                .get("actorCellId")
                .and_then(serde_json::Value::as_u64)
            else {
                continue;
            };
            let champion = action
                .get("championId")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u16;
            let id = action
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let completed = action
                .get("completed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if champion == 0 || !completed {
                continue;
            }
            picks.push((id, actor as u16, champion));
        }
    }
    picks.sort_by_key(|(id, ..)| *id);

    let same_side = |cell: u16| -> bool {
        if local_cell < 5 {
            cell < 5
        } else {
            cell >= 5
        }
    };

    let slot = pick_slot(actions, local_cell, same_side).ok_or(LcuError::NoPickAction)?;

    let mut ally_picks = Vec::new();
    let mut enemy_picks = Vec::new();
    let mut own_pick = None;
    for (_, actor, champion) in &picks {
        if *actor == local_cell {
            own_pick = Some(ChampionKey(*champion));
            continue;
        }
        if same_side(*actor) {
            ally_picks.push(ChampionKey(*champion));
        } else {
            enemy_picks.push(ChampionKey(*champion));
        }
    }

    let bans = bans_from_session(session);

    Ok(ChampSelectDraft {
        side,
        slot,
        role,
        own_pick,
        ally_picks,
        enemy_picks,
        bans,
        timer_phase: session
            .get("timer")
            .and_then(|timer| timer.get("phase"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    })
}

/// Posición (1..=5) de la acción de pick del jugador dentro de su equipo.
fn pick_slot(
    actions: &[serde_json::Value],
    local_cell: u16,
    same_side: impl Fn(u16) -> bool,
) -> Option<u8> {
    let mut team_actions: Vec<(i64, u16)> = Vec::new();
    for group in actions {
        let Some(group) = group.as_array() else {
            continue;
        };
        for action in group {
            if action.get("type").and_then(serde_json::Value::as_str) != Some("pick") {
                continue;
            }
            let Some(actor) = action
                .get("actorCellId")
                .and_then(serde_json::Value::as_u64)
            else {
                continue;
            };
            let actor = actor as u16;
            if !same_side(actor) {
                continue;
            }
            let id = action
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            team_actions.push((id, actor));
        }
    }
    team_actions.sort_by_key(|(id, _)| *id);
    let index = team_actions
        .iter()
        .position(|(_, actor)| *actor == local_cell)?;
    u8::try_from(index + 1).ok()
}

fn bans_from_session(session: &serde_json::Value) -> Vec<ChampionKey> {
    let mut bans = Vec::new();
    let mut seen = BTreeSet::new();

    // Fuente principal: las acciones de baneo completadas, en orden. El objeto
    // `bans` del cliente puede llegar vacío durante la fase de baneo, así que
    // no basta con leerlo al final.
    let mut completed: Vec<(i64, u16)> = Vec::new();
    if let Some(actions) = session.get("actions").and_then(serde_json::Value::as_array) {
        for group in actions {
            let Some(group) = group.as_array() else {
                continue;
            };
            for action in group {
                if action.get("type").and_then(serde_json::Value::as_str) != Some("ban") {
                    continue;
                }
                let is_completed = action
                    .get("completed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let champion = action
                    .get("championId")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u16;
                if !is_completed || champion == 0 {
                    continue;
                }
                let id = action
                    .get("id")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                completed.push((id, champion));
            }
        }
    }
    completed.sort_by_key(|(id, _)| *id);
    for (_, key) in completed {
        if seen.insert(key) {
            bans.push(ChampionKey(key));
        }
    }

    // Respaldo: el resumen `bans` del cliente. Se añade lo que aún no esté.
    if let Some(bans_object) = session.get("bans") {
        for field in ["myTeamBans", "theirTeamBans"] {
            let Some(list) = bans_object.get(field).and_then(serde_json::Value::as_array) else {
                continue;
            };
            for value in list {
                let key = value.as_u64().unwrap_or(0) as u16;
                if key != 0 && seen.insert(key) {
                    bans.push(ChampionKey(key));
                }
            }
        }
    }
    bans
}

/// Posición asignada por el cliente → rol del dominio. `""` es "sin asignar".
pub fn role_from_position(position: &str) -> Option<Role> {
    match position.to_ascii_uppercase().as_str() {
        "TOP" => Some(Role::Top),
        "JUNGLE" => Some(Role::Jungle),
        "MIDDLE" => Some(Role::Mid),
        "BOTTOM" => Some(Role::Adc),
        "UTILITY" => Some(Role::Support),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_fixture() -> serde_json::Value {
        serde_json::json!({
            "localPlayerCellId": 0,
            "myTeam": [
                {"cellId": 0, "team": 1, "assignedPosition": "TOP"},
                {"cellId": 1, "team": 1, "assignedPosition": "JUNGLE"},
                {"cellId": 2, "team": 1, "assignedPosition": "MIDDLE"},
                {"cellId": 3, "team": 1, "assignedPosition": "BOTTOM"},
                {"cellId": 4, "team": 1, "assignedPosition": "UTILITY"}
            ],
            "theirTeam": [
                {"cellId": 5, "team": 2, "assignedPosition": "TOP"},
                {"cellId": 6, "team": 2, "assignedPosition": "JUNGLE"}
            ],
            "actions": [
                [{"id": 1, "actorCellId": 0, "type": "ban", "championId": 103, "completed": true}],
                [{"id": 2, "actorCellId": 5, "type": "ban", "championId": 12, "completed": true}],
                [{"id": 3, "actorCellId": 1, "type": "ban", "championId": 34, "completed": true}],
                [{"id": 21, "actorCellId": 0, "type": "pick", "championId": 64, "completed": true}],
                [
                    {"id": 22, "actorCellId": 5, "type": "pick", "championId": 103, "completed": true},
                    {"id": 23, "actorCellId": 6, "type": "pick", "championId": 22, "completed": true}
                ],
                [{"id": 24, "actorCellId": 1, "type": "pick", "championId": 0, "completed": false, "isInProgress": true}],
                [{"id": 25, "actorCellId": 2, "type": "pick", "championId": 268, "completed": true}],
                [{"id": 26, "actorCellId": 3, "type": "pick", "championId": 0, "completed": false}],
                [{"id": 27, "actorCellId": 4, "type": "pick", "championId": 0, "completed": false}]
            ],
            "bans": {
                "myTeamBans": [103, 12, 34, 0, 0],
                "theirTeamBans": [99, 0, 0, 0, 0],
                "numBans": 10
            },
            "timer": {"phase": "BAN_PICK"}
        })
    }

    #[test]
    fn parses_a_lockfile() {
        let credentials = parse_lockfile("LeagueClient:4242:51234:abcDEF123==:https").unwrap();
        assert_eq!(credentials.port, 51234);
        assert_eq!(credentials.password, "abcDEF123==");
    }

    #[test]
    fn rejects_a_malformed_lockfile() {
        assert!(matches!(
            parse_lockfile("LeagueClient:4242:no-port:pass:https"),
            Err(LcuError::InvalidLockfile(_))
        ));
        assert!(matches!(
            parse_lockfile("LeagueClient:4242:51234::https"),
            Err(LcuError::InvalidLockfile(_))
        ));
        assert!(matches!(
            parse_lockfile("LeagueClient:4242:51234:pass:http"),
            Err(LcuError::InvalidLockfile(_))
        ));
    }

    #[test]
    fn reads_the_local_draft_from_a_session() {
        let draft = draft_from_session(&session_fixture()).unwrap();
        assert_eq!(draft.side, Side::Blue);
        assert_eq!(draft.slot, 1);
        assert_eq!(draft.role, Some(Role::Top));
        // Mi pick (64) no cuenta como aliado; el del compañero sí.
        assert_eq!(draft.own_pick, Some(ChampionKey(64)));
        assert_eq!(draft.ally_picks, vec![ChampionKey(268)]);
        assert_eq!(draft.enemy_picks, vec![ChampionKey(103), ChampionKey(22)]);
        assert_eq!(
            draft.bans,
            vec![
                ChampionKey(103),
                ChampionKey(12),
                ChampionKey(34),
                ChampionKey(99)
            ]
        );
        assert_eq!(draft.timer_phase.as_deref(), Some("BAN_PICK"));
    }

    #[test]
    fn reads_the_side_from_the_team_field() {
        let mut session = session_fixture();
        session["myTeam"][0]["team"] = serde_json::json!(2);
        session["theirTeam"][0]["team"] = serde_json::json!(1);
        let draft = draft_from_session(&session).unwrap();
        assert_eq!(draft.side, Side::Red);
        assert_eq!(draft.slot, 1);
        assert_eq!(draft.ally_picks, vec![ChampionKey(268)]);
        assert_eq!(draft.enemy_picks, vec![ChampionKey(103), ChampionKey(22)]);
    }

    #[test]
    fn bans_come_from_completed_actions_even_without_the_summary_object() {
        let mut session = session_fixture();
        session.as_object_mut().unwrap().remove("bans");
        let draft = draft_from_session(&session).unwrap();
        assert_eq!(
            draft.bans,
            vec![ChampionKey(103), ChampionKey(12), ChampionKey(34)]
        );
    }

    #[test]
    fn incomplete_ban_actions_are_not_tracked() {
        let mut session = session_fixture();
        session.as_object_mut().unwrap().remove("bans");
        session["actions"] = serde_json::json!([
            [{"id": 1, "actorCellId": 0, "type": "ban", "championId": 103, "completed": true}],
            [{"id": 2, "actorCellId": 5, "type": "ban", "championId": 12, "completed": false, "isInProgress": true}],
            [{"id": 3, "actorCellId": 1, "type": "ban", "championId": 0, "completed": true}],
            [{"id": 21, "actorCellId": 0, "type": "pick", "championId": 64, "completed": true}]
        ]);
        let draft = draft_from_session(&session).unwrap();
        assert_eq!(draft.bans, vec![ChampionKey(103)]);
    }

    #[test]
    fn later_pick_slots_are_detected_from_the_action_order() {
        let mut session = session_fixture();
        session["localPlayerCellId"] = serde_json::json!(3);
        let draft = draft_from_session(&session).unwrap();
        assert_eq!(draft.side, Side::Blue);
        assert_eq!(draft.slot, 4);
        assert_eq!(draft.role, Some(Role::Adc));
        // Ese jugador aún no ha bloqueado: no se le inventa un pick.
        assert_eq!(draft.own_pick, None);
        assert_eq!(draft.ally_picks, vec![ChampionKey(64), ChampionKey(268)]);
    }

    #[test]
    fn my_locked_pick_is_reported_once_the_action_completes() {
        let mut session = session_fixture();
        session["actions"][3][0]["completed"] = serde_json::json!(false);

        let draft = draft_from_session(&session).unwrap();
        assert_eq!(draft.own_pick, None);
        assert_eq!(draft.ally_picks, vec![ChampionKey(268)]);

        session["actions"][3][0]["completed"] = serde_json::json!(true);
        let draft = draft_from_session(&session).unwrap();
        assert_eq!(draft.own_pick, Some(ChampionKey(64)));
        assert_eq!(draft.ally_picks, vec![ChampionKey(268)]);
    }

    #[test]
    fn spectator_sessions_are_not_a_draft() {
        let mut session = session_fixture();
        session["localPlayerCellId"] = serde_json::json!(-1);
        assert!(matches!(
            draft_from_session(&session),
            Err(LcuError::NotInChampSelect)
        ));
    }

    #[test]
    fn a_session_without_my_pick_reports_it() {
        let mut session = session_fixture();
        session["actions"] = serde_json::json!([
            [{"id": 1, "actorCellId": 0, "type": "ban", "championId": 103, "completed": true}]
        ]);
        assert!(matches!(
            draft_from_session(&session),
            Err(LcuError::NoPickAction)
        ));
    }

    #[test]
    fn assigned_positions_map_to_roles() {
        assert_eq!(role_from_position("TOP"), Some(Role::Top));
        assert_eq!(role_from_position("middle"), Some(Role::Mid));
        assert_eq!(role_from_position("BOTTOM"), Some(Role::Adc));
        assert_eq!(role_from_position("UTILITY"), Some(Role::Support));
        assert_eq!(role_from_position("JUNGLE"), Some(Role::Jungle));
        assert_eq!(role_from_position(""), None);
    }
}
