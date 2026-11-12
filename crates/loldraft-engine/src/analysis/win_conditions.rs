//! Condiciones de victoria detectadas a partir de la cobertura agregada.

use super::params::Dimension;
use super::team::TeamAggregate;
use crate::taxonomy::{ArchetypeId, TraitId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinConditionId {
    FrontToBack,
    Dive,
    PokeSiege,
    Pick,
    Splitpush,
    Scaling,
    EarlySnowball,
    Wombo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WinCondition {
    pub id: WinConditionId,
    pub score: f32,
    /// Todos los requisitos se cumplen incluso en la cota inferior.
    pub confirmed: bool,
    pub evidence: Vec<Dimension>,
    /// Requisitos que ni siquiera la cota superior alcanza.
    pub missing: Vec<Dimension>,
}

const RULES: &[(WinConditionId, &[(Dimension, f32)])] = &[
    (
        WinConditionId::FrontToBack,
        &[
            (Dimension::Archetype(ArchetypeId::Frontline), 0.55),
            (Dimension::Archetype(ArchetypeId::Dps), 0.60),
            (Dimension::Archetype(ArchetypeId::Peel), 0.40),
        ],
    ),
    (
        WinConditionId::Dive,
        &[
            (Dimension::Archetype(ArchetypeId::Engage), 0.60),
            (Dimension::Archetype(ArchetypeId::Burst), 0.55),
            (Dimension::Archetype(ArchetypeId::Assassin), 0.50),
            (Dimension::Trait(TraitId::Mobility), 0.50),
        ],
    ),
    (
        WinConditionId::PokeSiege,
        &[
            (Dimension::Archetype(ArchetypeId::Poke), 0.60),
            (Dimension::Archetype(ArchetypeId::Siege), 0.50),
            (Dimension::Archetype(ArchetypeId::Disengage), 0.45),
        ],
    ),
    (
        WinConditionId::Pick,
        &[
            (Dimension::Archetype(ArchetypeId::Catch), 0.55),
            (Dimension::Archetype(ArchetypeId::Burst), 0.50),
            (Dimension::Archetype(ArchetypeId::Utility), 0.40),
        ],
    ),
    (
        WinConditionId::Splitpush,
        &[
            (Dimension::Archetype(ArchetypeId::Splitpush), 0.55),
            (Dimension::Trait(TraitId::Waveclear), 0.50),
            (Dimension::Trait(TraitId::Mobility), 0.45),
        ],
    ),
    (
        WinConditionId::Scaling,
        &[
            (Dimension::Archetype(ArchetypeId::Scaling), 0.60),
            (Dimension::Trait(TraitId::LatePower), 0.60),
            (Dimension::Trait(TraitId::Waveclear), 0.45),
        ],
    ),
    (
        WinConditionId::EarlySnowball,
        &[
            (Dimension::Archetype(ArchetypeId::EarlyGame), 0.60),
            (Dimension::Trait(TraitId::EarlyPower), 0.60),
            (Dimension::Trait(TraitId::HardCc), 0.40),
        ],
    ),
    (
        WinConditionId::Wombo,
        &[
            (Dimension::Archetype(ArchetypeId::WomboCombo), 0.55),
            (Dimension::Trait(TraitId::Aoe), 0.55),
            (Dimension::Archetype(ArchetypeId::Engage), 0.50),
        ],
    ),
];

pub(crate) fn detect(team: &TeamAggregate) -> Vec<WinCondition> {
    if team.picks == 0 {
        return Vec::new();
    }

    let mut conditions = Vec::new();
    for (id, requirements) in RULES {
        let mut confirmed = true;
        let mut viable = true;
        let mut score = 0.0;
        let mut evidence = Vec::with_capacity(requirements.len());
        let mut missing = Vec::new();

        for (dimension, threshold) in *requirements {
            let Some(coverage) = team.coverage(*dimension) else {
                viable = false;
                missing.push(*dimension);
                continue;
            };
            if coverage.value_low >= *threshold {
                evidence.push(*dimension);
            } else {
                confirmed = false;
            }
            if coverage.value_high < *threshold {
                viable = false;
                missing.push(*dimension);
            }
            score += (coverage.midpoint() / threshold).clamp(0.0, 1.0);
        }

        if !viable {
            continue;
        }
        score /= requirements.len() as f32;
        conditions.push(WinCondition {
            id: *id,
            score,
            confirmed,
            evidence,
            missing,
        });
    }

    conditions.sort_by(|left, right| {
        right
            .confirmed
            .cmp(&left.confirmed)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.id.cmp(&right.id))
    });
    conditions
}
