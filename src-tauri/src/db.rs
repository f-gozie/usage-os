use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use serde::{Deserialize, Serialize};

pub type DbConnection = Arc<Mutex<Connection>>;

/// Activity log entry representing a time block of app usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: i64,
    pub process_name: String,
    pub window_title: String,
    pub start_time: i64,
    pub end_time: i64,
    pub is_idle: bool,
}

/// Get the database file path in the app's data directory.
///
/// Creates the directory if it doesn't exist.
pub fn get_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir.join("usage.db"))
}

/// Initialize the SQLite database and create schema if needed.
///
/// Returns a thread-safe database connection wrapped in Arc<Mutex>.
pub fn init_database(db_path: &PathBuf) -> Result<DbConnection> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS activity_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            process_name TEXT NOT NULL,
            window_title TEXT NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER NOT NULL,
            is_idle INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_start_time ON activity_logs(start_time)",
        [],
    )?;

    println!("[Database] Initialized database at {:?}", db_path);
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn get_last_activity_log(conn: &Connection) -> Result<Option<ActivityLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, process_name, window_title, start_time, end_time, is_idle 
         FROM activity_logs 
         ORDER BY id DESC 
         LIMIT 1"
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(ActivityLog {
            id: row.get(0)?,
            process_name: row.get(1)?,
            window_title: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            is_idle: row.get::<_, i64>(5)? != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert_activity_log(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
    is_idle: bool,
    timestamp: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO activity_logs (process_name, window_title, start_time, end_time, is_idle)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (process_name, window_title, timestamp, timestamp, is_idle as i64),
    )?;
    Ok(())
}

pub fn update_last_activity_end_time(conn: &Connection, id: i64, timestamp: i64) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs SET end_time = ?1 WHERE id = ?2",
        (timestamp, id),
    )?;
    Ok(())
}

/// Log activity with smart coalescing logic.
///
/// If the last entry matches the current process/title/idle state,
/// updates its end_time. Otherwise, inserts a new entry.
pub fn log_activity(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
    is_idle: bool,
    timestamp: i64,
) -> Result<()> {
    match get_last_activity_log(conn)? {
        Some(last_log) => {
            if last_log.process_name == process_name
                && last_log.window_title == window_title
                && last_log.is_idle == is_idle
            {
                update_last_activity_end_time(conn, last_log.id, timestamp)?;
            } else {
                insert_activity_log(conn, process_name, window_title, is_idle, timestamp)?;
                println!(
                    "[Database] New activity: {} - {} (idle: {})",
                    process_name, window_title, is_idle
                );
            }
        }
        None => {
            insert_activity_log(conn, process_name, window_title, is_idle, timestamp)?;
            println!(
                "[Database] First activity: {} - {} (idle: {})",
                process_name, window_title, is_idle
            );
        }
    }
    Ok(())
}

/// Thread-safe wrapper for logging activity.
pub fn log_activity_safe(
    db_conn: &DbConnection,
    process_name: &str,
    window_title: &str,
    is_idle: bool,
    timestamp: i64,
) -> Result<(), String> {
    let conn = db_conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    log_activity(&conn, process_name, window_title, is_idle, timestamp)
        .map_err(|e| format!("Database error: {}", e))
}

/// Query activity logs within a time range.
///
/// # Arguments
/// * `conn` - Database connection
/// * `start_time` - Unix timestamp for range start
/// * `end_time` - Unix timestamp for range end
///
/// # Returns
/// Vector of activity logs sorted by start_time
pub fn get_activity_logs(
    conn: &Connection,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<ActivityLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, process_name, window_title, start_time, end_time, is_idle 
         FROM activity_logs 
         WHERE start_time >= ?1 AND start_time <= ?2
         ORDER BY start_time ASC"
    )?;

    let logs = stmt.query_map([start_time, end_time], |row| {
        Ok(ActivityLog {
            id: row.get(0)?,
            process_name: row.get(1)?,
            window_title: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            is_idle: row.get::<_, i64>(5)? != 0,
        })
    })?;

    logs.collect()
}

