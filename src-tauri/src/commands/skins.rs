use crate::auth::{self, minecraft::McProfile};
use crate::db::models::{Account, AccountKind};
use crate::db::AccountsRepo;
use crate::error::{AppError, AppResult};
use crate::paths;
use crate::state::AppState;
use std::io::Read;
use tauri::State;

/// The default "Steve" skin, read straight out of a downloaded vanilla
/// client jar rather than bundled, so Strata never redistributes it directly.
#[tauri::command]
pub fn get_default_skin() -> AppResult<Vec<u8>> {
    const CANDIDATES: &[&str] = &[
        "assets/minecraft/textures/entity/player/wide/steve.png",
        "assets/minecraft/textures/entity/steve.png",
    ];

    let versions_dir = paths::versions_dir();
    let entries = std::fs::read_dir(&versions_dir).map_err(AppError::Io)?;
    for entry in entries.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        let jar_path = entry.path().join(format!("{id}.jar"));
        let Ok(file) = std::fs::File::open(&jar_path) else { continue };
        let Ok(mut archive) = zip::ZipArchive::new(file) else { continue };
        for candidate in CANDIDATES {
            if let Ok(mut zf) = archive.by_name(candidate) {
                let mut buf = Vec::new();
                if zf.read_to_end(&mut buf).is_ok() {
                    return Ok(buf);
                }
            }
        }
    }
    Err(AppError::NotFound(
        "No downloaded Minecraft version yet to read the default skin from.".into(),
    ))
}

/// Skin/cape management needs a live Minecraft access token and only makes
/// sense for a Microsoft account; offline accounts have no Mojang profile.
async fn live_token_for_active(state: &State<'_, AppState>) -> AppResult<(String, Account)> {
    let account = {
        let conn = state.db.0.lock().unwrap();
        AccountsRepo::active(&conn)?.ok_or_else(|| AppError::Auth("No active account.".into()))?
    };
    if account.kind != AccountKind::Microsoft {
        return Err(AppError::Auth("Skin and cape management needs a Microsoft account.".into()));
    }
    let (session, _profile) = auth::ensure_live_session(state, &account.id).await?;
    Ok((session.access_token, account))
}

fn sync_account_from_profile(state: &State<'_, AppState>, mut account: Account, profile: &McProfile) -> AppResult<()> {
    if let Some(skin) = profile.skins.iter().find(|s| s.state == "ACTIVE") {
        account.skin_url = Some(skin.url.clone());
        account.skin_variant = Some(skin.variant.to_lowercase());
    }
    let conn = state.db.0.lock().unwrap();
    AccountsRepo::upsert(&conn, &account)
}

#[tauri::command]
pub async fn get_skin_profile(state: State<'_, AppState>) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn upload_skin(state: State<'_, AppState>, variant: String, bytes: Vec<u8>) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    auth::minecraft::upload_skin(&state.http, &token, &variant, bytes).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn reset_skin(state: State<'_, AppState>) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    auth::minecraft::reset_skin(&state.http, &token).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn equip_cape(state: State<'_, AppState>, cape_id: String) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    auth::minecraft::equip_cape(&state.http, &token, &cape_id).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn unequip_cape(state: State<'_, AppState>) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    auth::minecraft::unequip_cape(&state.http, &token).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &profile)?;
    Ok(profile)
}

/// Changes just the model (classic/slim) of the active skin: Mojang ties
/// the model to the upload itself, so this re-uploads the current texture.
#[tauri::command]
pub async fn set_skin_variant(state: State<'_, AppState>, variant: String) -> AppResult<McProfile> {
    let (token, account) = live_token_for_active(&state).await?;
    let profile = auth::minecraft::fetch_profile(&state.http, &token).await?;
    let active = profile
        .skins
        .iter()
        .find(|s| s.state == "ACTIVE")
        .ok_or_else(|| AppError::Auth("No active skin to change the model of.".into()))?;
    let bytes = state.http.get(&active.url).send().await?.bytes().await?.to_vec();
    auth::minecraft::upload_skin(&state.http, &token, &variant, bytes).await?;
    let updated = auth::minecraft::fetch_profile(&state.http, &token).await?;
    sync_account_from_profile(&state, account, &updated)?;
    Ok(updated)
}
