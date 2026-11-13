//! Perfiles estratégicos de campeones.
//!
//! Un perfil distingue explícitamente entre "no aporta" (`Some(0.0)`) y "no
//! sabemos" (`None`). Toda celda lleva procedencia y confianza. Los datos
//! `Inferred` (heurísticas o análisis de kit) nunca se presentan como
//! observados.

use crate::taxonomy::{Taxonomy, TraitId, TraitKind};
use loldraft_domain::{ChampionKey, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Tolerancia para el reparto de daño físico/mágico/verdadero.
pub const DAMAGE_SHARE_TOLERANCE: f32 = 0.01;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Provenance {
    /// Dato publicado por Riot (por ejemplo, identidad o tags).
    Riot,
    /// Dato curado a mano y revisado por una segunda persona.
    Curated { editor: String, review: String },
    /// Dato importado de una fuente y un dataset identificables.
    Imported { source: String, dataset: String },
    /// Propuesta automática pendiente de revisión.
    Inferred { method: String, version: String },
}

impl Provenance {
    /// `true` para datos observados o curados; `false` para propuestas
    /// automáticas. La UI y el motor deben tratar `Inferred` como evidencia
    /// débil.
    pub fn is_observed(&self) -> bool {
        !matches!(self, Provenance::Inferred { .. })
    }

    pub fn label_es(&self) -> &'static str {
        match self {
            Provenance::Riot => "Riot",
            Provenance::Curated { .. } => "Curado",
            Provenance::Imported { .. } => "Importado",
            Provenance::Inferred { .. } => "Inferido",
        }
    }

    /// Código estable para la UI y para futuros filtros por procedencia.
    pub fn code(&self) -> &'static str {
        match self {
            Provenance::Riot => "riot",
            Provenance::Curated { .. } => "curated",
            Provenance::Imported { .. } => "imported",
            Provenance::Inferred { .. } => "inferred",
        }
    }

    /// Detalle legible de la procedencia. `Riot` no necesita más contexto.
    pub fn detail(&self) -> Option<String> {
        match self {
            Provenance::Riot => None,
            Provenance::Curated { editor, review } => {
                Some(format!("editor: {editor} · revisión: {review}"))
            }
            Provenance::Imported { source, dataset } => {
                Some(format!("fuente: {source} · dataset: {dataset}"))
            }
            Provenance::Inferred { method, version } => {
                Some(format!("método: {method} · versión: {version}"))
            }
        }
    }

    fn is_complete(&self) -> bool {
        match self {
            Provenance::Riot => true,
            Provenance::Curated { editor, review } => {
                !editor.trim().is_empty() && !review.trim().is_empty()
            }
            Provenance::Imported { source, dataset } => {
                !source.trim().is_empty() && !dataset.trim().is_empty()
            }
            Provenance::Inferred { method, version } => {
                !method.trim().is_empty() && !version.trim().is_empty()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRole {
    pub role: Role,
    /// Calidad del campeón en ese rol, 0..1.
    pub proficiency: f32,
    pub confidence: f32,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeRating {
    /// `None` = desconocido; no es lo mismo que 0.
    pub intensity: Option<f32>,
    pub confidence: f32,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitRating {
    /// `None` = desconocido; no es lo mismo que 0.
    pub value: Option<f32>,
    pub confidence: f32,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChampionProfile {
    pub key: ChampionKey,
    pub slug: String,
    pub name: String,
    pub roles: Vec<ProfileRole>,
    pub archetypes: BTreeMap<crate::taxonomy::ArchetypeId, ArchetypeRating>,
    pub traits: BTreeMap<TraitId, TraitRating>,
}

impl ChampionProfile {
    /// Parsea un perfil y lo valida contra la taxonomía.
    pub fn from_json(json: &str, taxonomy: &Taxonomy) -> Result<Self, ProfileError> {
        let profile: ChampionProfile = serde_json::from_str(json)?;
        profile.validate(taxonomy)?;
        Ok(profile)
    }

    pub fn validate(&self, taxonomy: &Taxonomy) -> Result<(), ProfileError> {
        if self.key.0 == 0 {
            return Err(ProfileError::InvalidChampionKey);
        }
        require_champion_text(self.key, "slug", &self.slug)?;
        require_champion_text(self.key, "name", &self.name)?;

        if self.roles.is_empty() {
            return Err(ProfileError::MissingRoles { champion: self.key });
        }
        let mut seen_roles = HashSet::with_capacity(self.roles.len());
        for role in &self.roles {
            if !seen_roles.insert(role.role) {
                return Err(ProfileError::DuplicateRole {
                    champion: self.key,
                    role: role.role.as_str(),
                });
            }
            validate_unit(
                role.proficiency,
                self.key,
                format!("role:{}.proficiency", role.role.as_str()),
            )?;
            validate_unit(
                role.confidence,
                self.key,
                format!("role:{}.confidence", role.role.as_str()),
            )?;
            validate_provenance(
                &role.source,
                self.key,
                format!("role:{}", role.role.as_str()),
            )?;
        }

        for (id, rating) in &self.archetypes {
            if taxonomy.archetype(*id).is_none() {
                return Err(ProfileError::UnknownDimension {
                    champion: self.key,
                    dimension: format!("archetype:{id}"),
                });
            }
            let dimension = format!("archetype:{id}");
            validate_rating(rating.intensity, rating.confidence, self.key, &dimension)?;
            validate_provenance(&rating.source, self.key, dimension)?;
        }

        for (id, rating) in &self.traits {
            if taxonomy.trait_entry(*id).is_none() {
                return Err(ProfileError::UnknownDimension {
                    champion: self.key,
                    dimension: format!("trait:{id}"),
                });
            }
            let dimension = format!("trait:{id}");
            validate_rating(rating.value, rating.confidence, self.key, &dimension)?;
            validate_provenance(&rating.source, self.key, dimension)?;
        }

        self.validate_damage_shares()
    }

    fn validate_damage_shares(&self) -> Result<(), ProfileError> {
        let known: Vec<f32> = TraitId::ALL
            .iter()
            .filter(|id| id.kind() == TraitKind::DamageShare)
            .filter_map(|id| self.traits.get(id).and_then(|rating| rating.value))
            .collect();

        if known.is_empty() {
            return Ok(());
        }

        let sum: f32 = known.iter().sum();
        let invalid = if known.len()
            == TraitId::ALL
                .iter()
                .filter(|id| id.kind() == TraitKind::DamageShare)
                .count()
        {
            (sum - 1.0).abs() > DAMAGE_SHARE_TOLERANCE
        } else {
            sum > 1.0 + DAMAGE_SHARE_TOLERANCE
        };

        if invalid {
            return Err(ProfileError::InvalidDamageShares {
                champion: self.key,
                sum,
            });
        }
        Ok(())
    }

    /// Reparto de daño cuando los tres shares son conocidos.
    pub fn damage_shares(&self) -> Option<DamageShares> {
        let physical = self.trait_value(TraitId::DamagePhysical)?;
        let magic = self.trait_value(TraitId::DamageMagic)?;
        let true_damage = self.trait_value(TraitId::DamageTrue)?;
        Some(DamageShares {
            physical,
            magic,
            true_damage,
        })
    }

    pub fn trait_value(&self, id: TraitId) -> Option<f32> {
        self.traits.get(&id).and_then(|rating| rating.value)
    }

    /// Celdas conocidas sobre las 38 dimensiones de la taxonomía (20 arquetipos
    /// y 18 traits). Una dimensión ausente cuenta como desconocida, no como cero.
    pub fn known_cell_ratio(&self) -> f32 {
        let expected =
            crate::taxonomy::ArchetypeId::ALL.len() + crate::taxonomy::TraitId::ALL.len();
        let known = self
            .archetypes
            .values()
            .filter(|rating| rating.intensity.is_some())
            .count()
            + self
                .traits
                .values()
                .filter(|rating| rating.value.is_some())
                .count();
        known as f32 / expected as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageShares {
    pub physical: f32,
    pub magic: f32,
    pub true_damage: f32,
}

impl DamageShares {
    pub fn total(self) -> f32 {
        self.physical + self.magic + self.true_damage
    }

    /// Normaliza una suma que se desvía dentro de la tolerancia de importación.
    pub fn normalized(self) -> Self {
        let total = self.total();
        if total <= 0.0 {
            return self;
        }
        Self {
            physical: self.physical / total,
            magic: self.magic / total,
            true_damage: self.true_damage / total,
        }
    }
}

/// Conjunto versionado de perfiles. Se carga entero o no se carga: nunca deja
/// una versión a medias.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileSet {
    pub id: String,
    pub version: String,
    pub taxonomy_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
    pub champions: BTreeMap<ChampionKey, ChampionProfile>,
}

impl ProfileSet {
    /// Carga un fichero de perfiles y valida todas sus celdas.
    pub fn from_json(json: &str, taxonomy: &Taxonomy) -> Result<Self, ProfileError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawProfileSet {
            id: String,
            version: String,
            taxonomy_version: String,
            #[serde(default)]
            patch: Option<String>,
            champions: Vec<ChampionProfile>,
        }

        let raw: RawProfileSet = serde_json::from_str(json)?;
        let mut champions = BTreeMap::new();
        for profile in raw.champions {
            if champions.contains_key(&profile.key) {
                return Err(ProfileError::DuplicateChampion {
                    champion: profile.key,
                });
            }
            champions.insert(profile.key, profile);
        }

        let set = Self {
            id: raw.id,
            version: raw.version,
            taxonomy_version: raw.taxonomy_version,
            patch: raw.patch,
            champions,
        };
        set.validate(taxonomy)?;
        Ok(set)
    }

    pub fn validate(&self, taxonomy: &Taxonomy) -> Result<(), ProfileError> {
        require_set_text("id", &self.id)?;
        require_set_text("version", &self.version)?;
        if let Some(patch) = &self.patch {
            require_set_text("patch", patch)?;
        }
        if self.taxonomy_version != taxonomy.version {
            return Err(ProfileError::TaxonomyVersionMismatch {
                expected: taxonomy.version.clone(),
                found: self.taxonomy_version.clone(),
            });
        }

        for (map_key, profile) in &self.champions {
            if *map_key != profile.key {
                return Err(ProfileError::ChampionKeyMismatch {
                    map_key: *map_key,
                    profile_key: profile.key,
                });
            }
            profile.validate(taxonomy)?;
        }
        Ok(())
    }

    pub fn get(&self, key: ChampionKey) -> Option<&ChampionProfile> {
        self.champions.get(&key)
    }

    pub fn get_mut(&mut self, key: ChampionKey) -> Option<&mut ChampionProfile> {
        self.champions.get_mut(&key)
    }

    pub fn len(&self) -> usize {
        self.champions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.champions.is_empty()
    }
}

fn validate_rating(
    value: Option<f32>,
    confidence: f32,
    champion: ChampionKey,
    dimension: &str,
) -> Result<(), ProfileError> {
    validate_unit(confidence, champion, format!("{dimension}.confidence"))?;
    match value {
        Some(value) => validate_unit(value, champion, dimension.to_string()),
        None if confidence != 0.0 => Err(ProfileError::UnknownWithConfidence {
            champion,
            dimension: dimension.to_string(),
            confidence,
        }),
        None => Ok(()),
    }
}

fn validate_unit(value: f32, champion: ChampionKey, field: String) -> Result<(), ProfileError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ProfileError::InvalidValue {
            champion,
            field,
            value,
        })
    }
}

fn validate_provenance(
    source: &Provenance,
    champion: ChampionKey,
    dimension: String,
) -> Result<(), ProfileError> {
    if source.is_complete() {
        Ok(())
    } else {
        Err(ProfileError::IncompleteProvenance {
            champion,
            dimension,
        })
    }
}

fn require_champion_text(
    champion: ChampionKey,
    field: &'static str,
    value: &str,
) -> Result<(), ProfileError> {
    if value.trim().is_empty() {
        Err(ProfileError::EmptyChampionField { champion, field })
    } else {
        Ok(())
    }
}

fn require_set_text(field: &'static str, value: &str) -> Result<(), ProfileError> {
    if value.trim().is_empty() {
        Err(ProfileError::EmptySetField { field })
    } else {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("no se pudo leer el perfil: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("la clave de campeón 0 no es válida")]
    InvalidChampionKey,
    #[error("campo vacío `{field}` en el campeón {champion}")]
    EmptyChampionField {
        champion: ChampionKey,
        field: &'static str,
    },
    #[error("campo vacío `{field}` en el conjunto de perfiles")]
    EmptySetField { field: &'static str },
    #[error("el campeón {champion} no tiene roles")]
    MissingRoles { champion: ChampionKey },
    #[error("rol duplicado `{role}` en el campeón {champion}")]
    DuplicateRole {
        champion: ChampionKey,
        role: &'static str,
    },
    #[error("valor fuera de 0..1 en `{field}` del campeón {champion}: {value}")]
    InvalidValue {
        champion: ChampionKey,
        field: String,
        value: f32,
    },
    #[error(
        "`{dimension}` del campeón {champion} es desconocido pero declara confianza {confidence}"
    )]
    UnknownWithConfidence {
        champion: ChampionKey,
        dimension: String,
        confidence: f32,
    },
    #[error("procedencia incompleta en `{dimension}` del campeón {champion}")]
    IncompleteProvenance {
        champion: ChampionKey,
        dimension: String,
    },
    #[error(
        "los shares de daño conocidos del campeón {champion} suman {sum:.3}; deben sumar 1 ±0.01 si están los tres o dejar el resto desconocido"
    )]
    InvalidDamageShares { champion: ChampionKey, sum: f32 },
    #[error("el campeón {champion} aparece más de una vez en el conjunto")]
    DuplicateChampion { champion: ChampionKey },
    #[error("dimensión `{dimension}` ajena a la taxonomía en el campeón {champion}")]
    UnknownDimension {
        champion: ChampionKey,
        dimension: String,
    },
    #[error("el conjunto esperaba la taxonomía `{expected}` pero usa `{found}`")]
    TaxonomyVersionMismatch { expected: String, found: String },
    #[error("la clave {map_key} del conjunto no coincide con la del perfil {profile_key}")]
    ChampionKeyMismatch {
        map_key: ChampionKey,
        profile_key: ChampionKey,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::{taxonomy, TAXONOMY_VERSION};
    use serde_json::{json, Value};

    fn curated() -> Value {
        json!({"kind": "curated", "editor": "test", "review": "test"})
    }

    fn profile(key: u16, slug: &str, name: &str, magic: f64) -> Value {
        json!({
            "key": key,
            "slug": slug,
            "name": name,
            "roles": [
                {"role": "top", "proficiency": 0.9, "confidence": 0.9, "source": curated()}
            ],
            "archetypes": {
                "engage": {"intensity": 0.5, "confidence": 0.8, "source": curated()},
                "frontline": {"intensity": 0.75, "confidence": 0.8, "source": curated()},
                "sustain": {"intensity": 1.0, "confidence": 0.9, "source": curated()}
            },
            "traits": {
                "damage_physical": {"value": 1.0 - magic, "confidence": 0.8, "source": curated()},
                "damage_magic": {"value": magic, "confidence": 0.8, "source": curated()},
                "damage_true": {"value": 0.0, "confidence": 0.8, "source": curated()},
                "range": {"value": 0.2, "confidence": 0.8, "source": curated()},
                "mobility": {"value": 0.4, "confidence": 0.8, "source": curated()}
            }
        })
    }

    fn valid_set() -> Value {
        json!({
            "id": "test-set",
            "version": "1",
            "taxonomy_version": TAXONOMY_VERSION,
            "patch": "16.22",
            "champions": [
                profile(266, "Aatrox", "Aatrox", 0.0),
                profile(103, "Ahri", "Ahri", 1.0)
            ]
        })
    }

    fn load(set: &Value) -> Result<ProfileSet, ProfileError> {
        ProfileSet::from_json(&set.to_string(), taxonomy())
    }

    #[test]
    fn a_valid_set_loads_into_a_map() {
        let set = load(&valid_set()).expect("set válido");
        assert_eq!(set.len(), 2);
        assert_eq!(
            set.get(ChampionKey(266)).map(|p| p.slug.as_str()),
            Some("Aatrox")
        );
        assert_eq!(
            set.get(ChampionKey(103)).map(|p| p.name.as_str()),
            Some("Ahri")
        );
        assert_eq!(set.taxonomy_version, TAXONOMY_VERSION);
        assert_eq!(set.patch.as_deref(), Some("16.22"));
    }

    #[test]
    fn unknown_cells_are_not_zero() {
        let profile = profile(266, "Aatrox", "Aatrox", 0.0);
        let mut set = valid_set();
        set["champions"] = json!([profile]);
        set["champions"][0]["archetypes"]["engage"] =
            json!({"intensity": null, "confidence": 0.0, "source": curated()});
        let loaded = load(&set).expect("desconocido permitido");
        let champion = loaded.get(ChampionKey(266)).expect("perfil");
        assert_eq!(
            champion.archetypes[&crate::taxonomy::ArchetypeId::Engage].intensity,
            None
        );
    }

    #[test]
    fn unknown_cells_cannot_claim_confidence() {
        let mut set = valid_set();
        set["champions"][0]["archetypes"]["engage"] =
            json!({"intensity": null, "confidence": 0.5, "source": curated()});
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::UnknownWithConfidence { .. }));
    }

    #[test]
    fn out_of_range_intensity_is_rejected() {
        let mut set = valid_set();
        set["champions"][0]["archetypes"]["engage"]["intensity"] = json!(1.2);
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::InvalidValue { .. }));
    }

    #[test]
    fn damage_shares_must_sum_to_one_when_complete() {
        let mut set = valid_set();
        set["champions"][0]["traits"]["damage_physical"]["value"] = json!(0.4);
        set["champions"][0]["traits"]["damage_magic"]["value"] = json!(0.4);
        set["champions"][0]["traits"]["damage_true"]["value"] = json!(0.4);
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::InvalidDamageShares { .. }));
    }

    #[test]
    fn partial_damage_shares_cannot_exceed_one() {
        let mut set = valid_set();
        set["champions"][0]["traits"]["damage_physical"]["value"] = json!(0.9);
        set["champions"][0]["traits"]["damage_true"]["value"] = json!(0.4);
        set["champions"][0]["traits"]["damage_magic"] =
            json!({"value": null, "confidence": 0.0, "source": curated()});
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::InvalidDamageShares { .. }));
    }

    #[test]
    fn duplicate_champions_are_rejected() {
        let mut set = valid_set();
        let duplicate = set["champions"][0].clone();
        set["champions"]
            .as_array_mut()
            .expect("array")
            .push(duplicate);
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::DuplicateChampion { .. }));
    }

    #[test]
    fn taxonomy_version_must_match() {
        let mut set = valid_set();
        set["taxonomy_version"] = json!("0.0.0");
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(
            error,
            ProfileError::TaxonomyVersionMismatch { .. }
        ));
    }

    #[test]
    fn incomplete_provenance_is_rejected() {
        let mut set = valid_set();
        set["champions"][0]["archetypes"]["engage"]["source"] =
            json!({"kind": "curated", "editor": "", "review": "test"});
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::IncompleteProvenance { .. }));
    }

    #[test]
    fn duplicate_roles_are_rejected() {
        let mut set = valid_set();
        let role = set["champions"][0]["roles"][0].clone();
        set["champions"][0]["roles"]
            .as_array_mut()
            .expect("array")
            .push(role);
        let error = load(&set).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::DuplicateRole { .. }));
    }

    #[test]
    fn a_map_key_must_match_the_profile_key() {
        let champion = ChampionProfile::from_json(
            &profile(266, "Aatrox", "Aatrox", 0.0).to_string(),
            taxonomy(),
        )
        .expect("perfil válido");
        let set = ProfileSet {
            id: "test".into(),
            version: "1".into(),
            taxonomy_version: TAXONOMY_VERSION.into(),
            patch: None,
            champions: BTreeMap::from([(ChampionKey(999), champion)]),
        };
        let error = set.validate(taxonomy()).expect_err("debe fallar");
        assert!(matches!(error, ProfileError::ChampionKeyMismatch { .. }));
    }

    #[test]
    fn inferred_provenance_is_not_observed() {
        let inferred = Provenance::Inferred {
            method: "kit-heuristic".into(),
            version: "1".into(),
        };
        assert!(!inferred.is_observed());
        assert_eq!(inferred.label_es(), "Inferido");
        assert!(Provenance::Riot.is_observed());
    }

    #[test]
    fn damage_shares_normalize_within_tolerance() {
        let shares = DamageShares {
            physical: 0.5,
            magic: 0.25,
            true_damage: 0.24,
        };
        let normalized = shares.normalized();
        assert!((normalized.total() - 1.0).abs() < 1e-6);
        assert!((normalized.magic - 0.25252525).abs() < 1e-5);
    }

    #[test]
    fn known_cell_ratio_uses_the_whole_taxonomy() {
        let mut champion = ChampionProfile::from_json(
            &profile(266, "Aatrox", "Aatrox", 0.0).to_string(),
            taxonomy(),
        )
        .expect("perfil válido");
        // 3 arquetipos + 5 traits conocidos de 38 dimensiones esperadas.
        assert!((champion.known_cell_ratio() - 8.0 / 38.0).abs() < 1e-6);

        let engage = champion
            .archetypes
            .get_mut(&crate::taxonomy::ArchetypeId::Engage)
            .expect("engage declarado");
        engage.intensity = None;
        engage.confidence = 0.0;
        assert!((champion.known_cell_ratio() - 7.0 / 38.0).abs() < 1e-6);
    }

    #[test]
    fn a_profile_set_roundtrips_through_json() {
        let set = load(&valid_set()).expect("set válido");
        let json = serde_json::to_string(&set).expect("serializa");
        let parsed: ProfileSet = serde_json::from_str(&json).expect("deserializa");
        assert_eq!(parsed, set);
    }
}
