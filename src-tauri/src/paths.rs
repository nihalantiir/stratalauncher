//! Portable-first data location: a `data/` folder next to the executable,
//! falling back to the OS per-user app-data directory if that's unwritable.
//!
//! A user can also point Strata at a folder of their own choosing; see
//! `bootstrap` below for how that's remembered and applied safely.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("data");
    Some(dir)
}

fn os_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Strata")
}

fn is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn default_dir() -> PathBuf {
    if let Some(dir) = portable_dir() {
        if is_writable(&dir) {
            return dir;
        }
    }
    let dir = os_data_dir();
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// A tiny bootstrap file kept next to the exe (like `portable_dir()`) so two
/// separate installs never share one redirect; falls back to a per-install file under the OS config dir if that folder isn't writable.
mod bootstrap {
    use super::{is_writable, PathBuf};
    use serde::{Deserialize, Serialize};
    use std::sync::OnceLock;

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct Bootstrap {
        /// Where data actually lives right now. `None` means "still at
        /// `default_dir()`", needed so reset-to-default finds its way back.
        #[serde(default)]
        pub current_dir: Option<PathBuf>,
        /// A location queued for the next launch to move into. Cleared once
        /// applied.
        #[serde(default)]
        pub pending_dir: Option<PathBuf>,
    }

    static PATH: OnceLock<PathBuf> = OnceLock::new();

    fn install_key() -> String {
        use std::hash::{Hash, Hasher};
        let exe = std::env::current_exe().unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        exe.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn file_path() -> &'static PathBuf {
        PATH.get_or_init(|| {
            if let Some(exe) = std::env::current_exe().ok() {
                if let Some(dir) = exe.parent() {
                    if is_writable(dir) {
                        return dir.join("bootstrap.json");
                    }
                }
            }
            let base = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
            base.join("Strata").join(format!("bootstrap-{}.json", install_key()))
        })
    }

    pub fn load() -> Bootstrap {
        std::fs::read(file_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(data: &Bootstrap) -> std::io::Result<()> {
        let path = file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(data)?)
    }
}

/// Moves every file from `from` into `to` (`to` assumed freshly created and
/// empty), only at the one safe moment described on `bootstrap` above.
fn move_dir_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if std::fs::rename(entry.path(), &dest).is_err() {
            // Cross-drive moves can't rename(); fall back to copy+remove.
            if entry.file_type()?.is_dir() {
                crate::fsutil::copy_dir_recursive(&entry.path(), &dest)?;
                std::fs::remove_dir_all(entry.path())?;
            } else {
                std::fs::copy(entry.path(), &dest)?;
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

/// Resolve (and cache) the root data directory for this run. Migrates from
/// `current_dir`, not `default_dir()`, so a reset-to-default doesn't strand data.
pub fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        let mut boot = bootstrap::load();
        let source = boot.current_dir.clone().unwrap_or_else(default_dir);

        let Some(target) = boot.pending_dir.take() else {
            return source;
        };

        if target == source {
            // Already there (e.g. re-picking the same folder); just clear
            // the stale pending marker.
            boot.current_dir = Some(target.clone());
            let _ = bootstrap::save(&boot);
            return target;
        }

        if source.join("strata.sqlite").exists() {
            if std::fs::create_dir_all(&target).is_ok() {
                if let Err(e) = move_dir_contents(&source, &target) {
                    // Not `launcher_log`: its log file lives under
                    // `data_dir()`, still being resolved here (would deadlock).
                    eprintln!("[paths] Couldn't move data to the new folder ({e}); staying at the old location.");
                    boot.pending_dir = None;
                    let _ = bootstrap::save(&boot);
                    return source;
                }
                eprintln!("[paths] Moved app data to {}", target.display());
            }
        } else {
            std::fs::create_dir_all(&target).ok();
        }
        boot.current_dir = Some(target.clone());
        boot.pending_dir = None;
        let _ = bootstrap::save(&boot);
        target
    })
}

/// Whether `data_dir()` is currently the portable/OS default, or a location
/// the user chose. Purely informational, for Settings → Storage.
pub fn is_custom_data_dir() -> bool {
    let boot = bootstrap::load();
    let current = boot.current_dir.unwrap_or_else(default_dir);
    current != default_dir()
}

/// A folder queued to move into on the next launch, if the user picked one
/// this session but hasn't restarted yet. Purely informational.
pub fn pending_data_dir() -> Option<PathBuf> {
    bootstrap::load().pending_dir
}

/// Queues the user's chosen data folder for the *next* launch to move into.
/// Never touched live; the current run keeps using its already-resolved value.
pub fn set_pending_data_dir(path: PathBuf) -> std::io::Result<()> {
    let mut boot = bootstrap::load();
    boot.pending_dir = Some(path);
    bootstrap::save(&boot)
}

/// Queues a move back to the default on the next launch, same safe-migrate
/// behavior as `set_pending_data_dir` in the other direction.
pub fn clear_pending_data_dir() -> std::io::Result<()> {
    let mut boot = bootstrap::load();
    boot.pending_dir = Some(default_dir());
    bootstrap::save(&boot)
}

pub fn db_path() -> PathBuf {
    data_dir().join("strata.sqlite")
}

pub fn libraries_dir() -> PathBuf {
    data_dir().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}

/// Background videos, downloaded on first launch rather than bundled into
/// the exe. Unrelated to `assets_dir()`, which is Minecraft's own assets.
pub fn media_dir() -> PathBuf {
    data_dir().join("media")
}

pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}

pub fn instances_dir() -> PathBuf {
    data_dir().join("instances")
}

/// The isolated `.minecraft`-style game directory for one instance.
pub fn instance_dir(instance_id: &str) -> PathBuf {
    instances_dir().join(instance_id)
}

pub fn java_dir() -> PathBuf {
    data_dir().join("java")
}
