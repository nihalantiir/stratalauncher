use crate::commands::content::{prettify, ContentCategory};
use crate::commands::instances::{self, NewInstance};
use crate::content::{curseforge, modrinth, ContentSearchHit, ContentVersionEntry};
use crate::db::models::Instance;
use crate::error::{AppError, AppResult};
use crate::modpack::{self, curseforge_pack, mrpack, PackFormat};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn search_modpacks(
    state: State<'_, AppState>,
    source: String,
    query: String,
    mc_version: Option<String>,
    categories: Vec<String>,
    sort: String,
    offset: u32,
) -> AppResult<Vec<ContentSearchHit>> {
    match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            let category_ids: Vec<i64> = categories.iter().filter_map(|c| c.parse::<i64>().ok()).collect();
            let hits =
                curseforge::search_modpacks(&state.http, &api_key, &query, mc_version.as_deref(), &category_ids, &sort, offset).await?;
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
            let hits = modrinth::search(&state.http, "modpack", &query, mc_version.as_deref(), None, &categories, &sort, offset).await?;
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

/// Same shape as `list_content_categories` but for modpacks, which aren't
/// a `ContentKind`.
#[tauri::command]
pub async fn list_modpack_categories(state: State<'_, AppState>, source: String) -> AppResult<Vec<ContentCategory>> {
    match source.as_str() {
        "curseforge" => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            let cats = curseforge::modpack_categories(&state.http, &api_key).await?;
            Ok(cats.into_iter().map(|c| ContentCategory { id: c.id.to_string(), name: c.name, group: None }).collect())
        }
        _ => {
            let tags = modrinth::categories(&state.http, "modpack").await?;
            Ok(tags
                .into_iter()
                .map(|c| ContentCategory { id: c.name.clone(), name: prettify(&c.name), group: Some(prettify(&c.header)) })
                .collect())
        }
    }
}

async fn install_pack_file(
    app: &AppHandle,
    client: &reqwest::Client,
    format: &PackFormat,
    zip_path: &std::path::Path,
    instance_dir: &std::path::Path,
) -> AppResult<()> {
    match format {
        PackFormat::Mrpack => mrpack::install(app, client, zip_path, instance_dir).await,
        PackFormat::CurseForge => {
            let api_key = crate::config::curseforge_api_key()
                .ok_or_else(|| AppError::Other("CurseForge isn't configured yet.".into()))?;
            curseforge_pack::install(app, client, &api_key, zip_path, instance_dir).await
        }
    }
}

/// Resolves the target instance shape first, creates it, then installs the
/// pack's content; any failure past creation deletes the instance again.
async fn create_and_install(
    app: &AppHandle,
    state: &State<'_, AppState>,
    zip_path: &std::path::Path,
    name: String,
    group_name: Option<String>,
) -> AppResult<Instance> {
    let format = modpack::detect_format(zip_path)?;
    let target = match format {
        PackFormat::Mrpack => mrpack::resolve_target(zip_path)?,
        PackFormat::CurseForge => curseforge_pack::resolve_target(zip_path)?,
    };

    let instance = instances::create_instance(
        NewInstance {
            name,
            mc_version: target.mc_version,
            icon_biome: String::new(), // falls back to a default biome, see resolve_biome
            loader: target.loader,
            loader_version: target.loader_version,
            group_name,
        },
        state.clone(),
    )?;

    let instance_dir = crate::paths::instance_dir(&instance.id);
    if let Err(e) = std::fs::create_dir_all(&instance_dir).map_err(AppError::from) {
        instances::delete_instance(instance.id.clone(), state.clone()).ok();
        return Err(e);
    }

    if let Err(e) = install_pack_file(app, &state.http, &format, zip_path, &instance_dir).await {
        instances::delete_instance(instance.id.clone(), state.clone()).ok();
        return Err(e);
    }

    Ok(instance)
}

/// Unlike `install_content_version`, no `source`/`project_id`: a whole-pack
/// install has no per-item metadata to record against either field.
#[tauri::command]
pub async fn install_modpack_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version: ContentVersionEntry,
    name: String,
    group_name: Option<String>,
) -> AppResult<Instance> {
    let temp_path = std::env::temp_dir().join(format!("strata-modpack-{}.zip", uuid::Uuid::new_v4()));
    crate::minecraft::download::download_verified(&state.http, &version.file_url, &temp_path, version.sha1.as_deref()).await?;

    let result = create_and_install(&app, &state, &temp_path, name, group_name).await;
    std::fs::remove_file(&temp_path).ok();
    result
}

#[tauri::command]
pub async fn install_modpack_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: String,
    group_name: Option<String>,
) -> AppResult<Instance> {
    let zip_path = std::path::PathBuf::from(path);
    if !zip_path.is_file() {
        return Err(AppError::Other("That file doesn't exist.".into()));
    }
    create_and_install(&app, &state, &zip_path, name, group_name).await
}
