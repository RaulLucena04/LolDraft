use crate::dto::{
    AdvantageDto, AssignmentDto, BanContributionDto, BanRecommendationDto, BootstrapDto,
    ChampionDto, ChampionProfileContextDto, ChampionProfileDto, ChampionRefDto, ContributionDto,
    CoverageDto, DamageProfileDto, DataStatusDto, DraftAnalysisDto, DraftStepViewDto, DraftViewDto,
    FormatDto, LaneMatchupDto, NeedDto, NextStepDto, PickViewDto, ProfileDamageDto,
    ProfileProvenanceDto, ProfileRatingDto, ProfileRoleDto, ProfileSetDto, ReasonDto,
    RecommendationDto, RecommendationSetDto, RiskContributionDto, SessionSummaryDto, SignalDto,
    SoloqDraftInput, SoloqImportDto, SynergyChampionDto, SynergyDto, TeamAnalysisDto,
    TeamMatchupDto, TeamViewDto, TeamsDto, UpdateCheckDto, UpdateInstallDto, WinConditionDto,
};
use crate::error::CommandError;
use crate::state::{new_session_id, summary_from_record, ActiveSession, AppState};
use crate::updates::{self, Installer};
use loldraft_domain::{
    assign_roles_avoiding, assign_roles_with_locks, ChampionCatalog, ChampionKey, DraftAction,
    DraftError, DraftFormat, Role, Side,
};
use loldraft_engine::{
    analyze_draft, candidate_synergies, recommend, Advantage, AnalysisParams, ArchetypeId,
    BanRecommendation, Coverage, Dimension, LaneMatchup, MatchupRuleId, NeedSource, NeedStatus,
    PickedChampion, ProfileProposalSet, ProfileSet, Provenance, Reason, Recommendation,
    RecommendationParams, RecommendationRequest, RecommendationSet, SynergyId, Taxonomy,
    TeamMatchup, ThreatRuleId, TraitId, WinConditionId, PILOT_PROFILE_PROPOSAL_JSON,
};
use loldraft_sources::DdragonClient;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::MutexGuard;
use tauri::{Manager, State};

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
pub fn profiles_import_set(
    state: State<'_, AppState>,
    set_json: String,
    activate: Option<bool>,
) -> Result<ProfileSetDto, CommandError> {
    let set = ProfileSet::from_json(&set_json, loldraft_engine::taxonomy())?;
    save_profile_set(&state, &set, activate.unwrap_or(false))
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
    let (profile_set, profile_set_version) = active_profile_set(&state)?;

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
    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;

    let lane_matchups = analysis
        .lane_matchups
        .iter()
        .map(|matchup| lane_matchup_dto(matchup, &catalog))
        .collect();
    let matchups = analysis
        .matchups
        .iter()
        .map(|matchup| team_matchup_dto(matchup, taxonomy))
        .collect();

    Ok(DraftAnalysisDto {
        params_version: analysis.params_version,
        interactions_version: analysis.interactions_version,
        engine_version: analysis.engine_version,
        profile_set_id: profile_set.id.clone(),
        profile_set_version,
        blue: team_analysis_dto(analysis.blue, blue_skipped, taxonomy, &catalog),
        red: team_analysis_dto(analysis.red, red_skipped, taxonomy, &catalog),
        lane_matchups,
        matchups,
    })
}

#[tauri::command]
pub fn recommendations_current(
    state: State<'_, AppState>,
) -> Result<RecommendationSetDto, CommandError> {
    let (profile_set, profile_set_version) = active_profile_set(&state)?;

    let (format, action_index, action_number, blue_picks, red_picks, used) = {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        let guard = state
            .active
            .lock()
            .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
        let active = guard.as_ref().ok_or_else(CommandError::no_active_draft)?;
        let draft = active.state();
        if draft.is_complete() {
            return Err(CommandError::new(
                "draft_complete",
                "El draft ya está completo; no quedan acciones que recomendar.",
            ));
        }
        let (blue, _) = analysis_picks(Side::Blue, active, &catalog);
        let (red, _) = analysis_picks(Side::Red, active, &catalog);
        (
            draft.format.clone(),
            draft.current_step_index(),
            draft.next_step_number(),
            blue,
            red,
            draft.used_champions(),
        )
    };

    let analysis_params = AnalysisParams::default();
    let params = RecommendationParams::default();
    let output = recommend(&RecommendationRequest {
        format: &format,
        action_index,
        blue_picks: &blue_picks,
        red_picks: &red_picks,
        used: &used,
        profiles: &profile_set,
        analysis_params: &analysis_params,
        params: &params,
        fixed_role: None,
    })?;

    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
    Ok(recommendation_set_dto(
        output,
        action_number,
        profile_set_version,
        loldraft_engine::taxonomy(),
        &catalog,
    ))
}

