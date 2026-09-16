use crate::error::{AppError, AppResult};
use crate::minecraft::{java_detect, java_runtime, launch, manifest};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredJava {
    pub component: String,
    pub major_version: Option<u32>,
}

/// What Java a version needs, from its own version JSON, letting the
/// instance settings UI show "Automatic (Java 8)" instead of a mystery.
#[tauri::command]
pub async fn get_required_java(state: State<'_, AppState>, mc_version: String) -> AppResult<RequiredJava> {
    let manifest = manifest::fetch_manifest(&state.http).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| AppError::NotFound(format!("version {mc_version}")))?;
    let version_json = manifest::fetch_version_json(&state.http, &entry.url).await?;
    let component = java_runtime::required_component(&version_json).to_string();
    let major_version = java_runtime::component_major_version(&component);
    Ok(RequiredJava { component, major_version })
}

/// Validates a manual Java override path by running it, used by instance
/// settings to warn before a launch-time hard failure.
#[tauri::command]
pub fn probe_java_at(path: String) -> AppResult<u32> {
    launch::probe_java_major_version(std::path::Path::new(&path))
}

/// Real Java installs already on the system, for the Custom picker.
#[tauri::command]
pub fn list_java_installations() -> Vec<java_detect::JavaInstallation> {
    java_detect::detect_installations()
}
