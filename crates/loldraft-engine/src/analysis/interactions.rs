//! Sinergias y matchups derivados de los perfiles.
//!
//! Ninguna regla es un resultado observado: son comparaciones tipadas entre
//! celdas de perfil. Un requisito desconocido ensancha el intervalo en lugar de
//! contar como cero o como carencia confirmada. La UI debe presentar estas
//! lecturas como derivadas y no como probabilidades.

use super::params::Dimension;
use super::team::{PickedChampion, TeamAggregate};
use super::{clamp_unit, Coverage};
use crate::profile::{ChampionProfile, ProfileSet};
use crate::taxonomy::{ArchetypeId, TraitId};
use loldraft_domain::{ChampionKey, Role, Side};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Versión de las tablas de reglas de sinergia y matchup.
pub const INTERACTIONS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynergyId {
    EngageBurst,
    EngageDps,
    WomboAoe,
    PeelDps,
    FrontlineDps,
    PokeSiege,
    CatchBurst,
    SustainPoke,
    SplitWaveclear,
    ObjectiveFrontline,
}

impl SynergyId {
    pub const ALL: [SynergyId; 10] = [
        SynergyId::EngageBurst,
        SynergyId::EngageDps,
        SynergyId::WomboAoe,
        SynergyId::PeelDps,
        SynergyId::FrontlineDps,
        SynergyId::PokeSiege,
        SynergyId::CatchBurst,
        SynergyId::SustainPoke,
        SynergyId::SplitWaveclear,
        SynergyId::ObjectiveFrontline,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SynergyId::EngageBurst => "engage_burst",
            SynergyId::EngageDps => "engage_dps",
            SynergyId::WomboAoe => "wombo_aoe",
            SynergyId::PeelDps => "peel_dps",
            SynergyId::FrontlineDps => "frontline_dps",
            SynergyId::PokeSiege => "poke_siege",
            SynergyId::CatchBurst => "catch_burst",
            SynergyId::SustainPoke => "sustain_poke",
            SynergyId::SplitWaveclear => "split_waveclear",
            SynergyId::ObjectiveFrontline => "objective_frontline",
        }
    }
}

/// Sinergia derivada entre dos picks del mismo equipo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Synergy {
    pub id: SynergyId,
    /// Orden significativo: `[0]` cumple el requisito izquierdo de la regla.
    pub champions: [ChampionKey; 2],
    pub score_low: f32,
    pub score_high: f32,
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
    pub evidence: Vec<Dimension>,
}

/// Ventaja derivada, siempre con intervalo. `low > 0` implica ventaja del lado
/// atacante incluso en la cota inferior; `low <= 0 <= high` no afirma nada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Advantage {
    pub low: f32,
    pub high: f32,
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
    /// Solo se rellena cuando el intervalo no cruza el cero.
    pub favored: Option<Side>,
}

/// Lectura de una línea compartida por ambos equipos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaneMatchup {
    pub role: Role,
    pub blue: ChampionKey,
    pub red: ChampionKey,
    /// Ventaja de presión de línea desde el punto de vista azul.
    pub pressure: Advantage,
    /// Ventaja de amenaza de all-in desde el punto de vista azul.
    pub all_in: Advantage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchupRuleId {
    PokeSustain,
    AntiTankFrontline,
    CatchMobility,
    SplitWaveclear,
    EngageDisengage,
    DpsTank,
    BurstPeel,
    SiegeWaveclear,
    WomboDisengage,
}

impl MatchupRuleId {
    pub const ALL: [MatchupRuleId; 9] = [
        MatchupRuleId::PokeSustain,
        MatchupRuleId::AntiTankFrontline,
        MatchupRuleId::CatchMobility,
        MatchupRuleId::SplitWaveclear,
        MatchupRuleId::EngageDisengage,
        MatchupRuleId::DpsTank,
        MatchupRuleId::BurstPeel,
        MatchupRuleId::SiegeWaveclear,
        MatchupRuleId::WomboDisengage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            MatchupRuleId::PokeSustain => "poke_sustain",
            MatchupRuleId::AntiTankFrontline => "anti_tank_frontline",
            MatchupRuleId::CatchMobility => "catch_mobility",
            MatchupRuleId::SplitWaveclear => "split_waveclear",
            MatchupRuleId::EngageDisengage => "engage_disengage",
            MatchupRuleId::DpsTank => "dps_tank",
            MatchupRuleId::BurstPeel => "burst_peel",
            MatchupRuleId::SiegeWaveclear => "siege_waveclear",
            MatchupRuleId::WomboDisengage => "wombo_disengage",
        }
    }
}

