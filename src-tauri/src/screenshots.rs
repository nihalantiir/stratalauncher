//! Per-instance screenshot gallery: lists PNGs from `screenshots/`. Bytes
//! never go through IPC; the frontend renders them via Tauri's asset protocol.

use crate::error::AppResult;
use crate::timeutil::system_time_to_rfc3339;
use futures_util::StreamExt;
use image::imageops::FilterType;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Minecraft screenshots are full-resolution PNGs, often several MB each;
/// decoding dozens of them for a small grid tile is what made scrolling and
/// sidebar-resize recalculation laggy. The grid renders this instead.
const THUMB_MAX_DIM: u32 = 480;
const THUMB_JPEG_QUALITY: u8 = 82;
const THUMB_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub file_name: String,
    pub path: String,
    pub thumb_path: String,
    pub size_bytes: u64,
    pub taken_at: Option<String>,
}

fn screenshots_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("screenshots")
}

fn thumbnails_dir(instance_dir: &Path) -> PathBuf {
    screenshots_dir(instance_dir).join(".thumbnails")
}

fn thumbnail_path(instance_dir: &Path, file_name: &str) -> PathBuf {
    thumbnails_dir(instance_dir).join(file_name).with_extension("jpg")
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
        let file_name = entry.file_name().to_string_lossy().to_string();
        shots.push(ScreenshotInfo {
            thumb_path: thumbnail_path(instance_dir, &file_name).to_string_lossy().to_string(),
            file_name,
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
    std::fs::remove_file(thumbnail_path(instance_dir, file_name)).ok();
    Ok(())
}

pub fn ensure_screenshots_dir(instance_dir: &Path) -> AppResult<PathBuf> {
    let dir = screenshots_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn generate_thumbnail(src: &Path, dest: &Path) {
    let Ok(img) = image::open(src) else { return };
    let resized = img.resize(THUMB_MAX_DIM, THUMB_MAX_DIM, FilterType::Triangle);
    let tmp = dest.with_extension("jpg.part");
    let Ok(mut file) = std::fs::File::create(&tmp) else { return };
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, THUMB_JPEG_QUALITY);
    if resized.to_rgb8().write_with_encoder(encoder).is_ok() {
        drop(file);
        std::fs::rename(&tmp, dest).ok();
    } else {
        drop(file);
        std::fs::remove_file(&tmp).ok();
    }
}

/// Generates any missing thumbnails for `instance_dir`'s screenshots.
/// Screenshots are never modified after being taken, so an existing
/// thumbnail is always trusted as still current.
pub async fn ensure_thumbnails(instance_dir: &Path) -> AppResult<()> {
    let dir = screenshots_dir(instance_dir);
    let thumb_dir = thumbnails_dir(instance_dir);
    std::fs::create_dir_all(&thumb_dir)?;

    let mut jobs = Vec::new();
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
        let file_name = entry.file_name().to_string_lossy().to_string();
        let dest = thumbnail_path(instance_dir, &file_name);
        if !dest.exists() {
            jobs.push((path, dest));
        }
    }

    futures_util::stream::iter(jobs.into_iter().map(|(src, dest)| async move {
        tokio::task::spawn_blocking(move || generate_thumbnail(&src, &dest)).await.ok();
    }))
    .buffer_unordered(THUMB_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    Ok(())
}
