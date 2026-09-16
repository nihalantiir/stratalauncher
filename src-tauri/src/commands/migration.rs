use crate::db::models::{Instance, BIOME_KEYS};
use crate::db::InstancesRepo;
use crate::error::AppResult;
use crate::migration::{self, DetectedInstance, LauncherKind};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

const ALL_KINDS: [LauncherKind; 4] =
    [LauncherKind::Vanilla, LauncherKind::Curseforge, LauncherKind::Prism, LauncherKind::Multimc];

#[tauri::command]
pub fn scan_all_installs() -> Vec<DetectedInstance> {
    ALL_KINDS.iter().flat_map(|&kind| migration::scan(kind, None)).collect()
}

#[tauri::command]
pub fn scan_custom_folder(kind: LauncherKind, path: String) -> Vec<DetectedInstance> {
    migration::scan(kind, Some(Path::new(&path)))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub source_path: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub icon_biome: String,
}

#[tauri::command]
pub async fn import_instance(input: ImportRequest, state: State<'_, AppState>) -> AppResult<Instance> {
    let icon_biome = if BIOME_KEYS.contains(&input.icon_biome.as_str()) {
        input.icon_biome
    } else {
        BIOME_KEYS[0].to_string()
    };

    let instance = Instance {
        id: Uuid::new_v4().to_string(),
        name: input.name,
        loader: input.loader,
        loader_version: input.loader_version,
        mc_version: input.mc_version,
        memory_mb: None,
        jvm_args: None,
        java_path: None,
        icon_biome,
        group_name: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_played_at: None,
        last_crashed: None,
        min_memory_mb: None,
        window_width: None,
        window_height: None,
        window_maximized: false,
        skip_java_check: false,
        env_vars: None,
        pre_launch_cmd: None,
        wrapper_cmd: None,
        post_exit_cmd: None,
        console_mode: "never".to_string(),
        launcher_behavior: "keep_open".to_string(),
        quick_play_mode: "off".to_string(),
        quick_play_target: None,
        custom_client_jar: None,
    };

    let dest_dir = crate::paths::instance_dir(&instance.id);
    std::fs::create_dir_all(&dest_dir)?;

    let source_path = PathBuf::from(input.source_path);
    let dest_for_copy = dest_dir.clone();
    tokio::task::spawn_blocking(move || migration::import_content(&source_path, &dest_for_copy))
        .await
        .map_err(|e| crate::error::AppError::Other(format!("import task panicked: {e}")))??;

    let conn = state.db.0.lock().unwrap();
    InstancesRepo::insert(&conn, &instance)?;
    InstancesRepo::set_current_id(&conn, &instance.id)?;
    Ok(instance)
}