/// Ventaja de equipo de un lado frente a una herramienta del rival. El signo es
/// relativo al lado atacante, no siempre a azul.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamMatchup {
    pub id: MatchupRuleId,
    pub side: Side,
    /// Dimensión que aporta la ventaja al lado atacante.
    pub attack: Dimension,
    /// Dimensión del rival que la mitiga.
    pub defend: Dimension,
    pub advantage: Advantage,
}

struct SynergyRule {
    id: SynergyId,
    left: (Dimension, f32),
    right: (Dimension, f32),
}

const SYNERGY_RULES: &[SynergyRule] = &[
    SynergyRule {
        id: SynergyId::EngageBurst,
        left: (Dimension::Archetype(ArchetypeId::Engage), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Burst), 0.50),
    },
    SynergyRule {
        id: SynergyId::EngageDps,
        left: (Dimension::Archetype(ArchetypeId::Engage), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Dps), 0.50),
    },
    SynergyRule {
        id: SynergyId::WomboAoe,
        left: (Dimension::Archetype(ArchetypeId::WomboCombo), 0.50),
        right: (Dimension::Trait(TraitId::Aoe), 0.50),
    },
    SynergyRule {
        id: SynergyId::PeelDps,
        left: (Dimension::Archetype(ArchetypeId::Peel), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Dps), 0.50),
    },
    SynergyRule {
        id: SynergyId::FrontlineDps,
        left: (Dimension::Archetype(ArchetypeId::Frontline), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Dps), 0.50),
    },
    SynergyRule {
        id: SynergyId::PokeSiege,
        left: (Dimension::Archetype(ArchetypeId::Poke), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Siege), 0.50),
    },
    SynergyRule {
        id: SynergyId::CatchBurst,
        left: (Dimension::Archetype(ArchetypeId::Catch), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Burst), 0.50),
    },
    SynergyRule {
        id: SynergyId::SustainPoke,
        left: (Dimension::Archetype(ArchetypeId::Poke), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Sustain), 0.50),
    },
    SynergyRule {
        id: SynergyId::SplitWaveclear,
        left: (Dimension::Archetype(ArchetypeId::Splitpush), 0.50),
        right: (Dimension::Trait(TraitId::Waveclear), 0.50),
    },
    SynergyRule {
        id: SynergyId::ObjectiveFrontline,
        left: (Dimension::Trait(TraitId::ObjectiveDps), 0.50),
        right: (Dimension::Archetype(ArchetypeId::Frontline), 0.50),
    },
];

struct TeamMatchupRule {
    id: MatchupRuleId,
    attack: Dimension,
    defend: Dimension,
}

const TEAM_MATCHUP_RULES: &[TeamMatchupRule] = &[
    TeamMatchupRule {
        id: MatchupRuleId::PokeSustain,
        attack: Dimension::Archetype(ArchetypeId::Poke),
        defend: Dimension::Archetype(ArchetypeId::Sustain),
    },
    TeamMatchupRule {
        id: MatchupRuleId::AntiTankFrontline,
        attack: Dimension::Archetype(ArchetypeId::AntiTank),
        defend: Dimension::Archetype(ArchetypeId::Frontline),
    },
    TeamMatchupRule {
        id: MatchupRuleId::CatchMobility,
        attack: Dimension::Archetype(ArchetypeId::Catch),
        defend: Dimension::Trait(TraitId::Mobility),
    },
    TeamMatchupRule {
        id: MatchupRuleId::SplitWaveclear,
        attack: Dimension::Archetype(ArchetypeId::Splitpush),
        defend: Dimension::Trait(TraitId::Waveclear),
    },
    TeamMatchupRule {
        id: MatchupRuleId::EngageDisengage,
        attack: Dimension::Archetype(ArchetypeId::Engage),
        defend: Dimension::Archetype(ArchetypeId::Disengage),
    },
    TeamMatchupRule {
        id: MatchupRuleId::DpsTank,
        attack: Dimension::Archetype(ArchetypeId::Dps),
        defend: Dimension::Archetype(ArchetypeId::Tank),
    },
    TeamMatchupRule {
        id: MatchupRuleId::BurstPeel,
        attack: Dimension::Archetype(ArchetypeId::Burst),
        defend: Dimension::Archetype(ArchetypeId::Peel),
    },
    TeamMatchupRule {
        id: MatchupRuleId::SiegeWaveclear,
        attack: Dimension::Archetype(ArchetypeId::Siege),
        defend: Dimension::Trait(TraitId::Waveclear),
    },
    TeamMatchupRule {
        id: MatchupRuleId::WomboDisengage,
        attack: Dimension::Archetype(ArchetypeId::WomboCombo),
        defend: Dimension::Archetype(ArchetypeId::Disengage),
    },
];

