pub mod curseforge;
pub mod modrinth;

use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The resolved download target for one project's best-matching file,
/// whatever source (Modrinth/CurseForge) it came from.
pub struct ResolvedVersion {
    pub version_id: String,
    pub version_number: String,
    pub file_url: String,
    pub filename: String,
    pub sha1: Option<String>,
}

/// A search result annotated with which source it came from; the source
/// badge in the Download modal is driven by this field alone.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchHit {
    pub source: &'static str,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub author: Option<String>,
    /// "required" | "optional" | "unsupported" | "unknown" from Modrinth;
    /// always `None` for a CurseForge hit, which has no equivalent field.
    pub client_side: Option<String>,
    pub server_side: Option<String>,
}

/// One entry in a project's full, unfiltered version history for the
/// "Change Version" picker; carries everything needed to install it directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentVersionEntry {
    pub version_id: String,
    pub version_number: String,
    pub name: String,
    /// "release" | "beta" | "alpha" on Modrinth; CurseForge's numeric
    /// `releaseType` (1/2/3) is mapped onto the same three strings.
    pub version_type: String,
    pub game_versions: Vec<String>,
    /// Always empty for a CurseForge entry, whose file objects mix loader
    /// names into `gameVersions` instead of keeping them separate.
    pub loaders: Vec<String>,
    pub date_published: String,
    pub file_url: String,
    pub filename: String,
    pub sha1: Option<String>,
}

const DISABLED_SUFFIX: &str = ".disabled";
const METADATA_FILE: &str = ".strata-content.json";

