//! Motor de recomendaciones.
//!
//! Fase 2: taxonomía estratégica y perfiles de campeón con procedencia,
//! confianza y validación. La agregación de composiciones y la puntuación
//! multicriterio llegan en incrementos posteriores.

pub mod analysis;
pub mod profile;
pub mod proposals;
pub mod recommendation;
pub mod stats;
pub mod taxonomy;

pub use analysis::{
    analyze_draft, analyze_team, candidate_synergies, Advantage, AnalysisError, AnalysisParams,
    AnalysisParamsError, CcProfile, Coverage, DamageProfile, Dimension, DraftAnalysis, LaneMatchup,
    MatchupRuleId, Need, NeedSource, NeedStatus, PickedChampion, PowerCurve, RangeProfile, Signal,
    Synergy, SynergyId, TeamAnalysis, TeamMatchup, ThreatRuleId, WinCondition, WinConditionId,
    INTERACTIONS_VERSION,
};
pub use profile::{
    ArchetypeRating, ChampionProfile, DamageShares, ProfileError, ProfileRole, ProfileSet,
    Provenance, TraitRating, DAMAGE_SHARE_TOLERANCE,
};
pub use proposals::{
    ChampionProposal, ProfileProposalSet, ProposalError, ProposalRole, PILOT_PROFILE_PROPOSAL_JSON,
};
pub use recommendation::{
    recommend, BanContribution, BanFeatureId, BanRecommendation, BanWeights, Contribution,
    DraftPhase, FeatureId, FeatureWeights, NoDataReason, PickWeightPhase, Reason, Recommendation,
    RecommendationError, RecommendationParams, RecommendationParamsError, RecommendationRequest,
    RecommendationSet, RiskContribution, RiskId,
};
pub use stats::{
    parse_stats_csv, ChampionStatRow, DiscardedRow, ObservedStats, StatsCsvImport, StatsCsvMeta,
    StatsError, StatsQueue, MAX_ROWS, MIN_GAMES_FOR_POWER, PRIOR_GAMES, PRIOR_WINRATE,
    STATS_SCHEMA_VERSION,
};
pub use taxonomy::{
    taxonomy, ArchetypeCategory, ArchetypeEntry, ArchetypeId, RubricAnchor, Taxonomy,
    TaxonomyError, TraitEntry, TraitId, TraitKind, TAXONOMY_VERSION,
};

pub const ENGINE_VERSION: &str = "0.6.0-phase3-recommendations";

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn engine_version_is_set() {
        assert!(!super::engine_version().is_empty());
    }
}