/// Métricas documentadas de la lectura de línea. No son observaciones.
const LANE_PRESSURE: &[(Dimension, f32)] = &[
    (Dimension::Trait(TraitId::EarlyPower), 0.40),
    (Dimension::Trait(TraitId::Range), 0.25),
    (Dimension::Trait(TraitId::LaneSafety), 0.20),
    (Dimension::Archetype(ArchetypeId::Sustain), 0.15),
];

const LANE_ALL_IN: &[(Dimension, f32)] = &[
    (Dimension::Archetype(ArchetypeId::Burst), 0.45),
    (Dimension::Trait(TraitId::SingleTarget), 0.30),
    (Dimension::Trait(TraitId::HardCc), 0.25),
];

/// Sinergias del equipo. Solo se emiten si al menos una celda requerida es
/// conocida y el intervalo puede llegar a 0.40.
pub(crate) fn synergies(picks: &[PickedChampion], profiles: &ProfileSet) -> Vec<Synergy> {
    let mut found = Vec::new();
    for (index, first) in picks.iter().enumerate() {
        for second in picks.iter().skip(index + 1) {
            let (Some(first_profile), Some(second_profile)) =
                (profiles.get(first.key), profiles.get(second.key))
            else {
                continue;
            };
            for rule in SYNERGY_RULES {
                let forward = evaluate_synergy(rule, first, first_profile, second, second_profile);
                let backward = evaluate_synergy(rule, second, second_profile, first, first_profile);
                if let Some(synergy) = pick_stronger(forward, backward) {
                    found.push(synergy);
                }
            }
        }
    }
    found.sort_by(|left, right| {
        (right.score_low + right.score_high)
            .total_cmp(&(left.score_low + left.score_high))
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.champions.cmp(&right.champions))
    });
    // Una regla aporta una sola pareja: repetir "engage + DPS" con cada carry
    // llenaría la lista sin añadir información distinta.
    let mut seen = HashSet::new();
    found.retain(|synergy| seen.insert(synergy.id));
    found.truncate(8);
    found
}

fn evaluate_synergy(
    rule: &SynergyRule,
    first: &PickedChampion,
    first_profile: &ChampionProfile,
    second: &PickedChampion,
    second_profile: &ChampionProfile,
) -> Option<Synergy> {
    let requirements = [
        (first, first_profile, rule.left),
        (second, second_profile, rule.right),
    ];
    let total = requirements.len();
    let mut low_sum = 0.0;
    let mut high_sum = 0.0;
    let mut confidence_sum = 0.0;
    let mut known = 0;

    for (pick, profile, (dimension, threshold)) in requirements {
        let factor = clamp_unit(role_factor(Some(profile), pick.role));
        match rating(profile, dimension) {
            Some((value, confidence)) => {
                let satisfaction = clamp_unit(value * factor / threshold);
                low_sum += satisfaction;
                high_sum += satisfaction;
                confidence_sum += clamp_unit(confidence);
                known += 1;
            }
            None => high_sum += factor,
        }
    }

    if known == 0 {
        return None;
    }
    let score_low = (low_sum / total as f32).clamp(0.0, 1.0);
    let score_high = (high_sum / total as f32).clamp(0.0, 1.0);
    if score_high < 0.40 {
        return None;
    }
    let confidence = (confidence_sum / known as f32) * (known as f32 / total as f32);

    Some(Synergy {
        id: rule.id,
        champions: [first.key, second.key],
        score_low,
        score_high,
        confidence: clamp_unit(confidence),
        known,
        total,
        evidence: vec![rule.left.0, rule.right.0],
    })
}

fn pick_stronger(left: Option<Synergy>, right: Option<Synergy>) -> Option<Synergy> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if (left.score_low + left.score_high) >= (right.score_low + right.score_high) {
                Some(left)
            } else {
                Some(right)
            }
        }
        (left, right) => left.or(right),
    }
}

