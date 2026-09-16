use crate::db::models::{Instance, BIOME_KEYS};
use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

fn resolve_biome(requested: &str, fallback: &str) -> String {
    if BIOME_KEYS.contains(&requested) {
        requested.to_string()
    } else {
        fallback.to_string()
    }
}

#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> AppResult<Vec<Instance>> {
    let conn = state.db.0.lock().unwrap();
    InstancesRepo::list(&conn)
}

#[tauri::command]
pub fn get_current_instance(state: State<'_, AppState>) -> AppResult<Option<Instance>> {
    let conn = state.db.0.lock().unwrap();
    let Some(id) = InstancesRepo::current_id(&conn)? else {
        return Ok(None);
    };
    InstancesRepo::get(&conn, &id)
}

#[tauri::command]
pub fn get_instance_dir(id: String, state: State<'_, AppState>) -> AppResult<String> {
    let conn = state.db.0.lock().unwrap();
    if InstancesRepo::get(&conn, &id)?.is_none() {
        return Err(AppError::NotFound(format!("instance {id}")));
    }
    Ok(crate::paths::instance_dir(&id).to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_current_instance(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.0.lock().unwrap();
    if InstancesRepo::get(&conn, &id)?.is_none() {
        return Err(AppError::NotFound(format!("instance {id}")));
    }
    InstancesRepo::set_current_id(&conn, &id)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInstance {
    pub name: String,
    pub mc_version: String,
    pub icon_biome: String,
    /// Both optional so simple callers can omit them for a plain vanilla
    /// instance; the Create Instance modal's loader picker sets these directly.
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
}

#[tauri::command]
pub fn create_instance(input: NewInstance, state: State<'_, AppState>) -> AppResult<Instance> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Other("Instance name can't be empty.".into()));
    }
    let loader = input.loader.unwrap_or_else(|| "vanilla".to_string());
    if !SUPPORTED_LOADERS.contains(&loader.as_str()) {
        return Err(AppError::Other(format!("unsupported loader: {loader}")));
    }
    let loader_version = if loader == "vanilla" { None } else { input.loader_version };

    // New instances inherit the global Settings defaults at launch time;
    // only skip-check's default needs copying here (no "unset" state).
    let skip_java_check_default = {
        let conn = state.db.0.lock().unwrap();
        crate::commands::app_settings::read_settings(&conn).skip_java_check_default
    };

    let instance = Instance {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        loader,
        loader_version,
        mc_version: input.mc_version,
        memory_mb: None,
        jvm_args: None,
        java_path: None,
        icon_biome: resolve_biome(&input.icon_biome, BIOME_KEYS[0]),
        group_name: input.group_name.map(|g| g.trim().to_string()).filter(|g| !g.is_empty()),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_played_at: None,
        last_crashed: None,
        min_memory_mb: None,
        window_width: None,
        window_height: None,
        window_maximized: false,
        skip_java_check: skip_java_check_default,
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

    let conn = state.db.0.lock().unwrap();
    InstancesRepo::insert(&conn, &instance)?;
    InstancesRepo::set_current_id(&conn, &instance.id)?;
    Ok(instance)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub memory_mb: Option<u32>,
    pub min_memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub java_path: Option<String>,
    pub icon_biome: String,
    pub group_name: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub window_maximized: bool,
    pub skip_java_check: bool,
    pub env_vars: Option<String>,
    pub pre_launch_cmd: Option<String>,
    pub wrapper_cmd: Option<String>,
    pub post_exit_cmd: Option<String>,
    pub console_mode: String,
    pub quick_play_mode: String,
    pub quick_play_target: Option<String>,
}

#[tauri::command]
pub fn update_instance(input: InstanceUpdate, state: State<'_, AppState>) -> AppResult<Instance> {
    let conn = state.db.0.lock().unwrap();
    let mut instance = InstancesRepo::get(&conn, &input.id)?
        .ok_or_else(|| AppError::NotFound(format!("instance {}", input.id)))?;

    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Other("Instance name can't be empty.".into()));
    }
    if !["always", "on_crash", "never"].contains(&input.console_mode.as_str()) {
        return Err(AppError::Other(format!("Unknown console mode: {}", input.console_mode)));
    }
    if !["off", "world", "server"].contains(&input.quick_play_mode.as_str()) {
        return Err(AppError::Other(format!("Unknown quick play mode: {}", input.quick_play_mode)));
    }

    instance.name = name.to_string();
    instance.mc_version = input.mc_version;
    instance.memory_mb = input.memory_mb;
    instance.min_memory_mb = input.min_memory_mb;
    instance.jvm_args = input.jvm_args.filter(|s| !s.trim().is_empty());
    instance.java_path = input.java_path.filter(|s| !s.trim().is_empty());
    instance.icon_biome = resolve_biome(&input.icon_biome, &instance.icon_biome);
    instance.group_name = input.group_name.filter(|s| !s.trim().is_empty());
    instance.window_width = input.window_width;
    instance.window_height = input.window_height;
    instance.window_maximized = input.window_maximized;
    instance.skip_java_check = input.skip_java_check;
    instance.env_vars = input.env_vars.filter(|s| !s.trim().is_empty());
    instance.pre_launch_cmd = input.pre_launch_cmd.filter(|s| !s.trim().is_empty());
    instance.wrapper_cmd = input.wrapper_cmd.filter(|s| !s.trim().is_empty());
    instance.post_exit_cmd = input.post_exit_cmd.filter(|s| !s.trim().is_empty());
    instance.console_mode = input.console_mode;
    // launcher_behavior is intentionally left untouched: it's now a global
    // (Settings) preference, see commands::launch.
    instance.quick_play_mode = input.quick_play_mode;
    instance.quick_play_target = input.quick_play_target.filter(|s| !s.trim().is_empty());

    InstancesRepo::update(&conn, &instance)?;
    Ok(instance)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderUpdate {
    pub id: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub mc_version: String,
}

const SUPPORTED_LOADERS: &[&str] = &["vanilla", "fabric", "quilt", "forge", "neoforge"];

#[tauri::command]
pub fn set_instance_loader(input: LoaderUpdate, state: State<'_, AppState>) -> AppResult<Instance> {
    if !SUPPORTED_LOADERS.contains(&input.loader.as_str()) {
        return Err(AppError::Other(format!("unsupported loader: {}", input.loader)));
    }

    let conn = state.db.0.lock().unwrap();
    let mut instance = InstancesRepo::get(&conn, &input.id)?
        .ok_or_else(|| AppError::NotFound(format!("instance {}", input.id)))?;

    // Switching loader or Minecraft version can break existing saves, so
    // back every world up first to make the switch safely reversible.
    let risky = instance.loader != input.loader || instance.mc_version != input.mc_version;
    if risky {
        crate::launcher_log::info(
            "loader",
            format!(
                "Switching '{}' from {} {} to {} {}, backing up worlds first",
                instance.name, instance.loader, instance.mc_version, input.loader, input.mc_version
            ),
        );
        crate::worlds::backup_all_worlds(&crate::paths::instance_dir(&instance.id))?;
    }

    instance.loader = input.loader;
    instance.loader_version = input.loader_version;
    instance.mc_version = input.mc_version;

    InstancesRepo::update(&conn, &instance)?;
    Ok(instance)
}

/// The "Coremods" jar override on the Version page; `path: None` reverts to
/// the normal vanilla jar. Existence is checked here, not deferred to launch.
#[tauri::command]
pub fn set_custom_client_jar(id: String, path: Option<String>, state: State<'_, AppState>) -> AppResult<Instance> {
    if let Some(p) = &path {
        if !std::path::Path::new(p).is_file() {
            return Err(AppError::Other(format!("{p} doesn't exist or isn't a file.")));
        }
    }

    let conn = state.db.0.lock().unwrap();
    let mut instance = InstancesRepo::get(&conn, &id)?.ok_or_else(|| AppError::NotFound(format!("instance {id}")))?;
    instance.custom_client_jar = path;
    InstancesRepo::update(&conn, &instance)?;
    Ok(instance)
}

/// A coremod jar's icon lives at a conventional root-level `icon/{size}.png`
/// (same as vanilla); `None`, not an error, when nothing matches.
#[tauri::command]
pub fn get_coremod_icon(path: String) -> AppResult<Option<Vec<u8>>> {
    let file = std::fs::File::open(&path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::Other(format!("bad jar: {e}")))?;
    for candidate in ["icon/64.png", "icon/128.png", "icon/32.png", "icon/256.png", "icon/16.png", "pack.png"] {
        if let Ok(mut entry) = archive.by_name(candidate) {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes)?;
            return Ok(Some(bytes));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn delete_instance(id: String, state: State<'_, AppState>) -> AppResult<()> {
    {
        let conn = state.db.0.lock().unwrap();
        InstancesRepo::delete(&conn, &id)?;
        if InstancesRepo::current_id(&conn)?.as_deref() == Some(id.as_str()) {
            if let Some(next) = InstancesRepo::list(&conn)?.first() {
                InstancesRepo::set_current_id(&conn, &next.id)?;
            }
        }
    }

    let dir = crate::paths::instance_dir(&id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).ok();
    }
    Ok(())
}
