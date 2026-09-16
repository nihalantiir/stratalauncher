//! Installs a `.mrpack` (Modrinth's modpack format): a zip with
//! `modrinth.index.json` at the root, verified against Modrinth's docs.

use super::{extract_prefixed, read_zip_entry_bytes, safe_join, ResolvedModpackTarget};
use crate::error::AppResult;
use crate::minecraft::download;
use serde::Deserialize;
use std::path::Path;
use tauri::AppHandle;

#[derive(Debug, Deserialize)]
struct Index {
    dependencies: std::collections::HashMap<String, String>,
    files: Vec<IndexFile>,
}

#[derive(Debug, Deserialize)]
struct IndexFile {
    path: String,
    hashes: IndexHashes,
    #[serde(default)]
    env: Option<IndexEnv>,
    downloads: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct IndexHashes {
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndexEnv {
    client: Option<String>,
}

/// The four loader dependency keys `dependencies` can carry, mapped to
/// Strata's loader names (`minecraft` is handled separately).
fn loader_from_dependencies(deps: &std::collections::HashMap<String, String>) -> Option<(String, String)> {
    for (key, loader_name) in [
        ("forge", "forge"),
        ("neoforge", "neoforge"),
        ("fabric-loader", "fabric"),
        ("quilt-loader", "quilt"),
    ] {
        if let Some(version) = deps.get(key) {
            return Some((loader_name.to_string(), version.clone()));
        }
    }
    None
}

pub fn resolve_target(zip_path: &Path) -> AppResult<ResolvedModpackTarget> {
    let bytes = read_zip_entry_bytes(zip_path, "modrinth.index.json")?;
    let index: Index = serde_json::from_slice(&bytes)?;
    let mc_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| crate::error::AppError::Other("modrinth.index.json has no minecraft dependency".into()))?;
    let (loader, loader_version) = match loader_from_dependencies(&index.dependencies) {
        Some((l, v)) => (Some(l), Some(v)),
        None => (None, None),
    };
    Ok(ResolvedModpackTarget { mc_version, loader, loader_version })
}

/// Lays down `overrides/` then `client-overrides/` (never `server-overrides/`),
/// then downloads every file whose `env.client` isn't `"unsupported"`.
pub async fn install(app: &AppHandle, client: &reqwest::Client, zip_path: &Path, instance_dir: &Path) -> AppResult<()> {
    let bytes = read_zip_entry_bytes(zip_path, "modrinth.index.json")?;
    let index: Index = serde_json::from_slice(&bytes)?;

    {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| crate::error::AppError::Other(format!("bad mrpack archive: {e}")))?;
        extract_prefixed(&mut archive, "overrides/", instance_dir)?;
        extract_prefixed(&mut archive, "client-overrides/", instance_dir)?;
    }

    let wanted: Vec<&IndexFile> = index
        .files
        .iter()
        .filter(|f| f.env.as_ref().and_then(|e| e.client.as_deref()) != Some("unsupported"))
        .collect();

    let total = wanted.len();
    download::emit_progress(app, "modpack", 0, total);
    for (i, file) in wanted.into_iter().enumerate() {
        let dest = safe_join(instance_dir, &file.path)?;
        let Some(url) = file.downloads.first() else { continue };
        download::download_verified(client, url, &dest, file.hashes.sha1.as_deref()).await?;
        download::emit_progress(app, "modpack", i + 1, total);
    }

    Ok(())
}
