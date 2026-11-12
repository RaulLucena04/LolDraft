//! Taxonomía estratégica versionada.
//!
//! La taxonomía describe qué aporta un campeón (arquetipos) y cómo es
//! (traits). El seed vive en `data/taxonomy-v1.json` para poder revisarlo sin
//! recompilar la lógica, pero sus IDs son enums cerrados: un ID desconocido no
//! entra al motor.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::sync::OnceLock;

/// Versión del seed incluido en el binario.
pub const TAXONOMY_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeId {
    Engage,
    Disengage,
    Frontline,
    Tank,
    Bruiser,
    Assassin,
    Poke,
    Burst,
    Dps,
    Scaling,
    EarlyGame,
    Peel,
    Utility,
    Catch,
    Splitpush,
    WomboCombo,
    ZoneControl,
    Sustain,
    AntiTank,
    Siege,
}

impl ArchetypeId {
    pub const ALL: [ArchetypeId; 20] = [
        ArchetypeId::Engage,
        ArchetypeId::Disengage,
        ArchetypeId::Frontline,
        ArchetypeId::Tank,
        ArchetypeId::Bruiser,
        ArchetypeId::Assassin,
        ArchetypeId::Poke,
        ArchetypeId::Burst,
        ArchetypeId::Dps,
        ArchetypeId::Scaling,
        ArchetypeId::EarlyGame,
        ArchetypeId::Peel,
        ArchetypeId::Utility,
        ArchetypeId::Catch,
        ArchetypeId::Splitpush,
        ArchetypeId::WomboCombo,
        ArchetypeId::ZoneControl,
        ArchetypeId::Sustain,
        ArchetypeId::AntiTank,
        ArchetypeId::Siege,
    ];

    /// Código estable que se guarda en SQLite y en los ficheros de perfiles.
    pub fn as_str(self) -> &'static str {
        match self {
            ArchetypeId::Engage => "engage",
            ArchetypeId::Disengage => "disengage",
            ArchetypeId::Frontline => "frontline",
            ArchetypeId::Tank => "tank",
            ArchetypeId::Bruiser => "bruiser",
            ArchetypeId::Assassin => "assassin",
            ArchetypeId::Poke => "poke",
            ArchetypeId::Burst => "burst",
            ArchetypeId::Dps => "dps",
            ArchetypeId::Scaling => "scaling",
            ArchetypeId::EarlyGame => "early_game",
            ArchetypeId::Peel => "peel",
            ArchetypeId::Utility => "utility",
            ArchetypeId::Catch => "catch",
            ArchetypeId::Splitpush => "splitpush",
            ArchetypeId::WomboCombo => "wombo_combo",
            ArchetypeId::ZoneControl => "zone_control",
            ArchetypeId::Sustain => "sustain",
            ArchetypeId::AntiTank => "anti_tank",
            ArchetypeId::Siege => "siege",
        }
    }

    pub fn category(self) -> ArchetypeCategory {
        match self {
            ArchetypeId::Engage | ArchetypeId::Catch | ArchetypeId::WomboCombo => {
                ArchetypeCategory::Initiation
            }
            ArchetypeId::Disengage
            | ArchetypeId::Frontline
            | ArchetypeId::Tank
            | ArchetypeId::Peel
            | ArchetypeId::Sustain => ArchetypeCategory::Defense,
            ArchetypeId::Bruiser
            | ArchetypeId::Assassin
            | ArchetypeId::Poke
            | ArchetypeId::Burst
            | ArchetypeId::Dps
            | ArchetypeId::AntiTank => ArchetypeCategory::Damage,
            ArchetypeId::Scaling | ArchetypeId::EarlyGame => ArchetypeCategory::Tempo,
            ArchetypeId::Utility | ArchetypeId::ZoneControl => ArchetypeCategory::Utility,
            ArchetypeId::Splitpush | ArchetypeId::Siege => ArchetypeCategory::MapControl,
        }
    }
}

impl fmt::Display for ArchetypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeCategory {
    Initiation,
    Defense,
    Damage,
    Tempo,
    Utility,
    MapControl,
}

