//! Auto-picks the right LWJGL major version for a Coremods jar override
//! (e.g. BTA 7.3+ bundles an LWJGL 3 shim under old `org.lwjgl.*` names).

use std::path::Path;

/// Mojang library entries for LWJGL 3.3.3, lifted verbatim from `1.21.json`
/// (the first vanilla version to bundle it), covering every platform.
const LWJGL3_3_3_3_LIBRARIES: &str = include_str!("lwjgl3-3.3.3.json");

pub fn lwjgl3_libraries() -> Vec<serde_json::Value> {
    serde_json::from_str(LWJGL3_3_3_3_LIBRARIES).unwrap_or_default()
}

/// Scans a jar's `org/lwjgl/*` classes for a constant-pool reference to
/// `org/lwjgl/glfw/` or `org/lwjgl/system/`, packages that exist only in LWJGL 3.
pub fn coremod_needs_lwjgl3(jar_path: &Path) -> bool {
    let file = match std::fs::File::open(jar_path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return false,
    };
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .filter(|name| name.starts_with("org/lwjgl/") && name.ends_with(".class"))
        .collect();
    for name in names {
        let Ok(mut entry) = archive.by_name(&name) else { continue };
        let mut bytes = Vec::new();
        if std::io::Read::read_to_end(&mut entry, &mut bytes).is_err() {
            continue;
        }
        if contains(&bytes, b"org/lwjgl/glfw/") || contains(&bytes, b"org/lwjgl/system/") {
            return true;
        }
    }
    false
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
