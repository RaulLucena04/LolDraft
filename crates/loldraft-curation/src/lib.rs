//! Utilidades de curación de perfiles.
//!
//! La curación es una operación explícita: solo convierte celdas `Inferred` en
//! `Curated` y deja intactas las celdas que ya tienen una fuente. Este crate no
//! toca la base de datos ni la app; trabaja con ficheros para que el motor siga
//! siendo puro y el resultado pueda revisarse antes de importarlo.
//!
//! Nunca se rellenan celdas desconocidas. Una celda `None` sigue siendo
//! desconocida después de aprobarla.

use loldraft_domain::{ChampionKey, Role};
use loldraft_engine::{taxonomy, ArchetypeId, ChampionProfile, ProfileSet, Provenance, TraitId};
use std::collections::BTreeSet;

/// Recuento de celdas de perfil: arquetipos y traits. Los roles se cuentan
/// aparte porque no son una dimensión de la taxonomía.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellCounts {
    pub observed: usize,
    pub inferred: usize,
    pub unknown: usize,
}

impl CellCounts {
    pub fn known(self) -> usize {
        self.observed + self.inferred
    }

    pub fn total(self) -> usize {
        self.known() + self.unknown
    }

    pub fn known_ratio(self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.known() as f32 / total as f32
        }
    }
}

/// Celdas conocidas agrupadas por procedencia, más las desconocidas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProvenanceCounts {
    pub riot: usize,
    pub curated: usize,
    pub imported: usize,
    pub inferred: usize,
    pub unknown: usize,
}

impl ProvenanceCounts {
    pub fn cells(self) -> CellCounts {
        CellCounts {
            observed: self.riot + self.curated + self.imported,
            inferred: self.inferred,
            unknown: self.unknown,
        }
    }

    fn merge(self, other: Self) -> Self {
        Self {
            riot: self.riot + other.riot,
            curated: self.curated + other.curated,
            imported: self.imported + other.imported,
            inferred: self.inferred + other.inferred,
            unknown: self.unknown + other.unknown,
        }
    }
}

/// Celdas de un campeón, separando observadas, inferidas y desconocidas.
pub fn profile_cell_counts(profile: &ChampionProfile) -> CellCounts {
    profile_provenance_counts(profile).cells()
}

/// Celdas de un conjunto completo.
pub fn set_cell_counts(set: &ProfileSet) -> CellCounts {
    set_provenance_counts(set).cells()
}

pub fn profile_provenance_counts(profile: &ChampionProfile) -> ProvenanceCounts {
    let mut counts = ProvenanceCounts::default();
    // Una dimensión ausente del mapa no es un cero: es una celda desconocida.
    for id in ArchetypeId::ALL {
        match profile.archetypes.get(&id) {
            Some(rating) => count_rating(&mut counts, rating.intensity, &rating.source),
            None => counts.unknown += 1,
        }
    }
    for id in TraitId::ALL {
        match profile.traits.get(&id) {
            Some(rating) => count_rating(&mut counts, rating.value, &rating.source),
            None => counts.unknown += 1,
        }
    }
    counts
}

pub fn set_provenance_counts(set: &ProfileSet) -> ProvenanceCounts {
    set.champions
        .values()
        .map(profile_provenance_counts)
        .fold(ProvenanceCounts::default(), ProvenanceCounts::merge)
}

fn count_rating(counts: &mut ProvenanceCounts, value: Option<f32>, source: &Provenance) {
    if value.is_none() {
        counts.unknown += 1;
        return;
    }
    match source {
        Provenance::Riot => counts.riot += 1,
        Provenance::Curated { .. } => counts.curated += 1,
        Provenance::Imported { .. } => counts.imported += 1,
        Provenance::Inferred { .. } => counts.inferred += 1,
    }
}

/// Selección de campeones sobre los que aplicar una revisión. Vacía = todos.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewSelection {
    champions: BTreeSet<ChampionKey>,
}

impl ReviewSelection {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn only(champions: impl IntoIterator<Item = ChampionKey>) -> Self {
        Self {
            champions: champions.into_iter().collect(),
        }
    }

    pub fn is_all(&self) -> bool {
        self.champions.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.champions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.champions.len()
    }

    pub fn contains(&self, key: ChampionKey) -> bool {
        self.champions.is_empty() || self.champions.contains(&key)
    }

