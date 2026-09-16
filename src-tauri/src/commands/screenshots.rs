use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::screenshots::{self, ScreenshotInfo};
use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;

fn instance_dir_for(state: &State<'_, AppState>, instance_id: &str) -> AppResult<PathBuf> {
    let conn = state.db.0.lock().unwrap();
    if InstancesRepo::get(&conn, instance_id)?.is_none() {
        return Err(AppError::NotFound(format!("instance {instance_id}")));
    }
    Ok(crate::paths::instance_dir(instance_id))
}

#[tauri::command]
pub fn list_screenshots(instance_id: String, state: State<'_, AppState>) -> AppResult<Vec<ScreenshotInfo>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    screenshots::list_screenshots(&dir)
}

#[tauri::command]
pub fn delete_screenshot(instance_id: String, file_name: String, state: State<'_, AppState>) -> AppResult<()> {
    let dir = instance_dir_for(&state, &instance_id)?;
    screenshots::delete_screenshot(&dir, &file_name)
}

#[tauri::command]
pub fn get_screenshots_dir(instance_id: String, state: State<'_, AppState>) -> AppResult<String> {
    let dir = instance_dir_for(&state, &instance_id)?;
    let dir = screenshots::ensure_screenshots_dir(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}
