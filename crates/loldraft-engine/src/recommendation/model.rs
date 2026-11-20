//! Tipos de salida del motor de recomendaciones.
//!
//! Cada puntuación es un índice de ajuste estratégico (0–100), no una
//! probabilidad de victoria. Las contribuciones y los riesgos suman la
//! puntuación interna; la UI puede auditar el resultado sin confiar en texto
//! generado.

use super::params::{DraftPhase, FeatureId};
use crate::analysis::{Dimension, SynergyId};
use crate::stats::{StatsProvenance, StatsQueue};
use crate::taxonomy::TraitId;
use loldraft_domain::{ActionKind, ChampionKey, Role, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub feature: FeatureId,
    /// Valor de la feature, ya normalizado a 0..1.
    pub value: f32,
    /// Peso renormalizado tras excluir las features ausentes.
    pub weight: f32,
    /// `weight * value`.
    pub contribution: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskId {
    RedundantDamage,
    ResourceConflict,
    NoFrontline,
    ScalingConflict,
    UnknownProfile,
    RoleAmbiguous,
}

impl RiskId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedundantDamage => "redundant_damage",
            Self::ResourceConflict => "resource_conflict",
            Self::NoFrontline => "no_frontline",
            Self::ScalingConflict => "scaling_conflict",
            Self::UnknownProfile => "unknown_profile",
            Self::RoleAmbiguous => "role_ambiguous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskContribution {
    pub kind: RiskId,
    pub amount: f32,
    /// Negativo. Si el riesgo total se capa a 1, las contribuciones se escalan
    /// para que su suma siga coincidiendo con la penalización aplicada.
    pub contribution: f32,
}

/// Motivo tipado. La UI traduce el código y sus parámetros; el motor no genera
/// frases libres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Reason {
    FillsGap {
        dimension: Dimension,
        before: f32,
        after: f32,
        need: f32,
    },
    AnswersThreat {
        dimension: Dimension,
        before: f32,
        after: f32,
    },
    Synergy {
        id: SynergyId,
        champion: ChampionKey,
        score_low: f32,
        score_high: f32,
    },
    CountersLane {
        champion: ChampionKey,
        role: Role,
        edge_low: f32,
        edge_high: f32,
    },
    FlexValue {
        roles: u8,
    },
    DamageBalance {
        kind: TraitId,
        before: f32,
        after: f32,
    },
    RedundantDamage {
        kind: TraitId,
        share: f32,
    },
    ResourceConflict {
        allies: u8,
    },
    NoFrontline,
    ScalingConflict,
    UnknownProfile {
        knowledge: f32,
    },
    RoleAmbiguous {
        roles: u8,
    },
    /// Percentil de winrate ajustado dentro del rol, sacado de un dataset
    /// observado activo. `games` es la muestra de esa fila.
    ObservedPower {
        queue: StatsQueue,
        percentile: f32,
        games: u32,
        shrunk_winrate: f32,
        confidence: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub champion: ChampionKey,
    pub role: Role,
    /// Puntuación mostrada, 0–100. Índice de ajuste, no winrate.
    pub score: u8,
    /// Puntuación interna 0..1 antes de redondear.
    pub score_fraction: f32,
    /// `score_fraction - confidence_penalty * (1 - confidence)`.
    pub ranking_score: f32,
    pub confidence: f32,
    pub limited_data: bool,
    pub contributions: Vec<Contribution>,
    pub risks: Vec<RiskContribution>,
    pub reasons: Vec<Reason>,
    pub missing: Vec<FeatureId>,
    pub viable_roles: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BanFeatureId {
    OppNeed,
    OppSynergy,
    OppPower,
    OppFlex,
    OwnThreat,
    OwnPickValue,
}

impl BanFeatureId {
    pub const ALL: [BanFeatureId; 6] = [
        BanFeatureId::OppNeed,
        BanFeatureId::OppSynergy,
        BanFeatureId::OppPower,
        BanFeatureId::OppFlex,
        BanFeatureId::OwnThreat,
        BanFeatureId::OwnPickValue,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OppNeed => "opp_need",
            Self::OppSynergy => "opp_synergy",
            Self::OppPower => "opp_power",
            Self::OppFlex => "opp_flex",
            Self::OwnThreat => "own_threat",
            Self::OwnPickValue => "own_pick_value",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanContribution {
    pub feature: BanFeatureId,
    pub value: f32,
    /// Puede ser negativo en `OwnPickValue`, que resta valor al ban.
    pub weight: f32,
    pub contribution: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanRecommendation {
    pub champion: ChampionKey,
    pub score: u8,
    pub score_fraction: f32,
    pub ranking_score: f32,
    pub confidence: f32,
    pub limited_data: bool,
    pub contributions: Vec<BanContribution>,
    pub risks: Vec<RiskContribution>,
    pub reasons: Vec<Reason>,
    pub missing: Vec<BanFeatureId>,
    /// Valor del campeón para nuestro equipo, restado del ban.
    pub own_pick_value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoDataReason {
    NoProfiles,
    NoAvailableChampions,
    NoViableCandidate,
}

impl NoDataReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoProfiles => "no_profiles",
            Self::NoAvailableChampions => "no_available_champions",
            Self::NoViableCandidate => "no_viable_candidate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecommendationSet {
    pub phase: DraftPhase,
    pub side: Side,
    pub action_kind: ActionKind,
    pub engine_version: String,
    pub profile_set_id: String,
    pub params_version: u32,
    pub candidates_considered: usize,
    pub picks: Vec<Recommendation>,
    pub bans: Vec<BanRecommendation>,
    pub no_data: Option<NoDataReason>,
    /// `true` si la mejor recomendación se muestra con datos limitados.
    pub limited_data: bool,
    /// Unión de features ausentes en las recomendaciones de pick.
    pub missing_features: Vec<FeatureId>,
    /// Unión de features ausentes en las recomendaciones de baneo.
    pub missing_ban_features: Vec<BanFeatureId>,
    /// Dataset observado que alimentó `power`, si había uno activo. Se expone
    /// la procedencia para que la UI no muestre un percentil anónimo.
    pub stats: Option<StatsProvenance>,
}