    /// Devuelve las claves a tocar en orden y falla si se pidió un campeón que
    /// no está en el conjunto: un typo no debe pasar como una revisión vacía.
    pub fn resolve(&self, set: &ProfileSet) -> Result<Vec<ChampionKey>, CurationError> {
        if self.champions.is_empty() {
            return Ok(set.champions.keys().copied().collect());
        }
        let mut keys = Vec::with_capacity(self.champions.len());
        for key in &self.champions {
            if !set.champions.contains_key(key) {
                return Err(CurationError::ChampionNotInSet(*key));
            }
            keys.push(*key);
        }
        Ok(keys)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CurateReport {
    /// Campeones incluidos en la selección.
    pub champions: usize,
    pub roles: usize,
    pub archetypes: usize,
    pub traits: usize,
    /// Celdas que ya tenían una fuente y no se tocaron.
    pub skipped: usize,
}

impl CurateReport {
    /// Celdas de perfil convertidas (sin contar roles).
    pub fn cells(self) -> usize {
        self.archetypes + self.traits
    }

    pub fn total(self) -> usize {
        self.cells() + self.roles
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CurationError {
    #[error("el campeón {0} no está en el conjunto")]
    ChampionNotInSet(ChampionKey),
    #[error("el campo `{0}` no puede estar vacío")]
    EmptyField(&'static str),
    #[error("la confianza {value} no está en 0..=1")]
    InvalidConfidence { value: f32 },
    #[error("el valor {value} no está en 0..=1")]
    InvalidValue { value: f32 },
    #[error("la celda `{0}` está repetida")]
    DuplicateCell(&'static str),
    #[error("el rol `{role}` no está declarado para el campeón {champion}")]
    RoleNotOnProfile {
        champion: ChampionKey,
        role: &'static str,
    },
    #[error("el rol `{0}` está repetido")]
    DuplicateRole(&'static str),
}

/// Celda de perfil editable: un arquetipo o un trait. Los roles se editan
/// aparte porque no son una dimensión de la taxonomía.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellTarget {
    Archetype(ArchetypeId),
    Trait(TraitId),
}

impl CellTarget {
    pub fn code(self) -> &'static str {
        match self {
            CellTarget::Archetype(id) => id.as_str(),
            CellTarget::Trait(id) => id.as_str(),
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            CellTarget::Archetype(id) => taxonomy()
                .archetype(id)
                .map(|entry| entry.label_es.as_str())
                .unwrap_or_else(|| id.as_str()),
            CellTarget::Trait(id) => taxonomy()
                .trait_entry(id)
                .map(|entry| entry.label_es.as_str())
                .unwrap_or_else(|| id.as_str()),
        }
    }
}

/// Edición explícita de una celda. `Some(valor)` fija un valor conocido;
/// `None` marca la celda como desconocida.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellEdit {
    pub target: CellTarget,
    pub value: Option<f32>,
}

/// Edición explícita de la proficiencia de un rol ya declarado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleEdit {
    pub role: Role,
    pub proficiency: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditReport {
    pub cells: usize,
    pub roles: usize,
    /// Celdas que pasaron de desconocidas a conocidas.
    pub filled_unknown: usize,
}

/// Edita celdas y roles de un campeón con una decisión humana explícita.
///
/// A diferencia de [`curate_inferred_cells`], esta función **sí** puede dar
/// valor a una celda desconocida: el valor lo aporta una persona, no una
/// heurística. Todas las celdas tocadas pasan a `Curated` con editor y
/// revisor. Las celdas no mencionadas no se tocan.
pub fn apply_profile_edits(
    set: &mut ProfileSet,
    champion: ChampionKey,
    cells: &[CellEdit],
    roles: &[RoleEdit],
    editor: &str,
    reviewer: &str,
    confidence: f32,
) -> Result<EditReport, CurationError> {
    let editor = require_name("editor", editor)?;
    let reviewer = require_name("reviewer", reviewer)?;
    validate_confidence(confidence)?;

    let mut seen_cells = BTreeSet::new();
    for edit in cells {
        if !seen_cells.insert(edit.target) {
            return Err(CurationError::DuplicateCell(edit.target.code()));
        }
        if let Some(value) = edit.value {
            validate_value(value)?;
        }
    }

    let mut seen_roles = Vec::new();
    for edit in roles {
        if seen_roles.contains(&edit.role) {
            return Err(CurationError::DuplicateRole(edit.role.as_str()));
        }
        seen_roles.push(edit.role);
        validate_value(edit.proficiency)?;
    }

    let profile = set
        .get_mut(champion)
        .ok_or(CurationError::ChampionNotInSet(champion))?;
    let source = Provenance::Curated {
        editor: editor.to_string(),
        review: reviewer.to_string(),
    };
    let mut report = EditReport::default();

    for edit in cells {
        match edit.target {
            CellTarget::Archetype(id) => {
                // Una celda ausente del mapa también es desconocida: la
                // revisión humana puede darle un valor por primera vez.
                let rating = profile.archetypes.entry(id).or_insert_with(|| {
                    loldraft_engine::ArchetypeRating {
                        intensity: None,
                        confidence: 0.0,
                        source: source.clone(),
                    }
                });
                if rating.intensity.is_none() && edit.value.is_some() {
                    report.filled_unknown += 1;
                }
                rating.intensity = edit.value;
                rating.confidence = confidence_or_zero(edit.value, confidence);
                rating.source = source.clone();
            }
            CellTarget::Trait(id) => {
                let rating =
                    profile
                        .traits
                        .entry(id)
                        .or_insert_with(|| loldraft_engine::TraitRating {
                            value: None,
                            confidence: 0.0,
                            source: source.clone(),
                        });
                if rating.value.is_none() && edit.value.is_some() {
                    report.filled_unknown += 1;
                }
                rating.value = edit.value;
                rating.confidence = confidence_or_zero(edit.value, confidence);
                rating.source = source.clone();
            }
        }
        report.cells += 1;
    }

    for edit in roles {
        let role = profile
            .roles
            .iter_mut()
            .find(|entry| entry.role == edit.role)
            .ok_or(CurationError::RoleNotOnProfile {
                champion,
                role: edit.role.as_str(),
            })?;
        role.proficiency = edit.proficiency;
        role.confidence = confidence;
        role.source = source.clone();
        report.roles += 1;
    }

    Ok(report)
}

fn confidence_or_zero(value: Option<f32>, confidence: f32) -> f32 {
    // La validación exige confianza 0 en celdas desconocidas.
    if value.is_some() {
        confidence
    } else {
        0.0
    }
}

fn validate_confidence(confidence: f32) -> Result<(), CurationError> {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        Err(CurationError::InvalidConfidence { value: confidence })
    } else {
        Ok(())
    }
}

fn validate_value(value: f32) -> Result<(), CurationError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(CurationError::InvalidValue { value })
    } else {
        Ok(())
    }
}

/// Convierte celdas `Inferred` en `Curated` para los campeones seleccionados.
///
/// - Solo cambia la procedencia de celdas inferidas; `Riot`, `Curated` e
///   `Imported` quedan intactas.
/// - Con `confidence` fija la confianza de las celdas conocidas. Las celdas
///   desconocidas conservan confianza 0, como exige la validación.
/// - Sin `confidence` conserva la confianza existente (aprobación).
pub fn curate_inferred_cells(
    set: &mut ProfileSet,
    selection: &ReviewSelection,
    editor: &str,
    reviewer: &str,
    confidence: Option<f32>,
) -> Result<CurateReport, CurationError> {
    let editor = require_name("editor", editor)?;
    let reviewer = require_name("reviewer", reviewer)?;
    if let Some(confidence) = confidence {
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
            return Err(CurationError::InvalidConfidence { value: confidence });
        }
    }