/// Lectura de las líneas presentes en ambos equipos. Una línea sin picks en un
/// lado no se inventa: simplemente no aparece.
pub(crate) fn lane_matchups(
    blue_picks: &[PickedChampion],
    red_picks: &[PickedChampion],
    profiles: &ProfileSet,
) -> Vec<LaneMatchup> {
    let mut matchups = Vec::new();
    for role in Role::ALL {
        let (Some(blue), Some(red)) = (
            blue_picks.iter().find(|pick| pick.role == role),
            red_picks.iter().find(|pick| pick.role == role),
        ) else {
            continue;
        };
        let blue_profile = profiles.get(blue.key);
        let red_profile = profiles.get(red.key);
        let blue_pressure = metric(blue_profile, role, LANE_PRESSURE);
        let red_pressure = metric(red_profile, role, LANE_PRESSURE);
        let blue_all_in = metric(blue_profile, role, LANE_ALL_IN);
        let red_all_in = metric(red_profile, role, LANE_ALL_IN);

        matchups.push(LaneMatchup {
            role,
            blue: blue.key,
            red: red.key,
            pressure: signed_advantage(blue_pressure, red_pressure, Side::Blue),
            all_in: signed_advantage(blue_all_in, red_all_in, Side::Blue),
        });
    }
    matchups
}

/// Ventajas de equipo de ambos lados frente a la herramienta que las mitiga.
pub(crate) fn team_matchups(blue: &TeamAggregate, red: &TeamAggregate) -> Vec<TeamMatchup> {
    let mut matchups = Vec::new();
    for (side, attacker, defender) in [(Side::Blue, blue, red), (Side::Red, red, blue)] {
        for rule in TEAM_MATCHUP_RULES {
            let (Some(attack), Some(defend)) = (
                attacker.coverage(rule.attack),
                defender.coverage(rule.defend),
            ) else {
                continue;
            };
            if attack.known == 0 || attack.value_high <= 0.0 || defend.known == 0 {
                continue;
            }
            let gap = Metric {
                low: 1.0 - defend.value_high,
                high: 1.0 - defend.value_low,
                confidence: defend.confidence,
                known: defend.known,
                total: defend.total,
            };
            let advantage = product_advantage(metric_from_coverage(attack), gap, side);
            // Solo se muestra una ventaja que puede ser relevante; el resto se
            // omite en lugar de listar "no ventaja" como si fuera una señal.
            if advantage.high < 0.15 {
                continue;
            }
            matchups.push(TeamMatchup {
                id: rule.id,
                side,
                attack: rule.attack,
                defend: rule.defend,
                advantage,
            });
        }
    }
    matchups.sort_by(|left, right| {
        (right.advantage.low + right.advantage.high)
            .total_cmp(&(left.advantage.low + left.advantage.high))
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.side.as_str().cmp(right.side.as_str()))
    });
    matchups.truncate(8);
    matchups
}

#[derive(Debug, Clone, Copy)]
struct Metric {
    low: f32,
    high: f32,
    confidence: f32,
    known: usize,
    total: usize,
}

fn metric_from_coverage(coverage: &Coverage) -> Metric {
    Metric {
        low: coverage.value_low,
        high: coverage.value_high,
        confidence: coverage.confidence,
        known: coverage.known,
        total: coverage.total,
    }
}

/// Media ponderada de una lista de dimensiones con el mismo criterio que el
/// agregado de equipo: un desconocido vale 0 abajo y el factor arriba.
fn metric(profile: Option<&ChampionProfile>, role: Role, items: &[(Dimension, f32)]) -> Metric {
    let factor = clamp_unit(role_factor(profile, role));
    let total_weight: f32 = items.iter().map(|(_, weight)| *weight).sum();
    if total_weight <= 0.0 {
        return Metric {
            low: 0.0,
            high: 0.0,
            confidence: 0.0,
            known: 0,
            total: items.len(),
        };
    }

    let mut low = 0.0;
    let mut high = 0.0;
    let mut known_weight = 0.0;
    let mut known_confidence = 0.0;
    let mut known = 0;

    for (dimension, weight) in items {
        match profile.and_then(|profile| rating(profile, *dimension)) {
            Some((value, confidence)) => {
                let value = clamp_unit(value * factor);
                low += weight * value;
                high += weight * value;
                known_weight += weight;
                known_confidence += weight * clamp_unit(confidence);
                known += 1;
            }
            None => high += weight * factor,
        }
    }

    let confidence = if known_weight > 0.0 {
        (known_confidence / known_weight) * (known_weight / total_weight)
    } else {
        0.0
    };

    Metric {
        low: (low / total_weight).clamp(0.0, 1.0),
        high: (high / total_weight).clamp(0.0, 1.0),
        confidence: clamp_unit(confidence),
        known,
        total: items.len(),
    }
}

