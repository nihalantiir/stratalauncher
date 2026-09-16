pub mod download;
pub mod java_runtime;
pub mod launch;
pub mod loaders;
pub mod lwjgl3;
pub mod manifest;
mod merge;
pub mod rules;

use crate::error::AppResult;
use crate::paths;
use launch::{LaunchOptions, LaunchSession};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tokio::process::Child;

pub struct LaunchRequest {
    pub instance_id: String,
    pub instance_name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    /// The "Coremods" jar override from the Version page, replacing the
    /// normal vanilla client jar entirely (e.g. a total-conversion mod).
    pub custom_client_jar: Option<PathBuf>,
    pub game_dir: PathBuf,
    pub memory_mb: Option<u32>,
    pub min_memory_mb: Option<u32>,
    pub extra_jvm_args: Vec<String>,
    /// Overrides the auto-detected Java runtime, when the instance pins one.
    pub java_path: Option<PathBuf>,
    pub skip_java_check: bool,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub window_maximized: bool,
    pub env_vars: Option<String>,
    pub pre_launch_cmd: Option<String>,
    pub wrapper_cmd: Option<String>,
    pub post_exit_cmd: Option<String>,
    pub quick_play_mode: String,
    pub quick_play_target: Option<String>,
    pub session: LaunchSession,
    /// Global (Settings) fallbacks/companions for the fields above, see
    /// `merge_env_vars` for how they combine.
    pub global_window_width: Option<u32>,
    pub global_window_height: Option<u32>,
    pub global_window_maximized: bool,
    pub global_memory_mb: Option<u32>,
    pub global_min_memory_mb: Option<u32>,
    pub global_jvm_args: Option<String>,
    pub global_env_vars: Option<String>,
    pub global_pre_launch_cmd: Option<String>,
    pub global_wrapper_cmd: Option<String>,
    pub global_post_exit_cmd: Option<String>,
}

/// The result of a successful launch; `post_exit_cmd` is already fully
/// variable-expanded so callers don't need java-path context again.
pub struct LaunchOutcome {
    pub child: Child,
    pub post_exit_cmds: Vec<String>,
}

const DEFAULT_WINDOW_WIDTH: u32 = 925;
const DEFAULT_WINDOW_HEIGHT: u32 = 530;
const DEFAULT_MEMORY_MB: u32 = 2048;

fn parse_env_vars(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Global env vars first, then the instance's own; an instance-level
/// `KEY=` wins over the same key set globally.
fn merge_env_vars(global: Option<&str>, instance: Option<&str>) -> Vec<(String, String)> {
    let mut merged: Vec<(String, String)> = Vec::new();
    for (k, v) in global.map(parse_env_vars).unwrap_or_default() {
        merged.push((k, v));
    }
    for (k, v) in instance.map(parse_env_vars).unwrap_or_default() {
        if let Some(existing) = merged.iter_mut().find(|(ek, _)| *ek == k) {
            existing.1 = v;
        } else {
            merged.push((k, v));
        }
    }
    merged
}

/// Expands the real Prism-style launcher variables in a pre-launch/post-exit
/// command string (not the wrapper command, which is a program name, not a template).
fn expand_command_vars(template: &str, instance_id: &str, instance_name: &str, game_dir: &Path, java_path: &Path, jvm_args: &str) -> String {
    template
        .replace("$INST_NAME", instance_name)
        .replace("$INST_ID", instance_id)
        .replace("$INST_DIR", &game_dir.display().to_string())
        .replace("$INST_MC_DIR", &game_dir.display().to_string())
        .replace("$INST_JAVA", &java_path.display().to_string())
        .replace("$INST_JAVA_ARGS", jvm_args)
}

/// Runs a user-supplied command string through the platform shell; same
/// trust model as Prism's pre-launch/post-exit/wrapper commands.
pub async fn run_shell_command(cmd_str: &str, cwd: &Path) -> AppResult<std::process::ExitStatus> {
    let mut cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", cmd_str]);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", cmd_str]);
        c
    };
    cmd.current_dir(cwd);
    cmd.status().await.map_err(|e| crate::error::AppError::Launch(format!("Couldn't run command: {e}")))
}