    let keys = selection.resolve(set)?;
    let source = Provenance::Curated {
        editor: editor.to_string(),
        review: reviewer.to_string(),
    };
    let mut report = CurateReport::default();

    for key in keys {
        let profile = set
            .get_mut(key)
            .expect("la selección ya comprobó que el campeón existe");
        report.champions += 1;

        for role in &mut profile.roles {
            if matches!(role.source, Provenance::Inferred { .. }) {
                role.source = source.clone();
                if let Some(confidence) = confidence {
                    role.confidence = confidence;
                }
                report.roles += 1;
            } else {
                report.skipped += 1;
            }
        }

        for rating in profile.archetypes.values_mut() {
            match (rating.intensity, &rating.source) {
                // Una celda desconocida no se cura: no hay valor que revisar.
                (Some(_), Provenance::Inferred { .. }) => {
                    rating.source = source.clone();
                    if let Some(confidence) = confidence {
                        rating.confidence = confidence;
                    }
                    report.archetypes += 1;
                }
                _ => report.skipped += 1,
            }
        }

        for rating in profile.traits.values_mut() {
            match (rating.value, &rating.source) {
                (Some(_), Provenance::Inferred { .. }) => {
                    rating.source = source.clone();
                    if let Some(confidence) = confidence {
                        rating.confidence = confidence;
                    }
                    report.traits += 1;
                }
                _ => report.skipped += 1,
            }
        }
    }

