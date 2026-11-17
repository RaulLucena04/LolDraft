//! Motor de recomendaciones de picks y baneos.
//!
//! Devuelve un índice de ajuste estratégico (0–100), nunca una probabilidad de
//! victoria. Cada recomendación incluye contribuciones exactas, riesgos,
//! motivos tipados y las features que faltan. El motor es determinista y no
//! usa aleatoriedad ni texto generado.

mod engine;
mod model;
mod params;
mod score;

#[cfg(test)]
mod tests;

pub use engine::{recommend, RecommendationError, RecommendationRequest};
pub use model::{
    BanContribution, BanFeatureId, BanRecommendation, Contribution, NoDataReason, Reason,
    Recommendation, RecommendationSet, RiskContribution, RiskId,
};
pub use params::{
    BanWeights, DraftPhase, FeatureId, FeatureWeights, PickWeightPhase, RecommendationParams,
    RecommendationParamsError,
};
