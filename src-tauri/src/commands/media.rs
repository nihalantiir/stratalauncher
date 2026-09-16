//! Background videos, downloaded on first launch instead of bundled into
//! the exe (356MB of video would otherwise get embedded via `frontendDist`).

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, State};
use tokio::io::AsyncWriteExt;

pub const VIDEO_NAMES: &[&str] = &[
    "village",
    "aquarium",
    "beach-escape",
    "cherry-grove",
    "dreamy-deserts",
    "dungeons",
    "fireplace",
    "glowing-caves",
    "mining-blocks",
    "rainy-swamp",
    "serene-snow",
    "tricky-trials",
];

const MEDIA_TAG: &str = "media-v1";

fn manifest_url() -> String {
    let repo = crate::config::update_repo().unwrap_or_else(|| "nihalantiir/stratalauncher".to_string());
    format!("https://github.com/{repo}/releases/download/{MEDIA_TAG}/media.json")
}

fn version_marker_path() -> std::path::PathBuf {
    crate::paths::media_dir().join("version.txt")
}

#[derive(Debug, Deserialize)]
struct MediaManifest {
    version: u32,
    url: String,
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStatus {
    pub ready: bool,
    pub dir: String,
}

fn videos_present() -> bool {
    let dir = crate::paths::media_dir().join("videos");
    VIDEO_NAMES.iter().all(|name| dir.join(format!("{name}.mp4")).exists())
}

#[tauri::command]
pub fn media_status() -> MediaStatus {
    MediaStatus {
        ready: videos_present(),
        dir: crate::paths::media_dir().join("videos").display().to_string(),
    }
}

/// Fetches the manifest and compares against the locally cached version; a
/// mismatch (or missing files) means a real download is needed.
async fn needs_download(client: &reqwest::Client) -> AppResult<Option<MediaManifest>> {
    let manifest: MediaManifest = client.get(manifest_url()).send().await?.error_for_status()?.json().await?;
    let cached_version = std::fs::read_to_string(version_marker_path()).ok().and_then(|s| s.trim().parse::<u32>().ok());

    if cached_version == Some(manifest.version) && videos_present() {
        return Ok(None);
    }
    Ok(Some(manifest))
}

async fn download_and_extract(app: &AppHandle, client: &reqwest::Client, manifest: &MediaManifest) -> AppResult<()> {
    let resp = client.get(&manifest.url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0) as usize;

    let zip_path = std::env::temp_dir().join(format!("strata-media-{}.zip", uuid::Uuid::new_v4()));
    let mut file = tokio::fs::File::create(&zip_path).await?;
    let mut hasher = Sha256::new();
    let mut completed = 0usize;
    let mut stream = resp.bytes_stream();

    crate::minecraft::download::emit_progress(app, "media", 0, total);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        completed += chunk.len();
        crate::minecraft::download::emit_progress(app, "media", completed, total);
    }
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if !actual.eq_ignore_ascii_case(&manifest.sha256) {
        let _ = tokio::fs::remove_file(&zip_path).await;
        return Err(AppError::ChecksumMismatch { file: zip_path.display().to_string(), expected: manifest.sha256.clone(), actual });
    }

    let videos_dir = crate::paths::media_dir().join("videos");
    std::fs::create_dir_all(&videos_dir)?;
    {
        let zip_file = std::fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| AppError::Other(format!("bad media archive: {e}")))?;
        for name in VIDEO_NAMES {
            let file_name = format!("{name}.mp4");
            let mut entry = archive
                .by_name(&file_name)
                .map_err(|e| AppError::Other(format!("media archive is missing {file_name}: {e}")))?;
            let out_path = crate::fsutil::safe_join(&videos_dir, &file_name, "media archive")?;
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    let _ = tokio::fs::remove_file(&zip_path).await;
    std::fs::write(version_marker_path(), manifest.version.to_string())?;
    Ok(())
}

/// Best-effort: called from Settings and from the startup background task,
/// both swallow their own errors (there's real gameplay to get to either way).
#[tauri::command]
pub async fn download_media(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let Some(manifest) = needs_download(&state.http).await? else {
        return Ok(());
    };
    download_and_extract(&app, &state.http, &manifest).await
}
