use crate::error::SourceError;
use loldraft_domain::ChampionIdentity;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;

const BASE_URL: &str = "https://ddragon.leagueoflegends.com";
const LOCALE: &str = "en_US";

pub struct DdragonClient {
    client: reqwest::Client,
    base_url: String,
    locale: String,
}

#[derive(Debug, Deserialize)]
struct ChampionList {
    version: String,
    data: BTreeMap<String, ChampionEntry>,
}

#[derive(Debug, Deserialize)]
struct ChampionEntry {
    id: String,
    key: String,
    name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    image: ChampionImage,
}

#[derive(Debug, Deserialize)]
struct ChampionImage {
    full: String,
}

impl Default for DdragonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DdragonClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("LolDraft/0.1 (desktop draft assistant)")
            .timeout(Duration::from_secs(20))
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: BASE_URL.to_string(),
            locale: LOCALE.to_string(),
        }
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let mut client = Self::new();
        client.base_url = base_url.into();
        client
    }

    pub async fn latest_version(&self) -> Result<String, SourceError> {
        let url = format!("{}/api/versions.json", self.base_url);
        let versions: Vec<String> = self.get_json(&url).await?;
        versions
            .into_iter()
            .find(|version| is_stable_version(version))
            .ok_or(SourceError::NoVersions)
    }

    pub async fn champions(&self, version: &str) -> Result<Vec<ChampionIdentity>, SourceError> {
        let url = format!(
            "{}/cdn/{}/data/{}/champion.json",
            self.base_url, version, self.locale
        );
        let list: ChampionList = self.get_json(&url).await?;
        if list.version.is_empty() {
            return Err(SourceError::InvalidData(
                "Data Dragon devolvió una versión vacía".to_string(),
            ));
        }

        let mut identities = Vec::with_capacity(list.data.len());
        for entry in list.data.into_values() {
            let riot_key = entry.key.parse::<u16>().map_err(|_| {
                SourceError::InvalidData(format!("Clave de campeón inválida: {}", entry.key))
            })?;
            identities.push(ChampionIdentity {
                riot_key,
                slug: entry.id,
                name: entry.name,
                title: entry.title,
                image: entry.image.full,
                tags: entry.tags,
            });
        }
        identities.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(identities)
    }

    pub async fn download_champion_icon(
        &self,
        version: &str,
        image_file: &str,
    ) -> Result<Vec<u8>, SourceError> {
        let url = format!(
            "{}/cdn/{}/img/champion/{}",
            self.base_url, version, image_file
        );
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    pub fn champion_icon_url(&self, version: &str, image_file: &str) -> String {
        format!(
            "{}/cdn/{}/img/champion/{}",
            self.base_url, version, image_file
        )
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, SourceError> {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }
}

fn is_stable_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3 && parts.iter().all(|part| part.parse::<u32>().is_ok())
}

pub fn identities_from_champion_json(json: &str) -> Result<Vec<ChampionIdentity>, SourceError> {
    let list: ChampionList =
        serde_json::from_str(json).map_err(|error| SourceError::InvalidData(error.to_string()))?;
    let mut identities = Vec::with_capacity(list.data.len());
    for entry in list.data.into_values() {
        let riot_key = entry.key.parse::<u16>().map_err(|_| {
            SourceError::InvalidData(format!("Clave de campeón inválida: {}", entry.key))
        })?;
        identities.push(ChampionIdentity {
            riot_key,
            slug: entry.id,
            name: entry.name,
            title: entry.title,
            image: entry.image.full,
            tags: entry.tags,
        });
    }
    identities.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(identities)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "type": "champion",
      "version": "16.22.1",
      "data": {
        "TestMage": {
          "id": "TestMage",
          "key": "999",
          "name": "Test Mage",
          "title": "the Fixture",
          "tags": ["Mage"],
          "image": {"full": "TestMage.png"}
        }
      }
    }"#;

    #[test]
    fn parses_champion_fixture() {
        let identities = identities_from_champion_json(FIXTURE).unwrap();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities[0].riot_key, 999);
        assert_eq!(identities[0].slug, "TestMage");
        assert_eq!(identities[0].tags, vec!["Mage"]);
        assert_eq!(identities[0].image, "TestMage.png");
    }

    #[test]
    fn accepts_only_stable_versions() {
        assert!(is_stable_version("16.22.1"));
        assert!(!is_stable_version("lolpatch_7.20"));
        assert!(!is_stable_version("16.22"));
        assert!(!is_stable_version("16.22.1.5"));
    }

    #[test]
    fn builds_icon_url() {
        let client = DdragonClient::with_base_url("https://example.test");
        assert_eq!(
            client.champion_icon_url("16.22.1", "Aatrox.png"),
            "https://example.test/cdn/16.22.1/img/champion/Aatrox.png"
        );
    }
}
