//! Java runtime auto-management: downloads the exact JRE Mojang's own
//! launcher would use, keyed by the version JSON's `javaVersion.component`.

use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

const MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const MAX_CONCURRENT: usize = 12;

#[cfg(all(windows, target_arch = "x86_64"))]
const OS_KEY: &str = "windows-x64";
#[cfg(all(windows, target_arch = "aarch64"))]
const OS_KEY: &str = "windows-arm64";
#[cfg(all(windows, target_arch = "x86"))]
const OS_KEY: &str = "windows-x86";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const OS_KEY: &str = "mac-os-arm64";
#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
const OS_KEY: &str = "mac-os";
#[cfg(all(target_os = "linux", target_arch = "x86"))]
const OS_KEY: &str = "linux-i386";
#[cfg(all(target_os = "linux", not(target_arch = "x86")))]
const OS_KEY: &str = "linux";

/// Component -> Java major version, matching Mojang's current builds. Used
/// to validate a manual Java override; automatic launches never need this.
pub fn component_major_version(component: &str) -> Option<u32> {
    match component {
        "jre-legacy" => Some(8),
        "java-runtime-alpha" => Some(16),
        "java-runtime-beta" | "java-runtime-gamma" | "java-runtime-gamma-snapshot" => Some(17),
        "java-runtime-delta" => Some(21),
        "java-runtime-epsilon" => Some(25),
        _ => None,
    }
}

fn runtime_dir(component: &str) -> PathBuf {
    crate::paths::java_dir().join(component)
}

fn java_exe_path(component: &str) -> PathBuf {
    let dir = runtime_dir(component);
    if cfg!(windows) {
        dir.join("bin").join("java.exe")
    } else {
        dir.join("bin").join("java")
    }
}

/// Which component a version needs, straight from its version JSON; Mojang
/// backfilled this field for every version, so no era-based guessing needed.
pub fn required_component(version_json: &Value) -> &str {
    version_json
        .pointer("/javaVersion/component")
        .and_then(|v| v.as_str())
        .unwrap_or("jre-legacy")
}

/// Ensures the given runtime component is downloaded, returning the path
/// to its `java` executable. A no-op (besides a stat) once cached.
pub async fn ensure_runtime(app: &AppHandle, client: &reqwest::Client, component: &str) -> AppResult<PathBuf> {
    let exe = java_exe_path(component);
    if exe.exists() {
        return Ok(exe);
    }

    crate::launcher_log::info("java", format!("Downloading Java runtime ({component})…"));

    let all: Value = client.get(MANIFEST_URL).send().await?.error_for_status()?.json().await?;
    let entry = all
        .pointer(&format!("/{OS_KEY}/{component}/0"))
        .ok_or_else(|| AppError::Other(format!("No {component} Java runtime is available for this platform.")))?;
    let manifest_url = entry
        .pointer("/manifest/url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Other(format!("{component}'s runtime entry has no manifest url")))?;

    let files_manifest: Value = client.get(manifest_url).send().await?.error_for_status()?.json().await?;
    let files = files_manifest
        .get("files")
        .and_then(|f| f.as_object())
        .ok_or_else(|| AppError::Other(format!("{component}'s runtime manifest has no files")))?;

    let dir = runtime_dir(component);
    // Directories first, since a file's parent must exist before download
    // and the manifest doesn't guarantee ordering.
    for (rel_path, meta) in files {
        if meta.get("type").and_then(|v| v.as_str()) == Some("directory") {
            std::fs::create_dir_all(dir.join(rel_path))?;
        }
    }

    let file_jobs: Vec<(String, PathBuf, Option<String>, Option<u64>, bool)> = files
        .iter()
        .filter_map(|(rel_path, meta)| {
            if meta.get("type").and_then(|v| v.as_str()) != Some("file") {
                return None; // "directory" (handled above) or "link" (not seen/needed on Windows)
            }
            let raw = meta.pointer("/downloads/raw")?;
            let url = raw.get("url").and_then(|v| v.as_str())?.to_string();
            let sha1 = raw.get("sha1").and_then(|v| v.as_str()).map(str::to_string);
            let size = raw.get("size").and_then(|v| v.as_u64());
            let executable = meta.get("executable").and_then(|v| v.as_bool()).unwrap_or(false);
            Some((url, dir.join(rel_path), sha1, size, executable))
        })
        .collect();

    let total = file_jobs.len();
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let component_owned = component.to_string();

    let results = futures_util::stream::iter(file_jobs.into_iter().map(|(url, dest, sha1, size, executable)| {
        let client = client.clone();
        let completed = &completed;
        let app = app;
        let component = &component_owned;
        async move {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            super::download::download_verified(&client, &url, &dest, sha1.as_deref(), size).await?;
            set_executable(&dest, executable)?;
            let n = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            app.emit(
                "download://progress",
                super::download::DownloadProgress { stage: "java", completed: n, total },
            )
            .ok();
            let _ = component;
            Ok::<_, AppError>(())
        }
    }))
    .buffer_unordered(MAX_CONCURRENT)
    .collect::<Vec<_>>()
    .await;

    for result in results {
        result?;
    }

    crate::launcher_log::info("java", format!("{component} runtime ready"));
    Ok(exe)
}

#[cfg(unix)]
fn set_executable(path: &std::path::Path, executable: bool) -> AppResult<()> {
    if !executable {
        return Ok(());
    }
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &std::path::Path, _executable: bool) -> AppResult<()> {
    Ok(())
}
