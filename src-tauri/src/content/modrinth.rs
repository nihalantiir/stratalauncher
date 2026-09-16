use super::{ContentVersionEntry, ResolvedVersion};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::Mutex;

const API_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub author: Option<String>,
    /// "required" | "optional" | "unsupported" | "unknown"; `None` for
    /// CurseForge, which has no equivalent field.
    pub client_side: Option<String>,
    pub server_side: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<RawHit>,
}

#[derive(Debug, Deserialize)]
struct RawHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    downloads: u64,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    client_side: Option<String>,
    #[serde(default)]
    server_side: Option<String>,
}

// Loader and user-picked categories are separate AND-groups (each an
// OR-list): "loader AND (Tech OR Magic)", not "loader OR Tech OR Magic".
fn build_facets(project_type: &str, mc_version: Option<&str>, loader: Option<&str>, categories: &[String]) -> String {
    let mut groups = vec![format!(r#"["project_type:{project_type}"]"#)];
    if let Some(v) = mc_version {
        groups.push(format!(r#"["versions:{v}"]"#));
    }
    if let Some(l) = loader {
        groups.push(format!(r#"["categories:{l}"]"#));
    }
    if !categories.is_empty() {
        let ors: Vec<String> = categories.iter().map(|c| format!(r#""categories:{c}""#)).collect();
        groups.push(format!("[{}]", ors.join(",")));
    }
    format!("[{}]", groups.join(","))
}

/// Real values for Modrinth's `index` param: relevance (default),
/// downloads, follows, newest, updated. Passed straight through.
pub async fn search(
    client: &reqwest::Client,
    project_type: &str,
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
    categories: &[String],
    sort: &str,
    offset: u32,
) -> AppResult<Vec<SearchHit>> {
    let facets = build_facets(project_type, mc_version, loader, categories);
    let offset_str = offset.to_string();
    let resp = client
        .get(format!("{API_BASE}/search"))
        .query(&[
            ("query", query),
            ("facets", &facets),
            ("index", sort),
            ("limit", "30"),
            ("offset", &offset_str),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<SearchResponse>()
        .await?;

    Ok(resp
        .hits
        .into_iter()
        .map(|h| SearchHit {
            project_id: h.project_id,
            slug: h.slug,
            title: h.title,
            author: h.author,
            description: h.description,
            icon_url: h.icon_url,
            downloads: h.downloads,
            client_side: h.client_side,
            server_side: h.server_side,
        })
        .collect())
}

/// One entry from Modrinth's `/tag/category` list; `header` is Modrinth's
/// own grouping, used to section the sidebar the same way Modrinth's site does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTag {
    pub name: String,
    pub project_type: String,
    pub header: String,
}

static CATEGORY_CACHE: OnceLock<Mutex<Option<Vec<CategoryTag>>>> = OnceLock::new();

fn category_cache() -> &'static Mutex<Option<Vec<CategoryTag>>> {
    CATEGORY_CACHE.get_or_init(|| Mutex::new(None))
}

/// The full tag list is small and effectively static, fetched once per
/// process and reused, same shape as CurseForge's class-id cache.
pub async fn categories(client: &reqwest::Client, project_type: &str) -> AppResult<Vec<CategoryTag>> {
    {
        let cache = category_cache().lock().await;
        if let Some(all) = cache.as_ref() {
            return Ok(all.iter().filter(|c| c.project_type == project_type).cloned().collect());
        }
    }

    let all: Vec<CategoryTag> = client
        .get(format!("{API_BASE}/tag/category"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let filtered = all.iter().filter(|c| c.project_type == project_type).cloned().collect();
    *category_cache().lock().await = Some(all);
    Ok(filtered)
}

#[derive(Debug, Clone, Deserialize)]
struct VersionFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    hashes: VersionHashes,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionHashes {
    sha1: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawVersion {
    id: String,
    version_number: String,
    date_published: String,
    files: Vec<VersionFile>,
}

/// The newest published version matching this instance's version/loader;
/// what "Install"/"Update" resolve to. `mc_version: None` drops the constraint.
pub async fn latest_matching_version(
    client: &reqwest::Client,
    project_id: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> AppResult<Option<ResolvedVersion>> {
    let mut url = format!("{API_BASE}/project/{project_id}/version?");
    let mut parts = Vec::new();
    if let Some(v) = mc_version {
        parts.push(format!("game_versions=[\"{v}\"]"));
    }
    if let Some(l) = loader {
        parts.push(format!("loaders=[\"{l}\"]"));
    }
    url.push_str(&parts.join("&"));

    let mut versions = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<RawVersion>>()
        .await?;

    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));

    let Some(top) = versions.into_iter().next() else {
        return Ok(None);
    };

    let file = top
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| top.files.first())
        .ok_or_else(|| AppError::Other(format!("Modrinth version {} has no files", top.id)))?;

    Ok(Some(ResolvedVersion {
        version_id: top.id,
        version_number: top.version_number,
        file_url: file.url.clone(),
        filename: file.filename.clone(),
        sha1: file.hashes.sha1.clone(),
    }))
}

#[derive(Debug, Deserialize)]
struct RawVersionFull {
    id: String,
    version_number: String,
    name: String,
    version_type: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
    files: Vec<VersionFile>,
}

/// A project's complete version history for the "Change Version" picker,
/// deliberately unfiltered by game version/loader.
pub async fn list_versions(client: &reqwest::Client, project_id: &str) -> AppResult<Vec<ContentVersionEntry>> {
    let versions: Vec<RawVersionFull> = client
        .get(format!("{API_BASE}/project/{project_id}/version"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut list: Vec<ContentVersionEntry> = versions
        .into_iter()
        .filter_map(|v| {
            let file = v.files.iter().find(|f| f.primary).or_else(|| v.files.first())?;
            Some(ContentVersionEntry {
                version_id: v.id,
                version_number: v.version_number,
                name: v.name,
                version_type: v.version_type,
                game_versions: v.game_versions,
                loaders: v.loaders,
                date_published: v.date_published,
                file_url: file.url.clone(),
                filename: file.filename.clone(),
                sha1: file.hashes.sha1.clone(),
            })
        })
        .collect();
    list.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    Ok(list)
}
