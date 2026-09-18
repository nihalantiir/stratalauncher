use crate::content::{
    self, curseforge, loader_facet, modrinth, ContentKind, ContentSearchHit, ContentVersionEntry, InstalledItem, ResolvedVersion,
};
use crate::db::InstancesRepo;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use futures_util::StreamExt;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentCategory {
    pub id: String,
    pub name: String,
    /// Modrinth sections its category list (categories/features/etc);
    /// `None` for CurseForge, which is flat.
    pub group: Option<String>,
}

pub(crate) fn prettify(slug: &str) -> String {
    slug.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The category list a provider actually offers for one content kind,
/// straight from its own taxonomy, never a fixed app-owned list.
#[tauri::command]
pub async fn list_content_categories(
    state: State<'_, AppState>,
    source: String,
    kind: ContentKind,
) -> AppResult<Vec<ContentCategory>> {
    match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            let cats = curseforge::categories(&state.http, &api_key, kind).await?;
            Ok(cats
                .into_iter()
                .map(|c| ContentCategory { id: c.id.to_string(), name: c.name, group: None })
                .collect())
        }
        _ => {
            let tags = modrinth::categories(&state.http, kind.modrinth_project_type()).await?;
            Ok(tags
                .into_iter()
                .map(|c| ContentCategory { id: c.name.clone(), name: prettify(&c.name), group: Some(prettify(&c.header)) })
                .collect())
        }
    }
}

fn get_instance(state: &State<'_, AppState>, instance_id: &str) -> AppResult<crate::db::models::Instance> {
    let conn = state.db.0.lock().unwrap();
    InstancesRepo::get(&conn, instance_id)?.ok_or_else(|| AppError::NotFound(format!("instance {instance_id}")))
}

#[tauri::command]
pub fn list_installed_content(state: State<'_, AppState>, instance_id: String, kind: ContentKind) -> AppResult<Vec<InstalledItem>> {
    let legacy = content::is_legacy_texturepacks(&get_instance(&state, &instance_id)?.mc_version);
    content::list_installed(&crate::paths::instance_dir(&instance_id), kind, legacy)
}

#[tauri::command]
pub fn toggle_content(state: State<'_, AppState>, instance_id: String, kind: ContentKind, filename: String) -> AppResult<()> {
    let legacy = content::is_legacy_texturepacks(&get_instance(&state, &instance_id)?.mc_version);
    content::toggle_enabled(&crate::paths::instance_dir(&instance_id), kind, legacy, &filename)
}

#[tauri::command]
pub fn remove_content(state: State<'_, AppState>, instance_id: String, kind: ContentKind, filename: String) -> AppResult<()> {
    let legacy = content::is_legacy_texturepacks(&get_instance(&state, &instance_id)?.mc_version);
    content::remove(&crate::paths::instance_dir(&instance_id), kind, legacy, &filename)
}

#[tauri::command]
pub fn get_content_dir(state: State<'_, AppState>, instance_id: String, kind: ContentKind) -> AppResult<String> {
    let legacy = content::is_legacy_texturepacks(&get_instance(&state, &instance_id)?.mc_version);
    let dir = content::content_dir(&crate::paths::instance_dir(&instance_id), kind, legacy)?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn curseforge_configured() -> bool {
    crate::config::curseforge_api_key().is_some()
}

#[tauri::command]
pub async fn search_content(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
    source: String,
    query: String,
    sort: String,
    categories: Vec<String>,
    mc_version: String,
    offset: u32,
) -> AppResult<Vec<ContentSearchHit>> {
    let instance = get_instance(&state, &instance_id)?;
    let loader = loader_facet(kind, &instance.loader);
    // mc_version is whatever the sidebar's version picker is set to, not
    // necessarily this instance's own real version.
    let mc_version = Some(mc_version.as_str());

    match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            let category_ids: Vec<i64> = categories.iter().filter_map(|c| c.parse::<i64>().ok()).collect();
            let hits = curseforge::search(&state.http, &api_key, kind, &query, mc_version, loader.as_deref(), &category_ids, &sort, offset).await?;
            Ok(hits
                .into_iter()
                .map(|h| ContentSearchHit {
                    source: "curseforge",
                    project_id: h.project_id,
                    slug: h.slug,
                    title: h.title,
                    description: h.description,
                    icon_url: h.icon_url,
                    downloads: h.downloads,
                    author: h.author,
                    client_side: None,
                    server_side: None,
                })
                .collect())
        }
        _ => {
            let hits = modrinth::search(&state.http, kind.modrinth_project_type(), &query, mc_version, loader.as_deref(), &categories, &sort, offset)
                .await?;
            Ok(hits
                .into_iter()
                .map(|h| ContentSearchHit {
                    source: "modrinth",
                    project_id: h.project_id,
                    slug: h.slug,
                    title: h.title,
                    description: h.description,
                    icon_url: h.icon_url,
                    downloads: h.downloads,
                    author: h.author,
                    client_side: h.client_side,
                    server_side: h.server_side,
                })
                .collect())
        }
    }
}

