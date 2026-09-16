//! The launcher's own internal-actions log, written in the same log4j-ish
//! shape as real Minecraft logs so the frontend can parse both with one regex.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

const CAP: usize = 2000;

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

static BUFFER: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<String>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAP)))
}

fn log_path() -> std::path::PathBuf {
    crate::paths::data_dir().join("logs").join("launcher.log")
}

fn record(level: Level, category: &str, message: &str) {
    let time = chrono::Local::now().format("%H:%M:%S");
    let line = format!("[{time}] [main/{}] ({category}): {message}", level.as_str());

    {
        let mut buf = buffer().lock().unwrap();
        if buf.len() >= CAP {
            buf.pop_front();
        }
        buf.push_back(line.clone());
    }

    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
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

/// The current session's buffered lines, newest last; avoids re-reading
/// `launcher.log` from disk each time.
pub fn snapshot() -> String {
    buffer().lock().unwrap().iter().cloned().collect::<Vec<_>>().join("\n")
}
