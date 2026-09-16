//! Global (app-wide) settings: shared defaults an instance's own settings
//! compose with, plus launcher-lifecycle and update-check preferences.

use crate::error::AppResult;
use crate::state::AppState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub window_maximized: bool,
    pub default_memory_mb: Option<u32>,
    pub default_min_memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub skip_java_check_default: bool,
    pub env_vars: Option<String>,
    pub pre_launch_cmd: Option<String>,
    pub wrapper_cmd: Option<String>,
    pub post_exit_cmd: Option<String>,
    /// "keep_open" | "close_on_launch" | "quit_on_exit" | "hide_to_tray".
    pub launcher_behavior: String,
    pub auto_check_updates: bool,
    pub update_check_interval_hours: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window_width: None,
            window_height: None,
            window_maximized: false,
            default_memory_mb: None,
            default_min_memory_mb: None,
            jvm_args: None,
            skip_java_check_default: false,
            env_vars: None,
            pre_launch_cmd: None,
            wrapper_cmd: None,
            post_exit_cmd: None,
            launcher_behavior: "keep_open".into(),
            auto_check_updates: true,
            update_check_interval_hours: 24,
        }
    }
}

fn get_str(conn: &Connection, key: &str) -> Option<String> {
    crate::db::kv_get(conn, key).ok().flatten().filter(|s| !s.is_empty())
}
fn get_u32(conn: &Connection, key: &str) -> Option<u32> {
    get_str(conn, key).and_then(|s| s.parse().ok())
}
fn get_bool(conn: &Connection, key: &str, default: bool) -> bool {
    get_str(conn, key).map(|s| s == "1").unwrap_or(default)
}

pub fn read_settings(conn: &Connection) -> AppSettings {
    let d = AppSettings::default();
    AppSettings {
        window_width: get_u32(conn, "app_window_width"),
        window_height: get_u32(conn, "app_window_height"),
        window_maximized: get_bool(conn, "app_window_maximized", d.window_maximized),
        default_memory_mb: get_u32(conn, "app_default_memory_mb"),
        default_min_memory_mb: get_u32(conn, "app_default_min_memory_mb"),
        jvm_args: get_str(conn, "app_jvm_args"),
        skip_java_check_default: get_bool(conn, "app_skip_java_check_default", d.skip_java_check_default),
        env_vars: get_str(conn, "app_env_vars"),
        pre_launch_cmd: get_str(conn, "app_pre_launch_cmd"),
        wrapper_cmd: get_str(conn, "app_wrapper_cmd"),
        post_exit_cmd: get_str(conn, "app_post_exit_cmd"),
        launcher_behavior: get_str(conn, "app_launcher_behavior").unwrap_or(d.launcher_behavior),
        auto_check_updates: get_bool(conn, "app_auto_check_updates", d.auto_check_updates),
        update_check_interval_hours: get_u32(conn, "app_update_check_interval_hours").unwrap_or(d.update_check_interval_hours),
    }
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    let conn = state.db.0.lock().unwrap();
    read_settings(&conn)
}

#[tauri::command]
pub fn update_app_settings(input: AppSettings, state: State<'_, AppState>) -> AppResult<AppSettings> {
    if !["keep_open", "close_on_launch", "quit_on_exit", "hide_to_tray"].contains(&input.launcher_behavior.as_str()) {
        return Err(crate::error::AppError::Other(format!("Unknown launcher behavior: {}", input.launcher_behavior)));
    }
    if let (Some(min), Some(max)) = (input.default_min_memory_mb, input.default_memory_mb) {
        if min > max {
            return Err(crate::error::AppError::Other("Minimum memory can't be higher than maximum.".into()));
        }
    }
    let conn = state.db.0.lock().unwrap();
    crate::db::kv_set(&conn, "app_window_width", &input.window_width.map(|v| v.to_string()).unwrap_or_default())?;
    crate::db::kv_set(&conn, "app_window_height", &input.window_height.map(|v| v.to_string()).unwrap_or_default())?;
    crate::db::kv_set(&conn, "app_window_maximized", if input.window_maximized { "1" } else { "0" })?;
    crate::db::kv_set(&conn, "app_default_memory_mb", &input.default_memory_mb.map(|v| v.to_string()).unwrap_or_default())?;
    crate::db::kv_set(&conn, "app_default_min_memory_mb", &input.default_min_memory_mb.map(|v| v.to_string()).unwrap_or_default())?;
    crate::db::kv_set(&conn, "app_jvm_args", input.jvm_args.as_deref().unwrap_or(""))?;
    crate::db::kv_set(&conn, "app_skip_java_check_default", if input.skip_java_check_default { "1" } else { "0" })?;
    crate::db::kv_set(&conn, "app_env_vars", input.env_vars.as_deref().unwrap_or(""))?;
    crate::db::kv_set(&conn, "app_pre_launch_cmd", input.pre_launch_cmd.as_deref().unwrap_or(""))?;
    crate::db::kv_set(&conn, "app_wrapper_cmd", input.wrapper_cmd.as_deref().unwrap_or(""))?;
    crate::db::kv_set(&conn, "app_post_exit_cmd", input.post_exit_cmd.as_deref().unwrap_or(""))?;
    crate::db::kv_set(&conn, "app_launcher_behavior", &input.launcher_behavior)?;
    crate::db::kv_set(&conn, "app_auto_check_updates", if input.auto_check_updates { "1" } else { "0" })?;
    crate::db::kv_set(&conn, "app_update_check_interval_hours", &input.update_check_interval_hours.to_string())?;
    Ok(read_settings(&conn))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirInfo {
    pub path: String,
    pub is_custom: bool,
    /// A folder queued to move into on the next launch, if one hasn't been
    /// applied yet this session.
    pub pending_path: Option<String>,
}

#[tauri::command]
pub fn get_data_dir_info() -> DataDirInfo {
    DataDirInfo {
        path: crate::paths::data_dir().display().to_string(),
        is_custom: crate::paths::is_custom_data_dir(),
        pending_path: crate::paths::pending_data_dir().map(|p| p.display().to_string()),
    }
}

/// Only records the choice; the actual move happens at the start of the
/// next launch, before anything has either folder open. See `paths::data_dir`.
#[tauri::command]
pub fn set_pending_data_dir(path: String) -> AppResult<()> {
    crate::paths::set_pending_data_dir(std::path::PathBuf::from(path)).map_err(crate::error::AppError::Io)
}

#[tauri::command]
pub fn reset_data_dir() -> AppResult<()> {
    crate::paths::clear_pending_data_dir().map_err(crate::error::AppError::Io)
}
