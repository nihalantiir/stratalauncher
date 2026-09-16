mod auth;
mod cape_catalog;
mod commands;
mod config;
mod content;
mod db;
mod error;
mod fsutil;
mod launcher_log;
mod logs;
mod migration;
mod minecraft;
mod modpack;
mod paths;
mod screenshots;
mod servers;
mod state;
mod sync;
mod timeutil;
mod worlds;

use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Optional developer credentials can live in a .env next to the exe;
    // existing real environment variables still win over it.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = dotenvy::from_path(dir.join(".env"));
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::new()?;
            app.manage(state);

            // Screenshots render straight off disk via the asset protocol
            // instead of piping bytes through IPC.
            app.asset_protocol_scope().allow_directory(paths::instances_dir(), true)?;
            app.asset_protocol_scope().allow_directory(paths::media_dir(), true)?;

            // Always created, but only relevant when "hide to tray while
            // playing" is on; see commands::launch for show/hide.
            let show_item = MenuItem::with_id(app, "show", "Show Strata Launcher", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().ok_or("no default window icon")?)
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // One background version-sync pass per launch, feeds the
            // topbar's notification bell.
            let app_state = app.state::<AppState>();
            let client = app_state.http.clone();
            let (instances, check_updates) = {
                let conn = app_state.db.0.lock().unwrap();
                let settings = commands::app_settings::read_settings(&conn);
                (db::InstancesRepo::list(&conn).unwrap_or_default(), settings.auto_check_updates)
            };
            let startup_client = client.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = sync::run_sync_check(&startup_client, &instances, check_updates).await {
                    launcher_log::warn("sync", format!("startup sync check failed: {e}"));
                }
            });

            // Background videos aren't bundled into the exe; fetch them once,
            // silently, the first time they're missing.
            let app_for_media = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_for_media.state::<AppState>();
                if let Err(e) = commands::media::download_media(app_for_media.clone(), state).await {
                    launcher_log::warn("media", format!("background media download failed: {e}"));
                }
            });

            // Re-checks on the user's configured interval while Strata stays
            // open; polls every 15 minutes for whether it's actually elapsed.
            let app_for_updates = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
                ticker.tick().await; // the startup pass above already covers "right now"
                loop {
                    ticker.tick().await;
                    let app_state = app_for_updates.state::<AppState>();
                    let (auto_check, interval_hours) = {
                        let conn = app_state.db.0.lock().unwrap();
                        let s = commands::app_settings::read_settings(&conn);
                        (s.auto_check_updates, s.update_check_interval_hours)
                    };
                    if auto_check && sync::update_check_due(interval_hours) {
                        if let Err(e) = sync::check_launcher_update(&app_state.http).await {
                            launcher_log::warn("sync", format!("periodic update check failed: {e}"));
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::start_microsoft_login,
            commands::auth::cancel_microsoft_login,
            commands::auth::create_offline_profile,
            commands::auth::list_accounts,
            commands::auth::get_active_account,
            commands::auth::set_active_account,
            commands::auth::remove_account,
            commands::versions::list_versions,
            commands::launch::launch_instance,
            commands::launch::list_running_instances,
            commands::launch::stop_instance,
            commands::instances::list_instances,
            commands::instances::get_current_instance,
            commands::instances::set_current_instance,
            commands::instances::get_instance_dir,
            commands::instances::create_instance,
            commands::instances::update_instance,
            commands::instances::set_instance_loader,
            commands::instances::set_custom_client_jar,
            commands::instances::get_coremod_icon,
            commands::instances::delete_instance,
            commands::loaders::list_loader_versions,
            commands::loaders::check_loader_availability,
            commands::loaders::get_version_components,
            commands::java::get_required_java,
            commands::java::probe_java_at,
            commands::content::curseforge_configured,
            commands::content::list_installed_content,
            commands::content::toggle_content,
            commands::content::remove_content,
            commands::content::search_content,
            commands::content::install_content,
            commands::content::check_content_updates,
            commands::content::list_content_categories,
            commands::content::get_content_dir,
            commands::content::install_content_version,
            commands::content::list_content_versions,
            commands::modpack::search_modpacks,
            commands::modpack::list_modpack_categories,
            commands::modpack::install_modpack_version,
            commands::modpack::install_modpack_file,
            commands::worlds::list_worlds,
            commands::worlds::backup_world,
            commands::worlds::restore_latest_backup,
            commands::worlds::restore_backup,
            commands::worlds::list_backups,
            commands::worlds::delete_backup,
            commands::worlds::delete_world,
            commands::worlds::get_saves_dir,
            commands::worlds::get_world_dir,
            commands::servers::list_servers,
            commands::servers::ping_server,
            commands::screenshots::list_screenshots,
            commands::screenshots::delete_screenshot,
            commands::screenshots::get_screenshots_dir,
            commands::logs::list_log_files,
            commands::logs::read_minecraft_log,
            commands::logs::read_minecraft_log_raw,
            commands::logs::read_launcher_log,
            commands::logs::read_launcher_log_raw,
            commands::logs::get_logs_dir,
            commands::logs::get_launcher_logs_dir,
            commands::logs::upload_log_text,
            commands::logs::list_upload_targets,
            commands::logs::get_mclogs_insights,
            commands::migration::scan_all_installs,
            commands::migration::scan_custom_folder,
            commands::migration::import_instance,
            commands::skins::get_default_skin,
            commands::skins::get_skin_profile,
            commands::skins::upload_skin,
            commands::skins::reset_skin,
            commands::skins::equip_cape,
            commands::skins::unequip_cape,
            commands::skins::set_skin_variant,
            commands::sync::list_notifications,
            commands::sync::dismiss_notification,
            commands::sync::run_sync_check,
            commands::app_settings::get_app_settings,
            commands::app_settings::update_app_settings,
            commands::app_settings::get_data_dir_info,
            commands::app_settings::set_pending_data_dir,
            commands::app_settings::reset_data_dir,
            commands::app_settings::restart_app,
            commands::opener::open_in_explorer,
            commands::capes::get_cape_catalog,
            commands::updater::check_for_update,
            commands::updater::install_update,
            commands::media::media_status,
            commands::media::download_media,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