/// Diferencia `attacker - opponent` conservando el intervalo.
fn signed_advantage(attacker: Metric, opponent: Metric, attacking_side: Side) -> Advantage {
    let low = (attacker.low - opponent.high).clamp(-1.0, 1.0);
    let high = (attacker.high - opponent.low).clamp(-1.0, 1.0);
    let favored = if low > 0.02 {
        Some(attacking_side)
    } else if high < -0.02 {
        Some(attacking_side.opposite())
    } else {
        None
    };
    Advantage {
        low,
        high,
        confidence: clamp_unit((attacker.confidence + opponent.confidence) / 2.0),
        known: attacker.known + opponent.known,
        total: attacker.total + opponent.total,
        favored,
    }
}

/// Ventaja de una herramienta frente a la vulnerabilidad del rival. Un producto
/// evita declarar ventaja cuando el atacante no tiene la herramienta o cuando el
/// rival sí tiene la respuesta.
fn product_advantage(attack: Metric, vulnerability: Metric, attacking_side: Side) -> Advantage {
    let low = clamp_unit(attack.low * vulnerability.low);
    let high = clamp_unit(attack.high * vulnerability.high);
    Advantage {
        low,
        high,
        confidence: clamp_unit((attack.confidence + vulnerability.confidence) / 2.0),
        known: attack.known + vulnerability.known,
        total: attack.total + vulnerability.total,
        favored: if low > 0.05 {
            Some(attacking_side)
        } else {
            None
        },
    }
}

fn rating(profile: &ChampionProfile, dimension: Dimension) -> Option<(f32, f32)> {
    match dimension {
        Dimension::Archetype(id) => profile
            .archetypes
            .get(&id)
            .and_then(|rating| rating.intensity.map(|value| (value, rating.confidence))),
        Dimension::Trait(id) => profile
            .traits
            .get(&id)
            .and_then(|rating| rating.value.map(|value| (value, rating.confidence))),
    }
}

