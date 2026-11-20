//! Evaluación de candidatos y orquestación de recomendaciones.

use super::model::{
    BanContribution, BanFeatureId, BanRecommendation, NoDataReason, Reason, Recommendation,
    RecommendationSet,
};
use super::params::{
    BanWeights, DraftPhase, FeatureId, FeatureWeights, PickWeightPhase, RecommendationParams,
    RecommendationParamsError,
};
use super::score::{self, FeatureValue};
use crate::analysis::{
    analyze_draft, Advantage, AnalysisError, AnalysisParams, Dimension, DraftAnalysis, Need,
    NeedSource, PickedChampion, Synergy, TeamAnalysis,
};
use crate::profile::{ChampionProfile, ProfileSet};
use crate::stats::ObservedStats;
use crate::taxonomy::TraitId;
use loldraft_domain::{ChampionKey, DraftFormat, Role, Side};
use std::collections::{BTreeMap, BTreeSet, HashSet};
/// Solicitud completa de recomendaciones para la siguiente acción del draft.
pub struct RecommendationRequest<'a> {
    pub format: &'a DraftFormat,
    /// Índice de la siguiente acción (0-based), no de la última ejecutada.
    pub action_index: usize,
    pub blue_picks: &'a [PickedChampion],
    pub red_picks: &'a [PickedChampion],
    /// Baneos y picks ya usados; nunca se recomiendan.
    pub used: &'a [ChampionKey],
    pub profiles: &'a ProfileSet,
    pub analysis_params: &'a AnalysisParams,
    pub params: &'a RecommendationParams,
    /// Rol reservado para el pick que se va a hacer (SoloQ con posición
    /// conocida). Si es `None`, se evalúan todos los roles libres.
    pub fixed_role: Option<Role>,
    /// Estadísticas observadas del dataset activo para esta cola. `None`
    /// desactiva `power`: nunca se sustituye por una estimación.
    pub observed: Option<&'a ObservedStats>,
}

/// Índice de `power` ya calculado para todo el dataset. Se construye una vez
/// por recomendación, no por candidato.
struct ObservedPower<'a> {
    stats: &'a ObservedStats,
    percentiles: BTreeMap<(ChampionKey, Role), f32>,
}

