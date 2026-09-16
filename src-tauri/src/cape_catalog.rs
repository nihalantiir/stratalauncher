//! Remote-updatable "Try on" cape catalog, fetched from Strata's own repo;
//! gracefully skipped until `config::update_repo` is set.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogCache {
    fetched_at: String,
    capes: Vec<CapeEntry>,
}

const REFRESH_AFTER_HOURS: i64 = 24;

fn cache_path() -> PathBuf {
    crate::paths::data_dir().join("cape-catalog.json")
}

fn load_cache() -> Option<CatalogCache> {
    let bytes = std::fs::read(cache_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn save_cache(cache: &CatalogCache) {
    if let Ok(bytes) = serde_json::to_vec_pretty(cache) {
        let _ = std::fs::write(cache_path(), bytes);
    }
}

fn is_stale(fetched_at: &str) -> bool {
    let Ok(fetched_at) = chrono::DateTime::parse_from_rfc3339(fetched_at) else {
        return true;
    };
    chrono::Utc::now().signed_duration_since(fetched_at).num_hours() >= REFRESH_AFTER_HOURS
}

/// `None` means nothing remote to offer yet; the frontend falls back to
/// its bundled default. A stale cache is returned as-is rather than blocking.
pub async fn get_catalog(client: &reqwest::Client) -> Option<Vec<CapeEntry>> {
    let cached = load_cache();
    let should_refresh = match &cached {
        Some(c) => is_stale(&c.fetched_at),
        None => true,
    };

    if should_refresh {
        if let Some(repo) = crate::config::update_repo() {
            if let Some(fresh) = fetch_remote(client, &repo).await {
                let cache = CatalogCache { fetched_at: chrono::Utc::now().to_rfc3339(), capes: fresh };
                save_cache(&cache);
                return Some(cache.capes);
            }
        }
    }

    cached.map(|c| c.capes)
}

/// Fetches `cape-catalog.json` via GitHub's raw content CDN (no API rate
/// limit). Any failure just means "nothing new this time", never a user error.
async fn fetch_remote(client: &reqwest::Client, repo: &str) -> Option<Vec<CapeEntry>> {
    let url = format!("https://raw.githubusercontent.com/{repo}/main/cape-catalog.json");
    let resp = client
        .get(url)
        .header("User-Agent", concat!("StrataLauncher/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .ok()?;
    let resp = resp.error_for_status().ok()?;
    resp.json::<Vec<CapeEntry>>().await.ok()
}
