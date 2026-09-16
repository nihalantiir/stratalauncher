//! Real per-instance world listing (`saves/<world>/level.dat` + `icon.png`)
//! plus zip-based backup/restore, used before risky loader/version switches.

use crate::error::{AppError, AppResult};
use crate::timeutil::system_time_to_rfc3339;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    pub folder: String,
    pub name: String,
    pub icon_data_url: Option<String>,
    pub size_bytes: u64,
    pub last_played: Option<String>,
    pub last_backup: Option<String>,
    /// "survival" | "creative" | "adventure" | "spectator", or None if
    /// unreadable. Hardcore is a separate orthogonal flag, not a fifth mode.
    pub game_mode: Option<String>,
    pub hardcore: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub folder: String,
    pub file: String,
    pub created_at: String,
    pub size_bytes: u64,
}

fn saves_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("saves")
}

fn backups_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("backups")
}

fn millis_to_rfc3339(millis: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(millis).map(|d| d.to_rfc3339())
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Fields of interest pulled out of a `level.dat`'s NBT tree; everything else
/// is walked and discarded.
#[derive(Default)]
struct LevelData {
    level_name: Option<String>,
    last_played: Option<i64>,
    game_type: Option<i32>,
    hardcore: Option<bool>,
}

struct NbtReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> NbtReader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        if end > self.data.len() {
            return None;
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_be_bytes(self.take(8)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let len = self.u16()? as usize;
        Some(String::from_utf8_lossy(self.take(len)?).into_owned())
    }

    /// Reads (and, for the two fields we care about, records) one NBT tag's
    /// payload, recursing into compounds/lists to find nested fields.
    fn read_value(&mut self, tag: u8, name: &str, found: &mut LevelData) -> Option<()> {
        match tag {
            1 => {
                let v = self.u8()?;
                if name == "hardcore" {
                    found.hardcore = Some(v != 0);
                }
            }
            2 => {
                self.take(2)?;
            }
            3 => {
                let v = self.i32()?;
                if name == "GameType" {
                    found.game_type = Some(v);
                }
            }
            4 => {
                let v = self.i64()?;
                if name == "LastPlayed" {
                    found.last_played = Some(v);
                }
            }
            5 => {
                self.take(4)?;
            }
            6 => {
                self.take(8)?;
            }
            7 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n)?;
            }
            8 => {
                let s = self.string()?;
                if name == "LevelName" {
                    found.level_name = Some(s);
                }
            }
            9 => {
                let elem = self.u8()?;
                let n = self.i32()?.max(0);
                for _ in 0..n {
                    self.read_value(elem, "", found)?;
                }
            }
            10 => loop {
                let id = self.u8()?;
                if id == 0 {
                    break;
                }
                let nm = self.string()?;
                self.read_value(id, &nm, found)?;
            },
            11 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n.checked_mul(4)?)?;
            }
            12 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n.checked_mul(8)?)?;
            }
            _ => return None,
        }
        Some(())
    }
}

fn read_level_dat(path: &Path) -> Option<LevelData> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).ok()?;

    let mut reader = NbtReader { data: &buf, pos: 0 };
    let root_id = reader.u8()?;
    let root_name = reader.string()?;
    let mut found = LevelData::default();
    reader.read_value(root_id, &root_name, &mut found)?;
    Some(found)
}

fn find_latest_backup(backups_dir: &Path, folder: &str) -> Option<(SystemTime, PathBuf)> {
    let prefix = format!("{folder}_");
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(backups_dir).ok()?.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.starts_with(&prefix) || !fname.ends_with(".zip") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if latest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
            latest = Some((modified, entry.path()));
        }
    }
    latest
}

pub fn list_worlds(instance_dir: &Path) -> AppResult<Vec<WorldInfo>> {
    let saves = saves_dir(instance_dir);
    std::fs::create_dir_all(&saves)?;
    let backups = backups_dir(instance_dir);

    let mut worlds = Vec::new();
    for entry in std::fs::read_dir(&saves)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().to_string();
        let world_dir = entry.path();

        let level = read_level_dat(&world_dir.join("level.dat"));
        let name = level
            .as_ref()
            .and_then(|l| l.level_name.clone())
            .unwrap_or_else(|| folder.clone());
        let last_played = level
            .as_ref()
            .and_then(|l| l.last_played)
            .and_then(millis_to_rfc3339)
            .or_else(|| entry.metadata().ok().and_then(|m| m.modified().ok()).and_then(system_time_to_rfc3339));

        let icon_data_url = std::fs::read(world_dir.join("icon.png"))
            .ok()
            .map(|bytes| format!("data:image/png;base64,{}", base64_encode(&bytes)));

        let size_bytes = crate::fsutil::dir_size(&world_dir);
        let last_backup = find_latest_backup(&backups, &folder).and_then(|(t, _)| system_time_to_rfc3339(t));

        let game_mode = level.as_ref().and_then(|l| l.game_type).map(|gt| {
            match gt {
                1 => "creative",
                2 => "adventure",
                3 => "spectator",
                _ => "survival",
            }
            .to_string()
        });
        let hardcore = level.as_ref().and_then(|l| l.hardcore).unwrap_or(false);

        worlds.push(WorldInfo {
            folder,
            name,
            icon_data_url,
            size_bytes,
            last_played,
            last_backup,
            game_mode,
            hardcore,
        });
    }

    worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    Ok(worlds)
}

