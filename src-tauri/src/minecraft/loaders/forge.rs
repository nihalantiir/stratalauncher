//! Forge/NeoForge ship a real Java installer (binary patching, not a small
//! profile fetch); this shells it out, then merges the version JSON it writes.

use super::LoaderVersionEntry;
use crate::error::{AppError, AppResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const FORGE_PROMOTIONS_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_METADATA_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
// NeoForge forked from Forge at 1.20.1, before it had its own Maven
// artifact, so those builds live at this Forge-shaped legacy URL instead.
const NEOFORGE_LEGACY_METADATA_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml";
const NEOFORGE_LEGACY_MC_VERSION: &str = "1.20.1";

fn is_legacy_neoforge(loader: &str, mc_version: &str) -> bool {
    loader == "neoforge" && mc_version == NEOFORGE_LEGACY_MC_VERSION
}

// Both feeds list every build for every Minecraft version in one file, not
// scoped per `mc_version`, so a short cache avoids refetching on every lookup.
const CACHE_TTL: Duration = Duration::from_secs(10 * 60);

struct Cached<T> {
    value: T,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize, Clone)]
struct ForgePromotions {
    promos: HashMap<String, String>,
}

fn forge_cache() -> &'static Mutex<Option<Cached<ForgePromotions>>> {
    static CACHE: OnceLock<Mutex<Option<Cached<ForgePromotions>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn neoforge_cache() -> &'static Mutex<Option<Cached<String>>> {
    static CACHE: OnceLock<Mutex<Option<Cached<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn neoforge_legacy_cache() -> &'static Mutex<Option<Cached<String>>> {
    static CACHE: OnceLock<Mutex<Option<Cached<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

async fn cached_forge_promotions(client: &reqwest::Client) -> AppResult<ForgePromotions> {
    let mut guard = forge_cache().lock().await;
    if let Some(cached) = guard.as_ref() {
        if cached.fetched_at.elapsed() < CACHE_TTL {
            return Ok(cached.value.clone());
        }
    }
    let promos: ForgePromotions = client
        .get(FORGE_PROMOTIONS_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    *guard = Some(Cached { value: promos.clone(), fetched_at: Instant::now() });
    Ok(promos)
}

async fn cached_neoforge_metadata(client: &reqwest::Client) -> AppResult<String> {
    let mut guard = neoforge_cache().lock().await;
    if let Some(cached) = guard.as_ref() {
        if cached.fetched_at.elapsed() < CACHE_TTL {
            return Ok(cached.value.clone());
        }
    }
    let xml = client.get(NEOFORGE_METADATA_URL).send().await?.error_for_status()?.text().await?;
    *guard = Some(Cached { value: xml.clone(), fetched_at: Instant::now() });
    Ok(xml)
}

async fn cached_neoforge_legacy_metadata(client: &reqwest::Client) -> AppResult<String> {
    let mut guard = neoforge_legacy_cache().lock().await;
    if let Some(cached) = guard.as_ref() {
        if cached.fetched_at.elapsed() < CACHE_TTL {
            return Ok(cached.value.clone());
        }
    }
    let xml = client.get(NEOFORGE_LEGACY_METADATA_URL).send().await?.error_for_status()?.text().await?;
    *guard = Some(Cached { value: xml.clone(), fetched_at: Instant::now() });
    Ok(xml)
}

pub async fn fetch_forge_versions(client: &reqwest::Client, mc_version: &str) -> AppResult<Vec<LoaderVersionEntry>> {
    let promos = cached_forge_promotions(client).await?;

    let mut out = Vec::new();
    if let Some(v) = promos.promos.get(&format!("{mc_version}-recommended")) {
        out.push(LoaderVersionEntry { version: v.clone(), stable: true });
    }
    if let Some(v) = promos.promos.get(&format!("{mc_version}-latest")) {
        if !out.iter().any(|e| &e.version == v) {
            out.push(LoaderVersionEntry { version: v.clone(), stable: false });
        }
    }
    if out.is_empty() {
        return Err(AppError::Other(format!("No Forge builds available for Minecraft {mc_version}.")));
    }
    Ok(out)
}

/// NeoForge version numbers mirror the Minecraft version plus a build
/// number, but the shape changed at 26.x (pre-26.x strips a leading `1.`).
fn neoforge_prefix(mc_version: &str) -> Option<String> {
    if let Some(rest) = mc_version.strip_prefix("1.") {
        let mut parts = rest.splitn(2, '.');
        let minor = parts.next()?;
        let patch = parts.next().unwrap_or("0");
        return Some(format!("{minor}.{patch}."));
    }

    let parts: Vec<&str> = mc_version.split('.').collect();
    let all_numeric = parts.len() >= 2 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if !all_numeric {
        return None;
    }
    let normalized = if parts.len() == 2 { format!("{}.{}.0", parts[0], parts[1]) } else { parts.join(".") };
    Some(format!("{normalized}."))
}

fn extract_tag_contents(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        rest = &rest[start + open.len()..];
        let Some(end) = rest.find(&close) else { break };
        out.push(rest[..end].to_string());
        rest = &rest[end + close.len()..];
    }
    out
}