impl ArchetypeCategory {
    pub fn label_es(self) -> &'static str {
        match self {
            ArchetypeCategory::Initiation => "Inicio",
            ArchetypeCategory::Defense => "Defensa",
            ArchetypeCategory::Damage => "Daño",
            ArchetypeCategory::Tempo => "Tempo",
            ArchetypeCategory::Utility => "Utilidad",
            ArchetypeCategory::MapControl => "Control de mapa",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitId {
    DamagePhysical,
    DamageMagic,
    DamageTrue,
    Range,
    Mobility,
    HardCc,
    SoftCc,
    Waveclear,
    ObjectiveDps,
    Durability,
    LaneSafety,
    EarlyPower,
    MidPower,
    LatePower,
    Aoe,
    SingleTarget,
    GoldDependency,
    ExecutionDifficulty,
}

impl TraitId {
    pub const ALL: [TraitId; 18] = [
        TraitId::DamagePhysical,
        TraitId::DamageMagic,
        TraitId::DamageTrue,
        TraitId::Range,
        TraitId::Mobility,
        TraitId::HardCc,
        TraitId::SoftCc,
        TraitId::Waveclear,
        TraitId::ObjectiveDps,
        TraitId::Durability,
        TraitId::LaneSafety,
        TraitId::EarlyPower,
        TraitId::MidPower,
        TraitId::LatePower,
        TraitId::Aoe,
        TraitId::SingleTarget,
        TraitId::GoldDependency,
        TraitId::ExecutionDifficulty,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TraitId::DamagePhysical => "damage_physical",
            TraitId::DamageMagic => "damage_magic",
            TraitId::DamageTrue => "damage_true",
            TraitId::Range => "range",
            TraitId::Mobility => "mobility",
            TraitId::HardCc => "hard_cc",
            TraitId::SoftCc => "soft_cc",
            TraitId::Waveclear => "waveclear",
            TraitId::ObjectiveDps => "objective_dps",
            TraitId::Durability => "durability",
            TraitId::LaneSafety => "lane_safety",
            TraitId::EarlyPower => "early_power",
            TraitId::MidPower => "mid_power",
            TraitId::LatePower => "late_power",
            TraitId::Aoe => "aoe",
            TraitId::SingleTarget => "single_target",
            TraitId::GoldDependency => "gold_dependency",
            TraitId::ExecutionDifficulty => "execution_difficulty",
        }
    }

    pub fn kind(self) -> TraitKind {
        match self {
            TraitId::DamagePhysical | TraitId::DamageMagic | TraitId::DamageTrue => {
                TraitKind::DamageShare
            }
            _ => TraitKind::Scalar,
        }
    }

    pub fn is_damage_share(self) -> bool {
        self.kind() == TraitKind::DamageShare
    }
}

impl fmt::Display for TraitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitKind {
    /// Reparto de daño: los tres shares conocidos suman 1.
    DamageShare,
    /// Capacidad escalar 0..1.
    Scalar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeEntry {
    pub id: ArchetypeId,
    pub category: ArchetypeCategory,
    pub label_es: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitEntry {
    pub id: TraitId,
    pub kind: TraitKind,
    pub label_es: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RubricAnchor {
    pub value: f32,
    pub label_es: String,
    pub definition: String,
}

/// Taxonomía validada. Se construye una sola vez desde el seed incluido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Taxonomy {
    pub version: String,
    pub archetypes: Vec<ArchetypeEntry>,
    pub traits: Vec<TraitEntry>,
    pub rubric: Vec<RubricAnchor>,
}

impl Taxonomy {
    /// Parsea y valida el seed incluido en el binario.
    pub fn seed() -> Result<Self, TaxonomyError> {
        let taxonomy: Taxonomy = serde_json::from_str(include_str!("../data/taxonomy-v1.json"))?;
        taxonomy.validate()?;
        Ok(taxonomy)
    }

    pub fn validate(&self) -> Result<(), TaxonomyError> {
        if self.version.trim().is_empty() {
            return Err(TaxonomyError::MissingVersion);
        }

        validate_unique_archetypes(&self.archetypes)?;
        validate_unique_traits(&self.traits)?;

        for entry in &self.archetypes {
            require_text("arquetipo", entry.id.as_str(), &entry.label_es)?;
            require_text("arquetipo", entry.id.as_str(), &entry.definition)?;
            if entry.category != entry.id.category() {
                return Err(TaxonomyError::ArchetypeCategoryMismatch { id: entry.id });
            }
        }

        for entry in &self.traits {
            require_text("trait", entry.id.as_str(), &entry.label_es)?;
            require_text("trait", entry.id.as_str(), &entry.definition)?;
            if entry.kind != entry.id.kind() {
                return Err(TaxonomyError::TraitKindMismatch { id: entry.id });
            }
        }

        let expected = [0.0, 0.25, 0.5, 0.75, 1.0];
        if self.rubric.len() != expected.len() {
            return Err(TaxonomyError::InvalidRubric);
        }
        for (anchor, expected_value) in self.rubric.iter().zip(expected) {
            if !anchor.value.is_finite() || (anchor.value - expected_value).abs() > 1e-6 {
                return Err(TaxonomyError::InvalidRubric);
            }
            require_text("rúbrica", "ancla", &anchor.label_es)?;
            require_text("rúbrica", "ancla", &anchor.definition)?;
        }

        Ok(())
    }

    pub fn archetype(&self, id: ArchetypeId) -> Option<&ArchetypeEntry> {
        self.archetypes.iter().find(|entry| entry.id == id)
    }

    pub fn trait_entry(&self, id: TraitId) -> Option<&TraitEntry> {
        self.traits.iter().find(|entry| entry.id == id)
    }

    /// Etiqueta del ancla de intensidad más cercana, para mostrar 0..1 sin
    /// inventar precisión.
    pub fn intensity_label(&self, value: Option<f32>) -> Option<&str> {
        let value = value?;
        self.rubric
            .iter()
            .min_by(|a, b| (a.value - value).abs().total_cmp(&(b.value - value).abs()))
            .map(|anchor| anchor.label_es.as_str())
    }
}

fn validate_unique_archetypes(entries: &[ArchetypeEntry]) -> Result<(), TaxonomyError> {
    let mut seen = HashSet::with_capacity(entries.len());
    for entry in entries {
        if !seen.insert(entry.id) {
            return Err(TaxonomyError::DuplicateArchetype { id: entry.id });
        }
    }
    for id in ArchetypeId::ALL {
        if !seen.contains(&id) {
            return Err(TaxonomyError::MissingArchetype { id });
        }
    }
    Ok(())
}

fn validate_unique_traits(entries: &[TraitEntry]) -> Result<(), TaxonomyError> {
    let mut seen = HashSet::with_capacity(entries.len());
    for entry in entries {
        if !seen.insert(entry.id) {
            return Err(TaxonomyError::DuplicateTrait { id: entry.id });
        }
    }
    for id in TraitId::ALL {
        if !seen.contains(&id) {
            return Err(TaxonomyError::MissingTrait { id });
        }
    }
    Ok(())
}

fn require_text(section: &'static str, id: &'static str, value: &str) -> Result<(), TaxonomyError> {
    if value.trim().is_empty() {
        return Err(TaxonomyError::EmptyText {
            section,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// Taxonomía incluida en el binario. El seed está cubierto por tests, por lo
/// que un fallo aquí es un error de build, no una condición de ejecución.
pub fn taxonomy() -> &'static Taxonomy {
    static TAXONOMY: OnceLock<Taxonomy> = OnceLock::new();
    TAXONOMY.get_or_init(|| Taxonomy::seed().expect("la taxonomía incluida debe ser válida"))
}

#[derive(Debug, thiserror::Error)]
pub enum TaxonomyError {
    #[error("no se pudo leer el seed de taxonomía: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("la taxonomía no tiene versión")]
    MissingVersion,
    #[error("falta el arquetipo `{id}`")]
    MissingArchetype { id: ArchetypeId },
    #[error("el arquetipo `{id}` está duplicado")]
    DuplicateArchetype { id: ArchetypeId },
    #[error("la categoría del arquetipo `{id}` no coincide con su definición de código")]
    ArchetypeCategoryMismatch { id: ArchetypeId },
    #[error("falta el trait `{id}`")]
    MissingTrait { id: TraitId },
    #[error("el trait `{id}` está duplicado")]
    DuplicateTrait { id: TraitId },
    #[error("el tipo del trait `{id}` no coincide con su definición de código")]
    TraitKindMismatch { id: TraitId },
    #[error("la rúbrica debe tener cinco anclas en 0, 0.25, 0.5, 0.75 y 1")]
    InvalidRubric,
    #[error("texto vacío en {section} `{id}`")]
    EmptyText { section: &'static str, id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_has_the_expected_shape() {
        let taxonomy = Taxonomy::seed().expect("seed válido");
        assert_eq!(taxonomy.version, TAXONOMY_VERSION);
        assert_eq!(taxonomy.archetypes.len(), 20);
        assert_eq!(taxonomy.traits.len(), 18);
        assert_eq!(taxonomy.rubric.len(), 5);
        assert_eq!(
            taxonomy.rubric.iter().map(|a| a.value).collect::<Vec<_>>(),
            vec![0.0, 0.25, 0.5, 0.75, 1.0]
        );
    }

    #[test]
    fn every_id_is_unique_and_present() {
        let taxonomy = taxonomy();
        let archetypes: HashSet<_> = taxonomy.archetypes.iter().map(|e| e.id).collect();
        let traits: HashSet<_> = taxonomy.traits.iter().map(|e| e.id).collect();
        assert_eq!(archetypes.len(), ArchetypeId::ALL.len());
        assert_eq!(traits.len(), TraitId::ALL.len());
        for id in ArchetypeId::ALL {
            assert!(archetypes.contains(&id), "falta {id}");
            assert_eq!(taxonomy.archetype(id).map(|e| e.id), Some(id));
        }
        for id in TraitId::ALL {
            assert!(traits.contains(&id), "falta {id}");
            assert_eq!(taxonomy.trait_entry(id).map(|e| e.id), Some(id));
        }
    }

    #[test]
    fn ids_serialize_as_stable_snake_case_codes() {
        let cases = [
            (ArchetypeId::EarlyGame, "early_game"),
            (ArchetypeId::WomboCombo, "wombo_combo"),
            (ArchetypeId::AntiTank, "anti_tank"),
        ];
        for (id, code) in cases {
            assert_eq!(id.as_str(), code);
            let json = serde_json::to_string(&id).expect("serializa");
            assert_eq!(json, format!("\"{code}\""));
            let parsed: ArchetypeId = serde_json::from_str(&json).expect("deserializa");
            assert_eq!(parsed, id);
        }

        assert_eq!(TraitId::HardCc.as_str(), "hard_cc");
        assert_eq!(TraitId::ObjectiveDps.as_str(), "objective_dps");
        assert_eq!(TraitId::DamageMagic.kind(), TraitKind::DamageShare);
        assert_eq!(TraitId::LatePower.kind(), TraitKind::Scalar);
    }

    #[test]
    fn intensity_labels_use_the_nearest_anchor() {
        let taxonomy = taxonomy();
        assert_eq!(taxonomy.intensity_label(Some(0.0)), Some("No aporta"));
        assert_eq!(taxonomy.intensity_label(Some(0.27)), Some("Incidental"));
        assert_eq!(taxonomy.intensity_label(Some(0.48)), Some("Sólida"));
        assert_eq!(taxonomy.intensity_label(Some(0.72)), Some("Fuerte"));
        assert_eq!(taxonomy.intensity_label(Some(1.0)), Some("Definitoria"));
        assert_eq!(taxonomy.intensity_label(None), None);
    }

    #[test]
    fn a_missing_archetype_is_rejected() {
        let mut taxonomy = Taxonomy::seed().expect("seed válido");
        taxonomy.archetypes.pop();
        let error = taxonomy.validate().expect_err("debe fallar");
        assert!(matches!(error, TaxonomyError::MissingArchetype { .. }));
    }

    #[test]
    fn a_duplicated_archetype_is_rejected() {
        let mut taxonomy = Taxonomy::seed().expect("seed válido");
        let duplicate = taxonomy.archetypes[0].clone();
        taxonomy.archetypes.push(duplicate);
        let error = taxonomy.validate().expect_err("debe fallar");
        assert!(matches!(error, TaxonomyError::DuplicateArchetype { .. }));
    }

    #[test]
    fn a_wrong_rubric_is_rejected() {
        let mut taxonomy = Taxonomy::seed().expect("seed válido");
        taxonomy.rubric[2].value = 0.6;
        let error = taxonomy.validate().expect_err("debe fallar");
        assert!(matches!(error, TaxonomyError::InvalidRubric));
    }
}