/// Recomendación para un draft de SoloQ introducido a mano.
///
/// No lee ni escribe la sesión persistida: recibe el estado que el jugador ve
/// en el cliente y evalúa solo el pick indicado con el formato de ranked.
#[tauri::command]
pub fn soloq_recommend(
    state: State<'_, AppState>,
    draft: SoloqDraftInput,
) -> Result<RecommendationSetDto, CommandError> {
    if !(1..=5).contains(&draft.slot) {
        return Err(CommandError::new(
            "soloq_invalid_slot",
            "La posición de pick debe estar entre 1 y 5.",
        ));
    }

    let format = DraftFormat::soloq();
    let action_index = format
        .pick_action_index(draft.side, draft.slot as usize)
        .ok_or_else(|| {
            CommandError::new(
                "soloq_invalid_slot",
                "Esa posición de pick no existe en el formato de SoloQ.",
            )
        })?;

    validate_unique_champions(&draft)?;
    let (profile_set, profile_set_version) = active_profile_set(&state)?;

    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;

    let keys =
        |values: &[u16]| -> Vec<ChampionKey> { values.iter().copied().map(ChampionKey).collect() };
    let bans = keys(&draft.bans);
    let ally_keys = keys(&draft.ally_picks);
    let enemy_keys = keys(&draft.enemy_picks);

    for key in bans.iter().chain(ally_keys.iter()).chain(enemy_keys.iter()) {
        if catalog.get(*key).is_none() {
            return Err(CommandError::new(
                "soloq_unknown_champion",
                format!(
                    "El campeón #{} no existe en el catálogo local. Actualiza los datos para usarlo.",
                    key.0
                ),
            ));
        }
    }

    let overrides: HashMap<(Side, ChampionKey), Role> = HashMap::new();
    let reserved: Vec<Role> = draft.role.into_iter().collect();
    let (ally_picks, _) = picks_from_keys(&ally_keys, &catalog, &overrides, draft.side, &reserved);
    let (enemy_picks, _) = picks_from_keys(
        &enemy_keys,
        &catalog,
        &overrides,
        draft.side.opposite(),
        &[],
    );

    let (blue_picks, red_picks) = match draft.side {
        Side::Blue => (ally_picks, enemy_picks),
        Side::Red => (enemy_picks, ally_picks),
    };

    let mut used: Vec<ChampionKey> =
        Vec::with_capacity(bans.len() + ally_keys.len() + enemy_keys.len());
    used.extend(bans);
    used.extend(ally_keys);
    used.extend(enemy_keys);
    used.sort_by_key(|key| key.0);
    used.dedup();

    let analysis_params = AnalysisParams::default();
    let params = RecommendationParams::default();
    let output = recommend(&RecommendationRequest {
        format: &format,
        action_index,
        blue_picks: &blue_picks,
        red_picks: &red_picks,
        used: &used,
        profiles: &profile_set,
        analysis_params: &analysis_params,
        params: &params,
        fixed_role: draft.role,
    })?;

    Ok(recommendation_set_dto(
        output,
        None,
        profile_set_version,
        loldraft_engine::taxonomy(),
        &catalog,
    ))
}

