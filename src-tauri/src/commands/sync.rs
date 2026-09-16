use crate::error::AppResult;
use crate::state::AppState;
use crate::sync::{self, Notification};
use tauri::State;

#[tauri::command]
pub fn list_notifications() -> Vec<Notification> {
    sync::list_notifications()
}

#[tauri::command]
pub fn dismiss_notification(id: String) -> AppResult<()> {
    sync::dismiss_notification(&id)
}

#[tauri::command]
pub async fn run_sync_check(state: State<'_, AppState>) -> AppResult<Vec<Notification>> {
    let (instances, check_updates) = {
        let conn = state.db.0.lock().unwrap();
        (
            crate::db::InstancesRepo::list(&conn)?,
            crate::commands::app_settings::read_settings(&conn).auto_check_updates,
        )
    };
    sync::run_sync_check(&state.http, &instances, check_updates).await?;
    Ok(sync::list_notifications())
}
