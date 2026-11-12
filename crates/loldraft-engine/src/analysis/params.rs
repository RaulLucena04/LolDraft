//! Parámetros versionados del análisis de composiciones.
//!
//! Los valores por defecto son hipótesis iniciales documentadas en
//! `docs/03-motor-composiciones.md`. Cambiarlos exige subir `version`.

use crate::taxonomy::{ArchetypeId, TraitId};
use loldraft_domain::Role;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Dimensión estratégica: arquetipo o trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Dimension {
    Archetype(ArchetypeId),
    Trait(TraitId),
}

impl Dimension {
    pub fn as_str(self) -> &'static str {
        match self {
            Dimension::Archetype(id) => id.as_str(),
            Dimension::Trait(id) => id.as_str(),
        }
    }

    pub fn label_es(self) -> &'static str {
        // Las etiquetas completas viven en la taxonomía; el motor solo necesita
        // un identificador estable para explicaciones.
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisParams {
    pub version: u32,
    /// Pesos de la suma saturante `[1.0, 0.30, 0.10]`.
    pub saturation_weights: [f32; 3],
    /// Peso de oro por rol, en el orden de `Role::ALL`.
    pub role_gold_weights: [f32; 5],
    pub archetype_floors: BTreeMap<ArchetypeId, f32>,
    pub trait_floors: BTreeMap<TraitId, f32>,
    /// Multiplicadores de prioridad; ausente = 1.0.
    pub archetype_priorities: BTreeMap<ArchetypeId, f32>,
    pub trait_priorities: BTreeMap<TraitId, f32>,
}

impl Default for AnalysisParams {
    fn default() -> Self {
        Self {
            version: 1,
            saturation_weights: [1.0, 0.30, 0.10],
            role_gold_weights: [0.95, 0.85, 1.00, 1.10, 0.45],
            archetype_floors: BTreeMap::from([
                (ArchetypeId::Frontline, 0.50),
                (ArchetypeId::Engage, 0.35),
                (ArchetypeId::Disengage, 0.30),
                (ArchetypeId::Peel, 0.30),
                (ArchetypeId::Dps, 0.50),
                (ArchetypeId::Burst, 0.30),
                (ArchetypeId::AntiTank, 0.25),
                (ArchetypeId::Sustain, 0.20),
            ]),
            trait_floors: BTreeMap::from([(TraitId::Waveclear, 0.35), (TraitId::HardCc, 0.30)]),
            archetype_priorities: BTreeMap::new(),
            trait_priorities: BTreeMap::new(),
        }
    }
}

impl AnalysisParams {
    pub fn validate(&self) -> Result<(), AnalysisParamsError> {
        if self.version == 0 {
            return Err(AnalysisParamsError::InvalidVersion);
        }
        if self
            .saturation_weights
            .iter()
            .any(|weight| !valid_weight(*weight))
            || self.saturation_weights[0] <= 0.0
        {
            return Err(AnalysisParamsError::InvalidSaturation(
                self.saturation_weights,
            ));
        }
        for (role, weight) in Role::ALL.into_iter().zip(self.role_gold_weights) {
            if !valid_weight(weight) || weight <= 0.0 {
                return Err(AnalysisParamsError::InvalidGoldWeight {
                    role: role.as_str(),
                    value: weight,
                });
            }
        }
        for (id, floor) in &self.archetype_floors {
            require_unit(*floor, format!("archetype:{}", id.as_str()))?;
        }
        for (id, floor) in &self.trait_floors {
            require_unit(*floor, format!("trait:{}", id.as_str()))?;
        }
        for (id, priority) in &self.archetype_priorities {
            if !valid_weight(*priority) {
                return Err(AnalysisParamsError::InvalidPriority {
                    dimension: format!("archetype:{}", id.as_str()),
                    value: *priority,
                });
            }
        }
        for (id, priority) in &self.trait_priorities {
            if !valid_weight(*priority) {
                return Err(AnalysisParamsError::InvalidPriority {
                    dimension: format!("trait:{}", id.as_str()),
                    value: *priority,
                });
            }
        }
        Ok(())
    }

    pub fn floor(&self, dimension: Dimension) -> Option<f32> {
        match dimension {
            Dimension::Archetype(id) => self.archetype_floors.get(&id).copied(),
            Dimension::Trait(id) => self.trait_floors.get(&id).copied(),
        }
    }

    pub fn priority(&self, dimension: Dimension) -> f32 {
        match dimension {
            Dimension::Archetype(id) => self.archetype_priorities.get(&id).copied(),
            Dimension::Trait(id) => self.trait_priorities.get(&id).copied(),
        }
        .unwrap_or(1.0)
    }

    pub fn gold_weight(&self, role: Role) -> f32 {
        self.role_gold_weights[role.index()]
    }
}

fn valid_weight(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn require_unit(value: f32, dimension: String) -> Result<(), AnalysisParamsError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(AnalysisParamsError::InvalidFloor { dimension, value })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AnalysisParamsError {
    #[error("la versión de parámetros debe ser mayor que cero")]
    InvalidVersion,
    #[error("pesos de saturación inválidos: {0:?}")]
    InvalidSaturation([f32; 3]),
    #[error("peso de oro inválido para {role}: {value}")]
    InvalidGoldWeight { role: &'static str, value: f32 },
    #[error("suelo fuera de 0..1 en `{dimension}`: {value}")]
    InvalidFloor { dimension: String, value: f32 },
    #[error("prioridad inválida en `{dimension}`: {value}")]
    InvalidPriority { dimension: String, value: f32 },
}