/// Lee la selección de campeones del cliente de League of Legends.
///
/// Solo lectura: no modifica el draft del cliente ni envía acciones. Está
/// marcada como experimental porque depende de la API local no documentada.
#[tauri::command]
pub async fn lcu_champ_select(state: State<'_, AppState>) -> Result<SoloqImportDto, CommandError> {
    let path = loldraft_sources::find_lockfile()?;
    let text = std::fs::read_to_string(&path).map_err(|error| {
        CommandError::new(
            "lcu_unreachable",
            format!("No se pudo leer el lockfile del cliente: {error}"),
        )
    })?;
    let credentials = loldraft_sources::parse_lockfile(&text)?;
    let client = loldraft_sources::LcuClient::new(credentials)?;
    let session = client.champ_select().await?;

    let catalog = state
        .catalog
        .read()
        .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;

    let mut unknown: Vec<u16> = Vec::new();
    let known = |keys: Vec<ChampionKey>, unknown: &mut Vec<u16>| -> Vec<u16> {
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            if catalog.get(key).is_some() {
                out.push(key.0);
            } else if !unknown.contains(&key.0) {
                unknown.push(key.0);
            }
        }
        out
    };

    let bans = known(session.bans, &mut unknown);
    let ally_picks = known(session.ally_picks, &mut unknown);
    let enemy_picks = known(session.enemy_picks, &mut unknown);

    Ok(SoloqImportDto {
        side: session.side,
        slot: session.slot,
        role: session.role,
        bans,
        ally_picks,
        enemy_picks,
        timer_phase: session.timer_phase,
        unknown_champions: unknown,
    })
}

