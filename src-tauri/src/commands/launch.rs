use crate::auth;
use crate::db::models::{Account, AccountKind};
use crate::db::{AccountsRepo, InstancesRepo};
use crate::error::{AppError, AppResult};
use crate::minecraft::{self, launch::LaunchSession, LaunchRequest};
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

const RUNNING_CHANGED_EVENT: &str = "instance://running-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunningChanged {
    instance_id: String,
    running: bool,
    /// Only meaningful when `running` is false: whether the session that
    /// just ended exited cleanly.
    crashed: Option<bool>,
}

#[tauri::command]
pub fn list_running_instances(state: State<'_, AppState>) -> Vec<String> {
    state.running.lock().unwrap().keys().cloned().collect()
}

#[cfg(windows)]
fn kill_pid(pid: u32) -> std::io::Result<()> {
    std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F", "/T"])
        .status()
        .map(|_| ())
}

#[cfg(not(windows))]
fn kill_pid(pid: u32) -> std::io::Result<()> {
    std::process::Command::new("kill").arg("-9").arg(pid.to_string()).status().map(|_| ())
}

#[tauri::command]
pub fn stop_instance(instance_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let pid = { state.running.lock().unwrap().get(&instance_id).copied() };
    let Some(pid) = pid else {
        return Ok(()); // already stopped; nothing to do
    };
    kill_pid(pid).map_err(|e| AppError::Other(format!("Couldn't stop the game: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn launch_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> AppResult<()> {
    let (instance, account, global) = {
        let conn = state.db.0.lock().unwrap();
        let instance = InstancesRepo::get(&conn, &instance_id)?
            .ok_or_else(|| AppError::NotFound(format!("instance {instance_id}")))?;
        let account = AccountsRepo::active(&conn)?
            .ok_or_else(|| AppError::Auth("Sign in or choose offline mode before playing.".into()))?;
        let global = crate::commands::app_settings::read_settings(&conn);
        (instance, account, global)
    };

    let session = match account.kind {
        AccountKind::Microsoft => {
            let (mc_session, profile) = auth::ensure_live_session(&state, &account.id).await?;
            let active_skin = profile.skins.iter().find(|s| s.state == "ACTIVE");

            let updated = Account {
                username: profile.name.clone(),
                skin_url: active_skin.map(|s| s.url.clone()).or(account.skin_url.clone()),
                skin_variant: active_skin.map(|s| s.variant.to_lowercase()).or(account.skin_variant.clone()),
                ..account
            };
            {
                let conn = state.db.0.lock().unwrap();
                AccountsRepo::upsert(&conn, &updated)?;
            }

            LaunchSession {
                username: profile.name,
                uuid: profile.id.replace('-', ""),
                access_token: mc_session.access_token,
                user_type: "msa",
            }
        }
        AccountKind::Offline => LaunchSession {
            username: account.username.clone(),
            uuid: account.id.replace('-', ""),
            access_token: "0".into(),
            user_type: "legacy",
        },
    };

    let extra_jvm_args = instance
        .jvm_args
        .as_deref()
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();

    let req = LaunchRequest {
        instance_id: instance.id.clone(),
        instance_name: instance.name.clone(),
        mc_version: instance.mc_version.clone(),
        loader: instance.loader.clone(),
        loader_version: instance.loader_version.clone(),
        custom_client_jar: instance.custom_client_jar.as_deref().map(PathBuf::from),
        game_dir: crate::paths::instance_dir(&instance.id),
        memory_mb: instance.memory_mb,
        min_memory_mb: instance.min_memory_mb,
        extra_jvm_args,
        java_path: instance.java_path.as_deref().map(PathBuf::from),
        skip_java_check: instance.skip_java_check,
        window_width: instance.window_width,
        window_height: instance.window_height,
        window_maximized: instance.window_maximized,
        env_vars: instance.env_vars.clone(),
        pre_launch_cmd: instance.pre_launch_cmd.clone(),
        wrapper_cmd: instance.wrapper_cmd.clone(),
        post_exit_cmd: instance.post_exit_cmd.clone(),
        quick_play_mode: instance.quick_play_mode.clone(),
        quick_play_target: instance.quick_play_target.clone(),
        session,
        global_window_width: global.window_width,
        global_window_height: global.window_height,
        global_window_maximized: global.window_maximized,
        global_memory_mb: global.default_memory_mb,
        global_min_memory_mb: global.default_min_memory_mb,
        global_jvm_args: global.jvm_args.clone(),
        global_env_vars: global.env_vars.clone(),
        global_pre_launch_cmd: global.pre_launch_cmd.clone(),
        global_wrapper_cmd: global.wrapper_cmd.clone(),
        global_post_exit_cmd: global.post_exit_cmd.clone(),
    };

    // A global preference: "while the game is running" is naturally
    // app-wide, not worth setting per instance.
    let launcher_behavior = global.launcher_behavior.clone();
    let outcome = minecraft::prepare_and_launch(&app, &state.http, req).await?;
    let mut child = outcome.child;
    let post_exit_cmds = outcome.post_exit_cmds;
    let pid = child.id();

    {
        let conn = state.db.0.lock().unwrap();
        InstancesRepo::touch_last_played(&conn, &instance.id)?;
    }

    if let Some(pid) = pid {
        state.running.lock().unwrap().insert(instance.id.clone(), pid);
    }
    app.emit(
        RUNNING_CHANGED_EVENT,
        RunningChanged { instance_id: instance.id.clone(), running: true, crashed: None },
    )
    .ok();

    match launcher_behavior.as_str() {
        "close_on_launch" => {
            // Quits the whole process, so post-exit commands/crash tracking
            // are a known, accepted tradeoff of this option.
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.close();
            }
        }
        "hide_to_tray" => {
            // The tray icon brings it back, auto-restored once the game exits.
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.hide();
            }
        }
        _ => {}
    }

    // Detach: reap the process on exit so it doesn't linger as a zombie,
    // without blocking this command on the game actually closing.
    let instance_name = instance.name.clone();
    let instance_id = instance.id.clone();
    let game_dir = crate::paths::instance_dir(&instance.id);
    let app_for_task = app.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let crashed = !status.as_ref().map(|s| s.success()).unwrap_or(false);
        crate::launcher_log::info("launch", format!("Instance '{instance_name}' exited: {status:?}"));

        for cmd_str in post_exit_cmds {
            crate::launcher_log::info("launch", format!("Running post-exit command: {cmd_str}"));
            if let Err(e) = minecraft::run_shell_command(&cmd_str, &game_dir).await {
                crate::launcher_log::error("launch", format!("Post-exit command failed: {e}"));
            }
        }

        if launcher_behavior == "hide_to_tray" {
            if let Some(win) = app_for_task.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }

        let app_state = app_for_task.state::<AppState>();
        app_state.running.lock().unwrap().remove(&instance_id);
        {
            let conn = app_state.db.0.lock().unwrap();
            let _ = InstancesRepo::set_last_crashed(&conn, &instance_id, crashed);
        }
        app_for_task
            .emit(RUNNING_CHANGED_EVENT, RunningChanged { instance_id, running: false, crashed: Some(crashed) })
            .ok();

        if launcher_behavior == "quit_on_exit" {
            app_for_task.exit(0);
        }
    });

    Ok(())
}
