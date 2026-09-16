use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::logs::{self, LogFileMeta, LogInsights, LogLine, LogUploadTarget};
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
pub fn list_log_files(instance_id: String, state: State<'_, AppState>) -> AppResult<Vec<LogFileMeta>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    logs::list_log_files(&dir.join("logs"))
}

#[tauri::command]
pub fn read_minecraft_log(instance_id: String, filename: String, state: State<'_, AppState>) -> AppResult<Vec<LogLine>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    let text = logs::read_log_text(&dir.join("logs"), &filename)?;
    Ok(logs::parse_log_lines(&text))
}

#[tauri::command]
pub fn read_minecraft_log_raw(instance_id: String, filename: String, state: State<'_, AppState>) -> AppResult<String> {
    let dir = instance_dir_for(&state, &instance_id)?;
    logs::read_log_text(&dir.join("logs"), &filename)
}

#[tauri::command]
pub fn list_launcher_log_files() -> AppResult<Vec<LogFileMeta>> {
    logs::list_log_files(&crate::launcher_log::logs_dir())
}

#[tauri::command]
pub fn read_launcher_log(filename: String) -> AppResult<Vec<LogLine>> {
    let text = logs::read_log_text(&crate::launcher_log::logs_dir(), &filename)?;
    Ok(logs::parse_log_lines(&text))
}

#[tauri::command]
pub fn read_launcher_log_raw(filename: String) -> AppResult<String> {
    logs::read_log_text(&crate::launcher_log::logs_dir(), &filename)
}

#[tauri::command]
pub fn get_logs_dir(instance_id: String, state: State<'_, AppState>) -> AppResult<String> {
    let dir = instance_dir_for(&state, &instance_id)?;
    let logs_path = dir.join("logs");
    std::fs::create_dir_all(&logs_path)?;
    Ok(logs_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_launcher_logs_dir() -> AppResult<String> {
    let dir = crate::launcher_log::logs_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn upload_log_text(state: State<'_, AppState>, content: String, target: LogUploadTarget) -> AppResult<String> {
    logs::upload_log(&state.http, &content, target).await
}

#[tauri::command]
pub fn list_upload_targets() -> Vec<LogUploadTarget> {
    logs::available_upload_targets()
}

#[tauri::command]
pub async fn get_mclogs_insights(state: State<'_, AppState>, id: String) -> AppResult<LogInsights> {
    logs::fetch_mclogs_insights(&state.http, &id).await
}
