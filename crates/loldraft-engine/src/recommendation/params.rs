//! Parámetros versionados del motor de recomendaciones.
//!
//! Los pesos son una hipótesis razonada documentada en
//! `docs/04-motor-recomendaciones.md`; no son una probabilidad de victoria.
//! Cambiarlos exige subir `version` y volver a ejecutar los tests.

use loldraft_domain::{ActionKind, DraftFormat};
use serde::{Deserialize, Serialize};

/// Fase del draft para leer el contexto. La fase de pesos de pick se decide
/// aparte porque la tabla distingue pick temprano, medio y tardío.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftPhase {
    EarlyBan,
    EarlyPick,
    LateBan,
    LatePick,
}

impl DraftPhase {
    pub fn of(format: &DraftFormat, action_index: usize) -> Option<Self> {
        let step = format.step(action_index)?;
        Some(match (step.kind, action_index) {
            (ActionKind::Ban, 0..=5) => Self::EarlyBan,
            (ActionKind::Pick, 6..=11) => Self::EarlyPick,
            (ActionKind::Ban, 12..=15) => Self::LateBan,
            (ActionKind::Pick, _) => Self::LatePick,
            (ActionKind::Ban, _) => return None,
        })
    }

    pub fn is_ban(self) -> bool {
        matches!(self, Self::EarlyBan | Self::LateBan)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EarlyBan => "early_ban",
            Self::EarlyPick => "early_pick",
            Self::LateBan => "late_ban",
            Self::LatePick => "late_pick",
        }
    }
}

/// Fase de la tabla de pesos de pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickWeightPhase {
    Early,
    Mid,
    Late,
}

