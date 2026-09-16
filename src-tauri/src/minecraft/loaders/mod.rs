pub mod fabric;
pub mod forge;
pub mod quilt;

use crate::error::{AppError, AppResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersionEntry {
    pub version: String,
    pub stable: bool,
}

pub async fn fetch_versions(
    client: &reqwest::Client,
    loader: &str,
    mc_version: &str,
) -> AppResult<Vec<LoaderVersionEntry>> {
    match loader {
        "fabric" => fabric::fetch_loader_versions(client, mc_version).await,
        "quilt" => quilt::fetch_loader_versions(client, mc_version).await,
        "forge" => forge::fetch_forge_versions(client, mc_version).await,
        "neoforge" => forge::fetch_neoforge_versions(client, mc_version).await,
        other => Err(AppError::Other(format!("unsupported loader: {other}"))),
    }
}

/// Fetches the loader's merge-ready ("inheritsFrom" the vanilla profile)
/// version JSON for one specific loader build.
pub async fn fetch_profile_json(
    client: &reqwest::Client,
    loader: &str,
    mc_version: &str,
    loader_version: &str,
) -> AppResult<serde_json::Value> {
    match loader {
        "fabric" => fabric::fetch_profile_json(client, mc_version, loader_version).await,
        "quilt" => quilt::fetch_profile_json(client, mc_version, loader_version).await,
        other => Err(AppError::Other(format!("unsupported loader: {other}"))),
    }
}