fn version_key(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub async fn fetch_neoforge_versions(client: &reqwest::Client, mc_version: &str) -> AppResult<Vec<LoaderVersionEntry>> {
    if is_legacy_neoforge("neoforge", mc_version) {
        let prefix = format!("{mc_version}-");
        let xml = cached_neoforge_legacy_metadata(client).await?;
        let mut versions: Vec<String> = extract_tag_contents(&xml, "version")
            .into_iter()
            .filter_map(|v| v.strip_prefix(&prefix).map(|tail| tail.to_string()))
            .collect();
        versions.sort_by_key(|v| version_key(v));
        versions.reverse();

        if versions.is_empty() {
            return Err(AppError::Other(format!("No NeoForge builds available for Minecraft {mc_version}.")));
        }
        return Ok(versions
            .into_iter()
            .enumerate()
            .map(|(i, version)| LoaderVersionEntry { version, stable: i == 0 })
            .collect());
    }

    let prefix = neoforge_prefix(mc_version)
        .ok_or_else(|| AppError::Other(format!("NeoForge doesn't support Minecraft {mc_version}.")))?;

    let xml = cached_neoforge_metadata(client).await?;
    let mut versions: Vec<String> = extract_tag_contents(&xml, "version")
        .into_iter()
        .filter(|v| v.starts_with(&prefix))
        .collect();
    versions.sort_by_key(|v| version_key(v));
    versions.reverse();

    if versions.is_empty() {
        return Err(AppError::Other(format!("No NeoForge builds available for Minecraft {mc_version}.")));
    }
    Ok(versions
        .into_iter()
        .enumerate()
        .map(|(i, version)| LoaderVersionEntry { version, stable: i == 0 })
        .collect())
}

fn installer_url(loader: &str, mc_version: &str, loader_version: &str) -> String {
    if loader == "forge" {
        format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{loader_version}/forge-{mc_version}-{loader_version}-installer.jar"
        )
    } else if is_legacy_neoforge(loader, mc_version) {
        // Same "forge"-named installer artifact as real Forge (see
        // NEOFORGE_LEGACY_METADATA_URL), just on neoforged's Maven host.
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/forge/{mc_version}-{loader_version}/forge-{mc_version}-{loader_version}-installer.jar"
        )
    } else {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar"
        )
    }
}

/// The version id the real installer writes under `versions/`; must match
/// exactly since that's where the generated JSON is read back from.
pub fn effective_version_id(loader: &str, mc_version: &str, loader_version: &str) -> String {
    if loader == "forge" || is_legacy_neoforge(loader, mc_version) {
        format!("{mc_version}-forge-{loader_version}")
    } else {
        format!("neoforge-{loader_version}")
    }
}

async fn run_installer(java_path: &Path, installer_path: &Path, target_dir: &Path) -> AppResult<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut child = Command::new(java_path)
        .arg("-jar")
        .arg(installer_path)
        .arg("--installClient")
        .arg(target_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Other(format!("failed to start the installer: {e}")))?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let out_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            crate::launcher_log::info("installer", line);
        }
    });
    let err_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            crate::launcher_log::warn("installer", line);
        }
    });

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::Other(format!("installer process failed: {e}")))?;
    let _ = out_task.await;
    let _ = err_task.await;

    if !status.success() {
        return Err(AppError::Other(format!("installer exited with {status}")));
    }
    Ok(())
}

