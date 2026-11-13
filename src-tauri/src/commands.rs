use crate::dto::{
    AssignmentDto, BootstrapDto, ChampionDto, ChampionRefDto, CoverageDto, DamageProfileDto,
    DataStatusDto, DraftAnalysisDto, DraftStepViewDto, DraftViewDto, FormatDto, NeedDto,
    NextStepDto, PickViewDto, ProfileSetDto, SessionSummaryDto, SignalDto, TeamAnalysisDto,
    TeamViewDto, TeamsDto, WinConditionDto,
};
use crate::error::CommandError;
use crate::state::{new_session_id, summary_from_record, ActiveSession, AppState};
use loldraft_domain::{
    assign_roles_with_locks, ChampionCatalog, ChampionKey, DraftAction, DraftError, DraftFormat,
    Role, Side,
};
use loldraft_engine::{
    analyze_draft, AnalysisParams, Coverage, Dimension, NeedSource, NeedStatus, PickedChampion,
    ProfileProposalSet, Taxonomy, ThreatRuleId, WinConditionId, PILOT_PROFILE_PROPOSAL_JSON,
};
use loldraft_sources::DdragonClient;
use serde_json::Value;
use std::sync::MutexGuard;
use tauri::State;

const CODE_NOTHING_TO_UNDO: &str = "nothing_to_undo";
const CODE_NOTHING_TO_REDO: &str = "nothing_to_redo";
const CODE_NOT_PICKED: &str = "champion_not_picked";
const CODE_INVALID_ROLE: &str = "invalid_role";
const CODE_SESSION_NOT_FOUND: &str = "session_not_found";

#[tauri::command]
pub fn app_bootstrap(state: State<'_, AppState>) -> Result<BootstrapDto, CommandError> {
    let champions = {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        let mut champions: Vec<ChampionDto> = catalog.iter().map(ChampionDto::from).collect();
        champions.sort_by(|a, b| a.name.cmp(&b.name));
        champions
    };

    let data = state.data_status()?;

    let sessions = {
        let db = lock_db(&state)?;
        db.list_sessions()?
            .into_iter()
            .map(summary_from_record)
            .collect()
    };

    Ok(BootstrapDto {
        app_name: "LolDraft".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        format: FormatDto::from(&DraftFormat::standard()),
        data,
        champions,
        sessions,
    })
}

#[tauri::command]
pub fn champions_list(state: State<'_, AppState>) -> Result<Vec<ChampionDto>, CommandError> {
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
    let mut champions: Vec<ChampionDto> = catalog.iter().map(ChampionDto::from).collect();
    champions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(champions)
}

#[tauri::command]
pub fn data_status(state: State<'_, AppState>) -> Result<DataStatusDto, CommandError> {
    state.data_status()
}

#[tauri::command]
pub async fn data_sync_ddragon(state: State<'_, AppState>) -> Result<DataStatusDto, CommandError> {
    let client = DdragonClient::new();
    let version = client.latest_version().await?;
    let identities = client.champions(&version).await?;
    let fetched = identities.len();

    {
        let mut db = lock_db(&state)?;
        db.upsert_identities(&identities)?;
        db.set_setting("ddragon.version", &serde_json::to_string(&version)?)?;
        db.set_setting(
            "ddragon.last_sync_ms",
            &serde_json::to_string(&loldraft_data::now_ms())?,
        )?;
        db.set_setting("ddragon.source", &serde_json::to_string("Data Dragon")?)?;
    }

    state.reload_catalog()?;
    let mut status = state.data_status()?;
    if status.champion_count != fetched {
        status.warnings.push(format!(
            "Se sincronizaron {fetched} campeones, pero el catálogo local contiene {}.",
            status.champion_count
        ));
    }
    Ok(status)
}

#[tauri::command]
pub fn profiles_list(state: State<'_, AppState>) -> Result<Vec<ProfileSetDto>, CommandError> {
    let db = lock_db(&state)?;
    Ok(db
        .list_profile_sets()?
        .into_iter()
        .map(ProfileSetDto::from)
        .collect())
}

