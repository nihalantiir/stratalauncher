use crate::error::AppResult;
use crate::minecraft::manifest::{self, VersionManifest};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> AppResult<VersionManifest> {
    manifest::fetch_manifest(&state.http).await
}
