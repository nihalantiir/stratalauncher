use crate::error::{AppError, AppResult};
use crate::minecraft::loaders::{self, LoaderVersionEntry};
use crate::minecraft::manifest;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub async fn list_loader_versions(
    state: State<'_, AppState>,
    loader: String,
    mc_version: String,
) -> AppResult<Vec<LoaderVersionEntry>> {
    loaders::fetch_versions(&state.http, &loader, &mc_version).await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderAvailability {
    pub loader: String,
    pub available: bool,
    pub build_count: usize,
}

async fn check_one(client: &reqwest::Client, loader: &str, mc_version: &str) -> LoaderAvailability {
    let build_count = loaders::fetch_versions(client, loader, mc_version)
        .await
        .map(|list| list.len())
        .unwrap_or(0);
    LoaderAvailability { loader: loader.to_string(), available: build_count > 0, build_count }
}

/// Checks all 4 mod loaders against one Minecraft version at once, so the
/// Version page can grey out an unavailable one. Fired concurrently.
#[tauri::command]
pub async fn check_loader_availability(state: State<'_, AppState>, mc_version: String) -> AppResult<Vec<LoaderAvailability>> {
    let client = &state.http;
    let (fabric, quilt, forge, neoforge) = tokio::join!(
        check_one(client, "fabric", &mc_version),
        check_one(client, "quilt", &mc_version),
        check_one(client, "forge", &mc_version),
        check_one(client, "neoforge", &mc_version),
    );
    Ok(vec![fabric, quilt, forge, neoforge])
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionComponent {
    /// Drives both the i18n label and icon in the Version page's
    /// Components panel (`COMPONENT_ICONS`/`components.*` on the frontend).
    pub id: &'static str,
    pub version: String,
}

/// LWJGL's version is read off Mojang's version JSON. Three real group
/// naming schemes exist: `org.lwjgl`, bare `lwjgl`, `org.lwjgl.lwjgl`.
fn find_lwjgl_version(version_json: &serde_json::Value) -> Option<String> {
    version_json["libraries"].as_array()?.iter().find_map(|lib| {
        let name = lib["name"].as_str()?;
        let mut parts = name.split(':');
        let group = parts.next()?;
        let artifact = parts.next()?;
        let version = parts.next()?;
        (artifact == "lwjgl" && matches!(group, "org.lwjgl" | "lwjgl" | "org.lwjgl.lwjgl")).then(|| version.to_string())
    })
}

/// Purely informational version-pinning facts (VersionView's Components
/// panel), not an editable/reorderable chain the way Prism's Advanced tab is.
#[tauri::command]
pub async fn get_version_components(
    state: State<'_, AppState>,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
) -> AppResult<Vec<VersionComponent>> {
    let client = &state.http;
    let manifest = manifest::fetch_manifest(client).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| AppError::NotFound(format!("version {mc_version}")))?;
    let version_json = manifest::fetch_version_json(client, &entry.url).await?;

    let mut components = vec![VersionComponent { id: "minecraft", version: mc_version.clone() }];

    if loader == "fabric" || loader == "quilt" {
        // Fabric/Quilt's intermediary mappings version identically to the
        // Minecraft version they target, no separate lookup needed.
        components.push(VersionComponent { id: "intermediary", version: mc_version.clone() });
    }
    if let Some(lv) = loader_version.filter(|_| loader != "vanilla") {
        let id = match loader.as_str() {
            "fabric" => "fabric-loader",
            "quilt" => "quilt-loader",
            "forge" => "forge",
            "neoforge" => "neoforge",
            _ => "unknown",
        };
        components.push(VersionComponent { id, version: lv });
    }
    if let Some(lwjgl) = find_lwjgl_version(&version_json) {
        components.push(VersionComponent { id: "lwjgl", version: lwjgl });
    }

    Ok(components)
}
