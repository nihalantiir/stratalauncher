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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedRuntime {
    pub component: String,
    pub major_version: Option<u32>,
    pub size_bytes: u64,
}

/// Runtimes Strata itself has downloaded to `java_dir()`, for Settings'
/// "Downloaded Java runtimes" list.
#[tauri::command]
pub fn list_downloaded_runtimes() -> AppResult<Vec<DownloadedRuntime>> {
    let dir = crate::paths::java_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let component = entry.file_name().to_string_lossy().to_string();
        out.push(DownloadedRuntime {
            major_version: java_runtime::component_major_version(&component),
            size_bytes: crate::fsutil::dir_size(&entry.path()),
            component,
        });
    }
    out.sort_by(|a, b| b.major_version.cmp(&a.major_version));
    Ok(out)
}

/// Deletes one downloaded runtime; a launch that needs it again just
/// re-downloads, so this is safe to allow any time.
#[tauri::command]
pub fn delete_downloaded_runtime(component: String) -> AppResult<()> {
    let dir = crate::paths::java_dir().join(&component);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}
