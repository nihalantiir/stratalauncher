use super::models::Instance;
use crate::error::AppResult;
use rusqlite::{params, Connection, Row};

const CURRENT_INSTANCE_KEY: &str = "current_instance_id";

fn row_to_instance(row: &Row) -> rusqlite::Result<Instance> {
    Ok(Instance {
        id: row.get(0)?,
        name: row.get(1)?,
        loader: row.get(2)?,
        mc_version: row.get(3)?,
        memory_mb: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
        jvm_args: row.get(5)?,
        java_path: row.get(6)?,
        icon_biome: row.get(7)?,
        group_name: row.get(8)?,
        created_at: row.get(9)?,
        last_played_at: row.get(10)?,
        loader_version: row.get(11)?,
        last_crashed: row.get::<_, Option<i64>>(12)?.map(|v| v != 0),
        min_memory_mb: row.get::<_, Option<i64>>(13)?.map(|v| v as u32),
        window_width: row.get::<_, Option<i64>>(14)?.map(|v| v as u32),
        window_height: row.get::<_, Option<i64>>(15)?.map(|v| v as u32),
        window_maximized: row.get::<_, i64>(16)? != 0,
        skip_java_check: row.get::<_, i64>(17)? != 0,
        env_vars: row.get(18)?,
        pre_launch_cmd: row.get(19)?,
        wrapper_cmd: row.get(20)?,
        post_exit_cmd: row.get(21)?,
        console_mode: row.get(22)?,
        launcher_behavior: row.get(23)?,
        quick_play_mode: row.get(24)?,
        quick_play_target: row.get(25)?,
        custom_client_jar: row.get(26)?,
    })
}

const SELECT_COLUMNS: &str = "id, name, loader, mc_version, memory_mb, jvm_args, java_path, icon_biome, group_name, created_at, last_played_at, loader_version, last_crashed,
    min_memory_mb, window_width, window_height, window_maximized, skip_java_check, env_vars, pre_launch_cmd, wrapper_cmd, post_exit_cmd, console_mode, launcher_behavior, quick_play_mode, quick_play_target, custom_client_jar";

pub struct InstancesRepo;

impl InstancesRepo {
    pub fn list(conn: &Connection) -> AppResult<Vec<Instance>> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM instances ORDER BY created_at ASC");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_instance)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get(conn: &Connection, id: &str) -> AppResult<Option<Instance>> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM instances WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_instance(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn insert(conn: &Connection, instance: &Instance) -> AppResult<()> {
        conn.execute(
            "INSERT INTO instances (id, name, loader, mc_version, memory_mb, jvm_args, java_path, icon_biome, group_name, created_at, last_played_at, loader_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                instance.id,
                instance.name,
                instance.loader,
                instance.mc_version,
                instance.memory_mb,
                instance.jvm_args,
                instance.java_path,
                instance.icon_biome,
                instance.group_name,
                instance.created_at,
                instance.last_played_at,
                instance.loader_version,
            ],
        )?;
        Ok(())
    }

    pub fn update(conn: &Connection, instance: &Instance) -> AppResult<()> {
        conn.execute(
            "UPDATE instances SET name = ?2, loader = ?3, mc_version = ?4, memory_mb = ?5, jvm_args = ?6,
                java_path = ?7, icon_biome = ?8, group_name = ?9, last_played_at = ?10, loader_version = ?11,
                min_memory_mb = ?12, window_width = ?13, window_height = ?14, window_maximized = ?15,
                skip_java_check = ?16, env_vars = ?17, pre_launch_cmd = ?18, wrapper_cmd = ?19, post_exit_cmd = ?20,
                console_mode = ?21, launcher_behavior = ?22, quick_play_mode = ?23, quick_play_target = ?24,
                custom_client_jar = ?25
             WHERE id = ?1",
            params![
                instance.id,
                instance.name,
                instance.loader,
                instance.mc_version,
                instance.memory_mb,
                instance.jvm_args,
                instance.java_path,
                instance.icon_biome,
                instance.group_name,
                instance.last_played_at,
                instance.loader_version,
                instance.min_memory_mb,
                instance.window_width,
                instance.window_height,
                instance.window_maximized as i64,
                instance.skip_java_check as i64,
                instance.env_vars,
                instance.pre_launch_cmd,
                instance.wrapper_cmd,
                instance.post_exit_cmd,
                instance.console_mode,
                instance.launcher_behavior,
                instance.quick_play_mode,
                instance.quick_play_target,
                instance.custom_client_jar,
            ],
        )?;
        Ok(())
    }

    pub fn touch_last_played(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute(
            "UPDATE instances SET last_played_at = ?2 WHERE id = ?1",
            params![id, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn set_last_crashed(conn: &Connection, id: &str, crashed: bool) -> AppResult<()> {
        conn.execute(
            "UPDATE instances SET last_crashed = ?2 WHERE id = ?1",
            params![id, crashed as i64],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("DELETE FROM instances WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn current_id(conn: &Connection) -> AppResult<Option<String>> {
        super::kv_get(conn, CURRENT_INSTANCE_KEY)
    }

    pub fn set_current_id(conn: &Connection, id: &str) -> AppResult<()> {
        super::kv_set(conn, CURRENT_INSTANCE_KEY, id)
    }
}