/// Ensures `{loader} {loader_version}` is installed under Strata's shared
/// root, running the real installer only if needed, then returns its JSON.
pub async fn ensure_installed(
    client: &reqwest::Client,
    loader: &str,
    mc_version: &str,
    loader_version: &str,
) -> AppResult<serde_json::Value> {
    let effective_id = effective_version_id(loader, mc_version, loader_version);
    let version_dir = crate::paths::versions_dir().join(&effective_id);
    let version_json_path = version_dir.join(format!("{effective_id}.json"));
    // The caller overwrites `version_json_path` with a merged result, so it
    // can't double as our cache; keep a separate untouched pristine copy.
    let pristine_path = version_dir.join(format!("{effective_id}.strata-installer-profile.json"));

    if let Ok(bytes) = std::fs::read(&pristine_path) {
        if let Ok(json) = serde_json::from_slice(&bytes) {
            crate::launcher_log::info(loader, format!("{effective_id} already installed"));
            return Ok(json);
        }
    }

    let installer_dir = crate::paths::data_dir().join("installers");
    std::fs::create_dir_all(&installer_dir)?;
    let installer_path = installer_dir.join(format!("{loader}-{mc_version}-{loader_version}-installer.jar"));
    let url = installer_url(loader, mc_version, loader_version);

    crate::launcher_log::info(loader, format!("Downloading installer: {url}"));
    super::super::download::download_verified(client, &url, &installer_path, None, None).await?;

    let java_path = super::super::launch::find_java()?;

    // The installer refuses to run unless `launcher_profiles.json` already
    // exists in the target dir; a minimal stub satisfies its existence check.
    let profiles_path = crate::paths::data_dir().join("launcher_profiles.json");
    if !profiles_path.exists() {
        std::fs::write(&profiles_path, br#"{"profiles":{},"settings":{},"version":3}"#)?;
    }

    crate::launcher_log::info(loader, format!("Running the {loader} installer for Minecraft {mc_version}…"));
    run_installer(&java_path, &installer_path, crate::paths::data_dir()).await?;

    let bytes = std::fs::read(&version_json_path).map_err(|_| {
        AppError::Other(format!(
            "The {loader} installer finished but didn't produce {}",
            version_json_path.display()
        ))
    })?;
    std::fs::write(&pristine_path, &bytes)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)?;
    crate::launcher_log::info(loader, format!("{effective_id} installed"));
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_prefix_handles_both_mc_versioning_schemes() {
        // Pre-1.21.11 "1.X.Y" scheme.
        assert_eq!(neoforge_prefix("1.20.4"), Some("20.4.".to_string()));
        assert_eq!(neoforge_prefix("1.20"), Some("20.0.".to_string()));
        // Newer no-leading-"1." scheme (the real bug: used to return None here).
        // A bare "major.minor" normalizes in a ".0" patch to match real build ids like "26.2.0.88".
        assert_eq!(neoforge_prefix("26.2"), Some("26.2.0.".to_string()));
        assert_eq!(neoforge_prefix("25.1.3"), Some("25.1.3.".to_string()));
        assert_eq!(neoforge_prefix("not-a-version"), None);
    }

    #[test]
    fn installer_url_routes_1_20_1_neoforge_through_the_legacy_forge_host() {
        assert_eq!(
            installer_url("forge", "1.20.1", "47.1.106"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar"
        );
        assert_eq!(
            installer_url("neoforge", "1.20.1", "47.1.106"),
            "https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar"
        );
        assert_eq!(
            installer_url("neoforge", "1.21.1", "21.1.100"),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.100/neoforge-21.1.100-installer.jar"
        );
    }

    #[test]
    fn effective_version_id_must_match_what_the_installer_writes() {
        assert_eq!(effective_version_id("forge", "1.20.1", "47.1.106"), "1.20.1-forge-47.1.106");
        // 1.20.1 NeoForge shares Forge's version-id shape, not the modern "neoforge-*" one.
        assert_eq!(effective_version_id("neoforge", "1.20.1", "47.1.106"), "1.20.1-forge-47.1.106");
        assert_eq!(effective_version_id("neoforge", "1.21.1", "21.1.100"), "neoforge-21.1.100");
    }

    #[test]
    fn version_key_sorts_numerically_not_lexically() {
        let mut versions = vec!["9.20.0".to_string(), "10.5.0".to_string(), "9.5.0".to_string()];
        versions.sort_by_key(|v| version_key(v));
        assert_eq!(versions, vec!["9.5.0", "9.20.0", "10.5.0"]);
    }

    #[test]
    fn extract_tag_contents_reads_every_occurrence() {
        let xml = "<metadata><versioning><versions><version>1.20.1-47.1.100</version><version>1.20.1-47.1.106</version></versions></versioning></metadata>";
        assert_eq!(extract_tag_contents(xml, "version"), vec!["1.20.1-47.1.100", "1.20.1-47.1.106"]);
        assert_eq!(extract_tag_contents(xml, "missing"), Vec::<String>::new());
    }
}
