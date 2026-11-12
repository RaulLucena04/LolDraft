use crate::error::DataError;
pub use loldraft_domain::ChampionIdentity;
use loldraft_domain::RoleFit;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ChampionSeedFile {
    #[serde(default)]
    pub dataset: SeedDataset,
    pub champions: Vec<SeedChampion>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeedDataset {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub ddragon_version: Option<String>,
    #[serde(default)]
    pub roles_source: Option<String>,
    #[serde(default)]
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedChampion {
    pub riot_key: u16,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub roles: Vec<RoleFit>,
}

pub fn parse_seed(json: &str) -> Result<ChampionSeedFile, DataError> {
    let seed: ChampionSeedFile = serde_json::from_str(json)?;
    if seed.champions.is_empty() {
        return Err(DataError::Invalid(
            "La semilla no contiene campeones".to_string(),
        ));
    }
    Ok(seed)
}

/// Semilla empaquetada con la aplicación. Solo contiene identidad y roles
/// importados; no contiene estadísticas ni perfiles estratégicos inventados.
pub fn default_seed_json() -> &'static str {
    include_str!("../seeds/champions.json")
}

pub fn default_seed() -> Result<ChampionSeedFile, DataError> {
    parse_seed(default_seed_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimal_seed() {
        let json = r#"{
          "dataset": {"kind": "champion_identity"},
          "champions": [{
            "riot_key": 1,
            "slug": "TestChampion",
            "name": "Test Champion",
            "title": "the Test",
            "image": "TestChampion.png",
            "tags": ["Mage"],
            "roles": [{"role":"mid","proficiency":1.0,"confidence":0.8,"source":"test"}]
          }]
        }"#;
        let seed = parse_seed(json).unwrap();
        assert_eq!(seed.champions.len(), 1);
        assert_eq!(seed.champions[0].roles[0].role, loldraft_domain::Role::Mid);
    }

    #[test]
    fn packaged_seed_has_champions() {
        let seed = default_seed().unwrap();
        assert!(seed.champions.len() >= 170);
        assert!(seed
            .champions
            .iter()
            .any(|champion| champion.slug == "Aatrox"));
    }
}