#[tauri::command]
pub fn profiles_import_pilot(
    state: State<'_, AppState>,
    activate: Option<bool>,
) -> Result<ProfileSetDto, CommandError> {
    import_proposal(
        &state,
        PILOT_PROFILE_PROPOSAL_JSON,
        activate.unwrap_or(false),
    )
}

#[tauri::command]
pub fn profiles_import_proposal(
    state: State<'_, AppState>,
    proposal_json: String,
    activate: Option<bool>,
) -> Result<ProfileSetDto, CommandError> {
    import_proposal(&state, &proposal_json, activate.unwrap_or(false))
}

#[tauri::command]
pub fn profiles_activate(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ProfileSetDto>, CommandError> {
    let mut db = lock_db(&state)?;
    db.set_active_profile_set(&id)?;
    Ok(db
        .list_profile_sets()?
        .into_iter()
        .map(ProfileSetDto::from)
        .collect())
}

#[tauri::command]
pub fn analysis_current(state: State<'_, AppState>) -> Result<DraftAnalysisDto, CommandError> {
    let (profile_set, profile_set_version) = {
        let db = lock_db(&state)?;
        let active = db.active_profile_set()?.ok_or_else(|| {
            CommandError::new(
                "profiles_not_loaded",
                "No hay un conjunto de perfiles activo. Carga y activa un conjunto para analizar.",
            )
        })?;
        let set = db.load_profile_set(&active.id)?;
        (set, active.version)
    };

    let (blue_picks, blue_skipped, red_picks, red_skipped) = {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        let guard = state
            .active
            .lock()
            .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
        let active = guard.as_ref().ok_or_else(CommandError::no_active_draft)?;
        let (blue, blue_skipped) = analysis_picks(Side::Blue, active, &catalog);
        let (red, red_skipped) = analysis_picks(Side::Red, active, &catalog);
        (blue, blue_skipped, red, red_skipped)
    };

    let params = AnalysisParams::default();
    let analysis = analyze_draft(&blue_picks, &red_picks, &profile_set, &params)?;
    let taxonomy = loldraft_engine::taxonomy();

    Ok(DraftAnalysisDto {
        params_version: analysis.params_version,
        engine_version: analysis.engine_version,
        profile_set_id: profile_set.id.clone(),
        profile_set_version,
        blue: team_analysis_dto(analysis.blue, blue_skipped, taxonomy),
        red: team_analysis_dto(analysis.red, red_skipped, taxonomy),
    })
}

fn import_proposal(
    state: &AppState,
    json: &str,
    activate: bool,
) -> Result<ProfileSetDto, CommandError> {
    let proposal = ProfileProposalSet::from_json(json)?;
    let set = proposal.into_profile_set(loldraft_engine::taxonomy())?;
    let id = set.id.clone();
    let mut db = lock_db(state)?;
    db.save_profile_set(&set)?;
    if activate {
        db.set_active_profile_set(&id)?;
    }
    db.list_profile_sets()?
        .into_iter()
        .find(|summary| summary.id == id)
        .map(ProfileSetDto::from)
        .ok_or_else(|| CommandError::internal("el conjunto importado no aparece en la base"))
}

fn analysis_picks(
    side: Side,
    active: &ActiveSession,
    catalog: &ChampionCatalog,
) -> (Vec<PickedChampion>, usize) {
    let picks = active.state().picks(side);
    let mut champions = Vec::with_capacity(picks.len());
    let mut found_indices = Vec::with_capacity(picks.len());
    for (index, key) in picks.iter().enumerate() {
        if let Some(champion) = catalog.get(*key) {
            champions.push(champion);
            found_indices.push(index);
        }
    }

    let locks: Vec<Option<Role>> = champions
        .iter()
        .map(|champion| active.overrides.get(&(side, champion.key)).copied())
        .collect();
    let assignment = assign_roles_with_locks(&champions, &locks);
    let mut picked = Vec::with_capacity(champions.len());
    let mut skipped = 0;
    for (assignment_index, _pick_index) in found_indices.iter().enumerate() {
        let Some(role) = assignment.roles[assignment_index] else {
            skipped += 1;
            continue;
        };
        let champion = champions[assignment_index];
        let assignment_confidence = champion
            .role_fit(role)
            .map(|fit| fit.confidence)
            .unwrap_or(0.5);
        picked.push(PickedChampion::new(
            champion.key,
            role,
            assignment_confidence,
        ));
    }
    (picked, skipped)
}

fn team_analysis_dto(
    analysis: loldraft_engine::TeamAnalysis,
    skipped_without_role: usize,
    taxonomy: &Taxonomy,
) -> TeamAnalysisDto {
    let mut archetypes: Vec<CoverageDto> = analysis
        .archetypes
        .iter()
        .map(|(id, coverage)| {
            coverage_dto(
                "archetype",
                id.as_str(),
                taxonomy
                    .archetype(*id)
                    .map(|entry| entry.label_es.clone())
                    .unwrap_or_else(|| id.as_str().to_string()),
                coverage,
            )
        })
        .collect();
    archetypes.sort_by(|left, right| {
        (right.value_low + right.value_high)
            .total_cmp(&(left.value_low + left.value_high))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut traits: Vec<CoverageDto> = analysis
        .traits
        .iter()
        .map(|(id, coverage)| {
            coverage_dto(
                "trait",
                id.as_str(),
                taxonomy
                    .trait_entry(*id)
                    .map(|entry| entry.label_es.clone())
                    .unwrap_or_else(|| id.as_str().to_string()),
                coverage,
            )
        })
        .collect();
    traits.sort_by(|left, right| {
        (right.value_low + right.value_high)
            .total_cmp(&(left.value_low + left.value_high))
            .then_with(|| left.id.cmp(&right.id))
    });

    let needs = analysis
        .needs
        .iter()
        .map(|need| NeedDto {
            id: need.dimension.as_str().to_string(),
            kind: dimension_kind(need.dimension),
            label: dimension_label(need.dimension, taxonomy),
            amount: need.amount,
            status: match need.status {
                NeedStatus::Confirmed => "confirmed",
                NeedStatus::Possible => "possible",
            },
            reasons: need.sources.iter().map(need_reason).collect(),
        })
        .collect();

    let win_conditions = analysis
        .win_conditions
        .iter()
        .map(|condition| WinConditionDto {
            id: win_condition_code(condition.id),
            label: win_condition_label(condition.id),
            score: condition.score,
            confirmed: condition.confirmed,
        })
        .collect();

    let strengths = analysis
        .strengths
        .iter()
        .map(|signal| signal_dto(signal, taxonomy))
        .collect();
    let weaknesses = analysis
        .weaknesses
        .iter()
        .map(|signal| signal_dto(signal, taxonomy))
        .collect();

    TeamAnalysisDto {
        side: analysis.side,
        picks: analysis.picks,
        skipped_without_role,
        partial: analysis.partial,
        knowledge: analysis.knowledge,
        confidence: analysis.confidence,
        archetypes,
        traits,
        damage: analysis.damage.map(|damage| DamageProfileDto {
            physical: damage.physical,
            magic: damage.magic,
            true_damage: damage.true_damage,
            known_weight_ratio: damage.known_weight_ratio,
            confidence: damage.confidence,
        }),
        needs,
        win_conditions,
        strengths,
        weaknesses,
    }
}

fn coverage_dto(kind: &'static str, id: &str, label: String, coverage: &Coverage) -> CoverageDto {
    CoverageDto {
        id: id.to_string(),
        kind,
        label,
        value_low: coverage.value_low,
        value_high: coverage.value_high,
        value_estimate: coverage.value_estimate,
        confidence: coverage.confidence,
        known: coverage.known,
        total: coverage.total,
    }
}

fn signal_dto(signal: &loldraft_engine::Signal, taxonomy: &Taxonomy) -> SignalDto {
    SignalDto {
        id: signal.dimension.as_str().to_string(),
        kind: dimension_kind(signal.dimension),
        label: dimension_label(signal.dimension, taxonomy),
        value_low: signal.value_low,
        value_high: signal.value_high,
        confidence: signal.confidence,
    }
}

fn dimension_kind(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::Archetype(_) => "archetype",
        Dimension::Trait(_) => "trait",
    }
}

fn dimension_label(dimension: Dimension, taxonomy: &Taxonomy) -> String {
    match dimension {
        Dimension::Archetype(id) => taxonomy
            .archetype(id)
            .map(|entry| entry.label_es.clone())
            .unwrap_or_else(|| id.as_str().to_string()),
        Dimension::Trait(id) => taxonomy
            .trait_entry(id)
            .map(|entry| entry.label_es.clone())
            .unwrap_or_else(|| id.as_str().to_string()),
    }
}

fn need_reason(source: &NeedSource) -> String {
    match source {
        NeedSource::Baseline { floor } => format!("suelo {:.0} %", floor * 100.0),
        NeedSource::EnemyThreat { rule, .. } => {
            format!("amenaza: {}", threat_rule_label(*rule))
        }
        NeedSource::DamageBalance { share, threshold } => format!(
            "balance de daño: {:.0} % frente a {:.0} %",
            share * 100.0,
            threshold * 100.0
        ),
    }
}

fn threat_rule_label(rule: ThreatRuleId) -> &'static str {
    match rule {
        ThreatRuleId::EnemyEngage => "engage enemigo",
        ThreatRuleId::EnemyEngageFrontline => "engage y frontline enemigos",
        ThreatRuleId::EnemyPoke => "poke enemigo",
        ThreatRuleId::EnemySiege => "asedio enemigo",
        ThreatRuleId::EnemyFrontline => "frontline enemigo",
        ThreatRuleId::EnemyAssassin => "assassin/burst enemigo",
        ThreatRuleId::EnemyCatch => "catch enemigo",
        ThreatRuleId::EnemySplitpush => "splitpush enemigo",
        ThreatRuleId::EnemyWombo => "wombo combo enemigo",
        ThreatRuleId::EnemyEarly => "early game enemigo",
        ThreatRuleId::EnemyScaling => "escalado enemigo",
    }
}

fn win_condition_code(id: WinConditionId) -> &'static str {
    match id {
        WinConditionId::FrontToBack => "front_to_back",
        WinConditionId::Dive => "dive",
        WinConditionId::PokeSiege => "poke_siege",
        WinConditionId::Pick => "pick",
        WinConditionId::Splitpush => "splitpush",
        WinConditionId::Scaling => "scaling",
        WinConditionId::EarlySnowball => "early_snowball",
        WinConditionId::Wombo => "wombo",
    }
}

