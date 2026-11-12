//! Agregación de un equipo y orquestación del análisis de draft.

use super::needs;
use super::params::{AnalysisParams, AnalysisParamsError, Dimension};
use super::win_conditions;
use super::{
    clamp_unit, CcProfile, Coverage, DamageProfile, DraftAnalysis, PowerCurve, RangeProfile,
    Signal, TeamAnalysis,
};
use crate::profile::{ChampionProfile, ProfileSet};
use crate::taxonomy::{ArchetypeId, TraitId, TraitKind};
use loldraft_domain::{ChampionKey, Role, Side};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickedChampion {
    pub key: ChampionKey,
    pub role: Role,
    #[serde(default = "default_assignment_confidence")]
    pub assignment_confidence: f32,
}

impl PickedChampion {
    pub fn new(key: ChampionKey, role: Role, assignment_confidence: f32) -> Self {
        Self {
            key,
            role,
            assignment_confidence,
        }
    }
}

fn default_assignment_confidence() -> f32 {
    0.5
}

/// Analiza un equipo aislado. Las necesidades solo incluyen suelos y balance de
/// daño; para necesidades por amenazas usa `analyze_draft`.
pub fn analyze_team(
    side: Side,
    picks: &[PickedChampion],
    profiles: &ProfileSet,
    params: &AnalysisParams,
) -> Result<TeamAnalysis, AnalysisError> {
    params.validate()?;
    validate_team(side, picks)?;
    let aggregate = aggregate(side, picks, profiles, params);
    Ok(aggregate.finalize(None, params))
}

