use crate::ids::{ChampionKey, Role};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleFit {
    pub role: Role,
    pub proficiency: f32,
    pub confidence: f32,
    pub source: String,
}

/// Identidad de un campeón sin perfil estratégico. Se usa para sincronizar
/// catálogos sin pisar roles o arquetipos ya curados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionIdentity {
    pub riot_key: u16,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Champion {
    pub key: ChampionKey,
    pub slug: String,
    pub name: String,
    pub title: String,
    pub image: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub roles: Vec<RoleFit>,
}

impl Champion {
    pub fn primary_role(&self) -> Option<Role> {
        self.roles
            .iter()
            .max_by(|a, b| {
                a.proficiency
                    .partial_cmp(&b.proficiency)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|fit| fit.role)
    }

    pub fn role_fit(&self, role: Role) -> Option<&RoleFit> {
        self.roles.iter().find(|fit| fit.role == role)
    }

    pub fn is_flex(&self) -> bool {
        self.roles.len() > 1
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChampionCatalog {
    champions: BTreeMap<ChampionKey, Champion>,
}

impl ChampionCatalog {
    pub fn new(champions: impl IntoIterator<Item = Champion>) -> Self {
        let mut map = BTreeMap::new();
        for champion in champions {
            map.insert(champion.key, champion);
        }
        Self { champions: map }
    }

    pub fn get(&self, key: ChampionKey) -> Option<&Champion> {
        self.champions.get(&key)
    }

    pub fn contains(&self, key: ChampionKey) -> bool {
        self.champions.contains_key(&key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Champion> {
        self.champions.values()
    }

    pub fn len(&self) -> usize {
        self.champions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.champions.is_empty()
    }

    pub fn as_slice(&self) -> Vec<&Champion> {
        self.champions.values().collect()
    }
}
