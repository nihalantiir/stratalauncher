//! Minecraft Services auth (login-with-xbox), entitlement check, and profile fetch.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

const LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SKINS_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const CAPES_ACTIVE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";

#[derive(Debug, Clone, Serialize)]
pub struct McSession {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
    items: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<ProfileSkin>,
    #[serde(default)]
    pub capes: Vec<ProfileCape>,
}

/// `variant` comes back uppercase ("CLASSIC"/"SLIM") from Mojang; the
/// upload endpoint expects it lowercase, kept as distinct cased strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCape {
    pub id: String,
    pub state: String,
    pub url: String,
    pub alias: String,
}

/// Repeatedly tripping 429s on skin/cape mutation risks a temporary account
/// suspension, so a real 429 surfaces a clear error instead of auto-retrying.
fn rate_limit_error(resp: &reqwest::Response) -> Option<AppError> {
    if resp.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    let retry_after = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    Some(AppError::Auth(match retry_after {
        Some(secs) => format!("Mojang is rate-limiting this account. Try again in {secs}s."),
        None => "Mojang is rate-limiting this account. Wait a minute before trying again.".into(),
    }))
}

pub async fn login_with_xbox(client: &reqwest::Client, uhs: &str, xsts_token: &str) -> AppResult<McSession> {
    let identity_token = format!("XBL3.0 x={uhs};{xsts_token}");
    let resp = client
        .post(LOGIN_URL)
        .json(&json!({ "identityToken": identity_token }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!("Minecraft sign-in failed: {text}")));
    }

    let parsed: LoginResponse = resp.json().await?;
    Ok(McSession {
        access_token: parsed.access_token,
        expires_in: parsed.expires_in,
    })
}

pub async fn check_ownership(client: &reqwest::Client, mc_access_token: &str) -> AppResult<bool> {
    let resp = client
        .get(ENTITLEMENTS_URL)
        .bearer_auth(mc_access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        // If the entitlement check itself fails, don't hard-block sign-in on it;
        // the profile fetch below is the authoritative check.
        return Ok(true);
    }
    let parsed: EntitlementsResponse = resp.json().await?;
    Ok(!parsed.items.is_empty())
}

pub async fn fetch_profile(client: &reqwest::Client, mc_access_token: &str) -> AppResult<McProfile> {
    let resp = client
        .get(PROFILE_URL)
        .bearer_auth(mc_access_token)
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Auth(
            "This Microsoft account doesn't own Minecraft: Java Edition.".into(),
        ));
    }
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!("Could not fetch Minecraft profile: {text}")));
    }

    Ok(resp.json::<McProfile>().await?)
}

/// Uploads a new skin. `variant` must be lowercase ("classic"/"slim"),
/// not the uppercase form the profile response uses.
pub async fn upload_skin(client: &reqwest::Client, mc_access_token: &str, variant: &str, png_bytes: Vec<u8>) -> AppResult<()> {
    let part = reqwest::multipart::Part::bytes(png_bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| AppError::Other(format!("bad skin file: {e}")))?;
    let form = reqwest::multipart::Form::new().text("variant", variant.to_string()).part("file", part);

    let resp = client.post(SKINS_URL).bearer_auth(mc_access_token).multipart(form).send().await?;
    if let Some(err) = rate_limit_error(&resp) {
        return Err(err);
    }
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!("Skin upload failed: {text}")));
    }
    Ok(())
}

/// Resets to Mojang's default skin (Steve/Alex chosen by Mojang from the UUID).
pub async fn reset_skin(client: &reqwest::Client, mc_access_token: &str) -> AppResult<()> {
    let resp = client
        .delete(format!("{SKINS_URL}/active"))
        .bearer_auth(mc_access_token)
        .send()
        .await?;
    if let Some(err) = rate_limit_error(&resp) {
        return Err(err);
    }
    if !resp.status().is_success() {
        return Err(AppError::Auth("Couldn't reset the skin.".into()));
    }
    Ok(())
}

/// Equips one of the account's own capes; there's no API to upload a custom
/// one, only to select a `capeId` already in the account's `capes` list.
pub async fn equip_cape(client: &reqwest::Client, mc_access_token: &str, cape_id: &str) -> AppResult<()> {
    let resp = client
        .put(CAPES_ACTIVE_URL)
        .bearer_auth(mc_access_token)
        .json(&json!({ "capeId": cape_id }))
        .send()
        .await?;
    if let Some(err) = rate_limit_error(&resp) {
        return Err(err);
    }
    if !resp.status().is_success() {
        return Err(AppError::Auth("Couldn't equip that cape.".into()));
    }
    Ok(())
}

pub async fn unequip_cape(client: &reqwest::Client, mc_access_token: &str) -> AppResult<()> {
    let resp = client
        .delete(CAPES_ACTIVE_URL)
        .bearer_auth(mc_access_token)
        .send()
        .await?;
    if let Some(err) = rate_limit_error(&resp) {
        return Err(err);
    }
    if !resp.status().is_success() {
        return Err(AppError::Auth("Couldn't remove the cape.".into()));
    }
    Ok(())
}
