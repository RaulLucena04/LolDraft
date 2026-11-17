//! Puntuación, riesgo, confianza y diversidad.
//!
//! La suma de contribuciones y riesgos coincide con la puntuación interna. Las
//! features ausentes no valen 0.5: se excluyen y los pesos restantes se
//! renormalizan.

use super::model::{Contribution, Recommendation, RiskContribution, RiskId};
use super::params::{FeatureId, FeatureWeights, RecommendationParams};
use crate::profile::ProfileSet;
use crate::taxonomy::ArchetypeId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FeatureValue {
    pub id: FeatureId,
    pub value: f32,
    pub confidence: f32,
}

impl FeatureValue {
    pub(crate) fn new(id: FeatureId, value: f32, confidence: f32) -> Self {
        Self {
            id,
            value: clamp_unit(value),
            confidence: clamp_unit(confidence),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoreResult {
    pub contributions: Vec<Contribution>,
    pub risks: Vec<RiskContribution>,
    pub score: f32,
    pub ranking: f32,
}

/// Puntúa un candidato. Devuelve `None` si ninguna feature presente tiene peso,
/// porque inventar un 0.5 sería peor que no recomendar.
pub(crate) fn score(
    features: &[FeatureValue],
    weights: &FeatureWeights,
    risk_amounts: &[(RiskId, f32)],
    confidence: f32,
    params: &RecommendationParams,
) -> Option<ScoreResult> {
    let mut unique: BTreeMap<FeatureId, FeatureValue> = BTreeMap::new();
    for feature in features {
        unique.insert(feature.id, *feature);
    }

    let weight_sum: f32 = unique.keys().map(|id| weights.get(*id).max(0.0)).sum();
    if weight_sum <= 0.0 {
        return None;
    }

    let mut contributions = Vec::with_capacity(unique.len());
    let mut raw = 0.0;
    for (id, feature) in &unique {
        let weight = weights.get(*id).max(0.0) / weight_sum;
        let contribution = weight * clamp_unit(feature.value);
        raw += contribution;
        contributions.push(Contribution {
            feature: *id,
            value: clamp_unit(feature.value),
            weight,
            contribution,
        });
    }
    contributions.sort_by_key(|entry| entry.feature);

    let risk_sum: f32 = risk_amounts
        .iter()
        .map(|(_, amount)| clamp_unit(*amount))
        .sum();
    let risk_total = risk_sum.min(1.0);
    // El riesgo nunca puede dejar la puntuación por debajo de cero. Si lo
    // hiciera, se recorta el total aplicado y se reparte proporcionalmente para
    // que `sum(contributions) + sum(risks) == score` siga siendo exacto.
    let requested_risk = params.risk_coefficient * risk_total;
    let applied_risk = requested_risk.min(raw);
    let risk_scale = if risk_sum > 0.0 {
        (applied_risk / requested_risk).min(1.0) * (risk_total / risk_sum)
    } else {
        0.0
    };
    let risks: Vec<RiskContribution> = risk_amounts
        .iter()
        .map(|(kind, amount)| {
            let applied = clamp_unit(*amount) * risk_scale;
            RiskContribution {
                kind: *kind,
                amount: clamp_unit(*amount),
                contribution: -params.risk_coefficient * applied,
            }
        })
        .collect();

    let score = (raw - applied_risk).clamp(0.0, 1.0);
    let confidence = clamp_unit(confidence);
    let ranking = score - params.confidence_penalty * (1.0 - confidence);

    Some(ScoreResult {
        contributions,
        risks,
        score,
        ranking,
    })
}

pub(crate) fn display(score: f32) -> u8 {
    (clamp_unit(score) * 100.0).round().clamp(0.0, 100.0) as u8
}

pub(crate) fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Diversifica el top sin alterar las puntuaciones mostradas.
///
/// La primera recomendación es siempre la de mayor `ranking`. Después se aplica
/// diversificación MMR por solapamiento de rol y arquetipos, con un máximo de
/// `max_per_role` por rol salvo que no queden alternativas.
pub(crate) fn diversify(
    mut picks: Vec<Recommendation>,
    profiles: &ProfileSet,
    params: &RecommendationParams,
) -> Vec<Recommendation> {
    picks.sort_by(|left, right| {
        right
            .ranking_score
            .total_cmp(&left.ranking_score)
            .then_with(|| left.champion.cmp(&right.champion))
    });

    let mut selected: Vec<Recommendation> = Vec::with_capacity(params.limit);
    let mut remaining = picks;
    while selected.len() < params.limit && !remaining.is_empty() {
        let counts = role_counts(&selected);
        let mut best_index: Option<usize> = None;
        let mut best_adjusted = f32::NEG_INFINITY;
        let mut best_any_index: Option<usize> = None;
        let mut best_any_adjusted = f32::NEG_INFINITY;

        for (index, candidate) in remaining.iter().enumerate() {
            let overlap = selected
                .iter()
                .map(|chosen| overlap(candidate, chosen, profiles))
                .fold(0.0, f32::max);
            let adjusted = candidate.ranking_score - params.diversity_lambda * overlap;
            if adjusted > best_any_adjusted {
                best_any_adjusted = adjusted;
                best_any_index = Some(index);
            }
            let at_cap = counts[candidate.role.index()] >= params.max_per_role;
            if at_cap {
                continue;
            }
            if adjusted > best_adjusted {
                best_adjusted = adjusted;
                best_index = Some(index);
            }
        }

        // Si todos los candidatos restantes están en el tope de rol, se relaja
        // la restricción: es preferible completar la lista a dejarla corta.
        let chosen = best_index.or(best_any_index);
        let Some(chosen) = chosen else {
            break;
        };
        selected.push(remaining.remove(chosen));
    }
    selected
}

fn role_counts(selected: &[Recommendation]) -> [usize; 5] {
    let mut counts = [0usize; 5];
    for pick in selected {
        counts[pick.role.index()] += 1;
    }
    counts
}

/// `0.5 * mismo_rol + 0.5 * coseno(arquetipos)`.
fn overlap(left: &Recommendation, right: &Recommendation, profiles: &ProfileSet) -> f32 {
    let same_role = if left.role == right.role { 1.0 } else { 0.0 };
    let cosine = match (profiles.get(left.champion), profiles.get(right.champion)) {
        (Some(left), Some(right)) => archetype_cosine(left, right),
        _ => 0.0,
    };
    (0.5 * same_role + 0.5 * cosine).clamp(0.0, 1.0)
}

fn archetype_cosine(
    left: &crate::profile::ChampionProfile,
    right: &crate::profile::ChampionProfile,
) -> f32 {
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for id in ArchetypeId::ALL {
        let left_value = left
            .archetypes
            .get(&id)
            .and_then(|rating| rating.intensity)
            .unwrap_or(0.0);
        let right_value = right
            .archetypes
            .get(&id)
            .and_then(|rating| rating.intensity)
            .unwrap_or(0.0);
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }
    if left_norm <= 0.0 || right_norm <= 0.0 {
        return 0.0;
    }
    (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(0.0, 1.0)
}
