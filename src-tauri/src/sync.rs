//! Background version-sync: diffs Mojang's manifest, each in-use loader's
//! feed, and the launcher's release feed against what was last seen.

use crate::db::models::Instance;
use crate::error::AppResult;
use crate::minecraft::{loaders, manifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub instance_id: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncState {
    last_seen_mc_release: Option<String>,
    last_seen_mc_snapshot: Option<String>,
    last_seen_launcher_version: Option<String>,
    /// When the launcher-update check last ran; drives the configurable
    /// re-check interval, independent of the once-per-launch checks.
    last_update_check_at: Option<String>,
    #[serde(default)]
    last_seen_loader: HashMap<String, String>,
    #[serde(default)]
    notifications: Vec<Notification>,
}

const MAX_NOTIFICATIONS: usize = 50;

fn state_path() -> PathBuf {
    crate::paths::data_dir().join("sync-state.json")
}

fn load_state() -> SyncState {
    std::fs::read(state_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_state(state: &SyncState) -> AppResult<()> {
    std::fs::write(state_path(), serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn push_notification(state: &mut SyncState, notif: Notification) {
    if state.notifications.iter().any(|n| n.id == notif.id) {
        return;
    }
    state.notifications.insert(0, notif);
    state.notifications.truncate(MAX_NOTIFICATIONS);
}

pub fn list_notifications() -> Vec<Notification> {
    load_state().notifications
}

pub fn dismiss_notification(id: &str) -> AppResult<()> {
    let mut state = load_state();
    state.notifications.retain(|n| n.id != id);
    save_state(&state)
}

async fn check_mc_versions(client: &reqwest::Client, state: &mut SyncState, now: &str) {
    let Ok(manifest) = manifest::fetch_manifest(client).await else {
        return;
    };

    if let Some(prev) = &state.last_seen_mc_release {
        if prev != &manifest.latest.release {
            push_notification(
                state,
                Notification {
                    id: format!("mc-release-{}", manifest.latest.release),
                    title: format!("Minecraft {} is out", manifest.latest.release),
                    body: "A new stable release is available.".into(),
                    created_at: now.to_string(),
                    instance_id: None,
                    url: None,
                },
            );
        }
    }
    state.last_seen_mc_release = Some(manifest.latest.release.clone());

    if let Some(prev) = &state.last_seen_mc_snapshot {
        // Right after a release, `latest.snapshot` still points at the same
        // id until a new snapshot is cut; skip the duplicate notification.
        if prev != &manifest.latest.snapshot && manifest.latest.snapshot != manifest.latest.release {
            push_notification(
                state,
                Notification {
                    id: format!("mc-snapshot-{}", manifest.latest.snapshot),
                    title: format!("Minecraft snapshot {} is out", manifest.latest.snapshot),
                    body: "A new snapshot is available.".into(),
                    created_at: now.to_string(),
                    instance_id: None,
                    url: None,
                },
            );
        }
    }
    state.last_seen_mc_snapshot = Some(manifest.latest.snapshot.clone());
}

async fn check_loader_versions(client: &reqwest::Client, instances: &[Instance], state: &mut SyncState, now: &str) {
    // One check per distinct (loader, mc_version) actually in use, not
    // one per instance, since several instances commonly share a combo.
    let mut seen_combos: Vec<(String, String, String)> = Vec::new(); // (loader, mc_version, instance_id)
    for instance in instances {
        if instance.loader == "vanilla" {
            continue;
        }
        if seen_combos.iter().any(|(l, v, _)| l == &instance.loader && v == &instance.mc_version) {
            continue;
        }
        seen_combos.push((instance.loader.clone(), instance.mc_version.clone(), instance.id.clone()));
    }

    for (loader, mc_version, instance_id) in seen_combos {
        let Ok(versions) = loaders::fetch_versions(client, &loader, &mc_version).await else {
            continue;
        };
        let Some(newest) = versions.first() else { continue };
        let key = format!("{loader}:{mc_version}");

        if let Some(prev) = state.last_seen_loader.get(&key) {
            if prev != &newest.version {
                push_notification(
                    state,
                    Notification {
                        id: format!("loader-{key}-{}", newest.version),
                        title: format!("New {loader} build for Minecraft {mc_version}"),
                        body: format!("{} is now available.", newest.version),
                        created_at: now.to_string(),
                        instance_id: Some(instance_id.clone()),
                        url: None,
                    },
                );
            }
        }
        state.last_seen_loader.insert(key, newest.version.clone());
    }
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

/// Whether enough time has passed since the last check to run another one,
/// per the user's configured interval. A fresh install is always due.
pub fn update_check_due(interval_hours: u32) -> bool {
    let state = load_state();
    let Some(last) = state.last_update_check_at else {
        return true;
    };
    let Ok(last) = chrono::DateTime::parse_from_rfc3339(&last) else {
        return true;
    };
    let elapsed = chrono::Utc::now().signed_duration_since(last);
    elapsed.num_hours() >= interval_hours as i64
}

/// Stable-channel only. Skipped until `STRATA_UPDATE_REPO` is set; callable
/// standalone or as part of `run_sync_check`.
pub async fn check_launcher_update(client: &reqwest::Client) -> AppResult<()> {
    let mut state = load_state();
    let now = chrono::Utc::now().to_rfc3339();
    check_launcher_update_inner(client, &mut state, &now).await;
    state.last_update_check_at = Some(now);
    save_state(&state)
}

async fn check_launcher_update_inner(client: &reqwest::Client, state: &mut SyncState, now: &str) {
    let Some(repo) = crate::config::update_repo() else {
        return;
    };

    let Ok(resp) = client
        .get(format!("https://api.github.com/repos/{repo}/releases/latest"))
        .header("User-Agent", concat!("StrataLauncher/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
    else {
        return;
    };
    let Ok(resp) = resp.error_for_status() else {
        return;
    };
    let Ok(release) = resp.json::<GithubRelease>().await else {
        return;
    };

    let current = env!("CARGO_PKG_VERSION");
    let tag = release.tag_name.trim_start_matches('v');
    if tag == current {
        state.last_seen_launcher_version = Some(tag.to_string());
        return;
    }

    if let Some(prev) = &state.last_seen_launcher_version {
        if prev == tag {
            return; // already notified for this exact tag
        }
    }

    push_notification(
        state,
        Notification {
            id: format!("launcher-update-{tag}"),
            title: format!("Strata {tag} is available"),
            body: format!("You're on {current}."),
            created_at: now.to_string(),
            instance_id: None,
            url: Some(release.html_url),
        },
    );
    state.last_seen_launcher_version = Some(tag.to_string());
}

/// Runs every check and persists the result; the first run just seeds the
/// baseline rather than notifying. `check_updates` off skips only the launcher check.
pub async fn run_sync_check(client: &reqwest::Client, instances: &[Instance], check_updates: bool) -> AppResult<()> {
    let mut state = load_state();
    let now = chrono::Utc::now().to_rfc3339();

    check_mc_versions(client, &mut state, &now).await;
    check_loader_versions(client, instances, &mut state, &now).await;
    if check_updates {
        check_launcher_update_inner(client, &mut state, &now).await;
        state.last_update_check_at = Some(now);
    }

    save_state(&state)
}
