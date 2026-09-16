use super::LoaderVersionEntry;
use crate::error::AppResult;
use serde::Deserialize;

const META_BASE: &str = "https://meta.quiltmc.org/v3/versions/loader";
const INTERMEDIARY_BASE: &str = "https://meta.quiltmc.org/v3/versions/intermediary";

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default = "default_stable")]
    stable: bool,
}

fn default_stable() -> bool {
    true
}

/// Quilt's loader-build feed returns builds even without real mappings;
/// checking `/v3/versions/intermediary/{mc_version}` greys Quilt out correctly.
async fn intermediary_available(client: &reqwest::Client, mc_version: &str) -> bool {
    let url = format!("{INTERMEDIARY_BASE}/{mc_version}");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<Vec<serde_json::Value>>()
            .await
            .map(|list| !list.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> AppResult<Vec<LoaderVersionEntry>> {
    if !intermediary_available(client, mc_version).await {
        return Ok(Vec::new());
    }

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