#[tauri::command]
pub fn profile_champion(
    state: State<'_, AppState>,
    champion_key: u16,
) -> Result<ChampionProfileDto, CommandError> {
    let key = ChampionKey(champion_key);
    let (profile_set, set_version) = {
        let db = lock_db(&state)?;
        let active = db.active_profile_set()?.ok_or_else(|| {
            CommandError::new(
                "profiles_not_loaded",
                "No hay un conjunto de perfiles activo. Carga y activa un conjunto para ver perfiles.",
            )
        })?;
        let set = db.load_profile_set(&active.id)?;
        (set, active.version)
    };
    let profile = profile_set.get(key).ok_or_else(|| {
        CommandError::new(
            "profile_not_available",
            format!(
                "El campeón #{} no tiene perfil en el conjunto activo; no se inventan celdas.",
                key.0
            ),
        )
    })?;
    let taxonomy = loldraft_engine::taxonomy();

    let mut context = ChampionProfileContextDto {
        has_draft: false,
        side: None,
        role: None,
        picked: false,
        teammates: 0,
    };
    let mut synergies = Vec::new();
    {
        let catalog = state
            .catalog
            .read()
            .map_err(|_| CommandError::internal("el catálogo está bloqueado"))?;
        let guard = state
            .active
            .lock()
            .map_err(|_| CommandError::internal("la sesión activa está bloqueada"))?;
        if let Some(active) = guard.as_ref() {
            context.has_draft = true;
            let picked_blue = active.state().picks(Side::Blue).contains(&key);
            let picked_red = active.state().picks(Side::Red).contains(&key);
            let side = if picked_blue {
                Side::Blue
            } else if picked_red {
                Side::Red
            } else {
                active.session.side()
            };
            context.side = Some(side);
            context.picked = picked_blue || picked_red;

            let mut teammates = analysis_picks(side, active, &catalog).0;
            let assigned_role = teammates
                .iter()
                .find(|pick| pick.key == key)
                .map(|pick| pick.role);
            // Un campeón sin pick usa su mejor rol del perfil como referencia;
            // la UI lo etiqueta como rol de referencia, no como asignado.
            let role = assigned_role.or_else(|| {
                profile
                    .roles
                    .iter()
                    .max_by(|left, right| left.proficiency.total_cmp(&right.proficiency))
                    .map(|role| role.role)
            });
            teammates.retain(|pick| pick.key != key);
            context.role = role;
            context.teammates = teammates.len();
            if let Some(role) = role {
                let assignment_confidence = profile
                    .roles
                    .iter()
                    .find(|candidate| candidate.role == role)
                    .map(|candidate| candidate.confidence)
                    .unwrap_or(0.0);
                let candidate = PickedChampion::new(key, role, assignment_confidence);
                synergies = candidate_synergies(&teammates, candidate, &profile_set)
                    .iter()
                    .map(|synergy| synergy_dto(synergy, taxonomy, &catalog))
                    .collect();
            }
        }
    }

    let roles: Vec<ProfileRoleDto> = Role::ALL
        .iter()
        .filter_map(|role| {
            profile
                .roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .map(|candidate| ProfileRoleDto {
                    role: candidate.role,
                    proficiency: candidate.proficiency,
                    confidence: candidate.confidence,
                    provenance: provenance_dto(&candidate.source),
                })
        })
        .collect();

    let mut archetypes: Vec<ProfileRatingDto> = profile
        .archetypes
        .iter()
        .filter_map(|(id, rating)| {
            let value = rating.intensity?;
            Some(ProfileRatingDto {
                id: id.as_str().to_string(),
                label: taxonomy
                    .archetype(*id)
                    .map(|entry| entry.label_es.clone())
                    .unwrap_or_else(|| id.as_str().to_string()),
                value,
                confidence: rating.confidence,
                provenance: provenance_dto(&rating.source),
            })
        })
        .collect();
    archetypes.sort_by(|left, right| {
        right
            .value
            .total_cmp(&left.value)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut traits: Vec<ProfileRatingDto> = profile
        .traits
        .iter()
        .filter_map(|(id, rating)| {
            if id.is_damage_share() {
                return None;
            }
            let value = rating.value?;
            Some(ProfileRatingDto {
                id: id.as_str().to_string(),
                label: taxonomy
                    .trait_entry(*id)
                    .map(|entry| entry.label_es.clone())
                    .unwrap_or_else(|| id.as_str().to_string()),
                value,
                confidence: rating.confidence,
                provenance: provenance_dto(&rating.source),
            })
        })
        .collect();
    traits.sort_by(|left, right| {
        right
            .value
            .total_cmp(&left.value)
            .then_with(|| left.id.cmp(&right.id))
    });

    let damage = profile.damage_shares().map(|shares| ProfileDamageDto {
        physical: shares.physical,
        magic: shares.magic,
        true_damage: shares.true_damage,
    });

    let mut observed_cells = 0;
    let mut inferred_cells = 0;
    for rating in profile
        .archetypes
        .values()
        .filter(|rating| rating.intensity.is_some())
    {
        if rating.source.is_observed() {
            observed_cells += 1;
        } else {
            inferred_cells += 1;
        }
    }
    for rating in profile
        .traits
        .values()
        .filter(|rating| rating.value.is_some())
    {
        if rating.source.is_observed() {
            observed_cells += 1;
        } else {
            inferred_cells += 1;
        }
    }
    let total_cells = ArchetypeId::ALL.len() + TraitId::ALL.len();
    let unknown_cells = total_cells.saturating_sub(observed_cells + inferred_cells);

    Ok(ChampionProfileDto {
        key: key.0,
        name: profile.name.clone(),
        slug: profile.slug.clone(),
        set_id: profile_set.id.clone(),
        set_version,
        taxonomy_version: profile_set.taxonomy_version.clone(),
        patch: profile_set.patch.clone(),
        roles,
        archetypes,
        traits,
        damage,
        known_cell_ratio: profile.known_cell_ratio(),
        observed_cells,
        inferred_cells,
        unknown_cells,
        context,
        synergies,
    })
}

fn provenance_dto(source: &Provenance) -> ProfileProvenanceDto {
    ProfileProvenanceDto {
        code: source.code(),
        label: source.label_es(),
        detail: source.detail(),
    }
}

fn import_proposal(
    state: &AppState,
    json: &str,
    activate: bool,
) -> Result<ProfileSetDto, CommandError> {
    let proposal = ProfileProposalSet::from_json(json)?;
    let set = proposal.into_profile_set(loldraft_engine::taxonomy())?;
    save_profile_set(state, &set, activate)
}

fn save_profile_set(
    state: &AppState,
    set: &ProfileSet,
    activate: bool,
) -> Result<ProfileSetDto, CommandError> {
    let id = set.id.clone();
    let mut db = lock_db(state)?;
    db.save_profile_set(set)?;
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
    picks_from_keys(
        &active.state().picks(side),
        catalog,
        &active.overrides,
        side,
        &[],
    )
}

/// Asigna roles a una lista de campeones ya bloqueados.
///
/// `reserved` deja libres esos roles, para evaluar un pick propio pendiente sin
/// que los aliados lo ocupen por heurística. Los campeones sin rol viable se
/// omiten y se cuentan como `skipped`: nunca se les inventa un rol.
fn picks_from_keys(
    keys: &[ChampionKey],
    catalog: &ChampionCatalog,
    overrides: &HashMap<(Side, ChampionKey), Role>,
    side: Side,
    reserved: &[Role],
) -> (Vec<PickedChampion>, usize) {
    let mut champions = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(champion) = catalog.get(*key) {
            champions.push(champion);
        }
    }

    let locks: Vec<Option<Role>> = champions
        .iter()
        .map(|champion| overrides.get(&(side, champion.key)).copied())
        .collect();
    let assignment = assign_roles_avoiding(&champions, &locks, reserved);
    let mut picked = Vec::with_capacity(champions.len());
    let mut skipped = 0;
    for (index, champion) in champions.iter().enumerate() {
        let Some(role) = assignment.roles[index] else {
            skipped += 1;
            continue;
        };
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
    catalog: &ChampionCatalog,
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
    let synergies = analysis
        .synergies
        .iter()
        .map(|synergy| synergy_dto(synergy, taxonomy, catalog))
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
        synergies,
        strengths,
        weaknesses,
    }
}

fn synergy_dto(
    synergy: &loldraft_engine::Synergy,
    taxonomy: &Taxonomy,
    catalog: &ChampionCatalog,
) -> SynergyDto {
    SynergyDto {
        id: synergy.id.as_str(),
        label: synergy_label(synergy.id),
        champions: synergy
            .champions
            .iter()
            .map(|key| synergy_champion(*key, catalog))
            .collect(),
        score_low: synergy.score_low,
        score_high: synergy.score_high,
        confidence: synergy.confidence,
        evidence: synergy
            .evidence
            .iter()
            .map(|dimension| dimension_label(*dimension, taxonomy))
            .collect(),
    }
}

fn synergy_champion(key: ChampionKey, catalog: &ChampionCatalog) -> SynergyChampionDto {
    SynergyChampionDto {
        key: key.0,
        name: catalog
            .get(key)
            .map(|champion| champion.name.clone())
            .unwrap_or_else(|| format!("Campeón #{}", key.0)),
    }
}

fn advantage_dto(advantage: &Advantage) -> AdvantageDto {
    AdvantageDto {
        low: advantage.low,
        high: advantage.high,
        confidence: advantage.confidence,
        known: advantage.known,
        total: advantage.total,
        favored: advantage.favored,
    }
}

fn lane_matchup_dto(matchup: &LaneMatchup, catalog: &ChampionCatalog) -> LaneMatchupDto {
    LaneMatchupDto {
        role: matchup.role,
        blue: synergy_champion(matchup.blue, catalog),
        red: synergy_champion(matchup.red, catalog),
        pressure: advantage_dto(&matchup.pressure),
        all_in: advantage_dto(&matchup.all_in),
    }
}

fn team_matchup_dto(matchup: &TeamMatchup, taxonomy: &Taxonomy) -> TeamMatchupDto {
    TeamMatchupDto {
        id: matchup.id.as_str(),
        label: matchup_label(matchup.id),
        side: matchup.side,
        attack_label: dimension_label(matchup.attack, taxonomy),
        defend_label: dimension_label(matchup.defend, taxonomy),
        advantage_low: matchup.advantage.low,
        advantage_high: matchup.advantage.high,
        confidence: matchup.advantage.confidence,
        favored: matchup.advantage.favored,
    }
}

fn synergy_label(id: SynergyId) -> &'static str {
    match id {
        SynergyId::EngageBurst => "Engage + burst",
        SynergyId::EngageDps => "Engage + DPS",
        SynergyId::WomboAoe => "Wombo combo + AoE",
        SynergyId::PeelDps => "Peel + DPS",
        SynergyId::FrontlineDps => "Frontline + DPS",
        SynergyId::PokeSiege => "Poke + asedio",
        SynergyId::CatchBurst => "Catch + burst",
        SynergyId::SustainPoke => "Sustain + poke",
        SynergyId::SplitWaveclear => "Splitpush + waveclear",
        SynergyId::ObjectiveFrontline => "Objetivos + frontline",
    }
}