/// Versions through `1.5.2` use the old `texturepacks` folder, not
/// `resourcepacks` (1.6 renamed the feature); a finite, closed id list.
pub fn is_legacy_texturepacks(mc_version: &str) -> bool {
    if mc_version.starts_with("a1.") || mc_version.starts_with("b1.") || mc_version.starts_with("c0.") || mc_version.starts_with("rd-") {
        return true;
    }
    matches!(
        mc_version,
        "1.0" | "1.0.1"
            | "1.1"
            | "1.2.1"
            | "1.2.2"
            | "1.2.3"
            | "1.2.4"
            | "1.2.5"
            | "1.3.1"
            | "1.3.2"
            | "1.4.2"
            | "1.4.4"
            | "1.4.5"
            | "1.4.6"
            | "1.4.7"
            | "1.5"
            | "1.5.1"
            | "1.5.2"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Mod,
    Resourcepack,
    Shader,
}

impl ContentKind {
    /// `legacy` only changes anything for `Resourcepack` (pre-1.6 reads
    /// `texturepacks`, not `resourcepacks`); mods/shaders have no such split.
    pub fn folder(&self, legacy: bool) -> &'static str {
        match self {
            ContentKind::Mod => "mods",
            ContentKind::Resourcepack if legacy => "texturepacks",
            ContentKind::Resourcepack => "resourcepacks",
            ContentKind::Shader => "shaderpacks",
        }
    }

    pub fn modrinth_project_type(&self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::Resourcepack => "resourcepack",
            ContentKind::Shader => "shader",
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContentMeta {
    project_id: String,
    version_id: String,
    title: String,
    version_number: String,
    source: String,
    installed_at: String,
    #[serde(default)]
    icon_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MetadataFile {
    #[serde(default)]
    mods: HashMap<String, ContentMeta>,
    #[serde(default)]
    resourcepacks: HashMap<String, ContentMeta>,
    #[serde(default)]
    shaders: HashMap<String, ContentMeta>,
}

impl MetadataFile {
    fn section(&self, kind: ContentKind) -> &HashMap<String, ContentMeta> {
        match kind {
            ContentKind::Mod => &self.mods,
            ContentKind::Resourcepack => &self.resourcepacks,
            ContentKind::Shader => &self.shaders,
        }
    }

    fn section_mut(&mut self, kind: ContentKind) -> &mut HashMap<String, ContentMeta> {
        match kind {
            ContentKind::Mod => &mut self.mods,
            ContentKind::Resourcepack => &mut self.resourcepacks,
            ContentKind::Shader => &mut self.shaders,
        }
    }
}

fn metadata_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join(METADATA_FILE)
}

fn read_metadata(instance_dir: &Path) -> MetadataFile {
    std::fs::read(metadata_path(instance_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_metadata(instance_dir: &Path, meta: &MetadataFile) -> AppResult<()> {
    std::fs::write(metadata_path(instance_dir), serde_json::to_vec_pretty(meta)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledItem {
    pub filename: String,
    pub title: String,
    pub version_number: String,
    pub size_bytes: u64,
    pub enabled: bool,
    pub source: Option<String>,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub icon_url: Option<String>,
}

fn strip_disabled(name: &str) -> &str {
    name.strip_suffix(DISABLED_SUFFIX).unwrap_or(name)
}

pub fn content_dir(instance_dir: &Path, kind: ContentKind, legacy: bool) -> AppResult<PathBuf> {
    let dir = instance_dir.join(kind.folder(legacy));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn list_installed(instance_dir: &Path, kind: ContentKind, legacy: bool) -> AppResult<Vec<InstalledItem>> {
    let dir = instance_dir.join(kind.folder(legacy));
    std::fs::create_dir_all(&dir)?;
    let metadata = read_metadata(instance_dir);
    let section = metadata.section(kind);

    let mut items = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let on_disk_name = entry.file_name().to_string_lossy().to_string();
        let logical_name = strip_disabled(&on_disk_name).to_string();
        let enabled = on_disk_name == logical_name;
        let size_bytes = entry.metadata()?.len();

        let meta = section.get(&logical_name);
        items.push(InstalledItem {
            filename: logical_name.clone(),
            title: meta.map(|m| m.title.clone()).unwrap_or(logical_name),
            version_number: meta.map(|m| m.version_number.clone()).unwrap_or_default(),
            size_bytes,
            enabled,
            source: meta.map(|m| m.source.clone()),
            project_id: meta.map(|m| m.project_id.clone()),
            version_id: meta.map(|m| m.version_id.clone()),
            icon_url: meta.and_then(|m| m.icon_url.clone()),
        });
    }

    items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(items)
}

pub fn toggle_enabled(instance_dir: &Path, kind: ContentKind, legacy: bool, filename: &str) -> AppResult<()> {
    let dir = instance_dir.join(kind.folder(legacy));
    let enabled_path = dir.join(filename);
    let disabled_path = dir.join(format!("{filename}{DISABLED_SUFFIX}"));

    if enabled_path.exists() {
        std::fs::rename(&enabled_path, &disabled_path)?;
    } else if disabled_path.exists() {
        std::fs::rename(&disabled_path, &enabled_path)?;
    }
    Ok(())
}

pub fn remove(instance_dir: &Path, kind: ContentKind, legacy: bool, filename: &str) -> AppResult<()> {
    let dir = instance_dir.join(kind.folder(legacy));
    for candidate in [dir.join(filename), dir.join(format!("{filename}{DISABLED_SUFFIX}"))] {
        if candidate.exists() {
            std::fs::remove_file(candidate)?;
        }
    }

    let mut metadata = read_metadata(instance_dir);
    metadata.section_mut(kind).remove(filename);
    write_metadata(instance_dir, &metadata)?;
    Ok(())
}

pub fn record_install(
    instance_dir: &Path,
    kind: ContentKind,
    filename: &str,
    project_id: &str,
    version_id: &str,
    title: &str,
    version_number: &str,
    source: &str,
    icon_url: Option<&str>,
) -> AppResult<()> {
    let mut metadata = read_metadata(instance_dir);
    metadata.section_mut(kind).insert(
        filename.to_string(),
        ContentMeta {
            project_id: project_id.to_string(),
            version_id: version_id.to_string(),
            title: title.to_string(),
            version_number: version_number.to_string(),
            source: source.to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            icon_url: icon_url.map(str::to_string),
        },
    );
    write_metadata(instance_dir, &metadata)
}