fn win_condition_label(id: WinConditionId) -> &'static str {
    match id {
        WinConditionId::FrontToBack => "Front to back",
        WinConditionId::Dive => "Dive / engage",
        WinConditionId::PokeSiege => "Poke y asedio",
        WinConditionId::Pick => "Pick / catch",
        WinConditionId::Splitpush => "Splitpush",
        WinConditionId::Scaling => "Escalado",
        WinConditionId::EarlySnowball => "Snowball temprano",
        WinConditionId::Wombo => "Wombo combo",
    }
}

#[tauri::command]
pub fn draft_new(state: State<'_, AppState>, side: Side) -> Result<DraftViewDto, CommandError> {
    let active = ActiveSession::new(new_session_id(), side);
    persist_session(&state, &active)?;

    let view = {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        view_from(&active, &catalog)
    };

    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    *guard = Some(active);
    Ok(view)
}

#[tauri::command]
pub fn draft_action(
    state: State<'_, AppState>,
    champion_key: u16,
) -> Result<DraftViewDto, CommandError> {
    let key = ChampionKey(champion_key);
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;

    if !catalog.contains(key) {
        return Err(DraftError::UnknownChampion(key).into());
    }

    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    let active = guard.as_mut().ok_or_else(CommandError::no_active_draft)?;

    let step = active
        .state()
        .next_step()
        .copied()
        .ok_or(DraftError::Complete)?;
    let action = DraftAction::new(step.side, step.kind, key);
    active.session.apply_in(action, &catalog)?;

    let view = view_from(active, &catalog);
    drop(catalog);
    persist_session(&state, active)?;
    Ok(view)
}

