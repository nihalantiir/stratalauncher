//! The launcher's own internal-actions log, written in the same log4j-ish
//! shape as real Minecraft logs so the frontend can parse both with one regex.

use std::io::Write;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
enum Level {
    Info,
    Warn,
    Error,
    Debug,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Debug => "DEBUG",
        }
    }
}

pub fn logs_dir() -> std::path::PathBuf {
    crate::paths::data_dir().join("logs")
}

fn latest_log_path() -> std::path::PathBuf {
    logs_dir().join("latest.log")
}

/// First unused `{dir}/{date}-N.log` path, starting at N=1.
fn next_rotated_path(dir: &std::path::Path, date: &str) -> std::path::PathBuf {
    let mut n = 1;
    loop {
        let candidate = dir.join(format!("{date}-{n}.log"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Renames a previous session's `latest.log` out of the way (dated/numbered,
/// same shape Minecraft itself uses); runs once, lazily, on first write.
fn ensure_rotated() {
    static ROTATED: OnceLock<()> = OnceLock::new();
    ROTATED.get_or_init(|| {
        let dir = logs_dir();
        let _ = std::fs::create_dir_all(&dir);
        let latest = dir.join("latest.log");
        let Ok(meta) = std::fs::metadata(&latest) else { return };
        let date = meta
            .modified()
            .ok()
            .and_then(crate::timeutil::system_time_to_rfc3339)
            .and_then(|s| s.get(0..10).map(str::to_string))
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
        let _ = std::fs::rename(&latest, next_rotated_path(&dir, &date));
    });
}

fn record(level: Level, category: &str, message: &str) {
    ensure_rotated();
    let time = chrono::Local::now().format("%H:%M:%S");
    let line = format!("[{time}] [main/{}] ({category}): {message}", level.as_str());

    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(latest_log_path()) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn info(category: &str, message: impl AsRef<str>) {
    record(Level::Info, category, message.as_ref());
}
#[allow(dead_code)]
pub fn warn(category: &str, message: impl AsRef<str>) {
    record(Level::Warn, category, message.as_ref());
}
pub fn error(category: &str, message: impl AsRef<str>) {
    record(Level::Error, category, message.as_ref());
}
#[allow(dead_code)]
pub fn debug(category: &str, message: impl AsRef<str>) {
    record(Level::Debug, category, message.as_ref());
}

#[cfg(test)]
mod tests {
    use super::next_rotated_path;

    #[test]
    fn picks_first_free_numbered_slot_for_the_date() {
        let dir = std::env::temp_dir().join(format!("strata-log-rotate-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(next_rotated_path(&dir, "2026-09-17"), dir.join("2026-09-17-1.log"));

        std::fs::write(dir.join("2026-09-17-1.log"), "").unwrap();
        assert_eq!(next_rotated_path(&dir, "2026-09-17"), dir.join("2026-09-17-2.log"));

        std::fs::write(dir.join("2026-09-17-2.log"), "").unwrap();
        assert_eq!(next_rotated_path(&dir, "2026-09-17"), dir.join("2026-09-17-3.log"));

        // A different date starts its own sequence, unaffected by the other date's files.
        assert_eq!(next_rotated_path(&dir, "2026-09-18"), dir.join("2026-09-18-1.log"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
