use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub username: String,
    pub mc_uuid: Option<String>,
    pub skin_url: Option<String>,
    /// "classic" | "slim", lowercase (Mojang's response is uppercase,
    /// normalized on write).
    pub skin_variant: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Microsoft,
    Offline,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountKind::Microsoft => "microsoft",
            AccountKind::Offline => "offline",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "microsoft" => AccountKind::Microsoft,
            _ => AccountKind::Offline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    /// "vanilla" | "fabric" | "quilt" (Forge/NeoForge come later).
    pub loader: String,
    /// The loader's own build version (e.g. a Fabric loader version). Unset for vanilla.
    pub loader_version: Option<String>,
    pub mc_version: String,
    pub memory_mb: Option<u32>,
    /// `-Xms`. `None` falls back to `min(memory_mb, 1024)`, the historical
    /// behavior from before this was separately configurable.
    pub min_memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub java_path: Option<String>,
    pub icon_biome: String,
    pub group_name: Option<String>,
    pub created_at: String,
    pub last_played_at: Option<String>,
    /// `None` = never launched (or unknown); `Some(true)` = the last
    /// session ended in a non-zero exit; `Some(false)` = clean exit.
    pub last_crashed: Option<bool>,
    /// Real `--width`/`--height` launch args. `None` keeps the existing
    /// 925x530 default.
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    /// No real "start maximized" launch flag; substitutes the primary
    /// monitor's resolution for width/height instead.
    pub window_maximized: bool,
    /// Bypasses the hard block on a Java major-version mismatch for a
    /// manual override. Mirrors Prism's "Skip Java Compatibility Checks".
    pub skip_java_check: bool,
    /// Newline-separated `KEY=VALUE` pairs applied to the game process.
    pub env_vars: Option<String>,
    /// Run (blocking) immediately before the game process starts; a
    /// non-zero exit aborts the launch. Supports `$INST_*` variables.
    pub pre_launch_cmd: Option<String>,
    /// Prefixes the actual `java` invocation (e.g. `gamemoderun`); unlike
    /// pre/post-exit, no variable expansion, it's a program name.
    pub wrapper_cmd: Option<String>,
    /// Run after the game process exits, best-effort. Never runs if
    /// `launcher_behavior` is `close_on_launch` (the launcher exits first).
    pub post_exit_cmd: Option<String>,
    /// "always" | "on_crash" | "never": whether Strata auto-navigates to
    /// the Logs page on launch/crash, in place of a native console window.
    pub console_mode: String,
    /// "keep_open" (default) | "close_on_launch" | "quit_on_exit".
    pub launcher_behavior: String,
    /// "off" (default) | "world" | "server": Quick Play, 1.20+ only.
    /// `quick_play_target` is a world name or `host:port`.
    pub quick_play_mode: String,
    pub quick_play_target: Option<String>,
    /// Absolute path to a local jar replacing the vanilla client jar
    /// entirely (e.g. a total-conversion "coremod"). `None` keeps vanilla.
    pub custom_client_jar: Option<String>,
}

pub const BIOME_KEYS: &[&str] = &["ore", "verdant", "frost", "cobalt", "sun", "ember"];