#[tauri::command]
pub fn draft_undo(state: State<'_, AppState>) -> Result<DraftViewDto, CommandError> {
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    let active = guard.as_mut().ok_or_else(CommandError::no_active_draft)?;

    if !active.session.undo() {
        return Err(CommandError::new(
            CODE_NOTHING_TO_UNDO,
            "No hay ninguna acción que deshacer.",
        ));
    }

    let view = view_from(active, &catalog);
    drop(catalog);
    persist_session(&state, active)?;
    Ok(view)
}

#[tauri::command]
pub fn draft_redo(state: State<'_, AppState>) -> Result<DraftViewDto, CommandError> {
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    let active = guard.as_mut().ok_or_else(CommandError::no_active_draft)?;

    if !active.session.redo() {
        return Err(CommandError::new(
            CODE_NOTHING_TO_REDO,
            "No hay ninguna acción que rehacer.",
        ));
    }

    let view = view_from(active, &catalog);
    drop(catalog);
    persist_session(&state, active)?;
    Ok(view)
}

#[tauri::command]
pub fn draft_set_role(
    state: State<'_, AppState>,
    champion_key: u16,
    role: Option<Role>,
) -> Result<DraftViewDto, CommandError> {
    let key = ChampionKey(champion_key);
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
    let champion = catalog.get(key).ok_or(DraftError::UnknownChampion(key))?;

    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    let active = guard.as_mut().ok_or_else(CommandError::no_active_draft)?;

    let side = active.state().side_of_pick(key).ok_or_else(|| {
        CommandError::new(
            CODE_NOT_PICKED,
            "Solo se puede fijar el rol de un campeón ya picado.",
        )
    })?;

    match role {
        Some(role) => {
            if champion.role_fit(role).is_none() {
                return Err(CommandError::new(
                    CODE_INVALID_ROLE,
                    format!(
                        "{} no tiene un rol {} registrado en el catálogo.",
                        champion.name,
                        role.label_es()
                    ),
                ));
            }
            active.overrides.insert((side, key), role);
        }
        None => {
            active.overrides.remove(&(side, key));
        }
    }

    let view = view_from(active, &catalog);
    drop(catalog);
    persist_session(&state, active)?;
    Ok(view)
}

