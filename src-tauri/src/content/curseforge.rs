//! CurseForge Core API client. Every function takes an explicit API key;
//! category (class) ids are looked up by slug at runtime and cached.

use super::{ContentKind, ContentVersionEntry, ResolvedVersion};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};

const API_BASE: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: i64 = 432;

/// CurseForge publishes no fixed rate limit; this is a conservative default
/// that caps in-flight requests and backs off on a real 429.
const MAX_CONCURRENT_REQUESTS: usize = 4;
const MAX_RETRIES: u32 = 3;

static RATE_LIMITER: OnceLock<Semaphore> = OnceLock::new();

fn rate_limiter() -> &'static Semaphore {
    RATE_LIMITER.get_or_init(|| Semaphore::new(MAX_CONCURRENT_REQUESTS))
}

/// Every CurseForge call goes through here: holds a concurrency permit for
/// the whole request and retries a real 429 via `Retry-After` or backoff.
async fn send_limited(request: reqwest::RequestBuilder) -> AppResult<reqwest::Response> {
    let _permit = rate_limiter().acquire().await.expect("semaphore never closed");
    let mut attempt = 0u32;
    let mut current = request;
    loop {
        let retry_builder = current.try_clone();
        let resp = current.send().await?;
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < MAX_RETRIES {
            let Some(next) = retry_builder else {
                return Ok(resp); // body wasn't cloneable, surface the 429 as-is rather than retry blind
            };
            let wait = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or_else(|| Duration::from_millis(500 * 2u64.pow(attempt)));
            attempt += 1;
            tokio::time::sleep(wait).await;
            current = next;
            continue;
        }
        if !resp.status().is_success() {
            // CurseForge's own error body is real, useful text a bare
            // reqwest status error would otherwise swallow.
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!("CurseForge {status}: {body}")));
        }
        return Ok(resp);
    }
}

/// CurseForge's `ModLoaderType` enum.
fn loader_type_id(loader: &str) -> Option<i64> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

fn class_slug(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mc-mods",
        ContentKind::Resourcepack => "texture-packs",
        ContentKind::Shader => "shaders",
    }
}

