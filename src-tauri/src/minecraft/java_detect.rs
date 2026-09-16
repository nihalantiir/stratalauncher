//! Finds Java installs already on the system, for the Custom picker.
//! Registry keys verified against Prism Launcher's own JavaUtils.cpp.

use super::launch::probe_java_major_version;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstallation {
    pub path: String,
    pub major_version: u32,
    /// "JAVA_HOME" | "PATH" | "Registry" | "Common location" | "Strata-managed".
    pub source: String,
}

fn java_exe_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

fn env_and_path_candidates() -> Vec<(PathBuf, &'static str)> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("JAVA_HOME") {
        out.push((Path::new(&home).join("bin").join(java_exe_name()), "JAVA_HOME"));
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            out.push((dir.join(java_exe_name()), "PATH"));
        }
    }
    out
}

#[cfg(windows)]
fn registry_and_common_dir_candidates() -> Vec<(PathBuf, &'static str)> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    // (subkey path under the hive, value name holding the install dir).
    const ROOTS: &[(&str, &str)] = &[
        (r"SOFTWARE\JavaSoft\Java Runtime Environment", "JavaHome"),
        (r"SOFTWARE\JavaSoft\Java Development Kit", "JavaHome"),
        (r"SOFTWARE\JavaSoft\JRE", "JavaHome"),
        (r"SOFTWARE\JavaSoft\JDK", "JavaHome"),
        (r"SOFTWARE\AdoptOpenJDK\JRE", "Path"),
        (r"SOFTWARE\AdoptOpenJDK\JDK", "Path"),
        (r"SOFTWARE\Eclipse Foundation\JDK", "Path"),
        (r"SOFTWARE\Eclipse Adoptium\JRE", "Path"),
        (r"SOFTWARE\Eclipse Adoptium\JDK", "Path"),
        (r"SOFTWARE\Semeru\JRE", "Path"),
        (r"SOFTWARE\Semeru\JDK", "Path"),
        (r"SOFTWARE\Microsoft\JDK", "Path"),
        (r"SOFTWARE\Azul Systems\Zulu", "InstallationPath"),
        (r"SOFTWARE\BellSoft\Liberica", "InstallationPath"),
    ];

    let mut out = Vec::new();
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            for (root, value_name) in ROOTS {
                let Ok(base) = RegKey::predef(hive).open_subkey_with_flags(root, KEY_READ | view) else { continue };
                for version_key in base.enum_keys().flatten() {
                    let Ok(sub) = base.open_subkey(&version_key) else { continue };
                    let Ok(install_dir) = sub.get_value::<String, _>(*value_name) else { continue };
                    out.push((Path::new(&install_dir).join("bin").join("java.exe"), "Registry"));
                }
            }
        }
    }

    // A Java that predates, or whose installer skipped, the registry write.
    for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
        let Ok(pf) = std::env::var(env_var) else { continue };
        for vendor_dir in ["Java", "Eclipse Adoptium", "Microsoft"] {
            let Ok(entries) = std::fs::read_dir(Path::new(&pf).join(vendor_dir)) else { continue };
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    out.push((entry.path().join("bin").join("java.exe"), "Common location"));
                }
            }
        }
    }
    out
}

#[cfg(not(windows))]
fn registry_and_common_dir_candidates() -> Vec<(PathBuf, &'static str)> {
    Vec::new()
}

fn strata_managed_candidates() -> Vec<(PathBuf, &'static str)> {
    let Ok(entries) = std::fs::read_dir(crate::paths::java_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| (entry.path().join("bin").join(java_exe_name()), "Strata-managed"))
        .collect()
}

/// `canonicalize()` prepends this verbatim-path marker on Windows; harmless
/// to a real launch, but ugly to show a user and non-standard to store.
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    match path.to_str() {
        Some(s) => PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(s)),
        None => path,
    }
}

/// Every distinct, runnable Java on the system, deduped and probed for its
/// real major version. Not cached, so a fresh install shows up on re-scan.
pub fn detect_installations() -> Vec<JavaInstallation> {
    let mut candidates = env_and_path_candidates();
    candidates.extend(registry_and_common_dir_candidates());
    candidates.extend(strata_managed_candidates());

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (path, source) in candidates {
        if !path.is_file() {
            continue;
        }
        let canonical = std::fs::canonicalize(&path).map(strip_verbatim_prefix).unwrap_or(path);
        if !seen.insert(canonical.clone()) {
            continue;
        }
        if let Ok(major_version) = probe_java_major_version(&canonical) {
            out.push(JavaInstallation { path: canonical.display().to_string(), major_version, source: source.to_string() });
        }
    }
    out.sort_by(|a, b| b.major_version.cmp(&a.major_version));
    out
}
