//! Installs a CurseForge modpack export zip: a `manifest.json` at the root
//! plus an overrides folder, verified against CurseForge/GDLauncher's docs.

use super::{extract_prefixed, read_zip_entry_bytes, ResolvedModpackTarget};
use crate::content::{curseforge, is_legacy_texturepacks, ContentKind};
use crate::error::AppResult;
use crate::minecraft::download;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use tauri::AppHandle;

#[derive(Debug, Deserialize)]
struct Manifest {
    minecraft: ManifestMinecraft,
    files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

#[derive(Debug, Deserialize)]
struct ManifestMinecraft {
    version: String,
    #[serde(rename = "modLoaders")]
    mod_loaders: Vec<ManifestModLoader>,
}

#[derive(Debug, Deserialize)]
struct ManifestModLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    // `projectID` is also in the manifest but unneeded: `resolve_files`
    // already carries the owning mod's id back as `project_id`.
    #[serde(rename = "fileID")]
    file_id: i64,
}

fn read_manifest(zip_path: &Path) -> AppResult<Manifest> {
    let bytes = read_zip_entry_bytes(zip_path, "manifest.json")?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// CurseForge's loader ids are already Strata's names (`"forge-47.2.20"`),
/// so this splits on the first `-` rather than a name table.
fn split_loader_id(id: &str) -> Option<(String, String)> {
    let (name, version) = id.split_once('-')?;
    Some((name.to_string(), version.to_string()))
}

pub fn resolve_target(zip_path: &Path) -> AppResult<ResolvedModpackTarget> {
    let manifest = read_manifest(zip_path)?;
    let primary = manifest.minecraft.mod_loaders.iter().find(|l| l.primary).or_else(|| manifest.minecraft.mod_loaders.first());
    let (loader, loader_version) = match primary.and_then(|l| split_loader_id(&l.id)) {
        Some((l, v)) => (Some(l), Some(v)),
        None => (None, None),
    };
    Ok(ResolvedModpackTarget { mc_version: manifest.minecraft.version, loader, loader_version })
}

/// Lays down the overrides folder, resolves every `projectID`/`fileID` pair
/// via CurseForge's batch endpoints, and downloads each into the right folder.
pub async fn install(
    app: &AppHandle,
    client: &reqwest::Client,
    api_key: &str,
    zip_path: &Path,
    instance_dir: &Path,
) -> AppResult<()> {
    let manifest = read_manifest(zip_path)?;

    {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| crate::error::AppError::Other(format!("bad modpack archive: {e}")))?;
        extract_prefixed(&mut archive, &format!("{}/", manifest.overrides), instance_dir)?;
    }

    let file_ids: Vec<i64> = manifest.files.iter().map(|f| f.file_id).collect();
    let mut resolved = curseforge::resolve_files(client, api_key, &file_ids).await?;

    let project_ids: Vec<i64> = resolved.iter().map(|f| f.project_id).collect::<HashSet<_>>().into_iter().collect();
    let classes = curseforge::resolve_mod_classes(client, api_key, &project_ids).await?;
    for file in &mut resolved {
        file.class_id = classes.get(&file.project_id).copied();
    }
    let known_classes = curseforge::resolve_known_classes(client, api_key).await?;
    let legacy = is_legacy_texturepacks(&manifest.minecraft.version);

    let total = resolved.len();
    download::emit_progress(app, "modpack", 0, total);
    for (i, file) in resolved.into_iter().enumerate() {
        let kind = file.class_id.and_then(|id| known_classes.get(&id).copied()).unwrap_or(ContentKind::Mod);
        let dest_dir = instance_dir.join(kind.folder(legacy));
        std::fs::create_dir_all(&dest_dir)?;
        let dest = dest_dir.join(&file.file_name);
        download::download_verified(client, &file.download_url, &dest, file.sha1.as_deref()).await?;
        download::emit_progress(app, "modpack", i + 1, total);
    }

    Ok(())
}
