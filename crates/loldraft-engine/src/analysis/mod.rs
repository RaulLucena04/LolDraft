//! Análisis estratégico de composiciones.
//!
//! El motor agrega perfiles en intervalos: un dato desconocido vale 0 en la
//! cota inferior y 1 en la superior. Nunca se convierte en un cero silencioso.
//! Las salidas describen encaje estratégico, no probabilidad de victoria.

mod interactions;
mod needs;
mod params;
mod team;
mod win_conditions;

#[cfg(test)]
mod tests;

pub use interactions::{
    Advantage, LaneMatchup, MatchupRuleId, Synergy, SynergyId, TeamMatchup, INTERACTIONS_VERSION,
};
pub use needs::{Need, NeedSource, NeedStatus, ThreatRuleId};
pub use params::{AnalysisParams, AnalysisParamsError, Dimension};
pub use team::{analyze_draft, analyze_team, AnalysisError, PickedChampion};
pub use win_conditions::{WinCondition, WinConditionId};

use crate::taxonomy::{ArchetypeId, TraitId};
use loldraft_domain::Side;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cobertura de una dimensión con su intervalo de desconocimiento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub value_low: f32,
    pub value_high: f32,
    /// Solo se rellena cuando el intervalo es estrecho (`≤ 0.20`).
    pub value_estimate: Option<f32>,
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
}

impl Coverage {
    pub(crate) fn new(
        value_low: f32,
        value_high: f32,
        known: usize,
        total: usize,
        confidence: f32,
    ) -> Self {
        let mut low = clamp_unit(value_low);
        let mut high = clamp_unit(value_high);
        if low > high {
            std::mem::swap(&mut low, &mut high);
        }
        let width = high - low;
        let value_estimate = if total > 0 && known > 0 && width <= 0.20 {
            Some((low + high) / 2.0)
        } else {
            None
        };
        Self {
            value_low: low,
            value_high: high,
            value_estimate,
            confidence: clamp_unit(confidence),
            known,
            total,
        }
    }

    /// Valor central para ordenar; si el intervalo es ancho usa el punto medio
    /// solo como criterio interno, nunca como estimación mostrada.
    pub fn midpoint(&self) -> f32 {
        (self.value_low + self.value_high) / 2.0
    }

    pub fn is_confirmed(&self, threshold: f32) -> bool {
        self.value_low >= threshold
    }

    pub fn is_confirmed_lacking(&self, threshold: f32) -> bool {
        self.value_high < threshold
    }

    pub fn crosses(&self, threshold: f32) -> bool {
        self.value_low < threshold && self.value_high >= threshold
    }
}

/// Reparto de amenaza de daño, normalizado entre los campeones con datos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageProfile {
    pub physical: f32,
    pub magic: f32,
    pub true_damage: f32,
    /// Proporción del peso de oro total que tiene shares completos.
    pub known_weight_ratio: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerCurve {
    pub early: Coverage,
    pub mid: Coverage,
    pub late: Coverage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeProfile {
    pub ranged_share: Coverage,
    pub melee_count: usize,
    pub known: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CcProfile {
    pub hard: Coverage,
    pub soft: Coverage,
}

/// Señal explicable de fortaleza o carencia.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub dimension: Dimension,
    pub value_low: f32,
    pub value_high: f32,
    pub confidence: f32,
}

impl Signal {
    pub(crate) fn from_coverage(dimension: Dimension, coverage: &Coverage) -> Self {
        Self {
            dimension,
            value_low: coverage.value_low,
            value_high: coverage.value_high,
            confidence: coverage.confidence,
        }
    }
}

/// Análisis de un equipo de 0 a 5 campeones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamAnalysis {
    pub side: Side,
    pub picks: usize,
    /// `true` si faltan picks o si el conocimiento es inferior al 60 %.
    pub partial: bool,
    pub archetypes: BTreeMap<ArchetypeId, Coverage>,
    /// Solo traits escalares; el daño se expone en `damage`.
    pub traits: BTreeMap<TraitId, Coverage>,
    pub damage: Option<DamageProfile>,
    pub curve: PowerCurve,
    pub range: RangeProfile,
    pub cc: CcProfile,
    pub mobility: Coverage,
    pub waveclear: Coverage,
    pub objective_dps: Coverage,
    pub needs: Vec<Need>,
    pub win_conditions: Vec<WinCondition>,
    /// Sinergias entre dos picks del propio equipo, derivadas de reglas.
    pub synergies: Vec<Synergy>,
    pub strengths: Vec<Signal>,
    pub weaknesses: Vec<Signal>,
    pub knowledge: f32,
    pub confidence: f32,
}

impl TeamAnalysis {
    pub fn coverage(&self, dimension: Dimension) -> Option<&Coverage> {
        match dimension {
            Dimension::Archetype(id) => self.archetypes.get(&id),
            Dimension::Trait(id) => self.traits.get(&id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftAnalysis {
    pub params_version: u32,
    pub interactions_version: u32,
    pub engine_version: String,
    pub blue: TeamAnalysis,
    pub red: TeamAnalysis,
    /// Lectura derivada de cada línea con picks en ambos lados.
    pub lane_matchups: Vec<LaneMatchup>,
    /// Ventajas estratégicas de ambos lados frente a la herramienta rival.
    pub matchups: Vec<TeamMatchup>,
}

pub(crate) fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