impl PickWeightPhase {
    /// Primeros dos picks globales (acciones 7 y 8): blind pick. Del tercero al
    /// sexto: pick medio. Los cuatro últimos: respuesta.
    pub fn of(action_index: usize) -> Self {
        match action_index {
            6..=7 => Self::Early,
            8..=11 => Self::Mid,
            _ => Self::Late,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureId {
    Need,
    Answer,
    Synergy,
    Counter,
    Power,
    Flex,
    Balance,
    Safety,
}

impl FeatureId {
    pub const ALL: [FeatureId; 8] = [
        FeatureId::Need,
        FeatureId::Answer,
        FeatureId::Synergy,
        FeatureId::Counter,
        FeatureId::Power,
        FeatureId::Flex,
        FeatureId::Balance,
        FeatureId::Safety,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Need => "need",
            Self::Answer => "answer",
            Self::Synergy => "synergy",
            Self::Counter => "counter",
            Self::Power => "power",
            Self::Flex => "flex",
            Self::Balance => "balance",
            Self::Safety => "safety",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureWeights {
    pub need: f32,
    pub answer: f32,
    pub synergy: f32,
    pub counter: f32,
    pub power: f32,
    pub flex: f32,
    pub balance: f32,
    pub safety: f32,
}

impl FeatureWeights {
    pub fn get(&self, id: FeatureId) -> f32 {
        match id {
            FeatureId::Need => self.need,
            FeatureId::Answer => self.answer,
            FeatureId::Synergy => self.synergy,
            FeatureId::Counter => self.counter,
            FeatureId::Power => self.power,
            FeatureId::Flex => self.flex,
            FeatureId::Balance => self.balance,
            FeatureId::Safety => self.safety,
        }
    }

    pub fn set(&mut self, id: FeatureId, value: f32) {
        match id {
            FeatureId::Need => self.need = value,
            FeatureId::Answer => self.answer = value,
            FeatureId::Synergy => self.synergy = value,
            FeatureId::Counter => self.counter = value,
            FeatureId::Power => self.power = value,
            FeatureId::Flex => self.flex = value,
            FeatureId::Balance => self.balance = value,
            FeatureId::Safety => self.safety = value,
        }
    }

    pub fn sum(&self) -> f32 {
        FeatureId::ALL.iter().map(|id| self.get(*id)).sum()
    }

    fn clamp_negative(&mut self) {
        for id in FeatureId::ALL {
            if !self.get(id).is_finite() || self.get(id) < 0.0 {
                self.set(id, 0.0);
            }
        }
    }

    /// Normaliza para que la suma sea 1. Si todo es cero devuelve los pesos
    /// originales: el evaluador tratará el candidato como no puntuable.
    pub fn normalized(&self) -> Self {
        let mut weights = *self;
        weights.clamp_negative();
        let sum = weights.sum();
        if sum <= 0.0 {
            return weights;
        }
        for id in FeatureId::ALL {
            weights.set(id, weights.get(id) / sum);
        }
        weights
    }
}

fn early_pick_weights() -> FeatureWeights {
    FeatureWeights {
        need: 0.12,
        answer: 0.05,
        synergy: 0.10,
        counter: 0.03,
        power: 0.18,
        flex: 0.22,
        balance: 0.05,
        safety: 0.25,
    }
}

fn mid_pick_weights() -> FeatureWeights {
    FeatureWeights {
        need: 0.26,
        answer: 0.07,
        synergy: 0.24,
        counter: 0.10,
        power: 0.12,
        flex: 0.10,
        balance: 0.06,
        safety: 0.05,
    }
}

fn late_pick_weights() -> FeatureWeights {
    FeatureWeights {
        need: 0.24,
        answer: 0.15,
        synergy: 0.10,
        counter: 0.28,
        power: 0.08,
        flex: 0.00,
        balance: 0.10,
        safety: 0.05,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanWeights {
    pub opp_need: f32,
    pub opp_synergy: f32,
    pub opp_power: f32,
    pub opp_flex: f32,
    pub own_threat: f32,
}

impl BanWeights {
    pub fn sum(&self) -> f32 {
        self.opp_need + self.opp_synergy + self.opp_power + self.opp_flex + self.own_threat
    }

    pub fn normalized(&self) -> Self {
        let sum = self.sum();
        if sum <= 0.0 {
            return *self;
        }
        Self {
            opp_need: self.opp_need / sum,
            opp_synergy: self.opp_synergy / sum,
            opp_power: self.opp_power / sum,
            opp_flex: self.opp_flex / sum,
            own_threat: self.own_threat / sum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationParams {
    pub version: u32,
    pub weights_early_pick: FeatureWeights,
    pub weights_mid_pick: FeatureWeights,
    pub weights_late_pick: FeatureWeights,
    pub ban_weights: BanWeights,
    /// Coeficiente del riesgo total (doc 04: 0.15).
    pub risk_coefficient: f32,
    /// Penalización de ordenación por baja confianza (doc 04: 0.10).
    pub confidence_penalty: f32,
    pub diversity_lambda: f32,
    pub limit: usize,
    pub max_per_role: usize,
    pub min_role_proficiency: f32,
    pub role_ambiguity_margin: f32,
    /// Cuánto resta al ban el valor que ese campeón tendría para nosotros.
    pub ban_own_value_penalty: f32,
}

impl Default for RecommendationParams {
    fn default() -> Self {
        Self {
            version: 1,
            weights_early_pick: early_pick_weights(),
            weights_mid_pick: mid_pick_weights(),
            weights_late_pick: late_pick_weights(),
            ban_weights: BanWeights {
                opp_need: 0.40,
                opp_synergy: 0.20,
                opp_power: 0.15,
                opp_flex: 0.10,
                own_threat: 0.15,
            },
            risk_coefficient: 0.15,
            confidence_penalty: 0.10,
            diversity_lambda: 0.12,
            limit: 8,
            max_per_role: 2,
            min_role_proficiency: 0.30,
            role_ambiguity_margin: 0.05,
            ban_own_value_penalty: 0.25,
        }
    }
}

impl RecommendationParams {
    pub fn validate(&self) -> Result<(), RecommendationParamsError> {
        if self.version == 0 {
            return Err(RecommendationParamsError::InvalidVersion);
        }
        for (name, weights) in [
            ("early", self.weights_early_pick),
            ("mid", self.weights_mid_pick),
            ("late", self.weights_late_pick),
        ] {
            for id in FeatureId::ALL {
                let value = weights.get(id);
                if !unit(value) {
                    return Err(RecommendationParamsError::InvalidWeight {
                        phase: name,
                        feature: id.as_str(),
                        value,
                    });
                }
            }
            if weights.sum() <= 0.0 {
                return Err(RecommendationParamsError::EmptyWeights(name));
            }
        }
        if self.ban_weights.sum() <= 0.0 {
            return Err(RecommendationParamsError::EmptyWeights("ban"));
        }
        for (name, value) in [
            ("ban.opp_need", self.ban_weights.opp_need),
            ("ban.opp_synergy", self.ban_weights.opp_synergy),
            ("ban.opp_power", self.ban_weights.opp_power),
            ("ban.opp_flex", self.ban_weights.opp_flex),
            ("ban.own_threat", self.ban_weights.own_threat),
            ("risk_coefficient", self.risk_coefficient),
            ("confidence_penalty", self.confidence_penalty),
            ("diversity_lambda", self.diversity_lambda),
            ("min_role_proficiency", self.min_role_proficiency),
            ("role_ambiguity_margin", self.role_ambiguity_margin),
            ("ban_own_value_penalty", self.ban_own_value_penalty),
        ] {
            if !unit(value) {
                return Err(RecommendationParamsError::InvalidUnit { name, value });
            }
        }
        if !(1..=20).contains(&self.limit) {
            return Err(RecommendationParamsError::InvalidLimit(self.limit));
        }
        if !(1..=5).contains(&self.max_per_role) {
            return Err(RecommendationParamsError::InvalidMaxPerRole(
                self.max_per_role,
            ));
        }
        Ok(())
    }

    pub fn pick_weights(&self, phase: PickWeightPhase) -> FeatureWeights {
        match phase {
            PickWeightPhase::Early => self.weights_early_pick,
            PickWeightPhase::Mid => self.weights_mid_pick,
            PickWeightPhase::Late => self.weights_late_pick,
        }
    }
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[derive(Debug, thiserror::Error)]
pub enum RecommendationParamsError {
    #[error("la versión de parámetros debe ser mayor que cero")]
    InvalidVersion,
    #[error("peso inválido en {phase} para `{feature}`: {value}")]
    InvalidWeight {
        phase: &'static str,
        feature: &'static str,
        value: f32,
    },
    #[error("los pesos de {0} suman cero")]
    EmptyWeights(&'static str),
    #[error("`{name}` debe estar en 0..=1: {value}")]
    InvalidUnit { name: &'static str, value: f32 },
    #[error("el límite debe estar en 1..=20: {0}")]
    InvalidLimit(usize),
    #[error("`max_per_role` debe estar en 1..=5: {0}")]
    InvalidMaxPerRole(usize),
}
