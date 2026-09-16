//! Installs a modpack (`.mrpack` or a CurseForge zip) into a fresh instance.
//! See `mrpack.rs`/`curseforge_pack.rs` for the two formats.

pub mod curseforge_pack;
pub mod mrpack;

use crate::error::{AppError, AppResult};
use std::io::Read;
use std::path::Path;

/// What a pack's manifest says the instance should be created as, resolved
/// before the instance exists so `create_instance` gets it directly.
pub struct ResolvedModpackTarget {
    pub mc_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum PackFormat {
    Mrpack,
    CurseForge,
}

/// Peeks a zip for `modrinth.index.json` or `manifest.json` at the root to
/// tell the two supported formats apart without fully parsing either yet.
pub fn detect_format(zip_path: &Path) -> AppResult<PackFormat> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::Other(format!("not a valid zip: {e}")))?;
    if archive.by_name("modrinth.index.json").is_ok() {
        return Ok(PackFormat::Mrpack);
    }
    if archive.by_name("manifest.json").is_ok() {
        return Ok(PackFormat::CurseForge);
    }
    Err(AppError::Other(
        "Not a recognized modpack file. Expected a .mrpack or a CurseForge modpack zip.".into(),
    ))
}

/// Extracts every entry under `prefix` (e.g. `"overrides/"`) into
/// `dest_root`, stripping the prefix.
fn extract_prefixed(archive: &mut zip::ZipArchive<std::fs::File>, prefix: &str, dest_root: &Path) -> AppResult<()> {
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| AppError::Other(format!("bad zip entry: {e}")))?;
        let Some(rel) = entry.name().strip_prefix(prefix) else { continue };
        if rel.is_empty() {
            continue;
        }
        let out_path = crate::fsutil::safe_join(dest_root, rel, "modpack archive")?;
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out_file = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

fn read_zip_entry_bytes(zip_path: &Path, entry_name: &str) -> AppResult<Vec<u8>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::Other(format!("not a valid zip: {e}")))?;
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|e| AppError::Other(format!("modpack archive has no {entry_name}: {e}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}
