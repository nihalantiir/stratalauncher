use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::servers::{self, ServerEntry, ServerStatus};
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
pub fn list_servers(instance_id: String, state: State<'_, AppState>) -> AppResult<Vec<ServerEntry>> {
    let dir = instance_dir_for(&state, &instance_id)?;
    servers::list_servers(&dir)
}

#[tauri::command]
pub async fn ping_server(address: String) -> AppResult<ServerStatus> {
    servers::ping_server(&address).await
}
