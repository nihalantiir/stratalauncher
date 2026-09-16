use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::worlds::{self, BackupInfo, WorldInfo};
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
pub fn list_worlds(instance_id: String, state: State<'_, AppState>) -> AppResult<Vec<WorldInfo>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::list_worlds(&dir)
}

#[tauri::command]
pub fn backup_world(instance_id: String, folder: String, state: State<'_, AppState>) -> AppResult<BackupInfo> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::backup_world(&dir, &folder)
}

#[tauri::command]
pub fn restore_latest_backup(instance_id: String, folder: String, state: State<'_, AppState>) -> AppResult<()> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::restore_latest_backup(&dir, &folder)
}

#[tauri::command]
pub fn restore_backup(instance_id: String, folder: String, file: String, state: State<'_, AppState>) -> AppResult<()> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::restore_backup(&dir, &folder, &file)
}

#[tauri::command]
pub fn list_backups(instance_id: String, folder: String, state: State<'_, AppState>) -> AppResult<Vec<BackupInfo>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::list_backups(&dir, &folder)
}

#[tauri::command]
pub fn delete_backup(instance_id: String, file: String, state: State<'_, AppState>) -> AppResult<()> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::delete_backup(&dir, &file)
}

#[tauri::command]
pub fn delete_world(instance_id: String, folder: String, state: State<'_, AppState>) -> AppResult<()> {
    let dir = instance_dir_for(&state, &instance_id)?;
    worlds::delete_world(&dir, &folder)
}

#[tauri::command]
pub fn get_saves_dir(instance_id: String, state: State<'_, AppState>) -> AppResult<String> {
    let dir = instance_dir_for(&state, &instance_id)?;
    let saves = worlds::ensure_saves_dir(&dir)?;
    Ok(saves.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_world_dir(instance_id: String, folder: String, state: State<'_, AppState>) -> AppResult<String> {
    let dir = instance_dir_for(&state, &instance_id)?;
    Ok(dir.join("saves").join(&folder).to_string_lossy().to_string())
}