    Ok(report)
}

/// Cambia la identidad del conjunto publicado. Los campos vacíos se rechazan
/// para no escribir un set que luego no se pueda distinguir del original.
pub fn apply_identity(
    set: &mut ProfileSet,
    id: Option<&str>,
    version: Option<&str>,
) -> Result<(), CurationError> {
    if let Some(id) = id {
        set.id = require_name("id", id)?.to_string();
    }
    if let Some(version) = version {
        set.version = require_name("version", version)?.to_string();
    }
    Ok(())
}

/// `true` si el editor y el revisor son la misma persona. No se bloquea la
/// operación, pero la CLI lo avisa: `Curated` documenta una segunda revisión.
pub fn same_curator(editor: &str, reviewer: &str) -> bool {
    editor.trim().eq_ignore_ascii_case(reviewer.trim())
}

fn require_name<'a>(field: &'static str, value: &'a str) -> Result<&'a str, CurationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(CurationError::EmptyField(field))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loldraft_domain::Role;
    use loldraft_engine::{ArchetypeId, ArchetypeRating, ProfileRole, TraitId, TraitRating};
    use std::collections::BTreeMap;

    fn inferred() -> Provenance {
        Provenance::Inferred {
            method: "kit-review".into(),
            version: "1".into(),
        }
    }

    fn curated() -> Provenance {
        Provenance::Curated {
            editor: "ana".into(),
            review: "luis".into(),
        }
    }

    fn profile(key: u16, source: &Provenance) -> ChampionProfile {
        ChampionProfile {
            key: ChampionKey(key),
            slug: format!("campeon-{key}"),
            name: format!("Campeón {key}"),
            roles: vec![ProfileRole {
                role: Role::Top,
                proficiency: 0.9,
                confidence: 0.35,
                source: source.clone(),
            }],
            archetypes: BTreeMap::from([
                (
                    ArchetypeId::Bruiser,
                    ArchetypeRating {
                        intensity: Some(0.8),
                        confidence: 0.35,
                        source: source.clone(),
                    },
                ),
                (
                    ArchetypeId::Tank,
                    ArchetypeRating {
                        intensity: None,
                        confidence: 0.0,
                        source: source.clone(),
                    },
                ),
            ]),
            traits: BTreeMap::from([
                (
                    TraitId::DamagePhysical,
                    TraitRating {
                        value: Some(1.0),
                        confidence: 0.35,
                        source: source.clone(),
                    },
                ),
                (
                    TraitId::DamageMagic,
                    TraitRating {
                        value: Some(0.0),
                        confidence: 0.35,
                        source: source.clone(),
                    },
                ),
                (
                    TraitId::DamageTrue,
                    TraitRating {
                        value: Some(0.0),
                        confidence: 0.35,
                        source: source.clone(),
                    },
                ),
                (
                    TraitId::Mobility,
                    TraitRating {
                        value: None,
                        confidence: 0.0,
                        source: source.clone(),
                    },
                ),
            ]),
        }
    }

    fn set(champions: Vec<ChampionProfile>) -> ProfileSet {
        ProfileSet {
            id: "pilot".into(),
            version: "1".into(),
            taxonomy_version: loldraft_engine::TAXONOMY_VERSION.into(),
            patch: Some("16.22".into()),
            champions: champions
                .into_iter()
                .map(|profile| (profile.key, profile))
                .collect(),
        }
    }

    #[test]
    fn counts_separate_observed_inferred_and_unknown() {
        let profile = profile(1, &inferred());
        let counts = profile_cell_counts(&profile);
        assert_eq!(
            counts,
            CellCounts {
                observed: 0,
                inferred: 4,
                unknown: 34
            }
        );
        assert_eq!(counts.known(), 4);
        assert_eq!(counts.total(), 38);
    }

    #[test]
    fn curated_cells_are_observed_not_inferred() {
        let profile = profile(1, &curated());
        let counts = profile_cell_counts(&profile);
        assert_eq!(counts.observed, 4);
        assert_eq!(counts.inferred, 0);
        assert_eq!(counts.unknown, 34);
    }

    #[test]
    fn review_converts_only_inferred_cells_and_sets_confidence() {
        let mut profiles = set(vec![profile(1, &inferred()), profile(2, &curated())]);
        let report = curate_inferred_cells(
            &mut profiles,
            &ReviewSelection::all(),
            "ana",
            "luis",
            Some(0.8),
        )
        .expect("revisión válida");

        assert_eq!(report.champions, 2);
        assert_eq!(report.roles, 1);
        assert_eq!(report.archetypes, 1);
        assert_eq!(report.traits, 3);
        assert_eq!(report.cells(), 4);
        // Las celdas ya curadas del campeón 2 y las desconocidas del campeón 1
        // no se tocan.
        assert_eq!(report.skipped, 9);

        let converted = profiles.get(ChampionKey(1)).expect("campeón 1");
        assert!(matches!(
            converted.roles[0].source,
            Provenance::Curated { .. }
        ));
        assert_eq!(converted.roles[0].confidence, 0.8);
        assert_eq!(converted.archetypes[&ArchetypeId::Bruiser].confidence, 0.8);
        // Una celda desconocida no recibe confianza ni se presenta como curada.
        assert_eq!(converted.archetypes[&ArchetypeId::Tank].confidence, 0.0);
        assert!(matches!(
            converted.archetypes[&ArchetypeId::Tank].source,
            Provenance::Inferred { .. }
        ));
        assert!(matches!(
            converted.traits[&TraitId::Mobility].source,
            Provenance::Inferred { .. }
        ));

        let untouched = profiles.get(ChampionKey(2)).expect("campeón 2");
        assert_eq!(untouched.roles[0].confidence, 0.35);
        assert_eq!(untouched.roles[0].source, curated());
    }

    #[test]
    fn approve_keeps_the_existing_confidence() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        let report =
            curate_inferred_cells(&mut profiles, &ReviewSelection::all(), "ana", "luis", None)
                .expect("aprobación válida");
        assert_eq!(report.cells(), 4);
        let converted = profiles.get(ChampionKey(1)).expect("campeón 1");
        assert_eq!(converted.archetypes[&ArchetypeId::Bruiser].confidence, 0.35);
        assert_eq!(converted.roles[0].confidence, 0.35);
    }

    #[test]
    fn scope_limits_the_review_and_rejects_unknown_champions() {
        let mut profiles = set(vec![profile(1, &inferred()), profile(2, &inferred())]);
        let report = curate_inferred_cells(
            &mut profiles,
            &ReviewSelection::only([ChampionKey(1)]),
            "ana",
            "luis",
            Some(0.7),
        )
        .expect("revisión válida");
        assert_eq!(report.champions, 1);
        assert_eq!(report.cells(), 4);
        assert!(matches!(
            profiles.get(ChampionKey(2)).expect("campeón 2").roles[0].source,
            Provenance::Inferred { .. }
        ));

        let missing = curate_inferred_cells(
            &mut profiles,
            &ReviewSelection::only([ChampionKey(999)]),
            "ana",
            "luis",
            Some(0.7),
        );
        assert!(matches!(
            missing,
            Err(CurationError::ChampionNotInSet(ChampionKey(999)))
        ));
    }

    #[test]
    fn empty_names_and_bad_confidence_are_rejected() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        assert!(matches!(
            curate_inferred_cells(
                &mut profiles,
                &ReviewSelection::all(),
                "  ",
                "luis",
                Some(0.7)
            ),
            Err(CurationError::EmptyField("editor"))
        ));
        assert!(matches!(
            curate_inferred_cells(
                &mut profiles,
                &ReviewSelection::all(),
                "ana",
                "luis",
                Some(1.5)
            ),
            Err(CurationError::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn same_curator_is_detected_without_blocking() {
        assert!(same_curator("ana", " ANA "));
        assert!(!same_curator("ana", "luis"));
    }

    #[test]
    fn identity_overrides_reject_empty_values() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        apply_identity(&mut profiles, Some("reviewed"), Some("2")).expect("identidad válida");
        assert_eq!(profiles.id, "reviewed");
        assert_eq!(profiles.version, "2");
        assert!(matches!(
            apply_identity(&mut profiles, Some("  "), None),
            Err(CurationError::EmptyField("id"))
        ));
    }

    #[test]
    fn an_explicit_edit_sets_a_curated_value_and_confidence() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        let report = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[CellEdit {
                target: CellTarget::Archetype(ArchetypeId::Bruiser),
                value: Some(0.9),
            }],
            &[],
            "ana",
            "luis",
            0.85,
        )
        .expect("edición válida");

        assert_eq!(report.cells, 1);
        assert_eq!(report.filled_unknown, 0);
        let rating =
            &profiles.get(ChampionKey(1)).expect("campeón").archetypes[&ArchetypeId::Bruiser];
        assert_eq!(rating.intensity, Some(0.9));
        assert_eq!(rating.confidence, 0.85);
        assert_eq!(
            rating.source,
            Provenance::Curated {
                editor: "ana".into(),
                review: "luis".into()
            }
        );
    }

    #[test]
    fn an_explicit_edit_can_fill_a_previously_unknown_cell() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        let report = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[CellEdit {
                target: CellTarget::Trait(TraitId::Mobility),
                value: Some(0.6),
            }],
            &[],
            "ana",
            "luis",
            0.8,
        )
        .expect("edición válida");

        assert_eq!(report.filled_unknown, 1);
        let rating = &profiles.get(ChampionKey(1)).expect("campeón").traits[&TraitId::Mobility];
        assert_eq!(rating.value, Some(0.6));
        assert_eq!(rating.confidence, 0.8);
        assert!(rating.source.is_observed());
    }

    #[test]
    fn clearing_a_cell_keeps_it_unknown_with_zero_confidence() {
        let mut profiles = set(vec![profile(1, &curated())]);
        let report = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[CellEdit {
                target: CellTarget::Archetype(ArchetypeId::Bruiser),
                value: None,
            }],
            &[],
            "ana",
            "luis",
            0.9,
        )
        .expect("edición válida");

        assert_eq!(report.cells, 1);
        let rating =
            &profiles.get(ChampionKey(1)).expect("campeón").archetypes[&ArchetypeId::Bruiser];
        assert_eq!(rating.intensity, None);
        assert_eq!(rating.confidence, 0.0);
        // Una celda desconocida cuenta como desconocida, no como curada.
        assert_eq!(
            profile_cell_counts(profiles.get(ChampionKey(1)).expect("campeón")).unknown,
            35
        );
    }

    #[test]
    fn an_explicit_edit_can_change_a_role_proficiency() {
        let mut profiles = set(vec![profile(1, &inferred())]);
        let report = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[],
            &[RoleEdit {
                role: Role::Top,
                proficiency: 0.7,
            }],
            "ana",
            "luis",
            0.9,
        )
        .expect("edición válida");

        assert_eq!(report.roles, 1);
        let role = &profiles.get(ChampionKey(1)).expect("campeón").roles[0];
        assert_eq!(role.proficiency, 0.7);
        assert_eq!(role.confidence, 0.9);
        assert!(role.source.is_observed());
    }

    #[test]
    fn edits_fill_absent_cells_and_reject_duplicates_and_bad_values() {
        let mut profiles = set(vec![profile(1, &inferred())]);

        // Una celda ausente del mapa también es desconocida y puede rellenarse.
        let filled = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[CellEdit {
                target: CellTarget::Archetype(ArchetypeId::Engage),
                value: Some(0.5),
            }],
            &[],
            "ana",
            "luis",
            0.8,
        )
        .expect("rellena una celda ausente");
        assert_eq!(filled.filled_unknown, 1);
        assert_eq!(
            profiles.get(ChampionKey(1)).expect("campeón").archetypes[&ArchetypeId::Engage]
                .intensity,
            Some(0.5)
        );

        let duplicate = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[
                CellEdit {
                    target: CellTarget::Trait(TraitId::Range),
                    value: Some(0.5),
                },
                CellEdit {
                    target: CellTarget::Trait(TraitId::Range),
                    value: Some(0.6),
                },
            ],
            &[],
            "ana",
            "luis",
            0.8,
        );
        assert!(matches!(
            duplicate,
            Err(CurationError::DuplicateCell("range"))
        ));

        let bad_value = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[CellEdit {
                target: CellTarget::Trait(TraitId::Range),
                value: Some(1.5),
            }],
            &[],
            "ana",
            "luis",
            0.8,
        );
        assert!(matches!(bad_value, Err(CurationError::InvalidValue { .. })));

        let missing_role = apply_profile_edits(
            &mut profiles,
            ChampionKey(1),
            &[],
            &[RoleEdit {
                role: Role::Jungle,
                proficiency: 0.5,
            }],
            "ana",
            "luis",
            0.8,
        );
        assert!(matches!(
            missing_role,
            Err(CurationError::RoleNotOnProfile { .. })
        ));
    }
}
