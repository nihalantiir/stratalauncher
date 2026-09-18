use super::rules::rules_allow;
use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";
const PROGRESS_EVENT: &str = "download://progress";
const MAX_CONCURRENT: usize = 12;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub stage: &'static str,
    pub completed: usize,
    pub total: usize,
}

/// `group:artifact:version[:classifier]` -> the repository-relative jar path,
/// e.g. `net.fabricmc:fabric-loader:0.15.11` -> `net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar`.
fn maven_coordinate_to_path(coordinate: &str) -> Option<String> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    let [group, artifact, version] = parts.get(0..3)?.try_into().ok()?;
    let group_path = group.replace('.', "/");
    let file_name = match parts.get(3) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{group_path}/{artifact}/{version}/{file_name}"))
}

pub(crate) fn emit_progress(app: &AppHandle, stage: &'static str, completed: usize, total: usize) {
    if completed == 0 {
        crate::launcher_log::info("download", format!("Starting {stage} download: {total} file(s)"));
    }
    app.emit(PROGRESS_EVENT, DownloadProgress { stage, completed, total }).ok();
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// A cached file is trusted once its size matches, without re-reading and
/// re-hashing its full contents; re-launching a vanilla instance was hashing
/// thousands of already-good asset files every time, taking up to a minute.
/// Falls back to a full hash check when the expected size isn't known.
async fn file_matches(path: &Path, expected_sha1: Option<&str>, expected_size: Option<u64>) -> bool {
    if let Some(size) = expected_size {
        return tokio::fs::metadata(path).await.map(|m| m.len() == size).unwrap_or(false);
    }
    let Ok(bytes) = tokio::fs::read(path).await else { return false };
    match expected_sha1 {
        Some(expected) => sha1_hex(&bytes).eq_ignore_ascii_case(expected),
        None => !bytes.is_empty(),
    }
}

/// Downloads `url` to `dest` unless a valid cached copy already exists;
/// verifies the checksum of anything freshly downloaded.
pub(crate) async fn download_verified(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
) -> AppResult<()> {
    if dest.exists() && file_matches(dest, expected_sha1, expected_size).await {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let bytes = client.get(url).send().await?.error_for_status()?.bytes().await?;

    if let Some(expected) = expected_sha1 {
        let actual = sha1_hex(&bytes);
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(AppError::ChecksumMismatch {
                file: dest.display().to_string(),
                expected: expected.to_string(),
                actual,
            });
        }
    }

    let tmp = dest.with_extension("part");
    tokio::fs::write(&tmp, &bytes).await?;
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}

pub async fn ensure_client_jar(
    client: &reqwest::Client,
    version_json: &serde_json::Value,
    dest: &Path,
) -> AppResult<PathBuf> {
    let download = version_json
        .pointer("/downloads/client")
        .ok_or_else(|| AppError::Other("version JSON has no client download".into()))?;
    let url = download["url"].as_str().unwrap_or_default();
    let sha1 = download["sha1"].as_str();
    let size = download["size"].as_u64();

    download_verified(client, url, dest, sha1, size).await?;
    Ok(dest.to_path_buf())
}

pub struct LibraryResult {
    pub classpath: Vec<PathBuf>,
    pub native_jars: Vec<PathBuf>,
}

pub async fn ensure_libraries(
    app: &AppHandle,
    client: &reqwest::Client,
    version_json: &serde_json::Value,
    libraries_dir: &Path,
) -> AppResult<LibraryResult> {
    let features = HashMap::new();
    let libraries = version_json["libraries"].as_array().cloned().unwrap_or_default();

    let mut jobs: Vec<(String, Option<String>, Option<u64>, PathBuf, bool)> = Vec::new(); // (url, sha1, size, dest, is_native)

    for lib in &libraries {
        let rules = lib.get("rules").and_then(|r| r.as_array()).cloned();
        if !rules_allow(rules.as_ref(), &features) {
            continue;
        }

        if let Some(artifact) = lib.pointer("/downloads/artifact") {
            if let (Some(path), Some(url)) = (artifact["path"].as_str(), artifact["url"].as_str()) {
                jobs.push((
                    url.to_string(),
                    artifact["sha1"].as_str().map(str::to_string),
                    artifact["size"].as_u64(),
                    libraries_dir.join(path),
                    false,
                ));
            }
        } else if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
            // A Maven coordinate + repo base URL instead of a pre-resolved
            // artifact; defaults to Mojang's library host when no "url" is set.

            // A `natives` map here means only the classified jar exists;
            // skip the plain download, the natives block below resolves it.
            if lib.get("natives").is_none() {
                if let Some(path) = maven_coordinate_to_path(name) {
                    let base = lib
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("https://libraries.minecraft.net/");
                    let base = if base.ends_with('/') { base.to_string() } else { format!("{base}/") };
                    jobs.push((
                        format!("{base}{path}"),
                        lib.get("sha1").and_then(|v| v.as_str()).map(str::to_string),
                        None,
                        libraries_dir.join(path),
                        false,
                    ));
                }
            }
        }

        // Legacy (pre-1.19) per-OS native jars via the classifiers/natives maps.
        if let Some(natives_map) = lib.get("natives").and_then(|n| n.as_object()) {
            let os_key = super::rules::mojang_os_name();
            if let Some(classifier_key) = natives_map.get(os_key).and_then(|v| v.as_str()) {
                let classifier_key = classifier_key.replace("${arch}", "64");
                if let Some(classifier) = lib.pointer(&format!("/downloads/classifiers/{classifier_key}")) {
                    if let (Some(path), Some(url)) =
                        (classifier["path"].as_str(), classifier["url"].as_str())
                    {
                        jobs.push((
                            url.to_string(),
                            classifier["sha1"].as_str().map(str::to_string),
                            classifier["size"].as_u64(),
                            libraries_dir.join(path),
                            true,
                        ));
                    }
                } else if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
                    // Pre-manifest-v2 format: no `downloads` object at all,
                    // so build the classified maven path directly.
                    if let Some(path) = maven_coordinate_to_path(&format!("{name}:{classifier_key}")) {
                        let base = lib
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("https://libraries.minecraft.net/");
                        let base = if base.ends_with('/') { base.to_string() } else { format!("{base}/") };
                        jobs.push((format!("{base}{path}"), None, None, libraries_dir.join(path), true));
                    }
                }
            }
        }
    }

    let total = jobs.len();
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let app_ref = app;

    let results = futures_util::stream::iter(jobs.into_iter().map(|(url, sha1, size, dest, is_native)| {
        let client = client.clone();
        let completed = &completed;
        async move {
            download_verified(&client, &url, &dest, sha1.as_deref(), size).await?;
            let n = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            emit_progress(app_ref, "libraries", n, total);
            Ok::<_, AppError>((dest, is_native))
        }
    }))
    .buffer_unordered(MAX_CONCURRENT)
    .collect::<Vec<_>>()
    .await;

    let mut classpath = Vec::new();
    let mut native_jars = Vec::new();
    for result in results {
        let (path, is_native) = result?;
        if is_native {
            native_jars.push(path);
        } else {
            classpath.push(path);
        }
    }

    Ok(LibraryResult { classpath, native_jars })
}