/// Reads a jar's own `Main-Class` out of its `META-INF/MANIFEST.MF`.
/// `None` on any failure just falls back to the version JSON's `mainClass`.
fn read_jar_main_class(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut manifest = archive.by_name("META-INF/MANIFEST.MF").ok()?;
    let mut text = String::new();
    std::io::Read::read_to_string(&mut manifest, &mut text).ok()?;
    text.lines().find_map(|line| line.strip_prefix("Main-Class:")).map(|v| v.trim().to_string())
}

pub async fn prepare_and_launch(
    app: &AppHandle,
    client: &reqwest::Client,
    req: LaunchRequest,
) -> AppResult<LaunchOutcome> {
    let mc_version = req.mc_version.as_str();
    crate::launcher_log::info(
        "launch",
        format!("Preparing to launch Minecraft {mc_version} ({})", req.loader),
    );
    let version_manifest = manifest::fetch_manifest(client).await?;
    let entry = version_manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| crate::error::AppError::NotFound(format!("version {mc_version}")))?;

    let mut version_json = manifest::fetch_version_json(client, &entry.url).await?;

    let effective_version_id = if req.loader == "vanilla" {
        mc_version.to_string()
    } else if req.loader == "forge" || req.loader == "neoforge" {
        let loader_version = req.loader_version.as_deref().ok_or_else(|| {
            crate::error::AppError::Other(format!("{} loader version is not set", req.loader))
        })?;
        let effective_id = loaders::forge::effective_version_id(&req.loader, mc_version, loader_version);
        let profile = loaders::forge::ensure_installed(client, &req.loader, mc_version, loader_version).await?;
        version_json = merge::merge_inherited(version_json, profile);
        effective_id
    } else {
        let loader_version = req.loader_version.as_deref().ok_or_else(|| {
            crate::error::AppError::Other(format!("{} loader version is not set", req.loader))
        })?;
        let profile = loaders::fetch_profile_json(client, &req.loader, mc_version, loader_version).await?;
        version_json = merge::merge_inherited(version_json, profile);
        format!("{}-loader-{loader_version}-{mc_version}", req.loader)
    };

    let version_dir = paths::versions_dir().join(&effective_version_id);
    std::fs::create_dir_all(&version_dir)?;
    let version_json_path = version_dir.join(format!("{effective_version_id}.json"));
    tokio::fs::write(&version_json_path, serde_json::to_vec_pretty(&version_json)?).await?;

    // The client jar is cached under the vanilla version id unless a
    // Coremods override is set (see LaunchRequest), used as-is instead.

    // Set when a Coremods override turns out to be a "jarmod": the real
    // vanilla jar it layers over, added behind it on the classpath.
    let mut jarmod_base_jar: Option<PathBuf> = None;

    let client_jar = match &req.custom_client_jar {
        Some(custom) => {
            if !custom.is_file() {
                return Err(crate::error::AppError::Other(format!(
                    "The custom client jar {} no longer exists.",
                    custom.display()
                )));
            }
            crate::launcher_log::info("launch", format!("Using custom client jar: {}", custom.display()));

            // Pre-1.6 launchwrapper's default tweaker crashes on a non-vanilla
            // jar's bytecode; bypass it via the coremod jar's own Main-Class.
            if let Some(main_class) = read_jar_main_class(custom) {
                crate::launcher_log::info("launch", format!("Coremod jar's own main class: {main_class} (bypassing launchwrapper)"));
                version_json["mainClass"] = serde_json::Value::String(main_class);
            }

            // Some coremod jars replace LWJGL 2 with a GLFW-backed LWJGL 3
            // shim under the same package names (see the `lwjgl3` module).
            if lwjgl3::coremod_needs_lwjgl3(custom) {
                crate::launcher_log::info(
                    "launch",
                    "Coremod jar needs LWJGL 3 (GLFW-backed), not vanilla's LWJGL 2; swapping in real LWJGL 3.3.3 libraries",
                );
                if let Some(libs) = version_json["libraries"].as_array_mut() {
                    libs.retain(|lib| {
                        !lib["name"]
                            .as_str()
                            .map(|name| {
                                let group = name.split(':').next().unwrap_or("");
                                matches!(group, "org.lwjgl" | "lwjgl" | "org.lwjgl.lwjgl")
                            })
                            .unwrap_or(false)
                    });
                    libs.extend(lwjgl3::lwjgl3_libraries());
                }
                if version_json.get("minecraftArguments").is_some() {
                    version_json["minecraftArguments"] = serde_json::Value::String(
                        "--username ${auth_player_name} --session ${auth_session} --gameDir ${game_directory} --uuid ${auth_uuid}".into(),
                    );
                }
                // A from-scratch LWJGL3 rewrite has no tie to old Java 8
                // quirks, so pin a modern runtime instead of vanilla's jre-legacy.
                version_json["javaVersion"] = serde_json::json!({ "component": "java-runtime-delta", "majorVersion": 21 });
            }

            // Some coremod jars are "jarmods" meant to layer over a real
            // vanilla jar, not replace it; add vanilla behind it on the classpath.
            let vanilla_dir = paths::versions_dir().join(mc_version);
            std::fs::create_dir_all(&vanilla_dir)?;
            let vanilla_jar = vanilla_dir.join(format!("{mc_version}.jar"));
            download::ensure_client_jar(client, &version_json, &vanilla_jar).await?;
            jarmod_base_jar = Some(vanilla_jar);

            custom.clone()
        }
        None => {
            let vanilla_dir = paths::versions_dir().join(mc_version);
            std::fs::create_dir_all(&vanilla_dir)?;
            let client_jar = vanilla_dir.join(format!("{mc_version}.jar"));
            download::ensure_client_jar(client, &version_json, &client_jar).await?;
            client_jar
        }
    };

    let mut libs = download::ensure_libraries(app, client, &version_json, &paths::libraries_dir()).await?;
    if let Some(base_jar) = jarmod_base_jar {
        libs.classpath.push(base_jar);
    }

    let natives_dir = version_dir.join("natives");
    download::extract_natives(&libs.native_jars, &natives_dir)?;

    download::ensure_assets(app, client, &version_json, &paths::assets_dir()).await?;

    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or_else(|| crate::error::AppError::Other("version JSON has no mainClass".into()))?;
    let asset_index_id = version_json
        .pointer("/assetIndex/id")
        .and_then(|v| v.as_str())
        .unwrap_or("legacy");
    let version_type = version_json["type"].as_str().unwrap_or("release");

    let component = java_runtime::required_component(&version_json);
    let (java_path, java_major_version) = match req.java_path {
        Some(p) => {
            // A manual Java override still must match what this version
            // needs, unless the instance explicitly skips the check.
            let actual_major = launch::probe_java_major_version(&p)?;
            if let Some(required_major) = java_runtime::component_major_version(component) {
                if actual_major != required_major {
                    if req.skip_java_check {
                        crate::launcher_log::error(
                            "launch",
                            format!(
                                "Java compatibility check skipped: override is version {actual_major}, {mc_version} needs Java {required_major}. Launching anyway."
                            ),
                        );
                    } else {
                        return Err(crate::error::AppError::Launch(format!(
                            "This instance's Java override is version {actual_major}, but {mc_version} needs Java {required_major}. Switch back to Automatic, point the override at a Java {required_major} install, or turn on \"Skip Java compatibility check\"."
                        )));
                    }
                }
            }
            (p, actual_major)
        }
        None => {
            let path = java_runtime::ensure_runtime(app, client, component).await?;
            // Always a known-good major version; we chose the component.
            let major = java_runtime::component_major_version(component).unwrap_or(21);
            (path, major)
        }
    };

    // Same inherit chain as window size below: instance value, then global
    // default, then the hardcoded built-in.
    let memory_mb = req.memory_mb.or(req.global_memory_mb).unwrap_or(DEFAULT_MEMORY_MB);
    let min_memory_mb = req.min_memory_mb.or(req.global_min_memory_mb).unwrap_or_else(|| memory_mb.min(1024));

    // An instance either fully customizes its own window (width and height
    // both set) or fully inherits the global one, never a mix of the two.
    let instance_customized_window = req.window_width.is_some() || req.window_height.is_some();
    let (base_width, base_height, base_maximized) = if instance_customized_window {
        (req.window_width, req.window_height, req.window_maximized)
    } else {
        (req.global_window_width, req.global_window_height, req.global_window_maximized)
    };
    let (window_width, window_height) = if base_maximized {
        app.get_webview_window("main")
            .and_then(|w| w.primary_monitor().ok().flatten())
            .map(|m| {
                let size = *m.size();
                (size.width, size.height)
            })
            .unwrap_or((
                base_width.unwrap_or(DEFAULT_WINDOW_WIDTH),
                base_height.unwrap_or(DEFAULT_WINDOW_HEIGHT),
            ))
    } else {
        (base_width.unwrap_or(DEFAULT_WINDOW_WIDTH), base_height.unwrap_or(DEFAULT_WINDOW_HEIGHT))
    };
    let env_vars = merge_env_vars(req.global_env_vars.as_deref(), req.env_vars.as_deref());
    // Global JVM arguments apply to every launch; an instance's own are
    // additional, not a replacement, same model as env vars above.
    let global_jvm_args: Vec<String> = req
        .global_jvm_args
        .as_deref()
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    let extra_jvm_args: Vec<String> = global_jvm_args.into_iter().chain(req.extra_jvm_args.iter().cloned()).collect();
    let jvm_args_str = extra_jvm_args.join(" ");
    let effective_wrapper_cmd = req.wrapper_cmd.clone().or_else(|| req.global_wrapper_cmd.clone());

    // Global pre-launch runs first, then the instance's own; either
    // failing aborts the launch.
    for template in [req.global_pre_launch_cmd.as_deref(), req.pre_launch_cmd.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
    {
        let expanded = expand_command_vars(template, &req.instance_id, &req.instance_name, &req.game_dir, &java_path, &jvm_args_str);
        crate::launcher_log::info("launch", format!("Running pre-launch command: {expanded}"));
        let status = run_shell_command(&expanded, &req.game_dir).await?;
        if !status.success() {
            return Err(crate::error::AppError::Launch(format!(
                "Pre-launch command failed ({status}), launch aborted."
            )));
        }
    }
    // Instance post-exit runs first, then the global one, the reverse
    // order of pre-launch.
    let post_exit_cmds: Vec<String> = [req.post_exit_cmd.as_deref(), req.global_post_exit_cmd.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .map(|template| expand_command_vars(template, &req.instance_id, &req.instance_name, &req.game_dir, &java_path, &jvm_args_str))
        .collect();

    let opts = LaunchOptions {
        version_name: mc_version,
        version_type,
        main_class,
        java_path: &java_path,
        java_major_version,
        memory_mb,
        min_memory_mb,
        extra_jvm_args: &extra_jvm_args,
        game_dir: &req.game_dir,
        natives_dir: &natives_dir,
        assets_dir: &paths::assets_dir(),
        asset_index_id,
        client_jar: &client_jar,
        // A coremod jar can bundle its own replacement for a library Strata
        // also resolves, so it must come first on the classpath to win.
        client_jar_first: req.custom_client_jar.is_some(),
        classpath: &libs.classpath,
        session: &req.session,
        window_width,
        window_height,
        env_vars: &env_vars,
        wrapper_cmd: effective_wrapper_cmd.as_deref().filter(|s| !s.trim().is_empty()),
        quick_play_mode: &req.quick_play_mode,
        quick_play_target: req.quick_play_target.as_deref(),
    };

    crate::launcher_log::info(
        "launch",
        format!("Resolved classpath: {} librar{} + client jar", libs.classpath.len(), if libs.classpath.len() == 1 { "y" } else { "ies" }),
    );

    let cmd = launch::build_command(&version_json, &opts)?;
    let child = launch::spawn(cmd)
        .inspect_err(|e| crate::launcher_log::error("launch", format!("Failed to start Java: {e}")))?;
    crate::launcher_log::info("launch", format!("Java process started (pid {:?})", child.id()));
    Ok(LaunchOutcome { child, post_exit_cmds })
}