/// Analiza ambos equipos, incluyendo needs condicionadas por el rival.
pub fn analyze_draft(
    blue_picks: &[PickedChampion],
    red_picks: &[PickedChampion],
    profiles: &ProfileSet,
    params: &AnalysisParams,
) -> Result<DraftAnalysis, AnalysisError> {
    params.validate()?;
    validate_team(Side::Blue, blue_picks)?;
    validate_team(Side::Red, red_picks)?;
    reject_cross_team_duplicates(blue_picks, red_picks)?;

    let blue_aggregate = aggregate(Side::Blue, blue_picks, profiles, params);
    let red_aggregate = aggregate(Side::Red, red_picks, profiles, params);
    let blue = blue_aggregate.finalize(Some(&red_aggregate), params);
    let red = red_aggregate.finalize(Some(&blue_aggregate), params);

    Ok(DraftAnalysis {
        params_version: params.version,
        engine_version: crate::engine_version().to_string(),
        blue,
        red,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("el equipo {side} tiene {count} picks; el máximo es 5")]
    TooManyPicks { side: &'static str, count: usize },
    #[error("campeón duplicado en el equipo {side}: {key}")]
    DuplicateChampion {
        side: &'static str,
        key: ChampionKey,
    },
    #[error("campeón duplicado entre ambos equipos: {key}")]
    DuplicateChampionAcrossTeams { key: ChampionKey },
    #[error("rol duplicado en el equipo {side}: {role}")]
    DuplicateRole {
        side: &'static str,
        role: &'static str,
    },
    #[error("confianza de asignación inválida en {key}: {value}")]
    InvalidAssignmentConfidence { key: ChampionKey, value: f32 },
    #[error(transparent)]
    InvalidParams(#[from] AnalysisParamsError),
}

fn validate_team(side: Side, picks: &[PickedChampion]) -> Result<(), AnalysisError> {
    if picks.len() > 5 {
        return Err(AnalysisError::TooManyPicks {
            side: side.as_str(),
            count: picks.len(),
        });
    }
    let mut champions = HashSet::with_capacity(picks.len());
    let mut roles = HashSet::with_capacity(picks.len());
    for pick in picks {
        if !champions.insert(pick.key) {
            return Err(AnalysisError::DuplicateChampion {
                side: side.as_str(),
                key: pick.key,
            });
        }
        if !roles.insert(pick.role) {
            return Err(AnalysisError::DuplicateRole {
                side: side.as_str(),
                role: pick.role.as_str(),
            });
        }
        if !pick.assignment_confidence.is_finite()
            || !(0.0..=1.0).contains(&pick.assignment_confidence)
        {
            return Err(AnalysisError::InvalidAssignmentConfidence {
                key: pick.key,
                value: pick.assignment_confidence,
            });
        }
    }
    Ok(())
}

fn reject_cross_team_duplicates(
    blue: &[PickedChampion],
    red: &[PickedChampion],
) -> Result<(), AnalysisError> {
    let blue_keys: HashSet<ChampionKey> = blue.iter().map(|pick| pick.key).collect();
    if let Some(key) = red
        .iter()
        .map(|pick| pick.key)
        .find(|key| blue_keys.contains(key))
    {
        return Err(AnalysisError::DuplicateChampionAcrossTeams { key });
    }
    Ok(())
}

struct ResolvedPick<'a> {
    pick: &'a PickedChampion,
    profile: Option<&'a ChampionProfile>,
    role_factor: f32,
}

impl ResolvedPick<'_> {
    fn cell(&self, value: Option<f32>, confidence: f32, weight: f32) -> Cell {
        Cell {
            value,
            confidence,
            factor: self.role_factor,
            weight,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    value: Option<f32>,
    confidence: f32,
    factor: f32,
    weight: f32,
}

pub(crate) struct TeamAggregate {
    pub(crate) side: Side,
    pub(crate) picks: usize,
    pub(crate) archetypes: BTreeMap<ArchetypeId, Coverage>,
    pub(crate) traits: BTreeMap<TraitId, Coverage>,
    pub(crate) damage: Option<DamageProfile>,
    pub(crate) curve: PowerCurve,
    pub(crate) range: RangeProfile,
    pub(crate) cc: CcProfile,
    pub(crate) mobility: Coverage,
    pub(crate) waveclear: Coverage,
    pub(crate) objective_dps: Coverage,
    pub(crate) knowledge: f32,
    pub(crate) assignment_confidence: f32,
}

impl TeamAggregate {
    pub(crate) fn coverage(&self, dimension: Dimension) -> Option<&Coverage> {
        match dimension {
            Dimension::Archetype(id) => self.archetypes.get(&id),
            Dimension::Trait(id) => self.traits.get(&id),
        }
    }

    pub(crate) fn finalize(
        &self,
        enemy: Option<&TeamAggregate>,
        params: &AnalysisParams,
    ) -> TeamAnalysis {
        let needs = needs::compute(self, enemy, params);
        let win_conditions = win_conditions::detect(self);
        let strengths = strengths(self);
        let weaknesses = weaknesses(self, &needs);
        let confidence = if self.knowledge <= 0.0 {
            0.0
        } else {
            (0.70 * self.knowledge + 0.30 * self.assignment_confidence)
                .min(self.knowledge)
                .clamp(0.0, 1.0)
        };
        let partial = self.picks < 5 || self.knowledge < 0.60;

        TeamAnalysis {
            side: self.side,
            picks: self.picks,
            partial,
            archetypes: self.archetypes.clone(),
            traits: self.traits.clone(),
            damage: self.damage.clone(),
            curve: self.curve.clone(),
            range: self.range.clone(),
            cc: self.cc.clone(),
            mobility: self.mobility.clone(),
            waveclear: self.waveclear.clone(),
            objective_dps: self.objective_dps.clone(),
            needs,
            win_conditions,
            strengths,
            weaknesses,
            knowledge: self.knowledge,
            confidence,
        }
    }
}

fn aggregate(
    side: Side,
    picks: &[PickedChampion],
    profiles: &ProfileSet,
    params: &AnalysisParams,
) -> TeamAggregate {
    let resolved: Vec<ResolvedPick<'_>> = picks
        .iter()
        .map(|pick| {
            let profile = profiles.get(pick.key);
            let role_factor = profile
                .and_then(|profile| {
                    profile
                        .roles
                        .iter()
                        .find(|role| role.role == pick.role)
                        .map(|role| clamp_unit(role.proficiency))
                })
                .unwrap_or(1.0);
            ResolvedPick {
                pick,
                profile,
                role_factor,
            }
        })
        .collect();

    let mut archetypes = BTreeMap::new();
    for id in ArchetypeId::ALL {
        let cells: Vec<Cell> = resolved
            .iter()
            .map(|resolved| {
                let rating = resolved
                    .profile
                    .and_then(|profile| profile.archetypes.get(&id));
                resolved.cell(
                    rating.and_then(|rating| rating.intensity),
                    rating.map(|rating| rating.confidence).unwrap_or(0.0),
                    1.0,
                )
            })
            .collect();
        archetypes.insert(id, saturating(&cells, params.saturation_weights));
    }

    let mut traits = BTreeMap::new();
    for id in TraitId::ALL {
        if id.kind() == TraitKind::DamageShare {
            continue;
        }
        let weight_mode = if matches!(
            id,
            TraitId::EarlyPower | TraitId::MidPower | TraitId::LatePower
        ) {
            WeightMode::Gold
        } else {
            WeightMode::Equal
        };
        let cells: Vec<Cell> = resolved
            .iter()
            .map(|resolved| {
                let rating = resolved.profile.and_then(|profile| profile.traits.get(&id));
                let weight = match weight_mode {
                    WeightMode::Gold => params.gold_weight(resolved.pick.role),
                    WeightMode::Equal => 1.0,
                };
                resolved.cell(
                    rating.and_then(|rating| rating.value),
                    rating.map(|rating| rating.confidence).unwrap_or(0.0),
                    weight,
                )
            })
            .collect();
        let coverage = if is_saturating_trait(id) {
            saturating(&cells, params.saturation_weights)
        } else {
            weighted_mean(&cells)
        };
        traits.insert(id, coverage);
    }

    let damage = damage_profile(&resolved, params);
    let range = range_profile(&resolved);
    let cc = CcProfile {
        hard: traits[&TraitId::HardCc].clone(),
        soft: traits[&TraitId::SoftCc].clone(),
    };
    let curve = PowerCurve {
        early: traits[&TraitId::EarlyPower].clone(),
        mid: traits[&TraitId::MidPower].clone(),
        late: traits[&TraitId::LatePower].clone(),
    };
    let mobility = traits[&TraitId::Mobility].clone();
    let waveclear = traits[&TraitId::Waveclear].clone();
    let objective_dps = traits[&TraitId::ObjectiveDps].clone();

    let knowledge = if picks.is_empty() {
        0.0
    } else {
        resolved
            .iter()
            .map(|resolved| {
                resolved
                    .profile
                    .map(ChampionProfile::known_cell_ratio)
                    .unwrap_or(0.0)
            })
            .sum::<f32>()
            / picks.len() as f32
    };
    let assignment_confidence = if picks.is_empty() {
        0.0
    } else {
        picks
            .iter()
            .map(|pick| clamp_unit(pick.assignment_confidence))
            .sum::<f32>()
            / picks.len() as f32
    };

    TeamAggregate {
        side,
        picks: picks.len(),
        archetypes,
        traits,
        damage,
        curve,
        range,
        cc,
        mobility,
        waveclear,
        objective_dps,
        knowledge,
        assignment_confidence,
    }
}

enum WeightMode {
    Gold,
    Equal,
}

fn is_saturating_trait(id: TraitId) -> bool {
    matches!(
        id,
        TraitId::HardCc
            | TraitId::SoftCc
            | TraitId::Waveclear
            | TraitId::ObjectiveDps
            | TraitId::Durability
            | TraitId::Aoe
            | TraitId::SingleTarget
    )
}

/// Suma saturante de las tres contribuciones mayores. Un desconocido vale 0 en
/// la cota inferior y `factor` en la superior.
fn saturating(cells: &[Cell], weights: [f32; 3]) -> Coverage {
    let total = cells.len();
    let mut lows: Vec<f32> = cells
        .iter()
        .map(|cell| {
            cell.value
                .map(|value| clamp_unit(value) * cell.factor)
                .unwrap_or(0.0)
        })
        .collect();
    let mut highs: Vec<f32> = cells
        .iter()
        .map(|cell| {
            cell.value
                .map(|value| clamp_unit(value) * cell.factor)
                .unwrap_or(clamp_unit(cell.factor))
        })
        .collect();
    lows.sort_by(|a, b| b.total_cmp(a));
    highs.sort_by(|a, b| b.total_cmp(a));

    let sum = |values: &[f32]| -> f32 {
        values
            .iter()
            .take(weights.len())
            .zip(weights)
            .map(|(value, weight)| value * weight)
            .sum()
    };
    let known_cells: Vec<&Cell> = cells.iter().filter(|cell| cell.value.is_some()).collect();
    let known = known_cells.len();
    let average_confidence = if known > 0 {
        known_cells
            .iter()
            .map(|cell| clamp_unit(cell.confidence))
            .sum::<f32>()
            / known as f32
    } else {
        0.0
    };
    let confidence = if total > 0 {
        average_confidence * known as f32 / total as f32
    } else {
        0.0
    };

    Coverage::new(sum(&lows), sum(&highs), known, total, confidence)
}

/// Media ponderada. Los desconocidos valen 0/1 en los extremos; el peso de un
/// desconocido también cuenta en la confianza como no conocido.
fn weighted_mean(cells: &[Cell]) -> Coverage {
    let total = cells.len();
    let total_weight: f32 = cells.iter().map(|cell| cell.weight).sum();
    if total_weight <= 0.0 {
        return Coverage::new(0.0, 0.0, 0, total, 0.0);
    }

    let low = cells
        .iter()
        .map(|cell| {
            cell.weight
                * cell
                    .value
                    .map(|value| clamp_unit(value) * cell.factor)
                    .unwrap_or(0.0)
        })
        .sum::<f32>()
        / total_weight;
    let high = cells
        .iter()
        .map(|cell| {
            cell.weight
                * cell
                    .value
                    .map(|value| clamp_unit(value) * cell.factor)
                    .unwrap_or(clamp_unit(cell.factor))
        })
        .sum::<f32>()
        / total_weight;

    let known_cells: Vec<&Cell> = cells.iter().filter(|cell| cell.value.is_some()).collect();
    let known_weight: f32 = known_cells.iter().map(|cell| cell.weight).sum();
    let average_confidence = if known_weight > 0.0 {
        known_cells
            .iter()
            .map(|cell| cell.weight * clamp_unit(cell.confidence))
            .sum::<f32>()
            / known_weight
    } else {
        0.0
    };
    let confidence = average_confidence * (known_weight / total_weight).clamp(0.0, 1.0);

    Coverage::new(low, high, known_cells.len(), total, confidence)
}

fn damage_profile(resolved: &[ResolvedPick<'_>], params: &AnalysisParams) -> Option<DamageProfile> {
    let total_weight: f32 = resolved
        .iter()
        .map(|resolved| params.gold_weight(resolved.pick.role))
        .sum();
    if total_weight <= 0.0 {
        return None;
    }

    let mut known_weight = 0.0;
    let mut physical = 0.0;
    let mut magic = 0.0;
    let mut true_damage = 0.0;
    let mut confidence_sum = 0.0;

    for resolved in resolved {
        let Some(profile) = resolved.profile else {
            continue;
        };
        let Some(shares) = profile.damage_shares() else {
            continue;
        };
        let shares = shares.normalized();
        let weight = params.gold_weight(resolved.pick.role);
        known_weight += weight;
        physical += weight * shares.physical;
        magic += weight * shares.magic;
        true_damage += weight * shares.true_damage;

        let confidence = [
            TraitId::DamagePhysical,
            TraitId::DamageMagic,
            TraitId::DamageTrue,
        ]
        .iter()
        .filter_map(|id| profile.traits.get(id).map(|rating| rating.confidence))
        .sum::<f32>()
            / 3.0;
        confidence_sum += weight * clamp_unit(confidence);
    }

    if known_weight <= 0.0 {
        return None;
    }
    let known_weight_ratio = (known_weight / total_weight).clamp(0.0, 1.0);
    Some(DamageProfile {
        physical: (physical / known_weight).clamp(0.0, 1.0),
        magic: (magic / known_weight).clamp(0.0, 1.0),
        true_damage: (true_damage / known_weight).clamp(0.0, 1.0),
        known_weight_ratio,
        confidence: ((confidence_sum / known_weight) * known_weight_ratio).clamp(0.0, 1.0),
    })
}

fn range_profile(resolved: &[ResolvedPick<'_>]) -> RangeProfile {
    let cells: Vec<Cell> =
        resolved
            .iter()
            .map(|resolved| {
                let rating = resolved
                    .profile
                    .and_then(|profile| profile.traits.get(&TraitId::Range));
                let value = rating.and_then(|rating| rating.value).map(|value| {
                    if value >= 0.60 {
                        1.0
                    } else {
                        0.0
                    }
                });
                resolved.cell(
                    value,
                    rating.map(|rating| rating.confidence).unwrap_or(0.0),
                    1.0,
                )
            })
            .collect();
    let melee_count = resolved
        .iter()
        .filter(|resolved| {
            resolved
                .profile
                .and_then(|profile| profile.traits.get(&TraitId::Range))
                .and_then(|rating| rating.value)
                .is_some_and(|value| value < 0.35)
        })
        .count();
    let ranged_share = weighted_mean(&cells);
    let known = ranged_share.known;
    RangeProfile {
        ranged_share,
        melee_count,
        known,
    }
}

fn strengths(aggregate: &TeamAggregate) -> Vec<Signal> {
    let mut signals = Vec::new();
    for (id, coverage) in &aggregate.archetypes {
        if coverage.total > 0 && coverage.known > 0 && coverage.value_low >= 0.60 {
            signals.push(Signal::from_coverage(Dimension::Archetype(*id), coverage));
        }
    }
    for (id, coverage) in &aggregate.traits {
        if coverage.total > 0 && coverage.known > 0 && coverage.value_low >= 0.60 {
            signals.push(Signal::from_coverage(Dimension::Trait(*id), coverage));
        }
    }
    signals.sort_by(|left, right| {
        right
            .value_low
            .total_cmp(&left.value_low)
            .then_with(|| left.dimension.cmp(&right.dimension))
    });
    signals.truncate(6);
    signals
}

fn weaknesses(aggregate: &TeamAggregate, needs: &[needs::Need]) -> Vec<Signal> {
    let mut signals = Vec::new();
    for need in needs
        .iter()
        .filter(|need| need.status == needs::NeedStatus::Confirmed)
    {
        if let Some(coverage) = aggregate.coverage(need.dimension) {
            signals.push(Signal::from_coverage(need.dimension, coverage));
        }
        if signals.len() == 6 {
            break;
        }
    }
    signals
}