impl<'a> ObservedPower<'a> {
    fn new(stats: &'a ObservedStats) -> Self {
        Self {
            stats,
            percentiles: stats.power_percentiles(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RecommendationError {
    #[error("la acción {index} no existe en el formato `{format}`")]
    InvalidActionIndex { index: usize, format: String },
    #[error(transparent)]
    InvalidParams(#[from] RecommendationParamsError),
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
}

pub fn recommend(
    request: &RecommendationRequest<'_>,
) -> Result<RecommendationSet, RecommendationError> {
    request.params.validate()?;
    let step = request.format.step(request.action_index).ok_or_else(|| {
        RecommendationError::InvalidActionIndex {
            index: request.action_index,
            format: request.format.id.clone(),
        }
    })?;
    let phase = DraftPhase::of(request.format, request.action_index).ok_or_else(|| {
        RecommendationError::InvalidActionIndex {
            index: request.action_index,
            format: request.format.id.clone(),
        }
    })?;
    let side = step.side;
    let baseline = analyze_draft(
        request.blue_picks,
        request.red_picks,
        request.profiles,
        request.analysis_params,
    )?;

    let used: HashSet<ChampionKey> = request.used.iter().copied().collect();
    let candidates: Vec<ChampionKey> = request
        .profiles
        .champions
        .keys()
        .copied()
        .filter(|key| !used.contains(key))
        .collect();
    let candidates_considered = candidates.len();
    let observed_power = request.observed.map(ObservedPower::new);

    let (picks, bans, no_data) = if request.profiles.champions.is_empty() {
        (Vec::new(), Vec::new(), Some(NoDataReason::NoProfiles))
    } else if candidates.is_empty() {
        (
            Vec::new(),
            Vec::new(),
            Some(NoDataReason::NoAvailableChampions),
        )
    } else if phase.is_ban() {
        let bans = evaluate_bans(
            request,
            &baseline,
            side,
            phase,
            &candidates,
            observed_power.as_ref(),
        );
        let no_data = bans.is_empty().then_some(NoDataReason::NoViableCandidate);
        (Vec::new(), bans, no_data)
    } else {
        let picks = evaluate_picks(
            request,
            &baseline,
            side,
            &candidates,
            observed_power.as_ref(),
        );
        let no_data = picks.is_empty().then_some(NoDataReason::NoViableCandidate);
        (picks, Vec::new(), no_data)
    };

    let limited_data = picks
        .first()
        .map(|pick| pick.limited_data)
        .or_else(|| bans.first().map(|ban| ban.limited_data))
        .unwrap_or(false);
    let mut missing: BTreeSet<FeatureId> = BTreeSet::new();
    for pick in &picks {
        missing.extend(pick.missing.iter().copied());
    }
    let mut missing_ban: BTreeSet<BanFeatureId> = BTreeSet::new();
    for ban in &bans {
        missing_ban.extend(ban.missing.iter().copied());
    }

    Ok(RecommendationSet {
        phase,
        side,
        action_kind: step.kind,
        engine_version: crate::engine_version().to_string(),
        profile_set_id: request.profiles.id.clone(),
        params_version: request.params.version,
        candidates_considered,
        picks,
        bans,
        no_data,
        limited_data,
        missing_features: missing.into_iter().collect(),
        missing_ban_features: missing_ban.into_iter().collect(),
        stats: request.observed.map(|stats| stats.provenance()),
    })
}

// ---------------------------------------------------------------------------
// Picks
// ---------------------------------------------------------------------------

struct PickDraft {
    role: Role,
    features: Vec<FeatureValue>,
    risks: Vec<(super::model::RiskId, f32)>,
    reasons: Vec<Reason>,
    missing: Vec<FeatureId>,
    confidence: f32,
    score: score::ScoreResult,
}

fn evaluate_picks(
    request: &RecommendationRequest<'_>,
    baseline: &DraftAnalysis,
    side: Side,
    candidates: &[ChampionKey],
    observed: Option<&ObservedPower<'_>>,
) -> Vec<Recommendation> {
    let (our_picks, enemy_picks) = match side {
        Side::Blue => (request.blue_picks, request.red_picks),
        Side::Red => (request.red_picks, request.blue_picks),
    };
    let our_baseline = team_analysis(baseline, side);
    let occupied: HashSet<Role> = our_picks.iter().map(|pick| pick.role).collect();
    let enemy_revealed = enemy_picks.len();
    let our_carries = resource_carries(our_picks, request.profiles);
    let weights = pick_weights(
        PickWeightPhase::of(request.format, request.action_index),
        request.action_index,
        enemy_revealed,
        our_carries,
        request.params,
    );

    let mut recommendations = Vec::new();
    for key in candidates {
        let Some(profile) = request.profiles.get(*key) else {
            continue;
        };
        let roles: Vec<Role> = match request.fixed_role {
            Some(role) => {
                let viable = profile.roles.iter().any(|fit| {
                    fit.role == role && fit.proficiency >= request.params.min_role_proficiency
                });
                if viable {
                    vec![role]
                } else {
                    Vec::new()
                }
            }
            None => profile
                .roles
                .iter()
                .filter(|role| {
                    role.proficiency >= request.params.min_role_proficiency
                        && !occupied.contains(&role.role)
                })
                .map(|role| role.role)
                .collect(),
        };
        if roles.is_empty() {
            continue;
        }

        let mut drafts = Vec::new();
        for role in &roles {
            if let Some(draft) = evaluate_pick_role(
                request,
                baseline,
                side,
                our_baseline,
                profile,
                *role,
                &weights,
                &occupied,
                observed,
            ) {
                drafts.push(draft);
            }
        }
        drafts.sort_by(|left, right| {
            right
                .score
                .ranking
                .total_cmp(&left.score.ranking)
                .then_with(|| left.role.index().cmp(&right.role.index()))
        });
        if drafts.is_empty() {
            continue;
        }
        let mut best = drafts.remove(0);
        let ambiguous = drafts.first().is_some_and(|second| {
            best.score.ranking - second.score.ranking <= request.params.role_ambiguity_margin
        });
        if ambiguous {
            best.risks.push((super::model::RiskId::RoleAmbiguous, 0.15));
            best.reasons.push(Reason::RoleAmbiguous {
                roles: roles.len() as u8,
            });
            if let Some(rescored) = score::score(
                &best.features,
                &weights,
                &best.risks,
                best.confidence,
                request.params,
            ) {
                best.score = rescored;
            }
        }

        recommendations.push(Recommendation {
            champion: *key,
            role: best.role,
            score: score::display(best.score.score),
            score_fraction: best.score.score,
            ranking_score: best.score.ranking,
            confidence: best.confidence,
            limited_data: best.confidence < 0.45,
            contributions: best.score.contributions,
            risks: best.score.risks,
            reasons: best.reasons,
            missing: best.missing,
            viable_roles: roles.len() as u8,
        });
    }

    score::diversify(recommendations, request.profiles, request.params)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_pick_role(
    request: &RecommendationRequest<'_>,
    baseline: &DraftAnalysis,
    side: Side,
    our_baseline: &TeamAnalysis,
    candidate: &ChampionProfile,
    role: Role,
    weights: &FeatureWeights,
    occupied: &HashSet<Role>,
    observed: Option<&ObservedPower<'_>>,
) -> Option<PickDraft> {
    let mut blue = request.blue_picks.to_vec();
    let mut red = request.red_picks.to_vec();
    let candidate_pick = PickedChampion::new(candidate.key, role, role_confidence(candidate, role));
    match side {
        Side::Blue => blue.push(candidate_pick),
        Side::Red => red.push(candidate_pick),
    }
    let after = analyze_draft(&blue, &red, request.profiles, request.analysis_params).ok()?;
    let our_after = team_analysis(&after, side);

    let mut features = Vec::new();
    let mut reasons = Vec::new();
    let mut risks = Vec::new();

    // `need`: ganancia marginal sobre las carencias actuales.
    let need_gaps = gaps(&our_baseline.needs, our_baseline, our_after);
    if let Some(feature) = gap_feature(&need_gaps, FeatureId::Need) {
        features.push(feature);
        for gap in top_gaps(&need_gaps, 3) {
            reasons.push(Reason::FillsGap {
                dimension: gap.dimension,
                before: gap.before,
                after: gap.after,
                need: gap.need,
            });
        }
    }

    // `answer`: solo carencias nacidas de amenazas enemigas.
    let threat_needs: Vec<Need> = our_baseline
        .needs
        .iter()
        .filter(|need| is_threat_need(need))
        .cloned()
        .collect();
    let threat_gaps = gaps(&threat_needs, our_baseline, our_after);
    if let Some(feature) = gap_feature(&threat_gaps, FeatureId::Answer) {
        features.push(feature);
        for gap in top_gaps(&threat_gaps, 2) {
            reasons.push(Reason::AnswersThreat {
                dimension: gap.dimension,
                before: gap.before,
                after: gap.after,
            });
        }
    }

    // `synergy`: parejas derivadas que el candidato forma con sus aliados.
    if let Some((feature, notes)) = synergy_feature(our_after, candidate.key) {
        features.push(feature);
        for synergy in notes.iter().take(2) {
            let partner = synergy
                .champions
                .iter()
                .copied()
                .find(|key| *key != candidate.key);
            if let Some(partner) = partner {
                reasons.push(Reason::Synergy {
                    id: synergy.id,
                    champion: partner,
                    score_low: synergy.score_low,
                    score_high: synergy.score_high,
                });
            }
        }
    }

    // `counter`: línea compartida + mejora del enfrentamiento de equipo.
    if let Some((feature, lane)) = counter_feature(baseline, &after, side, role) {
        features.push(feature);
        if let Some(reading) = lane {
            reasons.push(Reason::CountersLane {
                champion: reading.opponent,
                role,
                edge_low: reading.low,
                edge_high: reading.high,
            });
        }
    }

    // `flex`: roles viables y calidad de los secundarios.
    if let Some(feature) = flex_feature(candidate, occupied, request.params.min_role_proficiency) {
        if candidate.roles.len() > 1 {
            reasons.push(Reason::FlexValue {
                roles: candidate.roles.len() as u8,
            });
        }
        features.push(feature);
    }

    // `balance`: penalizaciones concretas por daño, recursos y frontline.
    if let Some(balance) = balance_feature(
        our_baseline,
        our_after,
        candidate,
        request.blue_picks,
        request.red_picks,
        side,
        request.profiles,
    ) {
        features.push(balance.value);
        reasons.extend(balance.reasons);
        risks.extend(balance.risks);
    }

    // `safety`: lane_safety, movilidad y rango.
    if let Some(feature) = safety_feature(candidate) {
        features.push(feature);
    }

    // `power`: percentil observado dentro del rol, si hay dataset activo.
    if let Some((feature, reason)) = power_feature(observed, candidate.key, role) {
        features.push(feature);
        reasons.push(reason);
    }

    let missing = missing_features(&features, &FeatureId::ALL);

    let knowledge = candidate.known_cell_ratio();
    let context = our_baseline.confidence;
    let confidence = candidate_confidence(knowledge, role_confidence(candidate, role), context);
    if knowledge < 0.999 {
        let amount = (1.0 - knowledge) * 0.5;
        if amount > 0.01 {
            risks.push((super::model::RiskId::UnknownProfile, amount));
            reasons.push(Reason::UnknownProfile { knowledge });
        }
    }

    let score = score::score(&features, weights, &risks, confidence, request.params)?;

    Some(PickDraft {
        role,
        features,
        risks,
        reasons,
        missing,
        confidence,
        score,
    })
}

// ---------------------------------------------------------------------------
// Baneos
// ---------------------------------------------------------------------------

fn evaluate_bans(
    request: &RecommendationRequest<'_>,
    baseline: &DraftAnalysis,
    acting_side: Side,
    phase: DraftPhase,
    candidates: &[ChampionKey],
    observed: Option<&ObservedPower<'_>>,
) -> Vec<BanRecommendation> {
    let opponent = acting_side.opposite();
    let (our_picks, enemy_picks) = match acting_side {
        Side::Blue => (request.blue_picks, request.red_picks),
        Side::Red => (request.red_picks, request.blue_picks),
    };
    let our_baseline = team_analysis(baseline, acting_side);
    let enemy_baseline = team_analysis(baseline, opponent);
    let enemy_occupied: HashSet<Role> = enemy_picks.iter().map(|pick| pick.role).collect();
    let our_occupied: HashSet<Role> = our_picks.iter().map(|pick| pick.role).collect();
    let weights = ban_weights(phase, enemy_picks.len(), request.params);

    let mut recommendations = Vec::new();
    for key in candidates {
        let Some(profile) = request.profiles.get(*key) else {
            continue;
        };
        let roles: Vec<Role> = profile
            .roles
            .iter()
            .filter(|role| {
                role.proficiency >= request.params.min_role_proficiency
                    && !enemy_occupied.contains(&role.role)
            })
            .map(|role| role.role)
            .collect();
        if roles.is_empty() {
            continue;
        }

        let mut drafts = Vec::new();
        for role in &roles {
            if let Some(draft) = evaluate_ban_role(
                request,
                baseline,
                acting_side,
                opponent,
                our_baseline,
                enemy_baseline,
                profile,
                *role,
                &weights,
                &enemy_occupied,
                &our_occupied,
                observed,
            ) {
                drafts.push(draft);
            }
        }
        if drafts.is_empty() {
            continue;
        }
        drafts.sort_by(|left, right| {
            right
                .ranking
                .total_cmp(&left.ranking)
                .then_with(|| left.role.index().cmp(&right.role.index()))
        });
        let mut best = drafts.remove(0);
        let ambiguous = drafts.first().is_some_and(|second| {
            best.ranking - second.ranking <= request.params.role_ambiguity_margin
        });
        if ambiguous {
            best.risks.push((super::model::RiskId::RoleAmbiguous, 0.15));
            best.reasons.push(Reason::RoleAmbiguous {
                roles: roles.len() as u8,
            });
            if let Some((contributions, risks, score_value, ranking)) = score_ban(
                &best.features,
                &weights,
                Some(best.own_pick_value),
                &best.risks,
                best.confidence,
                request.params,
            ) {
                best.contributions = contributions;
                best.risks_rendered = risks;
                best.score_fraction = score_value;
                best.ranking = ranking;
            }
        }

        recommendations.push(BanRecommendation {
            champion: *key,
            score: score::display(best.score_fraction),
            score_fraction: best.score_fraction,
            ranking_score: best.ranking,
            confidence: best.confidence,
            limited_data: best.confidence < 0.45,
            contributions: best.contributions,
            risks: best.risks_rendered,
            reasons: best.reasons,
            missing: best.missing,
            own_pick_value: best.own_pick_value,
        });
    }

    recommendations.sort_by(|left, right| {
        right
            .ranking_score
            .total_cmp(&left.ranking_score)
            .then_with(|| left.champion.cmp(&right.champion))
    });
    recommendations.truncate(request.params.limit);
    recommendations
}

struct BanDraft {
    role: Role,
    features: Vec<(BanFeatureId, f32)>,
    risks: Vec<(super::model::RiskId, f32)>,
    contributions: Vec<BanContribution>,
    risks_rendered: Vec<super::model::RiskContribution>,
    reasons: Vec<Reason>,
    missing: Vec<BanFeatureId>,
    confidence: f32,
    own_pick_value: f32,
    score_fraction: f32,
    ranking: f32,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_ban_role(
    request: &RecommendationRequest<'_>,
    baseline: &DraftAnalysis,
    acting_side: Side,
    opponent: Side,
    our_baseline: &TeamAnalysis,
    enemy_baseline: &TeamAnalysis,
    candidate: &ChampionProfile,
    role: Role,
    weights: &BanWeights,
    enemy_occupied: &HashSet<Role>,
    our_occupied: &HashSet<Role>,
    observed: Option<&ObservedPower<'_>>,
) -> Option<BanDraft> {
    // Escenario rival con el candidato.
    let mut enemy_blue = request.blue_picks.to_vec();
    let mut enemy_red = request.red_picks.to_vec();
    let enemy_pick = PickedChampion::new(candidate.key, role, role_confidence(candidate, role));
    match opponent {
        Side::Blue => enemy_blue.push(enemy_pick),
        Side::Red => enemy_red.push(enemy_pick),
    }
    let enemy_after = analyze_draft(
        &enemy_blue,
        &enemy_red,
        request.profiles,
        request.analysis_params,
    )
    .ok()?;
    let enemy_analysis = team_analysis(&enemy_after, opponent);

    // Escenario propio con el candidato, para restar el valor que perdemos.
    let our_role = candidate
        .roles
        .iter()
        .filter(|entry| entry.proficiency >= request.params.min_role_proficiency)
        .map(|entry| entry.role)
        .find(|role| !our_occupied.contains(role));
    let own_feature = match our_role {
        Some(our_role) => {
            let mut blue = request.blue_picks.to_vec();
            let mut red = request.red_picks.to_vec();
            let pick = PickedChampion::new(
                candidate.key,
                our_role,
                role_confidence(candidate, our_role),
            );
            match acting_side {
                Side::Blue => blue.push(pick),
                Side::Red => red.push(pick),
            }
            analyze_draft(&blue, &red, request.profiles, request.analysis_params)
                .ok()
                .and_then(|analysis| {
                    let our_after = team_analysis(&analysis, acting_side);
                    own_pick_value(our_baseline, our_after, candidate.key)
                })
        }
        None => None,
    };

    let mut features: Vec<(BanFeatureId, f32)> = Vec::new();
    let mut reasons = Vec::new();
    let mut risks = Vec::new();

    let enemy_gaps = gaps(&enemy_baseline.needs, enemy_baseline, enemy_analysis);
    if let Some(feature) = gap_feature(&enemy_gaps, FeatureId::Need) {
        features.push((BanFeatureId::OppNeed, feature.value));
        for gap in top_gaps(&enemy_gaps, 2) {
            reasons.push(Reason::FillsGap {
                dimension: gap.dimension,
                before: gap.before,
                after: gap.after,
                need: gap.need,
            });
        }
    }

    if let Some((feature, notes)) = synergy_feature(enemy_analysis, candidate.key) {
        features.push((BanFeatureId::OppSynergy, feature.value));
        if let Some(synergy) = notes.first() {
            if let Some(partner) = synergy
                .champions
                .iter()
                .copied()
                .find(|key| *key != candidate.key)
            {
                reasons.push(Reason::Synergy {
                    id: synergy.id,
                    champion: partner,
                    score_low: synergy.score_low,
                    score_high: synergy.score_high,
                });
            }
        }
    }

    // `opp_power`: percentil observado del rival dentro del rol. Un rival con
    // muestra insuficiente no aporta señal; no se inventa presencia.
    if let Some((feature, reason)) = power_feature(observed, candidate.key, role) {
        features.push((BanFeatureId::OppPower, feature.value));
        reasons.push(reason);
    }

    if let Some(feature) = flex_feature(
        candidate,
        enemy_occupied,
        request.params.min_role_proficiency,
    ) {
        features.push((BanFeatureId::OppFlex, feature.value));
        if candidate.roles.len() > 1 {
            reasons.push(Reason::FlexValue {
                roles: candidate.roles.len() as u8,
            });
        }
    }

    if let Some((feature, lane)) = counter_feature(baseline, &enemy_after, opponent, role) {
        features.push((BanFeatureId::OwnThreat, feature.value));
        if let Some(reading) = lane {
            reasons.push(Reason::CountersLane {
                champion: reading.opponent,
                role,
                edge_low: reading.low,
                edge_high: reading.high,
            });
        }
    }

    let own_value = own_feature.as_ref().map(|feature| feature.value);
    let own_pick_value = own_value.unwrap_or(0.0);

    let knowledge = candidate.known_cell_ratio();
    let context = enemy_baseline.confidence;
    let confidence = candidate_confidence(knowledge, role_confidence(candidate, role), context);
    if knowledge < 0.999 {
        let amount = (1.0 - knowledge) * 0.5;
        if amount > 0.01 {
            risks.push((super::model::RiskId::UnknownProfile, amount));
            reasons.push(Reason::UnknownProfile { knowledge });
        }
    }

    let (contributions, risks_rendered, score_fraction, ranking) = score_ban(
        &features,
        weights,
        own_value,
        &risks,
        confidence,
        request.params,
    )?;

    let scored_features = [
        BanFeatureId::OppNeed,
        BanFeatureId::OppSynergy,
        BanFeatureId::OppPower,
        BanFeatureId::OppFlex,
        BanFeatureId::OwnThreat,
    ];
    let missing: Vec<BanFeatureId> = scored_features
        .iter()
        .copied()
        .filter(|id| !features.iter().any(|(present, _)| present == id))
        .collect();

    Some(BanDraft {
        role,
        features,
        risks,
        contributions,
        risks_rendered,
        reasons,
        missing,
        confidence,
        own_pick_value,
        score_fraction,
        ranking,
    })
}

fn own_pick_value(
    before: &TeamAnalysis,
    after: &TeamAnalysis,
    candidate: ChampionKey,
) -> Option<FeatureValue> {
    let gaps = gaps(&before.needs, before, after);
    let need = gap_feature(&gaps, FeatureId::Need).map(|feature| feature.value);
    let synergy = synergy_feature(after, candidate).map(|(feature, _)| feature.value);
    let (value, _) = match (need, synergy) {
        (Some(need), Some(synergy)) => (0.5 * need + 0.5 * synergy, 2.0),
        (Some(need), None) => (need, 1.0),
        (None, Some(synergy)) => (synergy, 1.0),
        (None, None) => return None,
    };
    let confidence = before.confidence;
    Some(FeatureValue::new(FeatureId::Need, value, confidence))
}

fn score_ban(
    features: &[(BanFeatureId, f32)],
    weights: &BanWeights,
    own_pick_value: Option<f32>,
    risks: &[(super::model::RiskId, f32)],
    confidence: f32,
    params: &RecommendationParams,
) -> Option<(
    Vec<BanContribution>,
    Vec<super::model::RiskContribution>,
    f32,
    f32,
)> {
    let mut present: BTreeMap<BanFeatureId, f32> = BTreeMap::new();
    for (id, value) in features {
        present.insert(*id, score::clamp_unit(*value));
    }
    let weight_of = |id: BanFeatureId| match id {
        BanFeatureId::OppNeed => weights.opp_need,
        BanFeatureId::OppSynergy => weights.opp_synergy,
        BanFeatureId::OppPower => weights.opp_power,
        BanFeatureId::OppFlex => weights.opp_flex,
        BanFeatureId::OwnThreat => weights.own_threat,
        BanFeatureId::OwnPickValue => 0.0,
    };
    let weight_sum: f32 = present
        .keys()
        .filter(|id| **id != BanFeatureId::OwnPickValue)
        .map(|id| weight_of(*id))
        .sum();
    if weight_sum <= 0.0 {
        return None;
    }

    let mut contributions = Vec::new();
    let mut raw = 0.0;
    for (id, value) in &present {
        if *id == BanFeatureId::OwnPickValue {
            continue;
        }
        let weight = weight_of(*id) / weight_sum;
        let contribution = weight * value;
        raw += contribution;
        contributions.push(BanContribution {
            feature: *id,
            value: *value,
            weight,
            contribution,
        });
    }
    if let Some(own) = own_pick_value {
        let contribution = -params.ban_own_value_penalty * score::clamp_unit(own);
        raw += contribution;
        contributions.push(BanContribution {
            feature: BanFeatureId::OwnPickValue,
            value: score::clamp_unit(own),
            weight: -params.ban_own_value_penalty,
            contribution,
        });
    }
    contributions.sort_by_key(|entry| entry.feature);

    let risk_sum: f32 = risks
        .iter()
        .map(|(_, amount)| score::clamp_unit(*amount))
        .sum();
    let risk_total = risk_sum.min(1.0);
    let requested_risk = params.risk_coefficient * risk_total;
    let applied_risk = requested_risk.min(raw.max(0.0));
    let risk_scale = if risk_sum > 0.0 {
        (applied_risk / requested_risk).min(1.0) * (risk_total / risk_sum)
    } else {
        0.0
    };
    let rendered: Vec<super::model::RiskContribution> = risks
        .iter()
        .map(|(kind, amount)| super::model::RiskContribution {
            kind: *kind,
            amount: score::clamp_unit(*amount),
            contribution: -params.risk_coefficient * score::clamp_unit(*amount) * risk_scale,
        })
        .collect();
    let score_value = (raw - applied_risk).clamp(0.0, 1.0);
    let confidence = score::clamp_unit(confidence);
    let ranking = score_value - params.confidence_penalty * (1.0 - confidence);
    Some((contributions, rendered, score_value, ranking))
}

// ---------------------------------------------------------------------------
// Features compartidas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Gap {
    dimension: Dimension,
    before: f32,
    after: f32,
    need: f32,
    gain: f32,
    confidence: f32,
}

fn gaps(needs: &[Need], before: &TeamAnalysis, after: &TeamAnalysis) -> Vec<Gap> {
    let mut gaps = Vec::new();
    for need in needs {
        let Some(before_coverage) = before
            .coverage(need.dimension)
            .filter(|coverage| coverage.total > 0)
        else {
            continue;
        };
        let Some(after_coverage) = after
            .coverage(need.dimension)
            .filter(|coverage| coverage.total > 0)
        else {
            continue;
        };
        let gain = (after_coverage.value_low - before_coverage.value_low).max(0.0);
        gaps.push(Gap {
            dimension: need.dimension,
            before: before_coverage.value_low,
            after: after_coverage.value_low,
            need: need.amount,
            gain,
            confidence: after_coverage.confidence.min(before_coverage.confidence),
        });
    }
    gaps
}

fn gap_feature(gaps: &[Gap], id: FeatureId) -> Option<FeatureValue> {
    let total: f32 = gaps.iter().map(|gap| gap.need).sum();
    if total <= 0.0 {
        return None;
    }
    let value: f32 = gaps.iter().map(|gap| gap.need * gap.gain).sum::<f32>() / total;
    let confidence: f32 = gaps
        .iter()
        .map(|gap| gap.need * gap.confidence)
        .sum::<f32>()
        / total;
    Some(FeatureValue::new(id, value, confidence))
}

fn top_gaps(gaps: &[Gap], limit: usize) -> Vec<&Gap> {
    let mut ranked: Vec<&Gap> = gaps.iter().filter(|gap| gap.gain > 0.0).collect();
    ranked.sort_by(|left, right| {
        (right.gain * right.need)
            .total_cmp(&(left.gain * left.need))
            .then_with(|| left.dimension.cmp(&right.dimension))
    });
    ranked.truncate(limit);
    ranked
}

fn is_threat_need(need: &Need) -> bool {
    need.sources
        .iter()
        .any(|source| matches!(source, NeedSource::EnemyThreat { .. }))
}

fn synergy_feature(
    analysis: &TeamAnalysis,
    candidate: ChampionKey,
) -> Option<(FeatureValue, Vec<Synergy>)> {
    let mut entries: Vec<Synergy> = analysis
        .synergies
        .iter()
        .filter(|synergy| synergy.champions.contains(&candidate))
        .cloned()
        .collect();
    if entries.is_empty() {
        return None;
    }
    entries.sort_by(|left, right| {
        (right.score_low + right.score_high)
            .total_cmp(&(left.score_low + left.score_high))
            .then_with(|| left.id.cmp(&right.id))
    });
    let value = entries
        .iter()
        .map(|synergy| (synergy.score_low + synergy.score_high) / 2.0)
        .sum::<f32>()
        / entries.len() as f32;
    let confidence = entries
        .iter()
        .map(|synergy| synergy.confidence)
        .sum::<f32>()
        / entries.len() as f32;
    Some((
        FeatureValue::new(FeatureId::Synergy, value, confidence),
        entries,
    ))
}

fn counter_feature(
    baseline: &DraftAnalysis,
    after: &DraftAnalysis,
    side: Side,
    role: Role,
) -> Option<(FeatureValue, Option<LaneReading>)> {
    let lane = lane_edge(after, side, role);
    let baseline_edge = team_edge_mean(baseline, side);
    let after_edge = team_edge_mean(after, side);
    let team = match (baseline_edge, after_edge) {
        (Some((before, _)), Some((after, confidence))) => {
            Some((((after - before) + 1.0) / 2.0, confidence))
        }
        (None, Some((after, confidence))) => Some(((after + 1.0) / 2.0, confidence)),
        _ => None,
    };

    let mut value = 0.0;
    let mut weight = 0.0;
    let mut confidence = 0.0;
    if let Some(reading) = &lane {
        let edge = (reading.low + reading.high) / 2.0;
        let normalized = (edge + 1.0) / 2.0;
        value += 0.55 * normalized;
        weight += 0.55;
        confidence += 0.55 * reading.confidence;
    }
    if let Some((normalized, team_confidence)) = team {
        value += 0.45 * normalized;
        weight += 0.45;
        confidence += 0.45 * team_confidence;
    }
    if weight <= 0.0 {
        return None;
    }
    let feature = FeatureValue::new(
        FeatureId::Counter,
        value / weight,
        if weight > 0.0 {
            confidence / weight
        } else {
            0.0
        },
    );
    Some((feature, lane))
}

/// Intervalo real de ventaja de línea para el lado que actúa. No se fabrica un
/// margen artificial: `low`/`high` salen de las cotas de presión y all-in ya
/// derivadas por el análisis de interacciones.
struct LaneReading {
    low: f32,
    high: f32,
    confidence: f32,
    opponent: ChampionKey,
}

fn lane_edge(analysis: &DraftAnalysis, side: Side, role: Role) -> Option<LaneReading> {
    let matchup = analysis
        .lane_matchups
        .iter()
        .find(|matchup| matchup.role == role)?;
    let blue_low = (matchup.pressure.low + matchup.all_in.low) / 2.0;
    let blue_high = (matchup.pressure.high + matchup.all_in.high) / 2.0;
    let (low, high) = match side {
        Side::Blue => (blue_low, blue_high),
        Side::Red => (-blue_high, -blue_low),
    };
    let confidence = (matchup.pressure.confidence + matchup.all_in.confidence) / 2.0;
    let opponent = match side {
        Side::Blue => matchup.red,
        Side::Red => matchup.blue,
    };
    Some(LaneReading {
        low: low.clamp(-1.0, 1.0),
        high: high.clamp(-1.0, 1.0),
        confidence,
        opponent,
    })
}

fn team_edge_mean(analysis: &DraftAnalysis, side: Side) -> Option<(f32, f32)> {
    let entries: Vec<&crate::analysis::TeamMatchup> = analysis
        .matchups
        .iter()
        .filter(|matchup| matchup.side == side)
        .collect();
    if entries.is_empty() {
        return None;
    }
    let total = entries.len() as f32;
    let mean = entries
        .iter()
        .map(|matchup| midpoint(&matchup.advantage))
        .sum::<f32>()
        / total;
    let confidence = entries
        .iter()
        .map(|matchup| matchup.advantage.confidence)
        .sum::<f32>()
        / total;
    Some((mean.clamp(-1.0, 1.0), confidence))
}

fn midpoint(advantage: &Advantage) -> f32 {
    (advantage.low + advantage.high) / 2.0
}

/// `power` observado: percentil del winrate ajustado dentro del mismo rol.
/// Solo existe para filas con muestra suficiente; por eso el mapa de
/// percentiles ya viene filtrado. La confianza es la de la propia fila.
fn power_feature(
    observed: Option<&ObservedPower<'_>>,
    champion: ChampionKey,
    role: Role,
) -> Option<(FeatureValue, Reason)> {
    let observed = observed?;
    let percentile = *observed.percentiles.get(&(champion, role))?;
    let row = observed.stats.row(champion, role)?;
    let confidence = row.confidence();
    Some((
        FeatureValue::new(FeatureId::Power, percentile, confidence),
        Reason::ObservedPower {
            queue: observed.stats.queue,
            percentile,
            games: row.games,
            shrunk_winrate: row.shrunk_winrate(),
            confidence,
        },
    ))
}

fn flex_feature(
    profile: &ChampionProfile,
    occupied: &HashSet<Role>,
    min_proficiency: f32,
) -> Option<FeatureValue> {
    let mut roles: Vec<&crate::profile::ProfileRole> = profile
        .roles
        .iter()
        .filter(|role| role.proficiency >= min_proficiency)
        .collect();
    if roles.is_empty() {
        return None;
    }
    roles.sort_by(|left, right| right.proficiency.total_cmp(&left.proficiency));
    let secondary = &roles[1..];
    if secondary.is_empty() {
        return Some(FeatureValue::new(FeatureId::Flex, 0.0, roles[0].confidence));
    }
    let base = ((secondary.len() as f32) / 2.0).min(1.0);
    let quality =
        secondary.iter().map(|role| role.proficiency).sum::<f32>() / secondary.len() as f32;
    let free_secondary = secondary
        .iter()
        .filter(|role| !occupied.contains(&role.role))
        .count();
    let availability = if free_secondary > 0 { 1.0 } else { 0.6 };
    let confidence =
        secondary.iter().map(|role| role.confidence).sum::<f32>() / secondary.len() as f32;
    Some(FeatureValue::new(
        FeatureId::Flex,
        (base * quality * availability).clamp(0.0, 1.0),
        confidence,
    ))
}

struct BalanceFeature {
    value: FeatureValue,
    reasons: Vec<Reason>,
    risks: Vec<(super::model::RiskId, f32)>,
}

fn balance_feature(
    before: &TeamAnalysis,
    after: &TeamAnalysis,
    candidate: &ChampionProfile,
    blue_picks: &[PickedChampion],
    red_picks: &[PickedChampion],
    side: Side,
    profiles: &ProfileSet,
) -> Option<BalanceFeature> {
    let mut penalties = 0.0;
    let mut reasons = Vec::new();
    let mut risks = Vec::new();
    let mut confidence_sum = 0.0;
    let mut confidence_count = 0;
    let mut any_signal = false;

    let before_damage = before.damage.as_ref().map(normalized_damage);
    let after_damage = after.damage.as_ref().map(normalized_damage);
    if let (Some(candidate_shares), Some(before_damage), Some(after_damage)) =
        (candidate.damage_shares(), before_damage, after_damage)
    {
        any_signal = true;
        confidence_sum += average_damage_confidence(candidate);
        confidence_count += 1;
        let shares = candidate_shares.normalized();
        let (kind, candidate_share) =
            if shares.physical >= shares.magic && shares.physical >= shares.true_damage {
                (TraitId::DamagePhysical, shares.physical)
            } else if shares.magic >= shares.true_damage {
                (TraitId::DamageMagic, shares.magic)
            } else {
                (TraitId::DamageTrue, shares.true_damage)
            };
        let before_share = damage_component(before_damage, kind);
        let after_share = damage_component(after_damage, kind);
        if candidate_share >= 0.50 && after_share > 0.70 {
            penalties += 0.35;
            reasons.push(Reason::RedundantDamage {
                kind,
                share: after_share,
            });
            risks.push((super::model::RiskId::RedundantDamage, 0.35));
        } else if before_share > 0.70 && after_share <= 0.70 {
            reasons.push(Reason::DamageBalance {
                kind,
                before: before_share,
                after: after_share,
            });
        }
    }

    let teammates = match side {
        Side::Blue => blue_picks,
        Side::Red => red_picks,
    };
    let carries = resource_carries(teammates, profiles);
    if let Some((candidate_gold, candidate_confidence)) =
        trait_known(candidate, TraitId::GoldDependency)
    {
        any_signal = true;
        confidence_sum += candidate_confidence;
        confidence_count += 1;
        if carries >= 2 && candidate_gold >= 0.70 {
            penalties += 0.20;
            reasons.push(Reason::ResourceConflict {
                allies: carries as u8,
            });
            risks.push((super::model::RiskId::ResourceConflict, 0.20));
        }
    }

    if let Some((frontline_before, frontline_confidence)) = coverage_known(
        before,
        Dimension::Archetype(crate::taxonomy::ArchetypeId::Frontline),
    ) {
        any_signal = true;
        confidence_sum += frontline_confidence;
        confidence_count += 1;
        let candidate_frontline = trait_known(candidate, TraitId::Durability)
            .map(|(value, _)| value)
            .or_else(|| {
                candidate
                    .archetypes
                    .get(&crate::taxonomy::ArchetypeId::Frontline)
                    .and_then(|rating| rating.intensity)
            });
        if frontline_before < 0.40 && candidate_frontline.is_some_and(|value| value < 0.40) {
            penalties += 0.25;
            reasons.push(Reason::NoFrontline);
            risks.push((super::model::RiskId::NoFrontline, 0.25));
        }
    }

    let late_before = coverage_known(before, Dimension::Trait(TraitId::LatePower));
    let early_before = coverage_known(before, Dimension::Trait(TraitId::EarlyPower));
    if let (Some((late, late_conf)), Some((early, _))) = (late_before, early_before) {
        any_signal = true;
        confidence_sum += late_conf;
        confidence_count += 1;
        let candidate_late = trait_known(candidate, TraitId::LatePower)
            .map(|(value, _)| value)
            .unwrap_or(0.0);
        if late >= 0.65 && early < 0.45 && candidate_late >= 0.65 {
            penalties += 0.20;
            reasons.push(Reason::ScalingConflict);
            risks.push((super::model::RiskId::ScalingConflict, 0.20));
        }
    }

    if !any_signal {
        return None;
    }
    let confidence = if confidence_count > 0 {
        confidence_sum / confidence_count as f32
    } else {
        0.0
    };
    Some(BalanceFeature {
        value: FeatureValue::new(
            FeatureId::Balance,
            (1.0_f32 - penalties).clamp(0.0, 1.0),
            confidence,
        ),
        reasons,
        risks,
    })
}

fn safety_feature(profile: &ChampionProfile) -> Option<FeatureValue> {
    let parts = [
        (TraitId::LaneSafety, 0.50),
        (TraitId::Mobility, 0.30),
        (TraitId::Range, 0.20),
    ];
    let mut value = 0.0;
    let mut known_weight = 0.0;
    let mut confidence = 0.0;
    for (id, weight) in parts {
        if let Some((rating, rating_confidence)) = trait_known(profile, id) {
            value += weight * rating;
            known_weight += weight;
            confidence += weight * rating_confidence;
        }
    }
    if known_weight <= 0.0 {
        return None;
    }
    Some(FeatureValue::new(
        FeatureId::Safety,
        value / known_weight,
        confidence / known_weight,
    ))
}

// ---------------------------------------------------------------------------
// Pesos y contexto
// ---------------------------------------------------------------------------

fn pick_weights(
    phase: PickWeightPhase,
    action_index: usize,
    enemy_revealed: usize,
    our_carries: usize,
    params: &RecommendationParams,
) -> FeatureWeights {
    let mut weights = params.pick_weights(phase);
    if action_index == 6 {
        weights.flex += 0.10;
        weights.safety += 0.10;
        weights.counter -= 0.05;
    }
    if action_index == 19 {
        weights.counter += 0.10;
        weights.answer += 0.05;
        weights.flex -= 0.10;
    }
    if enemy_revealed >= 3 {
        weights.counter += 0.05;
        weights.answer += 0.05;
    } else if enemy_revealed <= 1 {
        weights.flex += 0.05;
        weights.safety += 0.05;
    }
    if our_carries >= 2 {
        weights.balance += 0.05;
        weights.power -= 0.02;
    }
    weights.normalized()
}

fn ban_weights(
    phase: DraftPhase,
    enemy_revealed: usize,
    params: &RecommendationParams,
) -> BanWeights {
    let mut weights = params.ban_weights;
    if matches!(phase, DraftPhase::EarlyBan) {
        weights.opp_flex += 0.05;
        weights.opp_power += 0.05;
    }
    if matches!(phase, DraftPhase::LateBan) && enemy_revealed >= 3 {
        weights.own_threat += 0.10;
        weights.opp_need += 0.05;
    }
    weights.normalized()
}

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

fn team_analysis(analysis: &DraftAnalysis, side: Side) -> &TeamAnalysis {
    match side {
        Side::Blue => &analysis.blue,
        Side::Red => &analysis.red,
    }
}

fn missing_features(features: &[FeatureValue], all: &[FeatureId]) -> Vec<FeatureId> {
    all.iter()
        .copied()
        .filter(|id| !features.iter().any(|feature| feature.id == *id))
        .collect()
}

fn candidate_confidence(knowledge: f32, role: f32, context: f32) -> f32 {
    let combined = 0.55 * score::clamp_unit(knowledge)
        + 0.25 * score::clamp_unit(role)
        + 0.20 * score::clamp_unit(context);
    // La confianza contextual no puede presentar un perfil casi vacío como
    // concluyente.
    combined
        .min(score::clamp_unit(knowledge) + 0.15)
        .clamp(0.0, 1.0)
}

fn role_confidence(profile: &ChampionProfile, role: Role) -> f32 {
    profile
        .roles
        .iter()
        .find(|entry| entry.role == role)
        .map(|entry| entry.confidence)
        .unwrap_or(0.0)
}

fn resource_carries(picks: &[PickedChampion], profiles: &ProfileSet) -> usize {
    picks
        .iter()
        .filter(|pick| {
            profiles
                .get(pick.key)
                .and_then(|profile| profile.trait_value(TraitId::GoldDependency))
                .is_some_and(|value| value >= 0.70)
        })
        .count()
}

fn trait_known(profile: &ChampionProfile, id: TraitId) -> Option<(f32, f32)> {
    profile
        .traits
        .get(&id)
        .and_then(|rating| rating.value.map(|value| (value, rating.confidence)))
}

fn coverage_known(analysis: &TeamAnalysis, dimension: Dimension) -> Option<(f32, f32)> {
    analysis
        .coverage(dimension)
        .filter(|coverage| coverage.total > 0)
        .map(|coverage| (coverage.value_low, coverage.confidence))
}

fn normalized_damage(damage: &crate::analysis::DamageProfile) -> (f32, f32, f32) {
    let total = damage.physical + damage.magic + damage.true_damage;
    if total <= 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        (
            damage.physical / total,
            damage.magic / total,
            damage.true_damage / total,
        )
    }
}

fn damage_component(damage: (f32, f32, f32), kind: TraitId) -> f32 {
    match kind {
        TraitId::DamagePhysical => damage.0,
        TraitId::DamageMagic => damage.1,
        TraitId::DamageTrue => damage.2,
        _ => 0.0,
    }
}

fn average_damage_confidence(profile: &ChampionProfile) -> f32 {
    [
        TraitId::DamagePhysical,
        TraitId::DamageMagic,
        TraitId::DamageTrue,
    ]
    .iter()
    .filter_map(|id| profile.traits.get(id).map(|rating| rating.confidence))
    .sum::<f32>()
        / 3.0
}
