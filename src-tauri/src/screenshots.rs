//! Per-instance screenshot gallery: lists PNGs from `screenshots/`. Bytes
//! never go through IPC; the frontend renders them via Tauri's asset protocol.

use crate::error::AppResult;
use crate::timeutil::system_time_to_rfc3339;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub taken_at: Option<String>,
}

fn screenshots_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("screenshots")
}

pub fn list_screenshots(instance_dir: &Path) -> AppResult<Vec<ScreenshotInfo>> {
    let dir = screenshots_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;

    let mut shots = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let is_png = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if !is_png {
            continue;
        }
        let meta = entry.metadata()?;
        shots.push(ScreenshotInfo {
            file_name: entry.file_name().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            size_bytes: meta.len(),
            taken_at: meta.modified().ok().and_then(system_time_to_rfc3339),
        });
    }

    shots.sort_by(|a, b| b.taken_at.cmp(&a.taken_at));
    Ok(shots)
}

pub fn delete_screenshot(instance_dir: &Path, file_name: &str) -> AppResult<()> {
    let path = screenshots_dir(instance_dir).join(file_name);
    std::fs::remove_file(path)?;
    Ok(())
}

pub fn ensure_screenshots_dir(instance_dir: &Path) -> AppResult<PathBuf> {
    let dir = screenshots_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
