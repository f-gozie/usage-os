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
    pub category_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: i64,
    pub category_id: i64,
    pub match_field: String, // "process" or "title"
    pub pattern: String,
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
            is_idle INTEGER NOT NULL,
            category_id INTEGER REFERENCES categories(id)
        )",
        [],
    )?;

    // Create categories table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL
        )",
        [],
    )?;

    // Create rules table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
            match_field TEXT NOT NULL,
            pattern TEXT NOT NULL
        )",
        [],
    )?;

    // Migration: Add category_id to activity_logs if it doesn't exist (for existing DBs)
    // We check if the column exists by selecting from pragma_table_info
    let has_category_id: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('activity_logs') WHERE name='category_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;

    if !has_category_id {
        println!("[Database] Migrating: Adding category_id to activity_logs");
        if let Err(e) = conn.execute("ALTER TABLE activity_logs ADD COLUMN category_id INTEGER REFERENCES categories(id)", []) {
            eprintln!("[Database] Migration failed: {}", e);
        }
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_start_time ON activity_logs(start_time)",
        [],
    )?;

    println!("[Database] Initialized database at {:?}", db_path);
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn get_last_activity_log(conn: &Connection) -> Result<Option<ActivityLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, process_name, window_title, start_time, end_time, is_idle, category_id
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
            category_id: row.get(6)?,
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
    category_id: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO activity_logs (process_name, window_title, start_time, end_time, is_idle, category_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (process_name, window_title, timestamp, timestamp, is_idle as i64, category_id),
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
/// If the last entry matches the current process/title/idle state AND
/// the time gap is reasonable (< 30 seconds), updates its end_time.
/// Otherwise, inserts a new entry.
///
/// This prevents false duration inflation when the app is restarted
/// after being closed for hours/days.
pub fn log_activity(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
    is_idle: bool,
    timestamp: i64,
) -> Result<()> {
    const MAX_GAP_SECONDS: i64 = 30;

    // Calculate category_id
    // Note: This fetches rules every time. For high-frequency polling (5s), this is fine with SQLite.
    // Optimization: Cache rules in memory if needed later.
    let category_id = match find_category(conn, process_name, window_title) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("[Database] Failed to determine category: {}", e);
            None
        }
    };

    match get_last_activity_log(conn)? {
        Some(last_log) => {
            let time_gap = timestamp - last_log.end_time;
            // We also check if category changed? 
            // Ideally yes, if rules change mid-stream or dynamic categorization happens.
            // But usually category depends on process/title. 
            // If process/title are same, category should be same given same rules.
            // So checking process/title/idle is sufficient.
            
            let is_same_activity = last_log.process_name == process_name
                && last_log.window_title == window_title
                && last_log.is_idle == is_idle;

            if is_same_activity && time_gap <= MAX_GAP_SECONDS {
                update_last_activity_end_time(conn, last_log.id, timestamp)?;
            } else {
                insert_activity_log(conn, process_name, window_title, is_idle, timestamp, category_id)?;
                if time_gap > MAX_GAP_SECONDS {
                    println!(
                        "[Database] Gap detected ({} seconds), starting new entry",
                        time_gap
                    );
                }
                println!(
                    "[Database] New activity: {} - {} (idle: {}) [Category: {:?}]",
                    process_name, window_title, is_idle, category_id
                );
            }
        }
        None => {
            insert_activity_log(conn, process_name, window_title, is_idle, timestamp, category_id)?;
            println!(
                "[Database] First activity: {} - {} (idle: {}) [Category: {:?}]",
                process_name, window_title, is_idle, category_id
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
/// * Vector of activity logs sorted by start_time
pub fn get_activity_logs(
    conn: &Connection,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<ActivityLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, process_name, window_title, start_time, end_time, is_idle, category_id
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
            category_id: row.get(6)?,
        })
    })?;

    logs.collect()
}

// --- Category CRUD ---

pub fn get_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT id, name, color FROM categories ORDER BY name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn create_category(conn: &Connection, name: &str, color: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO categories (name, color) VALUES (?1, ?2)",
        (name, color),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_category(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE activity_logs SET category_id = NULL WHERE category_id = ?1", [id])?;
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

// --- Rule CRUD ---

pub fn get_rules(conn: &Connection) -> Result<Vec<Rule>> {
    let mut stmt = conn.prepare("SELECT id, category_id, match_field, pattern FROM rules ORDER BY id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Rule {
            id: row.get(0)?,
            category_id: row.get(1)?,
            match_field: row.get(2)?,
            pattern: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn create_rule(
    conn: &Connection,
    category_id: i64,
    match_field: &str,
    pattern: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO rules (category_id, match_field, pattern) VALUES (?1, ?2, ?3)",
        (category_id, match_field, pattern),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_rule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;
    Ok(())
}

// --- Categorization Logic ---

fn find_category(conn: &Connection, process_name: &str, window_title: &str) -> Result<Option<i64>> {
    let rules = get_rules(conn)?;
    for rule in rules {
        let match_target = if rule.match_field == "process" {
            process_name
        } else {
            window_title
        };
        
        if match_target.to_lowercase().contains(&rule.pattern.to_lowercase()) {
            return Ok(Some(rule.category_id));
        }
    }
    Ok(None)
}

pub fn reprocess_logs(conn: &Connection) -> Result<()> {
    // 1. Reset all categories
    conn.execute("UPDATE activity_logs SET category_id = NULL", [])?;
    
    // 2. Get all rules (ordered by ID, so first rule created = higher priority if we assume ID order)
    // To support re-ordering, we'd need a 'priority' column, but for now ID order is fine.
    // Logic: First rule that matches wins.
    let rules = get_rules(conn)?;
    
    // 3. Apply rules
    for rule in rules {
        let pattern = format!("%{}%", rule.pattern);
        if rule.match_field == "process" {
             conn.execute(
                "UPDATE activity_logs SET category_id = ?1 WHERE category_id IS NULL AND lower(process_name) LIKE lower(?2)",
                (rule.category_id, pattern),
            )?;
        } else {
             conn.execute(
                "UPDATE activity_logs SET category_id = ?1 WHERE category_id IS NULL AND lower(window_title) LIKE lower(?2)",
                (rule.category_id, pattern),
            )?;
        }
    }
    
    Ok(())
}
