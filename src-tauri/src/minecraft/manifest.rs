use crate::error::AppResult;
use serde::{Deserialize, Serialize};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<VersionEntry>,
}

pub async fn fetch_manifest(client: &reqwest::Client) -> AppResult<VersionManifest> {
    let manifest = client
        .get(MANIFEST_URL)
        .send()
        .await?
        .error_for_status()?
        .json::<VersionManifest>()
        .await?;
    Ok(manifest)
}

pub async fn fetch_version_json(client: &reqwest::Client, url: &str) -> AppResult<serde_json::Value> {
    let json = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    Ok(json)
}