fn zip_dir(
    writer: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> AppResult<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            writer
                .add_directory(format!("{rel}/"), options)
                .map_err(|e| AppError::Other(format!("zip write failed: {e}")))?;
            zip_dir(writer, base, &path, options)?;
        } else {
            writer
                .start_file(rel, options)
                .map_err(|e| AppError::Other(format!("zip write failed: {e}")))?;
            let mut f = std::fs::File::open(&path)?;
            std::io::copy(&mut f, writer)?;
        }
    }
    Ok(())
}

pub fn backup_world(instance_dir: &Path, folder: &str) -> AppResult<BackupInfo> {
    let world_dir = saves_dir(instance_dir).join(folder);
    if !world_dir.is_dir() {
        return Err(AppError::NotFound(format!("world {folder}")));
    }
    let backups = backups_dir(instance_dir);
    std::fs::create_dir_all(&backups)?;

    // Millisecond precision: two backups triggered close together would
    // otherwise collide on the same filename and silently overwrite.
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%3f");
    let file_name = format!("{folder}_{stamp}.zip");
    let dest = backups.join(&file_name);

    let file = std::fs::File::create(&dest)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip_dir(&mut writer, &world_dir, &world_dir, options)?;
    writer
        .finish()
        .map_err(|e| AppError::Other(format!("zip write failed: {e}")))?;

    let size_bytes = std::fs::metadata(&dest)?.len();
    Ok(BackupInfo {
        folder: folder.to_string(),
        file: file_name,
        created_at: chrono::Utc::now().to_rfc3339(),
        size_bytes,
    })
}

/// Backs up every world in the instance, used automatically before a risky
/// loader/version switch. Best-effort: an unreadable `saves/` just means none.
pub fn backup_all_worlds(instance_dir: &Path) -> AppResult<Vec<BackupInfo>> {
    let saves = saves_dir(instance_dir);
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(&saves) else {
        return Ok(results);
    };
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let folder = entry.file_name().to_string_lossy().to_string();
            results.push(backup_world(instance_dir, &folder)?);
        }
    }
    Ok(results)
}

/// Extracts into a fresh staging dir first, swapping it in only once
/// extraction succeeds, so a corrupt backup can't delete the live world.
fn restore_from_path(world_dir: &Path, backup_path: &Path) -> AppResult<()> {
    let parent = world_dir
        .parent()
        .ok_or_else(|| AppError::Other("invalid world path".into()))?;
    let staging_dir = parent.join(format!(".restore-staging-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&staging_dir)?;

    let extracted: AppResult<()> = (|| -> AppResult<()> {
        let file = std::fs::File::open(backup_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| AppError::Other(format!("bad backup archive: {e}")))?;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AppError::Other(format!("bad zip entry: {e}")))?;
            let out_path = staging_dir.join(entry.name());
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
            }
        }
        Ok(())
    })();

    if let Err(e) = extracted {
        let _ = std::fs::remove_dir_all(&staging_dir);
        return Err(e);
    }

    if world_dir.exists() {
        std::fs::remove_dir_all(world_dir)?;
    }
    std::fs::rename(&staging_dir, world_dir)?;
    Ok(())
}

pub fn restore_latest_backup(instance_dir: &Path, folder: &str) -> AppResult<()> {
    let backups = backups_dir(instance_dir);
    let (_, backup_path) =
        find_latest_backup(&backups, folder).ok_or_else(|| AppError::NotFound(format!("backup for {folder}")))?;
    let world_dir = saves_dir(instance_dir).join(folder);
    restore_from_path(&world_dir, &backup_path)
}

pub fn restore_backup(instance_dir: &Path, folder: &str, file: &str) -> AppResult<()> {
    let backup_path = backups_dir(instance_dir).join(file);
    if !backup_path.is_file() {
        return Err(AppError::NotFound(format!("backup {file}")));
    }
    let world_dir = saves_dir(instance_dir).join(folder);
    restore_from_path(&world_dir, &backup_path)
}

pub fn list_backups(instance_dir: &Path, folder: &str) -> AppResult<Vec<BackupInfo>> {
    let backups = backups_dir(instance_dir);
    let prefix = format!("{folder}_");
    let mut list = Vec::new();
    let Ok(entries) = std::fs::read_dir(&backups) else {
        return Ok(list);
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".zip") {
            continue;
        }
        let meta = entry.metadata()?;
        let created_at = meta
            .modified()
            .ok()
            .and_then(system_time_to_rfc3339)
            .unwrap_or_default();
        list.push(BackupInfo {
            folder: folder.to_string(),
            file: file_name,
            created_at,
            size_bytes: meta.len(),
        });
    }
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(list)
}

pub fn delete_backup(instance_dir: &Path, file: &str) -> AppResult<()> {
    let path = backups_dir(instance_dir).join(file);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn delete_world(instance_dir: &Path, folder: &str) -> AppResult<()> {
    let world_dir = saves_dir(instance_dir).join(folder);
    if world_dir.is_dir() {
        std::fs::remove_dir_all(&world_dir)?;
    }
    Ok(())
}

pub fn ensure_saves_dir(instance_dir: &Path) -> AppResult<PathBuf> {
    let dir = saves_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