#[tauri::command]
pub fn draft_list(state: State<'_, AppState>) -> Result<Vec<SessionSummaryDto>, CommandError> {
    let db = lock_db(&state)?;
    Ok(db
        .list_sessions()?
        .into_iter()
        .map(summary_from_record)
        .collect())
}

#[tauri::command]
pub fn draft_load(state: State<'_, AppState>, id: String) -> Result<DraftViewDto, CommandError> {
    let record = {
        let db = lock_db(&state)?;
        db.load_session(&id)?.ok_or_else(|| {
            CommandError::new(CODE_SESSION_NOT_FOUND, "La sesión solicitada no existe.")
        })?
    };

    let active = ActiveSession::restore(&record)?;
    let view = {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        view_from(&active, &catalog)
    };

    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    *guard = Some(active);
    Ok(view)
}

#[tauri::command]
pub fn draft_delete(state: State<'_, AppState>, id: String) -> Result<bool, CommandError> {
    let deleted = {
        let db = lock_db(&state)?;
        db.delete_session(&id)?
    };

    let mut guard = state
        .active
        .lock()
        .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
    if guard.as_ref().is_some_and(|active| active.id == id) {
        *guard = None;
    }
    Ok(deleted)
}

#[tauri::command]
pub fn settings_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<Value>, CommandError> {
    let db = lock_db(&state)?;
    let raw = db.get_setting(&key)?;
    Ok(raw.map(|raw| serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw))))
}

#[tauri::command]
pub fn settings_update(
    state: State<'_, AppState>,
    key: String,
    value: Value,
) -> Result<(), CommandError> {
    if key.is_empty() || key.len() > 128 {
        return Err(CommandError::new(
            "invalid_setting_key",
            "La clave de ajuste no es válida.",
        ));
    }
    let db = lock_db(&state)?;
    db.set_setting(&key, &serde_json::to_string(&value)?)?;
    Ok(())
}

