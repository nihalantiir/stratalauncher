use super::models::{Account, AccountKind};
use crate::error::AppResult;
use rusqlite::{params, Connection, Row};

fn row_to_account(row: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        kind: AccountKind::from_str(&row.get::<_, String>(1)?),
        username: row.get(2)?,
        mc_uuid: row.get(3)?,
        skin_url: row.get(4)?,
        is_active: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
        skin_variant: row.get(8)?,
    })
}

const SELECT_COLUMNS: &str =
    "id, kind, username, mc_uuid, skin_url, is_active, created_at, last_used_at, skin_variant";

pub struct AccountsRepo;

impl AccountsRepo {
    pub fn list(conn: &Connection) -> AppResult<Vec<Account>> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM accounts ORDER BY created_at ASC");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_account)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get(conn: &Connection, id: &str) -> AppResult<Option<Account>> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM accounts WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_account(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn active(conn: &Connection) -> AppResult<Option<Account>> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM accounts WHERE is_active = 1 LIMIT 1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_account(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn upsert(conn: &Connection, account: &Account) -> AppResult<()> {
        conn.execute(
            "INSERT INTO accounts (id, kind, username, mc_uuid, skin_url, is_active, created_at, last_used_at, skin_variant)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                username = excluded.username,
                mc_uuid = excluded.mc_uuid,
                skin_url = excluded.skin_url,
                is_active = excluded.is_active,
                last_used_at = excluded.last_used_at,
                skin_variant = excluded.skin_variant",
            params![
                account.id,
                account.kind.as_str(),
                account.username,
                account.mc_uuid,
                account.skin_url,
                account.is_active as i64,
                account.created_at,
                account.last_used_at,
                account.skin_variant,
            ],
        )?;
        Ok(())
    }

    pub fn set_active(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("UPDATE accounts SET is_active = 0", [])?;
        conn.execute(
            "UPDATE accounts SET is_active = 1, last_used_at = ?2 WHERE id = ?1",
            params![id, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn remove(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }
}
