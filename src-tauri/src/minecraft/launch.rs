use super::rules::rules_allow;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tokio::process::{Child, Command};

pub struct LaunchSession {
    pub username: String,
    /// Undashed 32-hex UUID, matching what the vanilla client expects.
    pub uuid: String,
    pub access_token: String,
    pub user_type: &'static str,
}

pub struct LaunchOptions<'a> {
    /// The vanilla Minecraft version id (e.g. `1.20.1`), not Strata's
    /// internal loader cache key. Fed to `--version`/`${version_name}`.
    pub version_name: &'a str,
    pub version_type: &'a str,
    pub main_class: &'a str,
    pub java_path: &'a Path,
    /// Needed to decide whether `@argfile` syntax is usable; Java 8's
    /// launcher doesn't understand it and fails with a cryptic error.
    pub java_major_version: u32,
    pub memory_mb: u32,
    /// `-Xms`. Already resolved by the caller (the instance's own setting,
    /// or `min(memory_mb, 1024)` if unset).
    pub min_memory_mb: u32,
    pub extra_jvm_args: &'a [String],
    pub game_dir: &'a Path,
    pub natives_dir: &'a Path,
    pub assets_dir: &'a Path,
    pub asset_index_id: &'a str,
    pub client_jar: &'a Path,
    /// Puts `client_jar` before the library jars on the classpath, so a
    /// Coremods jar's own bundled library replacements actually win.
    pub client_jar_first: bool,
    pub classpath: &'a [PathBuf],
    pub session: &'a LaunchSession,
    /// Already resolved by the caller: "maximized" substitutes the real
    /// primary-monitor resolution, since there's no native launch arg for it.
    pub window_width: u32,
    pub window_height: u32,
    pub env_vars: &'a [(String, String)],
    /// Prefixes the actual `java` invocation (e.g. `gamemoderun`); no
    /// variable expansion, unlike pre-launch/post-exit commands.
    pub wrapper_cmd: Option<&'a str>,
    /// "off" | "world" | "server", gated to 1.20+ by the caller.
    pub quick_play_mode: &'a str,
    pub quick_play_target: Option<&'a str>,
}

fn classpath_string(classpath: &[PathBuf], client_jar: &Path, client_jar_first: bool) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    // Vanilla's and a loader's library lists can both declare the same
    // dependency; Forge's BootstrapLauncher throws on a duplicate classpath entry.
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    let libs = classpath.iter().map(PathBuf::as_path);
    let ordered: Box<dyn Iterator<Item = &Path>> = if client_jar_first {
        Box::new(std::iter::once(client_jar).chain(libs))
    } else {
        Box::new(libs.chain(std::iter::once(client_jar)))
    };
    for path in ordered {
        let s = path.display().to_string();
        if seen.insert(s.clone()) {
            entries.push(s);
        }
    }
    entries.join(sep)
}

fn substitution_vars(opts: &LaunchOptions, classpath: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    vars.insert("auth_player_name".into(), opts.session.username.clone());
    vars.insert("version_name".into(), opts.version_name.to_string());
    vars.insert("game_directory".into(), opts.game_dir.display().to_string());
    vars.insert("assets_root".into(), opts.assets_dir.display().to_string());
    vars.insert("game_assets".into(), opts.assets_dir.display().to_string());
    vars.insert("assets_index_name".into(), opts.asset_index_id.to_string());
    vars.insert("auth_uuid".into(), opts.session.uuid.clone());
    vars.insert("auth_access_token".into(), opts.session.access_token.clone());
    vars.insert("auth_session".into(), opts.session.access_token.clone());
    vars.insert("user_type".into(), opts.session.user_type.to_string());
    vars.insert("user_properties".into(), "{}".into());
    vars.insert("version_type".into(), opts.version_type.to_string());
    vars.insert("natives_directory".into(), opts.natives_dir.display().to_string());
    vars.insert("launcher_name".into(), "Strata".into());
    vars.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath".into(), classpath.to_string());
    vars.insert(
        "classpath_separator".into(),
        (if cfg!(windows) { ";" } else { ":" }).into(),
    );
    vars.insert("library_directory".into(), crate::paths::libraries_dir().display().to_string());
    vars.insert("auth_xuid".into(), "0".into());
    vars.insert("clientid".into(), "strata-launcher".into());
    vars
}

fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

fn features() -> HashMap<String, bool> {
    HashMap::new()
}

/// Flattens a new-format (`arguments.jvm`/`arguments.game`, 1.13+) argument
/// array, each entry a bare string or a `{rules, value}` object.
fn flatten_argument_array(arr: &[serde_json::Value], vars: &HashMap<String, String>) -> Vec<String> {
    let features = features();
    let mut out = Vec::new();
    for entry in arr {
        if let Some(s) = entry.as_str() {
            out.push(substitute(s, vars));
            continue;
        }
        let rules = entry.get("rules").and_then(|r| r.as_array()).cloned();
        if !rules_allow(rules.as_ref(), &features) {
            continue;
        }
        match entry.get("value") {
            Some(serde_json::Value::String(s)) => out.push(substitute(s, vars)),
            Some(serde_json::Value::Array(values)) => {
                for v in values {
                    if let Some(s) = v.as_str() {
                        out.push(substitute(s, vars));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn default_jvm_args(natives_dir: &Path, classpath: &str, memory_mb: u32) -> Vec<String> {
    vec![
        format!("-Xmx{memory_mb}M"),
        format!("-Djava.library.path={}", natives_dir.display()),
        // Pre-1.6 clients (and coremods built on them, e.g. BTA) still use
        // AWT/Swing and can hit a real HeadlessException; force it off.
        "-Djava.awt.headless=false".into(),
        "-Dminecraft.launcher.brand=Strata".into(),
        format!("-Dminecraft.launcher.version={}", env!("CARGO_PKG_VERSION")),
        "-cp".into(),
        classpath.to_string(),
    ]
}

fn legacy_game_args(minecraft_arguments: &str, vars: &HashMap<String, String>) -> Vec<String> {
    minecraft_arguments
        .split_whitespace()
        .map(|tok| substitute(tok, vars))
        .collect()
}

/// Writes a Java "argument file" (`java @file`, supported since Java 9):
/// one quoted argument per line.
fn write_argfile(game_dir: &Path, args: &[String]) -> AppResult<PathBuf> {
    let mut contents = String::new();
    for arg in args {
        let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
        contents.push('"');
        contents.push_str(&escaped);
        contents.push_str("\"\n");
    }
    let path = game_dir.join(".strata-launch-args.txt");
    std::fs::write(&path, contents)?;
    Ok(path)
}

pub fn build_command(version_json: &serde_json::Value, opts: &LaunchOptions) -> AppResult<Command> {
    let classpath = classpath_string(opts.classpath, opts.client_jar, opts.client_jar_first);
    let vars = substitution_vars(opts, &classpath);

    let mut jvm_args: Vec<String> = vec![format!("-Xmx{}M", opts.memory_mb), format!("-Xms{}M", opts.min_memory_mb)];
    let mut game_args: Vec<String>;

    if let Some(arguments) = version_json.get("arguments") {
        if let Some(jvm) = arguments.get("jvm").and_then(|v| v.as_array()) {
            jvm_args.extend(flatten_argument_array(jvm, &vars));
        } else {
            jvm_args.extend(default_jvm_args(opts.natives_dir, &classpath, opts.memory_mb));
        }
        let game = arguments
            .get("game")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        game_args = flatten_argument_array(&game, &vars);
    } else if let Some(legacy) = version_json.get("minecraftArguments").and_then(|v| v.as_str()) {
        jvm_args.extend(default_jvm_args(opts.natives_dir, &classpath, opts.memory_mb));
        game_args = legacy_game_args(legacy, &vars);
    } else {
        return Err(AppError::Launch("version JSON has no launch arguments".into()));
    }

    if !game_args.iter().any(|a| a == "--width") {
        game_args.extend([
            "--width".into(),
            opts.window_width.to_string(),
            "--height".into(),
            opts.window_height.to_string(),
        ]);
    }
    match (opts.quick_play_mode, opts.quick_play_target) {
        ("world", Some(world)) if !world.is_empty() => {
            game_args.extend(["--quickPlaySingleplayer".into(), world.to_string()]);
        }
        ("server", Some(addr)) if !addr.is_empty() => {
            game_args.extend(["--quickPlayMultiplayer".into(), addr.to_string()]);
        }
        _ => {}
    }

    std::fs::create_dir_all(opts.game_dir)?;

    let mut all_args = jvm_args;
    all_args.extend(opts.extra_jvm_args.iter().cloned());
    all_args.push(opts.main_class.to_string());
    all_args.extend(game_args);

    // A wrapper prefixes the real invocation (`gamemoderun java ...`)
    // rather than running before/after it.
    let mut cmd = match opts.wrapper_cmd {
        Some(wrapper) => {
            let mut parts = wrapper.split_whitespace();
            let program = parts.next().unwrap_or(wrapper);
            let mut c = Command::new(program);
            c.args(parts);
            c.arg(opts.java_path);
            c
        }
        None => Command::new(opts.java_path),
    };
    cmd.current_dir(opts.game_dir);
    cmd.envs(opts.env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())));

    // Route through a Java "@argfile" rather than literal argv: Forge's large
    // classpath can exceed Windows' length limit. Java 8 doesn't support it.
    if opts.java_major_version >= 9 {
        let argfile = write_argfile(opts.game_dir, &all_args)?;
        cmd.arg(format!("@{}", argfile.display()));
    } else {
        cmd.args(&all_args);
    }
    Ok(cmd)
}

pub fn spawn(mut cmd: Command) -> AppResult<Child> {
    cmd.spawn().map_err(|e| AppError::Launch(format!("failed to start Java: {e}")))
}

/// Best-effort local JRE discovery via `JAVA_HOME` then `PATH`, for
/// installers that just need some JVM (see `java_runtime` for real launches).
pub fn find_java() -> AppResult<PathBuf> {
    let exe_name = if cfg!(windows) { "java.exe" } else { "java" };

    if let Ok(home) = std::env::var("JAVA_HOME") {
        let candidate = PathBuf::from(home).join("bin").join(exe_name);
        if candidate.exists() {
            crate::launcher_log::info("java", format!("Using JAVA_HOME: {}", candidate.display()));
            return Ok(candidate);
        }
    }

    let probe = StdCommand::new(exe_name).arg("-version").output();
    if let Ok(output) = probe {
        if output.status.success() || !output.stderr.is_empty() {
            crate::launcher_log::info("java", format!("Using '{exe_name}' resolved from PATH"));
            return Ok(PathBuf::from(exe_name));
        }
    }

    crate::launcher_log::error("java", "No Java runtime found on JAVA_HOME or PATH");
    Err(AppError::Launch(
        "No Java runtime found. Install a Java Runtime Environment matching the game version and make sure it's on PATH or JAVA_HOME.".into(),
    ))
}

/// Runs `<path> -version` and parses the major version out of its output;
/// handles both the legacy `1.8.0_401` and modern `17.0.15` schemes.
pub fn probe_java_major_version(java_path: &Path) -> AppResult<u32> {
    let output = StdCommand::new(java_path)
        .arg("-version")
        .output()
        .map_err(|e| AppError::Launch(format!("Couldn't run '{}': {e}", java_path.display())))?;
    let text = String::from_utf8_lossy(&output.stderr);
    parse_java_major_version(&text)
        .ok_or_else(|| AppError::Launch(format!("Couldn't determine the Java version of '{}'.", java_path.display())))
}

fn parse_java_major_version(version_output: &str) -> Option<u32> {
    let start = version_output.find('"')? + 1;
    let rest = &version_output[start..];
    let end = rest.find('"')?;
    let ver = &rest[..end];

    let numeric = ver.strip_prefix("1.").unwrap_or(ver);
    let digits_end = numeric.find(|c: char| !c.is_ascii_digit()).unwrap_or(numeric.len());
    numeric[..digits_end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_java_major_version;

    #[test]
    fn parses_legacy_and_modern_version_strings() {
        assert_eq!(parse_java_major_version(r#"java version "1.8.0_401""#), Some(8));
        assert_eq!(parse_java_major_version(r#"openjdk version "17.0.15" 2025-04-15"#), Some(17));
        assert_eq!(parse_java_major_version(r#"openjdk version "25.0.4.1" 2026-01-20"#), Some(25));
    }
}
