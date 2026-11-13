//! Motor de recomendaciones.
//!
//! Fase 2: taxonomía estratégica y perfiles de campeón con procedencia,
//! confianza y validación. La agregación de composiciones y la puntuación
//! multicriterio llegan en incrementos posteriores.

pub mod analysis;
pub mod profile;
pub mod proposals;
pub mod taxonomy;

pub use analysis::{
    analyze_draft, analyze_team, AnalysisError, AnalysisParams, AnalysisParamsError, CcProfile,
    Coverage, DamageProfile, Dimension, DraftAnalysis, Need, NeedSource, NeedStatus,
    PickedChampion, PowerCurve, RangeProfile, Signal, TeamAnalysis, ThreatRuleId, WinCondition,
    WinConditionId,
};
pub use profile::{
    ArchetypeRating, ChampionProfile, DamageShares, ProfileError, ProfileRole, ProfileSet,
    Provenance, TraitRating, DAMAGE_SHARE_TOLERANCE,
};
pub use proposals::{
    ChampionProposal, ProfileProposalSet, ProposalError, ProposalRole, PILOT_PROFILE_PROPOSAL_JSON,
};
pub use taxonomy::{
    taxonomy, ArchetypeCategory, ArchetypeEntry, ArchetypeId, RubricAnchor, Taxonomy,
    TaxonomyError, TraitEntry, TraitId, TraitKind, TAXONOMY_VERSION,
};

pub const ENGINE_VERSION: &str = "0.4.0-phase2-pilot";

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