fn role_factor(profile: Option<&ChampionProfile>, role: Role) -> f32 {
    profile
        .and_then(|profile| {
            profile
                .roles
                .iter()
                .find(|candidate| candidate.role == role)
                .map(|candidate| candidate.proficiency)
        })
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ArchetypeRating, ProfileRole, Provenance, TraitRating};
    use crate::taxonomy::TAXONOMY_VERSION;
    use std::collections::BTreeMap;

    fn curated() -> Provenance {
        Provenance::Curated {
            editor: "test".into(),
            review: "test".into(),
        }
    }

    fn profile(
        key: u16,
        role: Role,
        archetypes: &[(ArchetypeId, Option<f32>)],
        traits: &[(TraitId, Option<f32>)],
    ) -> ChampionProfile {
        ChampionProfile {
            key: ChampionKey(key),
            slug: format!("synth{key}"),
            name: format!("SYNTH_{key}"),
            roles: vec![ProfileRole {
                role,
                proficiency: 1.0,
                confidence: 1.0,
                source: curated(),
            }],
            archetypes: archetypes
                .iter()
                .map(|(id, value)| {
                    (
                        *id,
                        ArchetypeRating {
                            intensity: *value,
                            confidence: if value.is_some() { 1.0 } else { 0.0 },
                            source: curated(),
                        },
                    )
                })
                .collect(),
            traits: traits
                .iter()
                .map(|(id, value)| {
                    (
                        *id,
                        TraitRating {
                            value: *value,
                            confidence: if value.is_some() { 1.0 } else { 0.0 },
                            source: curated(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn set(profiles: Vec<ChampionProfile>) -> ProfileSet {
        ProfileSet {
            id: "synth".into(),
            version: "1".into(),
            taxonomy_version: TAXONOMY_VERSION.into(),
            patch: None,
            champions: profiles
                .into_iter()
                .map(|profile| (profile.key, profile))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn engage_and_burst_produce_a_confirmed_synergy() {
        let profiles = set(vec![
            profile(1, Role::Support, &[(ArchetypeId::Engage, Some(1.0))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Burst, Some(1.0))], &[]),
        ]);
        let picks = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        ];
        let synergies = synergies(&picks, &profiles);
        let synergy = synergies
            .iter()
            .find(|synergy| synergy.id == SynergyId::EngageBurst)
            .expect("sinergia engage + burst");
        assert_eq!(synergy.score_low, 1.0);
        assert_eq!(synergy.score_high, 1.0);
        assert_eq!(synergy.known, 2);
        assert_eq!(synergy.confidence, 1.0);
        assert_eq!(synergy.champions, [ChampionKey(1), ChampionKey(2)]);
    }

    #[test]
    fn an_unknown_requirement_widens_the_interval() {
        let profiles = set(vec![
            profile(1, Role::Support, &[(ArchetypeId::Engage, Some(1.0))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Burst, None)], &[]),
        ]);
        let picks = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        ];
        let synergy = synergies(&picks, &profiles)
            .into_iter()
            .find(|synergy| synergy.id == SynergyId::EngageBurst)
            .expect("sinergia posible");
        assert_eq!(synergy.score_low, 0.5);
        assert_eq!(synergy.score_high, 1.0);
        assert_eq!(synergy.known, 1);
        assert!(synergy.confidence < 1.0);
    }

    #[test]
    fn no_profiles_means_no_synergies() {
        let picks = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        ];
        assert!(synergies(&picks, &set(Vec::new())).is_empty());
    }

    #[test]
    fn a_weak_requirement_does_not_emit_a_synergy() {
        let profiles = set(vec![
            profile(1, Role::Support, &[(ArchetypeId::Engage, Some(0.1))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Burst, Some(0.1))], &[]),
        ]);
        let picks = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        ];
        assert!(synergies(&picks, &profiles)
            .iter()
            .all(|synergy| synergy.id != SynergyId::EngageBurst));
    }

    #[test]
    fn lane_matchups_are_symmetric() {
        let profiles = set(vec![
            profile(
                1,
                Role::Top,
                &[],
                &[
                    (TraitId::EarlyPower, Some(0.9)),
                    (TraitId::Range, Some(0.2)),
                    (TraitId::LaneSafety, Some(0.8)),
                ],
            ),
            profile(
                2,
                Role::Top,
                &[],
                &[
                    (TraitId::EarlyPower, Some(0.4)),
                    (TraitId::Range, Some(0.8)),
                    (TraitId::LaneSafety, Some(0.5)),
                ],
            ),
        ]);
        let blue = vec![PickedChampion::new(ChampionKey(1), Role::Top, 1.0)];
        let red = vec![PickedChampion::new(ChampionKey(2), Role::Top, 1.0)];

        let forward = lane_matchups(&blue, &red, &profiles);
        let backward = lane_matchups(&red, &blue, &profiles);
        assert_eq!(forward.len(), 1);
        assert_eq!(backward.len(), 1);
        assert!((forward[0].pressure.low + backward[0].pressure.high).abs() < 1e-6);
        assert!((forward[0].pressure.high + backward[0].pressure.low).abs() < 1e-6);
    }

    #[test]
    fn lane_matchups_skip_roles_without_both_picks() {
        let profiles = set(vec![profile(1, Role::Top, &[], &[])]);
        let blue = vec![PickedChampion::new(ChampionKey(1), Role::Top, 1.0)];
        assert!(lane_matchups(&blue, &[], &profiles).is_empty());
    }

    #[test]
    fn a_champion_without_a_profile_produces_an_unknown_lane_read() {
        let profiles = set(Vec::new());
        let blue = vec![PickedChampion::new(ChampionKey(1), Role::Mid, 1.0)];
        let red = vec![PickedChampion::new(ChampionKey(2), Role::Mid, 1.0)];
        let matchups = lane_matchups(&blue, &red, &profiles);
        let pressure = &matchups[0].pressure;
        assert_eq!(pressure.favored, None);
        assert_eq!(pressure.confidence, 0.0);
        assert_eq!(pressure.known, 0);
        assert!(pressure.low.is_finite() && pressure.high.is_finite());
    }

    #[test]
    fn every_rule_id_has_a_stable_code() {
        for id in SynergyId::ALL {
            assert!(!id.as_str().is_empty());
        }
        for id in MatchupRuleId::ALL {
            assert!(!id.as_str().is_empty());
        }
    }

    #[test]
    fn each_rule_reports_at_most_one_pair() {
        let profiles = set(vec![
            profile(1, Role::Support, &[(ArchetypeId::Engage, Some(1.0))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Dps, Some(1.0))], &[]),
            profile(3, Role::Adc, &[(ArchetypeId::Dps, Some(0.9))], &[]),
        ]);
        let picks = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
            PickedChampion::new(ChampionKey(3), Role::Adc, 1.0),
        ];
        let synergies = synergies(&picks, &profiles);
        assert_eq!(
            synergies
                .iter()
                .filter(|synergy| synergy.id == SynergyId::EngageDps)
                .count(),
            1
        );
        // La pareja elegida es la más fuerte, no la primera encontrada.
        assert_eq!(
            synergies
                .iter()
                .find(|synergy| synergy.id == SynergyId::EngageDps)
                .map(|synergy| synergy.champions),
            Some([ChampionKey(1), ChampionKey(2)])
        );
    }

    #[test]
    fn a_tool_against_a_known_low_answer_is_an_edge() {
        let profiles = set(vec![
            profile(1, Role::Mid, &[(ArchetypeId::Poke, Some(0.9))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Sustain, Some(0.1))], &[]),
        ]);
        let blue = vec![PickedChampion::new(ChampionKey(1), Role::Mid, 1.0)];
        let red = vec![PickedChampion::new(ChampionKey(2), Role::Mid, 1.0)];
        let analysis = crate::analysis::analyze_draft(
            &blue,
            &red,
            &profiles,
            &crate::analysis::AnalysisParams::default(),
        )
        .expect("análisis");
        let edge = analysis
            .matchups
            .iter()
            .find(|matchup| matchup.id == MatchupRuleId::PokeSustain && matchup.side == Side::Blue)
            .expect("ventaja de poke");
        assert!(edge.advantage.low > 0.05);
        assert_eq!(edge.advantage.favored, Some(Side::Blue));
    }

    #[test]
    fn a_tool_against_an_unknown_answer_is_not_an_edge() {
        let profiles = set(vec![
            profile(1, Role::Mid, &[(ArchetypeId::Poke, Some(0.9))], &[]),
            profile(2, Role::Mid, &[(ArchetypeId::Sustain, None)], &[]),
        ]);
        let blue = vec![PickedChampion::new(ChampionKey(1), Role::Mid, 1.0)];
        let red = vec![PickedChampion::new(ChampionKey(2), Role::Mid, 1.0)];
        let analysis = crate::analysis::analyze_draft(
            &blue,
            &red,
            &profiles,
            &crate::analysis::AnalysisParams::default(),
        )
        .expect("análisis");
        assert!(analysis
            .matchups
            .iter()
            .all(|matchup| matchup.id != MatchupRuleId::PokeSustain));
    }

    #[test]
    fn interaction_outputs_are_finite_and_bounded() {
        let profiles = set(vec![
            profile(
                1,
                Role::Support,
                &[
                    (ArchetypeId::Engage, Some(1.0)),
                    (ArchetypeId::Frontline, Some(0.8)),
                ],
                &[(TraitId::HardCc, Some(0.9))],
            ),
            profile(
                2,
                Role::Adc,
                &[(ArchetypeId::Dps, Some(0.9))],
                &[
                    (TraitId::Range, Some(0.9)),
                    (TraitId::EarlyPower, Some(0.4)),
                ],
            ),
        ]);
        let blue = vec![
            PickedChampion::new(ChampionKey(1), Role::Support, 1.0),
            PickedChampion::new(ChampionKey(2), Role::Adc, 1.0),
        ];
        let analysis = crate::analysis::analyze_draft(
            &blue,
            &[],
            &profiles,
            &crate::analysis::AnalysisParams::default(),
        )
        .expect("análisis");

        for synergy in &analysis.blue.synergies {
            for value in [synergy.score_low, synergy.score_high, synergy.confidence] {
                assert!(value.is_finite());
                assert!((0.0..=1.0).contains(&value));
            }
            assert!(synergy.score_low <= synergy.score_high);
        }
        for matchup in &analysis.matchups {
            for value in [
                matchup.advantage.low,
                matchup.advantage.high,
                matchup.advantage.confidence,
            ] {
                assert!(value.is_finite());
                assert!((0.0..=1.0).contains(&value));
            }
            assert!(matchup.advantage.low <= matchup.advantage.high);
        }
    }
}
