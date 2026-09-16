//! Migration importer: detects vanilla/CurseForge/Prism/MultiMC installs
//! and imports their content into a new Strata instance.

use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LauncherKind {
    Vanilla,
    Curseforge,
    Prism,
    Multimc,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstance {
    pub kind: LauncherKind,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    /// Absolute path to the content root, already resolved past any
    /// `.minecraft` wrapper.
    pub path: String,
    pub size_bytes: u64,
}

fn appdata_roaming() -> Option<PathBuf> {
    dirs::config_dir()
}

fn default_roots(kind: LauncherKind) -> Vec<PathBuf> {
    match kind {
        LauncherKind::Vanilla => appdata_roaming().map(|p| vec![p.join(".minecraft")]).unwrap_or_default(),
        LauncherKind::Curseforge => dirs::home_dir()
            .map(|p| vec![p.join("curseforge").join("minecraft").join("Instances")])
            .unwrap_or_default(),
        LauncherKind::Prism => appdata_roaming().map(|p| vec![p.join("PrismLauncher").join("instances")]).unwrap_or_default(),
        LauncherKind::Multimc => appdata_roaming().map(|p| vec![p.join("MultiMC").join("instances")]).unwrap_or_default(),
    }
}

/// Newest-modified entry under `versions/`; vanilla has no single "current
/// version" concept, so this is a best guess the user can change.
fn latest_version_dir(versions_dir: &Path) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, String)> = None;
    for entry in std::fs::read_dir(versions_dir).ok()?.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else { continue };
        if best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
            best = Some((modified, name));
        }
    }
    best.map(|(_, name)| name)
}

fn scan_vanilla(root: &Path) -> Option<DetectedInstance> {
    if !root.join("saves").exists() && !root.join("versions").exists() {
        return None;
    }
    let mc_version = latest_version_dir(&root.join("versions")).unwrap_or_else(|| "unknown".to_string());
    Some(DetectedInstance {
        kind: LauncherKind::Vanilla,
        name: "Vanilla Minecraft".to_string(),
        mc_version,
        loader: "vanilla".to_string(),
        loader_version: None,
        path: root.display().to_string(),
        size_bytes: crate::fsutil::dir_size(root),
    })
}

/// CurseForge's `baseModLoader.name` is like `"forge-36.2.22"`; the loader
/// family is a prefix on this string, not a separate field.
fn parse_curseforge_loader_name(name: &str) -> (String, Option<String>) {
    let lower = name.to_lowercase();
    for (prefix, key) in [("forge-", "forge"), ("fabric-", "fabric"), ("quilt-", "quilt"), ("neoforge-", "neoforge")] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return (key.to_string(), Some(rest.to_string()));
        }
    }
    ("vanilla".to_string(), None)
}

fn scan_curseforge_root(root: &Path) -> Vec<DetectedInstance> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let inst_dir = entry.path();
        let Ok(bytes) = std::fs::read(inst_dir.join("minecraftinstance.json")) else { continue };
        let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };

        let Some(mc_version) = json.get("gameVersion").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("Imported Instance").to_string();
        let (loader, loader_version) = json
            .pointer("/baseModLoader/name")
            .and_then(|v| v.as_str())
            .map(parse_curseforge_loader_name)
            .unwrap_or(("vanilla".to_string(), None));

        out.push(DetectedInstance {
            kind: LauncherKind::Curseforge,
            name,
            mc_version: mc_version.to_string(),
            loader,
            loader_version,
            path: inst_dir.display().to_string(),
            size_bytes: crate::fsutil::dir_size(&inst_dir),
        });
    }
    out
}

fn instance_cfg_name(inst_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(inst_dir.join("instance.cfg")).ok()?;
    text.lines().find_map(|l| l.strip_prefix("name=").map(|s| s.trim().to_string()))
}

/// Prism/MultiMC share the same `mmc-pack.json` component-list format.
/// Component uids beyond the two confirmed ones are matched by substring.
fn scan_mmc_style_root(root: &Path, kind: LauncherKind) -> Vec<DetectedInstance> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let inst_dir = entry.path();
        let Ok(bytes) = std::fs::read(inst_dir.join("mmc-pack.json")) else { continue };
        let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
        let Some(components) = json.get("components").and_then(|v| v.as_array()) else { continue };

        let mut mc_version = None;
        let mut loader = "vanilla".to_string();
        let mut loader_version = None;
        for c in components {
            let uid = c.get("uid").and_then(|v| v.as_str()).unwrap_or_default();
            let version = c.get("version").and_then(|v| v.as_str()).map(str::to_string);
            if uid == "net.minecraft" {
                mc_version = version;
            } else if uid == "net.minecraftforge" {
                loader = "forge".to_string();
                loader_version = version;
            } else if uid.contains("fabric") {
                loader = "fabric".to_string();
                loader_version = version;
            } else if uid.contains("quilt") {
                loader = "quilt".to_string();
                loader_version = version;
            } else if uid.contains("neoforge") {
                loader = "neoforge".to_string();
                loader_version = version;
            }
        }
        let Some(mc_version) = mc_version else { continue };

        // Confirmed real-world inconsistency: some installs use
        // `.minecraft`, others just `minecraft`.
        let content_root = [inst_dir.join(".minecraft"), inst_dir.join("minecraft")]
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| inst_dir.join(".minecraft"));

        let name = instance_cfg_name(&inst_dir).unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());

        out.push(DetectedInstance {
            kind,
            name,
            mc_version,
            loader,
            loader_version,
            path: content_root.display().to_string(),
            size_bytes: crate::fsutil::dir_size(&content_root),
        });
    }
    out
}

pub fn scan(kind: LauncherKind, root_override: Option<&Path>) -> Vec<DetectedInstance> {
    let roots: Vec<PathBuf> = match root_override {
        Some(p) => vec![p.to_path_buf()],
        None => default_roots(kind),
    };

    let mut found = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        match kind {
            LauncherKind::Vanilla => found.extend(scan_vanilla(&root)),
            LauncherKind::Curseforge => found.extend(scan_curseforge_root(&root)),
            LauncherKind::Prism | LauncherKind::Multimc => found.extend(scan_mmc_style_root(&root, kind)),
        }
    }
    found
}

const COPY_FOLDERS: &[&str] = &["mods", "resourcepacks", "shaderpacks", "saves", "screenshots"];

/// Copies the known content folders into a fresh Strata instance dir.
/// Doesn't copy `config`/`logs`/`libraries`; Strata resolves the loader itself.
pub fn import_content(source_root: &Path, dest_instance_dir: &Path) -> AppResult<()> {
    for folder in COPY_FOLDERS {
        let src = source_root.join(folder);
        if !src.is_dir() {
            continue;
        }
        crate::fsutil::copy_dir_recursive(&src, &dest_instance_dir.join(folder))?;
    }
    Ok(())
}
