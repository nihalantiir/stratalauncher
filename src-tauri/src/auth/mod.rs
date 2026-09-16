pub mod keychain;
pub mod minecraft;
pub mod msa;
pub mod offline;
pub mod xbox;

use crate::db::models::{Account, AccountKind};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// A previously-obtained live session, cached so repeated calls within the
/// token's lifetime skip the network chain (see `ensure_live_session`).
pub struct CachedSession {
    session: minecraft::McSession,
    profile: minecraft::McProfile,
    expires_at: Instant,
}

/// Mojang's access token is valid for 24h (`expires_in: 86400`); this
/// buffer avoids reusing one that's about to expire mid-use.
const SESSION_EXPIRY_BUFFER_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage", rename_all = "kebab-case")]
pub enum LoginProgress {
    #[serde(rename_all = "camelCase")]
    AwaitingCode {
        user_code: String,
        verification_uri: String,
        expires_in: u64,
    },
    SigningIntoXbox,
    SigningIntoMinecraft,
    FetchingProfile,
}

const LOGIN_EVENT: &str = "auth://login-progress";

/// Exchanges an MSA access token all the way through to a Minecraft profile.
async fn xbox_to_profile_chain(
    client: &reqwest::Client,
    ms_access_token: &str,
) -> AppResult<(minecraft::McSession, minecraft::McProfile)> {
    let xbl = xbox::authenticate_xbl(client, ms_access_token).await?;
    let xsts = xbox::authenticate_xsts(client, &xbl.token).await?;
    let session = minecraft::login_with_xbox(client, &xsts.uhs, &xsts.token).await?;

    if !minecraft::check_ownership(client, &session.access_token).await? {
        return Err(AppError::Auth(
            "This Microsoft account doesn't own Minecraft: Java Edition.".into(),
        ));
    }

    let profile = minecraft::fetch_profile(client, &session.access_token).await?;
    Ok((session, profile))
}

pub async fn sign_in_with_device_code(
    app: &AppHandle,
    client: &reqwest::Client,
    cancel: &AtomicBool,
) -> AppResult<(Account, String)> {
    cancel.store(false, Ordering::SeqCst);

    let device = msa::request_device_code(client).await?;
    app.emit(
        LOGIN_EVENT,
        LoginProgress::AwaitingCode {
            user_code: device.user_code.clone(),
            verification_uri: device.verification_uri.clone(),
            expires_in: device.expires_in,
        },
    )
    .ok();

    let mut interval = Duration::from_secs(device.interval.max(2));
    let deadline = std::time::Instant::now() + Duration::from_secs(device.expires_in);
    let ms_tokens = loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(AppError::Auth("Sign-in cancelled.".into()));
        }
        if std::time::Instant::now() >= deadline {
            return Err(AppError::Auth("The sign-in code expired. Please try again.".into()));
        }
        tokio::time::sleep(interval).await;

        match msa::poll_device_token(client, &device.device_code).await? {
            msa::PollOutcome::Success(tokens) => break tokens,
            msa::PollOutcome::Pending => continue,
            msa::PollOutcome::SlowDown => {
                interval += Duration::from_secs(5);
                continue;
            }
            msa::PollOutcome::ExpiredOrDenied(msg) => return Err(AppError::Auth(msg)),
        }
    };

    app.emit(LOGIN_EVENT, LoginProgress::SigningIntoXbox).ok();
    let xbl = xbox::authenticate_xbl(client, &ms_tokens.access_token).await?;
    let xsts = xbox::authenticate_xsts(client, &xbl.token).await?;

    app.emit(LOGIN_EVENT, LoginProgress::SigningIntoMinecraft).ok();
    let session = minecraft::login_with_xbox(client, &xsts.uhs, &xsts.token).await?;
    if !minecraft::check_ownership(client, &session.access_token).await? {
        return Err(AppError::Auth(
            "This Microsoft account doesn't own Minecraft: Java Edition.".into(),
        ));
    }

    app.emit(LOGIN_EVENT, LoginProgress::FetchingProfile).ok();
    let profile = minecraft::fetch_profile(client, &session.access_token).await?;

    let active_skin = profile.skins.iter().find(|s| s.state == "ACTIVE");
    let account = Account {
        id: profile.id.clone(),
        kind: AccountKind::Microsoft,
        username: profile.name,
        mc_uuid: Some(profile.id),
        skin_url: active_skin.map(|s| s.url.clone()),
        skin_variant: active_skin.map(|s| s.variant.to_lowercase()),
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_used_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    Ok((account, ms_tokens.refresh_token))
}

/// Returns a live access token + profile, reusing a cached session when
/// valid, otherwise running the full MSA/Xbox/XSTS/Minecraft refresh chain.
pub async fn ensure_live_session(
    state: &AppState,
    account_id: &str,
) -> AppResult<(minecraft::McSession, minecraft::McProfile)> {
    let cached = {
        let cache = state.session_cache.lock().unwrap();
        cache
            .get(account_id)
            .filter(|c| Instant::now() < c.expires_at)
            .map(|c| (c.session.clone(), c.profile.clone()))
    };
    if let Some(result) = cached {
        return Ok(result);
    }

    crate::launcher_log::info("auth", format!("Refreshing Microsoft session for account {account_id}"));
    let refresh_token = keychain::load_refresh_token(account_id)?.ok_or_else(|| {
        AppError::Auth("No saved Microsoft session for this account. Please sign in again.".into())
    })?;

    let ms_tokens = msa::refresh_tokens(&state.http, &refresh_token)
        .await
        .inspect_err(|e| crate::launcher_log::error("auth", format!("Token refresh failed: {e}")))?;
    keychain::store_refresh_token(account_id, &ms_tokens.refresh_token)?;

    let result = xbox_to_profile_chain(&state.http, &ms_tokens.access_token).await;
    match &result {
        Ok((session, profile)) => {
            crate::launcher_log::info("auth", format!("Session refreshed for {}", profile.name));
            let ttl = Duration::from_secs(session.expires_in.saturating_sub(SESSION_EXPIRY_BUFFER_SECS).max(60));
            state.session_cache.lock().unwrap().insert(
                account_id.to_string(),
                CachedSession {
                    session: session.clone(),
                    profile: profile.clone(),
                    expires_at: Instant::now() + ttl,
                },
            );
        }
        Err(e) => crate::launcher_log::error("auth", format!("Xbox/Minecraft sign-in chain failed: {e}")),
    }
    result
}
