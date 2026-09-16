//! Xbox Live user auth + XSTS token exchange, chained from an MSA access token.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

#[derive(Debug, Clone, Serialize)]
pub struct XblToken {
    pub token: String,
    pub uhs: String,
}

#[derive(Debug, Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct XstsErrorBody {
    #[serde(rename = "XErr")]
    x_err: Option<u64>,
}

pub async fn authenticate_xbl(client: &reqwest::Client, ms_access_token: &str) -> AppResult<XblToken> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = client
        .post(XBL_AUTH_URL)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!("Xbox Live sign-in failed: {text}")));
    }

    let parsed: XblResponse = resp.json().await?;
    let uhs = parsed
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or_else(|| AppError::Auth("Xbox Live response missing user hash".into()))?;

    Ok(XblToken {
        token: parsed.token,
        uhs,
    })
}

pub async fn authenticate_xsts(client: &reqwest::Client, xbl_token: &str) -> AppResult<XblToken> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = client
        .post(XSTS_AUTH_URL)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<XstsErrorBody>(&text) {
            if let Some(msg) = xsts_error_message(err.x_err) {
                return Err(AppError::Auth(msg.to_string()));
            }
        }
        return Err(AppError::Auth(format!("Xbox security token exchange failed: {text}")));
    }

    let parsed: XblResponse = serde_json::from_str(&text)?;
    let uhs = parsed
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or_else(|| AppError::Auth("XSTS response missing user hash".into()))?;

    Ok(XblToken {
        token: parsed.token,
        uhs,
    })
}

fn xsts_error_message(code: Option<u64>) -> Option<&'static str> {
    match code {
        Some(2148916233) => Some(
            "This Microsoft account has no Xbox Live profile. Create one at xbox.com, then sign in again.",
        ),
        Some(2148916235) => Some("Xbox Live is not available in this account's region."),
        Some(2148916236) | Some(2148916237) => {
            Some("This account needs adult verification from the Xbox website.")
        }
        Some(2148916238) => Some(
            "This is a child account. Add it to a Microsoft Family group before signing in.",
        ),
        _ => None,
    }
}
