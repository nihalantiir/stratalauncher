mod accounts;
mod instances;
pub mod models;

pub use accounts::AccountsRepo;
pub use instances::InstancesRepo;

use crate::error::AppResult;
use crate::paths;
use rusqlite::Connection;
use std::sync::Mutex;

/// Migrations applied in order. Each one runs once, tracked via `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_instances.sql"),
    include_str!("migrations/0003_loader_version.sql"),
    include_str!("migrations/0004_skin_variant.sql"),
    include_str!("migrations/0005_last_crashed.sql"),
    include_str!("migrations/0006_advanced_launch_settings.sql"),
    include_str!("migrations/0007_custom_client_jar.sql"),
];

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open() -> AppResult<Self> {
        std::fs::create_dir_all(paths::data_dir())?;
        let conn = Connection::open(paths::db_path())?;
        conn.pragma_update(None, "foreign_keys", true)?;
        run_migrations(&conn)?;
        Ok(Db(Mutex::new(conn)))
    }
}

fn run_migrations(conn: &Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let current = current as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        if i < current {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", (i + 1) as i64)?;
    }
    Ok(())
}

pub fn kv_get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM kv_settings WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn kv_set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO kv_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}
