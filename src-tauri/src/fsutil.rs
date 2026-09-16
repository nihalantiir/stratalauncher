//! Shared filesystem helpers used by more than one module.

use crate::error::{AppError, AppResult};
use std::path::{Component, Path, PathBuf};

/// A zip entry's path is third-party input; rejects anything that could
/// escape `dest_root` via `..`, an absolute path, or a drive letter.
pub fn safe_join(dest_root: &Path, entry_name: &str, context: &str) -> AppResult<PathBuf> {
    let rel = Path::new(entry_name);
    // `has_root()`, not `is_absolute()`: on Windows a leading-slash path like
    // "/etc/passwd" has no drive letter so `is_absolute()` is false, but
    // `Path::join` still treats it as rooted and replaces `dest_root` entirely.
    if rel.has_root() || rel.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_))) {
        return Err(AppError::Other(format!("unsafe path in {context}: {entry_name}")));
    }
    Ok(dest_root.join(rel))
}

pub fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

pub fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            total += dir_size(&entry.path());
        } else {
            total += meta.len();
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::safe_join;
    use std::path::Path;

    #[test]
    fn joins_ordinary_relative_entries() {
        let root = Path::new("/data/instances/abc");
        assert_eq!(safe_join(root, "mods/sodium.jar", "test").unwrap(), root.join("mods/sodium.jar"));
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let root = Path::new("/data/instances/abc");
        assert!(safe_join(root, "../../etc/passwd", "test").is_err());
        assert!(safe_join(root, "mods/../../escape.jar", "test").is_err());
    }

    #[test]
    fn rejects_absolute_and_drive_letter_entries() {
        let root = Path::new("/data/instances/abc");
        assert!(safe_join(root, "/etc/passwd", "test").is_err());
        assert!(safe_join(root, "C:\\Windows\\System32\\evil.dll", "test").is_err());
    }
}
