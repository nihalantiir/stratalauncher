use super::LoaderVersionEntry;
use crate::error::AppResult;
use serde::Deserialize;

const META_BASE: &str = "https://meta.fabricmc.net/v2/versions/loader";

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    stable: bool,
}

pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> AppResult<Vec<LoaderVersionEntry>> {
    let url = format!("{META_BASE}/{mc_version}");
    let entries = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<LoaderEntry>>()
        .await?;

    Ok(entries
        .into_iter()
        .map(|e| LoaderVersionEntry {
            version: e.loader.version,
            stable: e.loader.stable,
        })
        .collect())
}

pub async fn fetch_profile_json(
    client: &reqwest::Client,
    mc_version: &str,
    loader_version: &str,
) -> AppResult<serde_json::Value> {
    let url = format!("{META_BASE}/{mc_version}/{loader_version}/profile/json");
    let json = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    Ok(json)
}
