//! Propuestas compactas de perfiles para revisión asistida.
//!
//! Una propuesta solo declara las celdas que se pueden justificar. Las celdas
//! ausentes quedan desconocidas. Al expandirla, **todas** las celdas llevan
//! procedencia `Inferred`: nunca se presentan como observadas ni curadas.

use crate::profile::{
    ArchetypeRating, ChampionProfile, ProfileError, ProfileRole, ProfileSet, Provenance,
    TraitRating, DAMAGE_SHARE_TOLERANCE,
};
use crate::taxonomy::{ArchetypeId, Taxonomy, TraitId, TraitKind};
use loldraft_domain::{ChampionKey, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileProposalSet {
    pub id: String,
    pub version: String,
    pub taxonomy_version: String,
    #[serde(default)]
    pub patch: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub method: String,
    pub method_version: String,
    #[serde(default = "default_proposal_confidence")]
    pub confidence: f32,
    pub champions: Vec<ChampionProposal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChampionProposal {
    pub key: ChampionKey,
    pub slug: String,
    pub name: String,
    pub roles: Vec<ProposalRole>,
    #[serde(default)]
    pub archetypes: BTreeMap<ArchetypeId, f32>,
    #[serde(default)]
    pub traits: BTreeMap<TraitId, f32>,
    /// Confianza específica del campeón; si falta se usa la del conjunto.
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalRole {
    pub role: Role,
    pub proficiency: f32,
    #[serde(default)]
    pub confidence: Option<f32>,
}

fn default_proposal_confidence() -> f32 {
    0.35
}

/// Piloto asistido incluido con el motor. Es `Inferred`, no curado.
pub const PILOT_PROFILE_PROPOSAL_JSON: &str =
    include_str!("../data/profiles-pilot-v1.proposals.json");

impl ProfileProposalSet {
    pub fn from_json(json: &str) -> Result<Self, ProposalError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Expande la propuesta a un `ProfileSet` completo y lo valida contra la
    /// taxonomía. Las celdas no declaradas no se rellenan.
    pub fn into_profile_set(&self, taxonomy: &Taxonomy) -> Result<ProfileSet, ProposalError> {
        require_text("id", &self.id)?;
        require_text("version", &self.version)?;
        require_text("method", &self.method)?;
        require_text("method_version", &self.method_version)?;
        if let Some(patch) = &self.patch {
            require_text("patch", patch)?;
        }
        validate_unit("confidence", self.confidence, None)?;

        let provenance = Provenance::Inferred {
            method: self.method.clone(),
            version: self.method_version.clone(),
        };
        let mut champions = BTreeMap::new();

        for proposal in &self.champions {
            if champions.contains_key(&proposal.key) {
                return Err(ProposalError::DuplicateChampion(proposal.key));
            }
            let confidence = proposal.confidence.unwrap_or(self.confidence);
            validate_unit(
                &format!("champion:{}:confidence", proposal.key),
                confidence,
                Some(proposal.key),
            )?;
            if proposal.roles.is_empty() {
                return Err(ProposalError::Profile(ProfileError::MissingRoles {
                    champion: proposal.key,
                }));
            }

            let mut roles = Vec::with_capacity(proposal.roles.len());
            let mut seen_roles = HashSet::new();
            for role in &proposal.roles {
                if !seen_roles.insert(role.role) {
                    return Err(ProposalError::Profile(ProfileError::DuplicateRole {
                        champion: proposal.key,
                        role: role.role.as_str(),
                    }));
                }
                validate_unit(
                    &format!("role:{}:proficiency", role.role.as_str()),
                    role.proficiency,
                    Some(proposal.key),
                )?;
                let role_confidence = role.confidence.unwrap_or(confidence);
                validate_unit(
                    &format!("role:{}:confidence", role.role.as_str()),
                    role_confidence,
                    Some(proposal.key),
                )?;
                roles.push(ProfileRole {
                    role: role.role,
                    proficiency: role.proficiency,
                    confidence: role_confidence,
                    source: provenance.clone(),
                });
            }

            let mut archetypes = BTreeMap::new();
            for (id, intensity) in &proposal.archetypes {
                validate_unit(
                    &format!("archetype:{}", id.as_str()),
                    *intensity,
                    Some(proposal.key),
                )?;
                archetypes.insert(
                    *id,
                    ArchetypeRating {
                        intensity: Some(*intensity),
                        confidence,
                        source: provenance.clone(),
                    },
                );
            }

            let mut traits = BTreeMap::new();
            for (id, value) in &proposal.traits {
                validate_unit(
                    &format!("trait:{}", id.as_str()),
                    *value,
                    Some(proposal.key),
                )?;
                traits.insert(
                    *id,
                    TraitRating {
                        value: Some(*value),
                        confidence,
                        source: provenance.clone(),
                    },
                );
            }

            validate_proposal_damage_shares(proposal.key, &traits)?;
            champions.insert(
                proposal.key,
                ChampionProfile {
                    key: proposal.key,
                    slug: proposal.slug.clone(),
                    name: proposal.name.clone(),
                    roles,
                    archetypes,
                    traits,
                },
            );
        }

        let set = ProfileSet {
            id: self.id.clone(),
            version: self.version.clone(),
            taxonomy_version: self.taxonomy_version.clone(),
            patch: self.patch.clone(),
            champions,
        };
        set.validate(taxonomy)?;
        Ok(set)
    }
}

fn validate_proposal_damage_shares(
    champion: ChampionKey,
    traits: &BTreeMap<TraitId, TraitRating>,
) -> Result<(), ProposalError> {
    let shares: Vec<f32> = TraitId::ALL
        .iter()
        .filter(|id| id.kind() == TraitKind::DamageShare)
        .filter_map(|id| traits.get(id).and_then(|rating| rating.value))
        .collect();
    if shares.is_empty() {
        return Ok(());
    }
    let sum: f32 = shares.iter().sum();
    let expected = TraitId::ALL
        .iter()
        .filter(|id| id.kind() == TraitKind::DamageShare)
        .count();
    let invalid = if shares.len() == expected {
        (sum - 1.0).abs() > DAMAGE_SHARE_TOLERANCE
    } else {
        sum > 1.0 + DAMAGE_SHARE_TOLERANCE
    };
    if invalid {
        return Err(ProposalError::InvalidDamageShares { champion, sum });
    }
    Ok(())
}

fn require_text(field: &'static str, value: &str) -> Result<(), ProposalError> {
    if value.trim().is_empty() {
        Err(ProposalError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_unit(
    field: &str,
    value: f32,
    champion: Option<ChampionKey>,
) -> Result<(), ProposalError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ProposalError::InvalidValue {
            champion,
            field: field.to_string(),
            value,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProposalError {
    #[error("no se pudo leer la propuesta: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("campo vacío en la propuesta: `{0}`")]
    EmptyField(&'static str),
    #[error("valor fuera de 0..1 en `{field}` del campeón {champion:?}: {value}")]
    InvalidValue {
        champion: Option<ChampionKey>,
        field: String,
        value: f32,
    },
    #[error("campeón duplicado en la propuesta: {0}")]
    DuplicateChampion(ChampionKey),
    #[error("shares de daño inválidos en {champion}: suman {sum:.3}")]
    InvalidDamageShares { champion: ChampionKey, sum: f32 },
    #[error("el perfil expandido no es válido: {0}")]
    Profile(#[from] ProfileError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::{taxonomy, TAXONOMY_VERSION};

    fn proposal_json() -> String {
        serde_json::json!({
            "id": "pilot",
            "version": "1",
            "taxonomy_version": TAXONOMY_VERSION,
            "patch": "16.22",
            "method": "kit-heuristic",
            "method_version": "1",
            "confidence": 0.35,
            "champions": [
                {
                    "key": 266,
                    "slug": "Aatrox",
                    "name": "Aatrox",
                    "roles": [{"role": "top", "proficiency": 0.95}],
                    "archetypes": {"frontline": 0.65, "sustain": 0.70},
                    "traits": {
                        "damage_physical": 1.0,
                        "damage_magic": 0.0,
                        "damage_true": 0.0,
                        "range": 0.1,
                        "mobility": 0.3
                    }
                },
                {
                    "key": 103,
                    "slug": "Ahri",
                    "name": "Ahri",
                    "roles": [{"role": "mid", "proficiency": 0.9}],
                    "archetypes": {"catch": 0.55, "burst": 0.65},
                    "traits": {
                        "damage_physical": 0.1,
                        "damage_magic": 0.9,
                        "damage_true": 0.0,
                        "range": 0.6,
                        "mobility": 0.75
                    }
                }
            ]
        })
        .to_string()
    }

    #[test]
    fn a_proposal_expands_into_inferred_profiles() {
        let proposal = ProfileProposalSet::from_json(&proposal_json()).unwrap();
        let set = proposal.into_profile_set(taxonomy()).unwrap();
        assert_eq!(set.len(), 2);
        let aatrox = set.get(ChampionKey(266)).unwrap();
        assert_eq!(aatrox.roles.len(), 1);
        assert_eq!(
            aatrox.archetypes[&ArchetypeId::Frontline].intensity,
            Some(0.65)
        );
        assert!(matches!(
            aatrox.archetypes[&ArchetypeId::Frontline].source,
            Provenance::Inferred { .. }
        ));
        assert!(matches!(
            aatrox.roles[0].source,
            Provenance::Inferred { .. }
        ));
        assert_eq!(
            aatrox.traits[&TraitId::Range].confidence,
            0.35,
            "la confianza del conjunto se propaga"
        );
        // Las celdas no declaradas quedan desconocidas, no en cero.
        assert!(!aatrox.archetypes.contains_key(&ArchetypeId::Engage));
    }

    #[test]
    fn a_proposal_can_override_champion_confidence() {
        let mut value: serde_json::Value = serde_json::from_str(&proposal_json()).unwrap();
        value["champions"][0]["confidence"] = serde_json::json!(0.2);
        let proposal = ProfileProposalSet::from_json(&value.to_string()).unwrap();
        let set = proposal.into_profile_set(taxonomy()).unwrap();
        assert_eq!(
            set.get(ChampionKey(266)).unwrap().archetypes[&ArchetypeId::Frontline].confidence,
            0.2
        );
    }

    #[test]
    fn invalid_damage_shares_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(&proposal_json()).unwrap();
        value["champions"][0]["traits"]["damage_physical"] = serde_json::json!(0.4);
        value["champions"][0]["traits"]["damage_magic"] = serde_json::json!(0.4);
        value["champions"][0]["traits"]["damage_true"] = serde_json::json!(0.4);
        let proposal = ProfileProposalSet::from_json(&value.to_string()).unwrap();
        let error = proposal.into_profile_set(taxonomy()).unwrap_err();
        assert!(matches!(error, ProposalError::InvalidDamageShares { .. }));
    }

    #[test]
    fn duplicate_champions_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(&proposal_json()).unwrap();
        let duplicate = value["champions"][0].clone();
        value["champions"].as_array_mut().unwrap().push(duplicate);
        let proposal = ProfileProposalSet::from_json(&value.to_string()).unwrap();
        let error = proposal.into_profile_set(taxonomy()).unwrap_err();
        assert!(matches!(
            error,
            ProposalError::DuplicateChampion(ChampionKey(266))
        ));
    }

    #[test]
    fn the_pilot_proposal_is_valid_and_covers_all_roles() {
        let proposal = ProfileProposalSet::from_json(PILOT_PROFILE_PROPOSAL_JSON).unwrap();
        assert_eq!(proposal.champions.len(), 38);
        let set = proposal.into_profile_set(taxonomy()).unwrap();
        assert_eq!(set.len(), 38);

        let mut roles = std::collections::HashSet::new();
        for profile in set.champions.values() {
            assert!(profile.damage_shares().is_some());
            assert!(!profile.archetypes.is_empty());
            assert!(!profile.traits.is_empty());
            for role in &profile.roles {
                roles.insert(role.role);
                assert!(matches!(role.source, Provenance::Inferred { .. }));
            }
            assert!(profile
                .archetypes
                .values()
                .all(|rating| matches!(rating.source, Provenance::Inferred { .. })));
            assert!(profile
                .traits
                .values()
                .all(|rating| matches!(rating.source, Provenance::Inferred { .. })));
        }
        assert_eq!(roles.len(), Role::ALL.len());
    }
}