fn matchup_label(id: MatchupRuleId) -> &'static str {
    match id {
        MatchupRuleId::PokeSustain => "Poke contra sustain",
        MatchupRuleId::AntiTankFrontline => "Anti-tanque contra frontline",
        MatchupRuleId::CatchMobility => "Catch contra baja movilidad",
        MatchupRuleId::SplitWaveclear => "Splitpush contra waveclear bajo",
        MatchupRuleId::EngageDisengage => "Engage contra disengage",
        MatchupRuleId::DpsTank => "DPS contra tanque",
        MatchupRuleId::BurstPeel => "Burst contra peel",
        MatchupRuleId::SiegeWaveclear => "Asedio contra waveclear bajo",
        MatchupRuleId::WomboDisengage => "Wombo contra disengage",
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

fn recommendation_set_dto(
    output: RecommendationSet,
    action_number: Option<usize>,
    profile_set_version: String,
    taxonomy: &Taxonomy,
    catalog: &ChampionCatalog,
) -> RecommendationSetDto {
    RecommendationSetDto {
        phase: output.phase.as_str(),
        side: output.side,
        action_kind: output.action_kind,
        action_number,
        engine_version: output.engine_version,
        profile_set_id: output.profile_set_id,
        profile_set_version,
        params_version: output.params_version,
        candidates_considered: output.candidates_considered,
        picks: output
            .picks
            .iter()
            .map(|pick| recommendation_dto(pick, taxonomy, catalog))
            .collect(),
        bans: output
            .bans
            .iter()
            .map(|ban| ban_recommendation_dto(ban, taxonomy, catalog))
            .collect(),
        no_data: output.no_data.map(|reason| reason.as_str()),
        limited_data: output.limited_data,
        missing_features: output
            .missing_features
            .iter()
            .map(|feature| feature.as_str())
            .collect(),
        missing_ban_features: output
            .missing_ban_features
            .iter()
            .map(|feature| feature.as_str())
            .collect(),
    }
}

fn recommendation_dto(
    pick: &Recommendation,
    taxonomy: &Taxonomy,
    catalog: &ChampionCatalog,
) -> RecommendationDto {
    RecommendationDto {
        champion: champion_ref(pick.champion, catalog),
        role: pick.role,
        score: pick.score,
        score_fraction: pick.score_fraction,
        ranking_score: pick.ranking_score,
        confidence: pick.confidence,
        limited_data: pick.limited_data,
        contributions: pick
            .contributions
            .iter()
            .map(ContributionDto::from)
            .collect(),
        risks: pick.risks.iter().map(RiskContributionDto::from).collect(),
        reasons: pick
            .reasons
            .iter()
            .map(|reason| reason_dto(reason, taxonomy, catalog))
            .collect(),
        missing: pick
            .missing
            .iter()
            .map(|feature| feature.as_str())
            .collect(),
        viable_roles: pick.viable_roles,
    }
}

fn ban_recommendation_dto(
    ban: &BanRecommendation,
    taxonomy: &Taxonomy,
    catalog: &ChampionCatalog,
) -> BanRecommendationDto {
    BanRecommendationDto {
        champion: champion_ref(ban.champion, catalog),
        score: ban.score,
        score_fraction: ban.score_fraction,
        ranking_score: ban.ranking_score,
        confidence: ban.confidence,
        limited_data: ban.limited_data,
        contributions: ban
            .contributions
            .iter()
            .map(BanContributionDto::from)
            .collect(),
        risks: ban.risks.iter().map(RiskContributionDto::from).collect(),
        reasons: ban
            .reasons
            .iter()
            .map(|reason| reason_dto(reason, taxonomy, catalog))
            .collect(),
        missing: ban.missing.iter().map(|feature| feature.as_str()).collect(),
        own_pick_value: ban.own_pick_value,
    }
}

fn reason_dto(reason: &Reason, taxonomy: &Taxonomy, catalog: &ChampionCatalog) -> ReasonDto {
    let (code, params) = match reason {
        Reason::FillsGap {
            dimension,
            before,
            after,
            need,
        } => (
            "fills_gap",
            json!({
                "kind": dimension_kind(*dimension),
                "dimension": dimension.as_str(),
                "label": dimension_label(*dimension, taxonomy),
                "before": before,
                "after": after,
                "need": need,
            }),
        ),
        Reason::AnswersThreat {
            dimension,
            before,
            after,
        } => (
            "answers_threat",
            json!({
                "kind": dimension_kind(*dimension),
                "dimension": dimension.as_str(),
                "label": dimension_label(*dimension, taxonomy),
                "before": before,
                "after": after,
            }),
        ),
        Reason::Synergy {
            id,
            champion,
            score_low,
            score_high,
        } => (
            "synergy",
            json!({
                "id": id.as_str(),
                "label": synergy_label(*id),
                "champion": champion.0,
                "championName": champion_name(*champion, catalog),
                "scoreLow": score_low,
                "scoreHigh": score_high,
            }),
        ),
        Reason::CountersLane {
            champion,
            role,
            edge_low,
            edge_high,
        } => (
            "counters_lane",
            json!({
                "champion": champion.0,
                "championName": champion_name(*champion, catalog),
                "role": role.as_str(),
                "roleLabel": role.label_es(),
                "edgeLow": edge_low,
                "edgeHigh": edge_high,
            }),
        ),
        Reason::FlexValue { roles } => ("flex_value", json!({ "roles": roles })),
        Reason::DamageBalance {
            kind,
            before,
            after,
        } => (
            "damage_balance",
            json!({
                "kind": kind.as_str(),
                "label": trait_label(*kind, taxonomy),
                "before": before,
                "after": after,
            }),
        ),
        Reason::RedundantDamage { kind, share } => (
            "redundant_damage",
            json!({
                "kind": kind.as_str(),
                "label": trait_label(*kind, taxonomy),
                "share": share,
            }),
        ),
        Reason::ResourceConflict { allies } => ("resource_conflict", json!({ "allies": allies })),
        Reason::NoFrontline => ("no_frontline", json!({})),
        Reason::ScalingConflict => ("scaling_conflict", json!({})),
        Reason::UnknownProfile { knowledge } => {
            ("unknown_profile", json!({ "knowledge": knowledge }))
        }
        Reason::RoleAmbiguous { roles } => ("role_ambiguous", json!({ "roles": roles })),
    };
    ReasonDto {
        code: code.to_string(),
        params,
    }
}

fn champion_name(key: ChampionKey, catalog: &ChampionCatalog) -> String {
    catalog
        .get(key)
        .map(|champion| champion.name.clone())
        .unwrap_or_else(|| format!("#{}", key.0))
}

fn trait_label(id: TraitId, taxonomy: &Taxonomy) -> String {
    taxonomy
        .trait_entry(id)
        .map(|entry| entry.label_es.clone())
        .unwrap_or_else(|| id.as_str().to_string())
}

/// Referencia de catálogo. Si el perfil existe para un campeón que aún no está
/// en el catálogo local, se muestra el identificador sin inventar nombre ni
/// imagen.
fn champion_ref(key: ChampionKey, catalog: &ChampionCatalog) -> ChampionRefDto {
    catalog
        .get(key)
        .map(ChampionRefDto::from)
        .unwrap_or_else(|| ChampionRefDto {
            key: key.0,
            slug: String::new(),
            name: format!("#{}", key.0),
            title: "Sin identidad en el catálogo local".to_string(),
            image: String::new(),
            image_path: String::new(),
        })
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

const UPDATE_SOURCE_KEY: &str = "updates.source";

/// Canal configurado: vacío o ausente usa la carpeta de datos de la app.
fn update_target(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<updates::UpdateTarget, CommandError> {
    let configured = {
        let db = lock_db(state)?;
        db.get_setting(UPDATE_SOURCE_KEY)?.and_then(|raw| {
            match serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw)) {
                Value::String(text) if !text.trim().is_empty() => Some(text),
                _ => None,
            }
        })
    };
    Ok(updates::resolve_target(
        configured.as_deref(),
        &app_data_dir(app)?,
    ))
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, CommandError> {
    app.path().app_data_dir().map_err(|error| {
        CommandError::internal(format!(
            "no se pudo resolver el directorio de datos: {error}"
        ))
    })
}

/// Consulta el canal y dice si hay una versión más nueva. No instala nada.
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckDto, CommandError> {
    let target = update_target(&app, &state)?;
    let manifest = updates::load(&target).await?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let available = updates::is_newer(&manifest.version, &current_version)?;
    let installer = if available {
        Some(updates::installer_for(&target, &manifest)?.describe())
    } else {
        None
    };

    Ok(UpdateCheckDto {
        current_version,
        latest_version: manifest.version,
        available,
        channel: target.channel(),
        source: target.describe(),
        manifest: target.manifest_location(),
        notes: manifest.notes,
        installer,
    })
}

/// Ejecuta el instalador del canal local o abre su descarga si el canal es HTTP.
///
/// Nunca descarga ni ejecuta un binario de internet por su cuenta: en HTTP se
/// abre el navegador y decide el usuario.
#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    version: String,
) -> Result<UpdateInstallDto, CommandError> {
    let target = update_target(&app, &state)?;
    let manifest = updates::load(&target).await?;

    if manifest.version.trim() != version.trim() {
        return Err(CommandError::new(
            "update_changed",
            "El canal ha cambiado desde la comprobación. Vuelve a buscar actualizaciones.",
        ));
    }
    if !updates::is_newer(&manifest.version, env!("CARGO_PKG_VERSION"))? {
        return Err(CommandError::new(
            "update_not_newer",
            "La versión del canal ya no es más nueva que la instalada.",
        ));
    }

    let installer = updates::installer_for(&target, &manifest)?;
    match installer {
        Installer::Local(path) => {
            std::process::Command::new(&path).spawn().map_err(|error| {
                CommandError::new(
                    "update_install_failed",
                    format!("No se pudo abrir el instalador {}: {error}", path.display()),
                )
            })?;
            Ok(UpdateInstallDto {
                action: "installer",
                version: manifest.version,
                target: path.display().to_string(),
                message: "Se ha abierto el instalador. Sigue sus pasos; si LolDraft sigue abierto, te pedirá cerrarlo.".to_string(),
            })
        }
        Installer::Url(url) => {
            // `explorer` abre la URL con el navegador predeterminado sin pasar
            // por un shell que interprete `&` u otros caracteres de la URL.
            std::process::Command::new("explorer")
                .arg(&url)
                .spawn()
                .map_err(|error| {
                    CommandError::new(
                        "update_launch_failed",
                        format!("No se pudo abrir el navegador para descargar la actualización: {error}"),
                    )
                })?;
            Ok(UpdateInstallDto {
                action: "browser",
                version: manifest.version,
                target: url,
                message: "Se ha abierto la descarga en el navegador. Ejecuta el instalador cuando termine."
                    .to_string(),
            })
        }
    }
}

/// Conjunto de perfiles activo y su versión, con el error que espera la UI.
fn active_profile_set(state: &AppState) -> Result<(ProfileSet, String), CommandError> {
    let db = lock_db(state)?;
    let active = db.active_profile_set()?.ok_or_else(|| {
        CommandError::new(
            "profiles_not_loaded",
            "No hay un conjunto de perfiles activo. Carga y activa un conjunto para analizar y recomendar.",
        )
    })?;
    let set = db.load_profile_set(&active.id)?;
    Ok((set, active.version))
}

/// Rechaza el mismo campeón dos veces: el motor no puede evaluar un estado
/// imposible y preferimos decirlo antes que devolver una recomendación dudosa.
fn validate_unique_champions(draft: &SoloqDraftInput) -> Result<(), CommandError> {
    let mut seen = HashSet::new();
    for key in draft
        .bans
        .iter()
        .chain(draft.ally_picks.iter())
        .chain(draft.enemy_picks.iter())
    {
        if !seen.insert(*key) {
            return Err(CommandError::new(
                "soloq_duplicate_champion",
                format!(
                    "El campeón #{} aparece más de una vez en el draft; revisa los picks y baneos.",
                    key
                ),
            ));
        }
    }
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