pub async fn ensure_assets(
    app: &AppHandle,
    client: &reqwest::Client,
    version_json: &serde_json::Value,
    assets_dir: &Path,
) -> AppResult<()> {
    let asset_index = version_json
        .get("assetIndex")
        .ok_or_else(|| AppError::Other("version JSON has no assetIndex".into()))?;
    let index_id = asset_index["id"].as_str().unwrap_or("legacy");
    let index_url = asset_index["url"].as_str().unwrap_or_default();
    let index_sha1 = asset_index["sha1"].as_str();
    let index_size = asset_index["size"].as_u64();

    let index_path = assets_dir.join("indexes").join(format!("{index_id}.json"));
    download_verified(client, index_url, &index_path, index_sha1, index_size).await?;

    let index_bytes = tokio::fs::read(&index_path).await?;
    let index_json: serde_json::Value = serde_json::from_slice(&index_bytes)?;
    let is_virtual = index_json.get("virtual").and_then(|v| v.as_bool()).unwrap_or(false);
    let objects = index_json["objects"]
        .as_object()
        .cloned()
        .unwrap_or_default();

    let total = objects.len();
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let app_ref = app;

    let jobs = objects.into_iter().map(|(name, meta)| {
        let hash = meta["hash"].as_str().unwrap_or_default().to_string();
        let size = meta["size"].as_u64();
        let assets_dir = assets_dir.to_path_buf();
        let client = client.clone();
        let completed = &completed;
        async move {
            let prefix = &hash[0..2.min(hash.len())];
            let object_path = assets_dir.join("objects").join(prefix).join(&hash);
            let url = format!("{ASSETS_BASE_URL}/{prefix}/{hash}");
            download_verified(&client, &url, &object_path, Some(&hash), size).await?;

            if is_virtual {
                let virtual_path = assets_dir.join("virtual").join(index_id).join(&name);
                if !virtual_path.exists() {
                    if let Some(parent) = virtual_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::copy(&object_path, &virtual_path).await?;
                }
            }

            let n = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            emit_progress(app_ref, "assets", n, total);
            Ok::<_, AppError>(())
        }
    });

    let results = futures_util::stream::iter(jobs)
        .buffer_unordered(MAX_CONCURRENT)
        .collect::<Vec<_>>()
        .await;
    for result in results {
        result?;
    }

    Ok(())
}

/// Extracts native-library jars (pre-1.19 LWJGL layout) into `dest_dir`,
/// skipping signature/metadata entries as the vanilla launcher does.
pub fn extract_natives(native_jars: &[PathBuf], dest_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dest_dir)?;
    for jar_path in native_jars {
        let file = std::fs::File::open(jar_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::Other(format!("bad native jar {}: {e}", jar_path.display())))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AppError::Other(format!("bad zip entry: {e}")))?;
            let name = entry.name().to_string();
            if name.starts_with("META-INF/") || name.ends_with('/') {
                continue;
            }
            let out_path = dest_dir.join(Path::new(&name).file_name().unwrap_or_default());
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::file_matches;
    use std::io::Write;

    fn temp_file(contents: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("strata-test-{}", uuid::Uuid::new_v4()));
        std::fs::File::create(&path).unwrap().write_all(contents).unwrap();
        path
    }

    #[tokio::test]
    async fn size_match_skips_hashing_a_corrupt_file() {
        // Same length as "hello", but not actually "hello"; the size-only
        // fast path is expected to accept it anyway, unlike a real hash check.
        let path = temp_file(b"hallo");
        assert!(file_matches(&path, Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"), Some(5)).await);
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn size_mismatch_fails_even_without_hashing() {
        let path = temp_file(b"hello");
        assert!(!file_matches(&path, None, Some(999)).await);
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn falls_back_to_a_real_hash_check_when_size_is_unknown() {
        let path = temp_file(b"hello");
        assert!(file_matches(&path, Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"), None).await);
        assert!(!file_matches(&path, Some("0000000000000000000000000000000000000"), None).await);
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn missing_file_never_matches() {
        let path = std::env::temp_dir().join(format!("strata-test-missing-{}", uuid::Uuid::new_v4()));
        assert!(!file_matches(&path, None, Some(5)).await);
    }
}
