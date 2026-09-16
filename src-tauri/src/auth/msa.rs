//! Microsoft identity platform device-code flow (personal/consumer accounts).

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceCodeInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TokenErrorBody {
    error: String,
    #[serde(default)]
    error_description: String,
}

pub async fn request_device_code(client: &reqwest::Client) -> AppResult<DeviceCodeInfo> {
    let client_id = crate::config::ms_client_id();
    let resp = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPE)])
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!(
            "failed to start Microsoft sign-in: {text}"
        )));
    }
    Ok(resp.json::<DeviceCodeInfo>().await?)
}

pub enum PollOutcome {
    Success(MsTokens),
    Pending,
    SlowDown,
    ExpiredOrDenied(String),
}

pub async fn poll_device_token(
    client: &reqwest::Client,
    device_code: &str,
) -> AppResult<PollOutcome> {
    let client_id = crate::config::ms_client_id();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id.as_str()),
            ("device_code", device_code),
        ])
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if status.is_success() {
        let tokens: MsTokens = serde_json::from_str(&text)?;
        return Ok(PollOutcome::Success(tokens));
    }

    let err: TokenErrorBody = serde_json::from_str(&text).unwrap_or(TokenErrorBody {
        error: "unknown_error".into(),
        error_description: text.clone(),
    });

    match err.error.as_str() {
        "authorization_pending" => Ok(PollOutcome::Pending),
        "slow_down" => Ok(PollOutcome::SlowDown),
        "expired_token" => Ok(PollOutcome::ExpiredOrDenied(
            "The sign-in code expired. Please try again.".into(),
        )),
        "authorization_declined" => Ok(PollOutcome::ExpiredOrDenied(
            "Sign-in was declined.".into(),
        )),
        _ => Ok(PollOutcome::ExpiredOrDenied(if err.error_description.is_empty() {
            err.error
        } else {
            err.error_description
        })),
    }
}

pub async fn refresh_tokens(client: &reqwest::Client, refresh_token: &str) -> AppResult<MsTokens> {
    let client_id = crate::config::ms_client_id();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!(
            "failed to refresh Microsoft session: {text}"
        )));
    }
    Ok(resp.json::<MsTokens>().await?)
}
