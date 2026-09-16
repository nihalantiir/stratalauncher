//! Opens a path via the opener plugin's direct Rust API, skipping its IPC
//! command's static scope check (can't express our dynamic data directory).

use crate::error::{AppError, AppResult};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn open_in_explorer(app: AppHandle, path: String) -> AppResult<()> {
    app.opener().open_path(path, None::<&str>).map_err(|e| AppError::Other(e.to_string()))
}