#[derive(Debug, Deserialize)]
struct CategoriesResponse {
    data: Vec<CategoryEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryEntry {
    id: i64,
    name: String,
    slug: String,
    #[serde(default)]
    is_class: bool,
}

static CLASS_ID_CACHE: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn class_id_cache() -> &'static Mutex<HashMap<String, i64>> {
    CLASS_ID_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn resolve_class_id(client: &reqwest::Client, api_key: &str, slug: &str) -> AppResult<i64> {
    {
        let cache = class_id_cache().lock().await;
        if let Some(id) = cache.get(slug) {
            return Ok(*id);
        }
    }

    let resp: CategoriesResponse = send_limited(
        client
            .get(format!("{API_BASE}/categories"))
            .query(&[("gameId", MINECRAFT_GAME_ID.to_string()), ("classesOnly", "true".to_string())])
            .header("x-api-key", api_key),
    )
    .await?
    .json()
    .await?;

    let mut cache = class_id_cache().lock().await;
    for entry in &resp.data {
        if entry.is_class {
            cache.insert(entry.slug.clone(), entry.id);
        }
    }

    cache
        .get(slug)
        .copied()
        .ok_or_else(|| AppError::Other(format!("CurseForge has no '{slug}' category for Minecraft right now")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfCategory {
    pub id: i64,
    pub name: String,
}

/// Subcategories under one class slug. Not cached like the class ids: one
/// lookup per modal open isn't hot enough to be worth it.
async fn categories_for_slug(client: &reqwest::Client, api_key: &str, slug: &str) -> AppResult<Vec<CfCategory>> {
    let class_id = resolve_class_id(client, api_key, slug).await?;
    let resp: CategoriesResponse = send_limited(
        client
            .get(format!("{API_BASE}/categories"))
            .query(&[("gameId", MINECRAFT_GAME_ID.to_string()), ("classId", class_id.to_string())])
            .header("x-api-key", api_key),
    )
    .await?
    .json()
    .await?;

    Ok(resp
        .data
        .into_iter()
        .filter(|c| !c.is_class && c.id != class_id)
        .map(|c| CfCategory { id: c.id, name: c.name })
        .collect())
}

pub async fn categories(client: &reqwest::Client, api_key: &str, kind: ContentKind) -> AppResult<Vec<CfCategory>> {
    categories_for_slug(client, api_key, class_slug(kind)).await
}

pub async fn modpack_categories(client: &reqwest::Client, api_key: &str) -> AppResult<Vec<CfCategory>> {
    categories_for_slug(client, api_key, "modpacks").await
}

/// CurseForge's `sortField` enum, verified against their docs. Only values
/// meaningful as a user-facing sort are exposed.
fn sort_field_id(sort: &str) -> Option<i64> {
    match sort {
        "featured" => Some(1),
        "popularity" => Some(2),
        "lastUpdated" => Some(3),
        "name" => Some(4),
        "author" => Some(5),
        "totalDownloads" => Some(6),
        "rating" => Some(11),
        _ => None,
    }
}

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
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<RawMod>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMod {
    id: i64,
    slug: String,
    name: String,
    summary: String,
    logo: Option<RawLogo>,
    #[serde(default)]
    download_count: u64,
    /// `false` means the author opted out of 3rd-party distribution;
    /// filtered out of search results rather than surfaced as a dead end.
    #[serde(default)]
    allow_mod_distribution: Option<bool>,
    #[serde(default)]
    authors: Vec<RawAuthor>,
}

#[derive(Debug, Deserialize)]
struct RawLogo {
    url: String,
}

#[derive(Debug, Deserialize)]
struct RawAuthor {
    name: String,
}

/// Shared param-building for `search()` and `search_modpacks()`, which only
/// differ in which class id they resolve and whether a loader facet applies.
#[allow(clippy::too_many_arguments)]
async fn search_class(
    client: &reqwest::Client,
    api_key: &str,
    class_id: i64,
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
    category_ids: &[i64],
    sort: &str,
    offset: u32,
) -> AppResult<Vec<SearchHit>> {
    let mut params: Vec<(&str, String)> = vec![
        ("gameId", MINECRAFT_GAME_ID.to_string()),
        ("classId", class_id.to_string()),
        ("searchFilter", query.to_string()),
        ("pageSize", "30".to_string()),
        ("index", offset.to_string()),
        ("sortOrder", "desc".to_string()),
    ];
    if let Some(v) = mc_version {
        params.push(("gameVersion", v.to_string()));
    }
    if let Some(id) = loader.and_then(loader_type_id) {
        params.push(("modLoaderType", id.to_string()));
    }
    // categoryIds takes precedence over categoryId when both are present;
    // always send the array form.
    if !category_ids.is_empty() {
        let ids: Vec<String> = category_ids.iter().map(|id| id.to_string()).collect();
        params.push(("categoryIds", format!("[{}]", ids.join(","))));
    }
    if let Some(id) = sort_field_id(sort) {
        params.push(("sortField", id.to_string()));
    }

    let resp: SearchResponse = send_limited(
        client
            .get(format!("{API_BASE}/mods/search"))
            .query(&params)
            .header("x-api-key", api_key),
    )
    .await?
    .json()
    .await?;

    Ok(raw_hits_to_search_hits(resp.data))
}

pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    kind: ContentKind,
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
    category_ids: &[i64],
    sort: &str,
    offset: u32,
) -> AppResult<Vec<SearchHit>> {
    let class_id = resolve_class_id(client, api_key, class_slug(kind)).await?;
    search_class(client, api_key, class_id, query, mc_version, loader, category_ids, sort, offset).await
}

fn raw_hits_to_search_hits(data: Vec<RawMod>) -> Vec<SearchHit> {
    data.into_iter()
        .filter(|m| m.allow_mod_distribution != Some(false))
        .map(|m| SearchHit {
            project_id: m.id.to_string(),
            slug: m.slug,
            title: m.name,
            description: m.summary,
            icon_url: m.logo.map(|l| l.url),
            downloads: m.download_count,
            author: m.authors.into_iter().next().map(|a| a.name),
        })
        .collect()
}

/// Same shape as `search()` but against the "modpacks" class instead of a
/// `ContentKind`, which modpacks don't participate in.
pub async fn search_modpacks(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    mc_version: Option<&str>,
    category_ids: &[i64],
    sort: &str,
    offset: u32,
) -> AppResult<Vec<SearchHit>> {
    let class_id = resolve_class_id(client, api_key, "modpacks").await?;
    search_class(client, api_key, class_id, query, mc_version, None, category_ids, sort, offset).await
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    data: Vec<RawFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFileDependency {
    mod_id: i64,
    relation_type: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    id: i64,
    #[serde(default)]
    mod_id: i64,
    display_name: String,
    file_name: String,
    file_date: String,
    download_url: Option<String>,
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    hashes: Vec<RawHash>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    release_type: i64,
    #[serde(default)]
    dependencies: Vec<RawFileDependency>,
}

/// CurseForge's `FileRelationType` enum; 3 is the only one that means
/// "install this too or the mod won't work."
const REQUIRED_DEPENDENCY: i64 = 3;

/// CurseForge's `FileReleaseType` enum.
fn release_type_label(release_type: i64) -> &'static str {
    match release_type {
        2 => "beta",
        3 => "alpha",
        _ => "release",
    }
}

#[derive(Debug, Deserialize)]
struct RawHash {
    value: String,
    algo: i64,
}

/// The newest file matching this instance's version/loader; what "Install"
/// and "Update" resolve to. `mc_version: None` drops the constraint entirely.
pub async fn latest_matching_version(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> AppResult<Option<ResolvedVersion>> {
    let mut params: Vec<(&str, String)> = vec![("pageSize", "20".to_string())];
    if let Some(v) = mc_version {
        params.push(("gameVersion", v.to_string()));
    }
    if let Some(id) = loader.and_then(loader_type_id) {
        params.push(("modLoaderType", id.to_string()));
    }

    let resp: FilesResponse = send_limited(
        client
            .get(format!("{API_BASE}/mods/{mod_id}/files"))
            .query(&params)
            .header("x-api-key", api_key),
    )
    .await?
    .json()
    .await?;

    let mut files: Vec<RawFile> = resp
        .data
        .into_iter()
        .filter(|f| f.is_available && f.download_url.is_some())
        .collect();
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));

    let Some(top) = files.into_iter().next() else {
        return Ok(None);
    };

    // CurseForge's HashAlgo enum: 1 = Sha1, 2 = Md5. Best-effort; a wrong
    // guess just leaves sha1 None and download_verified() skips the check.
    let sha1 = top.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone());
    let file_url = top.download_url.expect("filtered to Some above");
    let dependency_project_ids = top
        .dependencies
        .iter()
        .filter(|d| d.relation_type == REQUIRED_DEPENDENCY)
        .map(|d| d.mod_id.to_string())
        .collect();

    Ok(Some(ResolvedVersion {
        version_id: top.id.to_string(),
        version_number: top.display_name,
        file_url,
        filename: top.file_name,
        sha1,
        dependency_project_ids,
    }))
}

/// A mod's complete file history for the "Change Version" picker; capped
/// to CurseForge's first page rather than paginating fully.
pub async fn list_versions(client: &reqwest::Client, api_key: &str, mod_id: &str) -> AppResult<Vec<ContentVersionEntry>> {
    let resp: FilesResponse = send_limited(
        client
            .get(format!("{API_BASE}/mods/{mod_id}/files"))
            .query(&[("pageSize", "50")])
            .header("x-api-key", api_key),
    )
    .await?
    .json()
    .await?;

    let mut list: Vec<ContentVersionEntry> = resp
        .data
        .into_iter()
        .filter(|f| f.is_available && f.download_url.is_some())
        .map(|f| {
            let sha1 = f.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone());
            ContentVersionEntry {
                version_id: f.id.to_string(),
                version_number: f.display_name.clone(),
                name: f.display_name,
                version_type: release_type_label(f.release_type).to_string(),
                game_versions: f.game_versions,
                loaders: Vec::new(),
                date_published: f.file_date,
                file_url: f.download_url.expect("filtered to Some above"),
                filename: f.file_name,
                sha1,
            }
        })
        .collect();
    list.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    Ok(list)
}