fn lock_db(state: &AppState) -> Result<MutexGuard<'_, loldraft_data::Database>, CommandError> {
    state
        .db
        .lock()
        .map_err(|_| CommandError::internal("la base de datos está bloqueada"))
}

fn persist_session(state: &AppState, active: &ActiveSession) -> Result<(), CommandError> {
    let json = active.stored_json()?;
    let db = lock_db(state)?;
    db.save_session(
        &active.id,
        &active.state().format.id,
        active.session.side(),
        &json,
        active.state().actions().len(),
    )?;
    Ok(())
}

fn view_from(active: &ActiveSession, catalog: &ChampionCatalog) -> DraftViewDto {
    let state = active.state();
    let actions = state.actions();
    let current = actions.len();
    let total = state.format.steps.len();

    let steps = state
        .format
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let champion = actions
                .get(index)
                .and_then(|action| catalog.get(action.champion_key))
                .map(ChampionRefDto::from);
            let status = if index < current {
                "done"
            } else if index == current && current < total {
                "current"
            } else {
                "future"
            };
            DraftStepViewDto {
                index,
                number: index + 1,
                side: step.side,
                kind: step.kind,
                status,
                champion,
            }
        })
        .collect();

    let next_step = state.next_step().map(|step| NextStepDto {
        index: current,
        number: current + 1,
        side: step.side,
        kind: step.kind,
    });

    DraftViewDto {
        id: active.id.clone(),
        format_id: state.format.id.clone(),
        format_name: state.format.name.clone(),
        side: active.session.side(),
        steps,
        next_step,
        is_complete: state.is_complete(),
        can_undo: active.session.can_undo(),
        can_redo: active.session.can_redo(),
        action_count: current,
        total_actions: total,
        teams: TeamsDto {
            blue: team_view(Side::Blue, active, catalog),
            red: team_view(Side::Red, active, catalog),
        },
    }
}

