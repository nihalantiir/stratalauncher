//! Opens an already-resolved path in the OS file explorer via the opener
//! plugin's direct Rust API, which skips its IPC command's static scope
//! check (that scope can't express Strata's dynamic, possibly user-chosen
//! data directory, so the plugin's own `open_path` command always denies it).

use crate::error::{AppError, AppResult};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn open_in_explorer(app: AppHandle, path: String) -> AppResult<()> {
    app.opener().open_path(path, None::<&str>).map_err(|e| AppError::Other(e.to_string()))
}