/// One file resolved from a modpack manifest's `projectID`/`fileID` pair,
/// plus the owning mod's class so the installer can route it correctly.
pub struct ResolvedPackFile {
    pub project_id: i64,
    pub file_name: String,
    pub download_url: String,
    pub sha1: Option<String>,
    /// From the owning mod's `classId` (see `resolve_mod_classes`); `None`
    /// falls back to treating it as a mod.
    pub class_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BatchFilesResponse {
    data: Vec<RawFile>,
}

/// Resolves every `fileID` in a modpack manifest to a download URL in one
/// batch request. `class_id` is filled in separately by `resolve_mod_classes`.
pub async fn resolve_files(client: &reqwest::Client, api_key: &str, file_ids: &[i64]) -> AppResult<Vec<ResolvedPackFile>> {
    if file_ids.is_empty() {
        return Ok(Vec::new());
    }
    let resp: BatchFilesResponse = send_limited(
        client
            .post(format!("{API_BASE}/mods/files"))
            .header("x-api-key", api_key)
            .json(&serde_json::json!({ "fileIds": file_ids })),
    )
    .await?
    .json()
    .await?;

    Ok(resp
        .data
        .into_iter()
        .filter(|f| f.is_available && f.download_url.is_some())
        .map(|f| ResolvedPackFile {
            project_id: f.mod_id,
            file_name: f.file_name,
            download_url: f.download_url.expect("filtered to Some above"),
            sha1: f.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone()),
            class_id: None,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct ModsResponse {
    data: Vec<RawModClass>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawModClass {
    id: i64,
    class_id: i64,
}

/// The 3 class ids a modpack's files can belong to, cached like every
/// other class lookup here, so the installer can route files correctly.
pub async fn resolve_known_classes(client: &reqwest::Client, api_key: &str) -> AppResult<HashMap<i64, ContentKind>> {
    let mut map = HashMap::new();
    for (kind, slug) in [
        (ContentKind::Mod, "mc-mods"),
        (ContentKind::Resourcepack, "texture-packs"),
        (ContentKind::Shader, "shaders"),
    ] {
        if let Ok(id) = resolve_class_id(client, api_key, slug).await {
            map.insert(id, kind);
        }
    }
    Ok(map)
}

/// `modId -> classId` for a batch of mods, one call per distinct project
/// instead of one per file. A missing mod id just leaves its class unresolved.
pub async fn resolve_mod_classes(client: &reqwest::Client, api_key: &str, mod_ids: &[i64]) -> AppResult<HashMap<i64, i64>> {
    if mod_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let resp: ModsResponse = send_limited(
        client
            .post(format!("{API_BASE}/mods"))
            .header("x-api-key", api_key)
            .json(&serde_json::json!({ "modIds": mod_ids })),
    )
    .await?
    .json()
    .await?;

    Ok(resp.data.into_iter().map(|m| (m.id, m.class_id)).collect())
}