/// Shared by `install_content` and `install_content_version`: everything
/// past "which file" is identical (download-verify, record, return it).
async fn install_resolved(
    state: &State<'_, AppState>,
    instance_id: &str,
    kind: ContentKind,
    resolved: ResolvedVersion,
    project_id: &str,
    title: &str,
    icon_url: Option<&str>,
    source: &str,
) -> AppResult<InstalledItem> {
    let instance = get_instance(state, instance_id)?;
    // The instance's own pinned version, not whatever the search/version
    // picker is pointed at (see search_content's comment).
    let legacy = content::is_legacy_texturepacks(&instance.mc_version);

    let instance_dir = crate::paths::instance_dir(instance_id);
    let dest_dir = instance_dir.join(kind.folder(legacy));
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(&resolved.filename);

    crate::minecraft::download::download_verified(&state.http, &resolved.file_url, &dest, resolved.sha1.as_deref(), None)
        .await?;

    content::record_install(
        &instance_dir,
        kind,
        &resolved.filename,
        project_id,
        &resolved.version_id,
        title,
        &resolved.version_number,
        source,
        icon_url,
    )?;

    content::list_installed(&instance_dir, kind, legacy)?
        .into_iter()
        .find(|i| i.filename == resolved.filename)
        .ok_or_else(|| AppError::Other("installed item vanished".into()))
}

#[tauri::command]
pub async fn install_content(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
    source: String,
    project_id: String,
    title: String,
    icon_url: Option<String>,
    mc_version: String,
) -> AppResult<InstalledItem> {
    let instance = get_instance(&state, &instance_id)?;
    let loader = loader_facet(kind, &instance.loader);
    let installing_for_other_version = mc_version != instance.mc_version;
    let mc_version_opt = Some(mc_version.as_str());

    let resolved = match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            curseforge::latest_matching_version(&state.http, &api_key, &project_id, mc_version_opt, loader.as_deref()).await?
        }
        _ => modrinth::latest_matching_version(&state.http, &project_id, mc_version_opt, loader.as_deref()).await?,
    }
    .ok_or_else(|| {
        AppError::Other(if installing_for_other_version {
            "No file found for that Minecraft version.".into()
        } else {
            "No compatible version found for this instance.".into()
        })
    })?;

    install_resolved(&state, &instance_id, kind, resolved, &project_id, &title, icon_url.as_deref(), &source).await
}

/// Installs an exact version from the "Change Version" list; no
/// re-resolution, the picker's entry already carries everything needed.
#[tauri::command]
pub async fn install_content_version(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
    source: String,
    project_id: String,
    title: String,
    icon_url: Option<String>,
    version: ContentVersionEntry,
) -> AppResult<InstalledItem> {
    let resolved = ResolvedVersion {
        version_id: version.version_id,
        version_number: version.version_number,
        file_url: version.file_url,
        filename: version.filename,
        sha1: version.sha1,
    };
    install_resolved(&state, &instance_id, kind, resolved, &project_id, &title, icon_url.as_deref(), &source).await
}

#[tauri::command]
pub async fn list_content_versions(
    state: State<'_, AppState>,
    source: String,
    project_id: String,
) -> AppResult<Vec<ContentVersionEntry>> {
    match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            curseforge::list_versions(&state.http, &api_key, &project_id).await
        }
        _ => modrinth::list_versions(&state.http, &project_id).await,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub filename: String,
    pub update_available: bool,
    pub latest_version_number: Option<String>,
}

#[tauri::command]
pub async fn check_content_updates(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
) -> AppResult<Vec<UpdateCheck>> {
    let instance = get_instance(&state, &instance_id)?;
    let loader = loader_facet(kind, &instance.loader);
    let instance_dir = crate::paths::instance_dir(&instance_id);
    let legacy = content::is_legacy_texturepacks(&instance.mc_version);
    let installed = content::list_installed(&instance_dir, kind, legacy)?;

    const MAX_CONCURRENT: usize = 6;

    let http = &state.http;
    let mc_version = &instance.mc_version;
    let loader = loader.as_deref();

    let results = futures_util::stream::iter(installed.into_iter().map(|item| async move {
        let project_id = item.project_id?;

        // One flaky project shouldn't stop the rest of the instance's
        // updates from being checked; skip it rather than propagating.
        let latest = match item.source.as_deref() {
            Some("curseforge") => {
                let api_key = crate::config::curseforge_api_key()?;
                curseforge::latest_matching_version(http, &api_key, &project_id, Some(mc_version), loader).await
            }
            _ => modrinth::latest_matching_version(http, &project_id, Some(mc_version), loader).await,
        };
        let latest = latest.ok().flatten()?;

        Some(UpdateCheck {
            filename: item.filename,
            update_available: Some(latest.version_id) != item.version_id,
            latest_version_number: Some(latest.version_number),
        })
    }))
    .buffer_unordered(MAX_CONCURRENT)
    .filter_map(|result| async move { result })
    .collect::<Vec<_>>()
    .await;

    Ok(results)
}