fn team_view(side: Side, active: &ActiveSession, catalog: &ChampionCatalog) -> TeamViewDto {
    let picks = active.state().picks(side);
    let mut champions = Vec::with_capacity(picks.len());
    let mut found_indices = Vec::with_capacity(picks.len());

    for (index, key) in picks.iter().enumerate() {
        if let Some(champion) = catalog.get(*key) {
            champions.push(champion);
            found_indices.push(index);
        }
    }

    let locks: Vec<Option<Role>> = champions
        .iter()
        .map(|champion| active.overrides.get(&(side, champion.key)).copied())
        .collect();
    let assignment = assign_roles_with_locks(&champions, &locks);

    let mut roles_by_pick = vec![None; picks.len()];
    let mut source_by_pick = vec!["unassigned"; picks.len()];
    let mut roles = vec![None; picks.len()];

    for (assignment_index, pick_index) in found_indices.iter().enumerate() {
        let role = assignment.roles[assignment_index];
        roles_by_pick[*pick_index] = role;
        roles[*pick_index] = role;
        if role.is_some() {
            let champion = champions[assignment_index];
            source_by_pick[*pick_index] = if active.overrides.contains_key(&(side, champion.key)) {
                "manual"
            } else {
                "auto"
            };
        }
    }

    let pick_views = picks
        .iter()
        .enumerate()
        .filter_map(|(index, key)| {
            let champion = catalog.get(*key)?;
            let role = roles_by_pick[index];
            Some(PickViewDto {
                champion: ChampionRefDto::from(champion),
                role,
                role_confidence: role
                    .and_then(|role| champion.role_fit(role))
                    .map(|fit| fit.confidence),
                role_source: source_by_pick[index],
                role_locked: active.overrides.contains_key(&(side, *key)),
                available_roles: champion.roles.iter().map(|fit| fit.role).collect(),
            })
        })
        .collect();

    let bans = active
        .state()
        .bans(side)
        .iter()
        .filter_map(|key| catalog.get(*key).map(ChampionRefDto::from))
        .collect();

    TeamViewDto {
        side,
        picks: pick_views,
        bans,
        assignment: AssignmentDto {
            roles,
            score: assignment.score,
            confidence: assignment.confidence,
            assigned: assignment.assigned,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loldraft_data::{default_seed, Database};

    fn catalog() -> ChampionCatalog {
        let mut db = Database::open_in_memory().expect("db");
        db.seed_champions(&default_seed().expect("seed"))
            .expect("seed");
        db.load_catalog().expect("catalog")
    }

    #[test]
    fn completes_a_full_standard_draft_and_builds_both_teams() {
        let catalog = catalog();
        let format = DraftFormat::standard();
        let mut active = ActiveSession::new("test-draft".to_string(), Side::Blue);
        let mut keys = catalog.iter().map(|champion| champion.key);

        for step in &format.steps {
            let key = keys.next().expect("enough champions");
            active
                .session
                .apply_in(DraftAction::new(step.side, step.kind, key), &catalog)
                .expect("valid action");
        }

        let view = view_from(&active, &catalog);
        assert!(view.is_complete);
        assert!(view.next_step.is_none());
        assert_eq!(view.action_count, 20);
        assert_eq!(view.steps.len(), 20);
        assert!(view.steps.iter().all(|step| step.status == "done"));
        assert_eq!(view.teams.blue.picks.len(), 5);
        assert_eq!(view.teams.red.picks.len(), 5);
        assert_eq!(view.teams.blue.bans.len(), 5);
        assert_eq!(view.teams.red.bans.len(), 5);
        assert!(view.teams.blue.assignment.assigned <= 5);
    }

    #[test]
    fn manual_role_override_wins_over_the_automatic_assignment() {
        let catalog = catalog();
        let flex = catalog
            .iter()
            .find(|champion| champion.roles.len() >= 2)
            .expect("a flex champion");
        let primary = flex.primary_role().expect("primary role");
        let forced = flex
            .roles
            .iter()
            .map(|fit| fit.role)
            .find(|role| *role != primary)
            .expect("a second role");

        let format = DraftFormat::standard();
        let mut active = ActiveSession::new("test-override".to_string(), Side::Blue);
        let bans: Vec<_> = catalog
            .iter()
            .filter(|champion| champion.key != flex.key)
            .take(6)
            .map(|champion| champion.key)
            .collect();

        for (index, step) in format.steps.iter().take(6).enumerate() {
            active
                .session
                .apply_in(
                    DraftAction::new(step.side, step.kind, bans[index]),
                    &catalog,
                )
                .expect("valid ban");
        }
        active
            .session
            .apply_in(
                DraftAction::new(Side::Blue, loldraft_domain::ActionKind::Pick, flex.key),
                &catalog,
            )
            .expect("valid pick");
        active.overrides.insert((Side::Blue, flex.key), forced);

        let view = view_from(&active, &catalog);
        let pick = &view.teams.blue.picks[0];
        assert_eq!(pick.role, Some(forced));
        assert_eq!(pick.role_source, "manual");
        assert!(pick.role_locked);
    }

    #[test]
    fn unassigned_pick_reports_no_invented_role() {
        let catalog = catalog();
        let format = DraftFormat::standard();
        let mut active = ActiveSession::new("test-unassigned".to_string(), Side::Blue);

        let no_role = catalog
            .iter()
            .find(|champion| champion.roles.is_empty())
            .expect("catalog keeps champions without imported roles");
        let bans: Vec<_> = catalog
            .iter()
            .filter(|champion| champion.key != no_role.key)
            .take(6)
            .map(|champion| champion.key)
            .collect();

        for (index, step) in format.steps.iter().take(6).enumerate() {
            active
                .session
                .apply_in(
                    DraftAction::new(step.side, step.kind, bans[index]),
                    &catalog,
                )
                .expect("valid ban");
        }
        active
            .session
            .apply_in(
                DraftAction::new(Side::Blue, loldraft_domain::ActionKind::Pick, no_role.key),
                &catalog,
            )
            .expect("valid pick");

        let view = view_from(&active, &catalog);
        let pick = &view.teams.blue.picks[0];
        assert_eq!(pick.role, None);
        assert_eq!(pick.role_source, "unassigned");
        assert_eq!(pick.role_confidence, None);
    }
}
