//! Self-update: checks the GitHub release manifest, downloads and verifies
//! a new build on demand, then hands off to `strata-updater.exe` to swap it in.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, State};
use tokio::io::AsyncWriteExt;

fn manifest_url() -> String {
    let repo = crate::config::update_repo().unwrap_or_else(|| "nihalantiir/stratalauncher".to_string());
    format!("https://github.com/{repo}/releases/latest/download/latest.json")
}

#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    windows: ManifestPlatform,
}

#[derive(Debug, Deserialize)]
struct ManifestPlatform {
    url: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub sha256: String,
}

/// Dotted numeric version compare, no external semver crate; a shorter
/// version is treated as zero-padded (`"0.2"` == `"0.2.0"`).
fn is_newer(candidate: &str, current: &str) -> bool {
    let a: Vec<u32> = candidate.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    let b: Vec<u32> = current.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    for i in 0..a.len().max(b.len()) {
        let na = a.get(i).copied().unwrap_or(0);
        let nb = b.get(i).copied().unwrap_or(0);
        if na != nb {
            return na > nb;
        }
    }
    false
}

/// Shared by the Settings page's manual check and the periodic background
/// check in `sync.rs`, so both can never disagree on "is there an update."
pub async fn fetch_update_info(client: &reqwest::Client) -> AppResult<Option<UpdateInfo>> {
    let manifest: Manifest = client.get(manifest_url()).send().await?.error_for_status()?.json().await?;
    let current = env!("CARGO_PKG_VERSION");

    if !is_newer(&manifest.version, current) {
        return Ok(None);
    }

    Ok(Some(UpdateInfo {
        version: manifest.version,
        download_url: manifest.windows.url,
        sha256: manifest.windows.sha256,
    }))
}

#[tauri::command]
pub async fn check_for_update(state: State<'_, AppState>) -> AppResult<Option<UpdateInfo>> {
    fetch_update_info(&state.http).await
}

/// A zip entry's path is third-party input; rejects anything that could
/// escape `dest_root` via `..`, an absolute path, or a drive letter.
fn safe_join(dest_root: &Path, entry_name: &str) -> AppResult<PathBuf> {
    let rel = Path::new(entry_name);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_))) {
        return Err(AppError::Other(format!("unsafe path in update archive: {entry_name}")));
    }
    Ok(dest_root.join(rel))
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>, download_url: String, sha256: String) -> AppResult<()> {
    let resp = state.http.get(&download_url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0) as usize;

    let zip_path = std::env::temp_dir().join(format!("strata-update-{}.zip", uuid::Uuid::new_v4()));
    let mut file = tokio::fs::File::create(&zip_path).await?;
    let mut hasher = Sha256::new();
    let mut completed = 0usize;
    let mut stream = resp.bytes_stream();

    crate::minecraft::download::emit_progress(&app, "update", 0, total);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        completed += chunk.len();
        crate::minecraft::download::emit_progress(&app, "update", completed, total);
    }
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if !actual.eq_ignore_ascii_case(&sha256) {
        let _ = tokio::fs::remove_file(&zip_path).await;
        return Err(AppError::ChecksumMismatch { file: zip_path.display().to_string(), expected: sha256, actual });
    }

    let extract_dir = std::env::temp_dir().join(format!("strata-update-extract-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&extract_dir)?;
    {
        let zip_file = std::fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| AppError::Other(format!("bad update archive: {e}")))?;
        for name in ["strata.exe", "strata-updater.exe"] {
            let mut entry = archive
                .by_name(name)
                .map_err(|e| AppError::Other(format!("update archive is missing {name}: {e}")))?;
            let out_path = safe_join(&extract_dir, name)?;
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    let _ = tokio::fs::remove_file(&zip_path).await;

    let new_exe = extract_dir.join("strata.exe");
    let new_updater = extract_dir.join("strata-updater.exe");

    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| AppError::Other("the running exe has no parent directory".into()))?;
    let target_updater = exe_dir.join("strata-updater.exe");
    std::fs::copy(&new_updater, &target_updater)?;

    std::process::Command::new(&target_updater)
        .arg("--pid")
        .arg(std::process::id().to_string())
        .arg("--new-exe")
        .arg(&new_exe)
        .arg("--target-exe")
        .arg(&current_exe)
        .spawn()?;

    app.exit(0);
    Ok(())
}
