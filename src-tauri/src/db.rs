use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub type DbConnection = Arc<Mutex<Connection>>;

/// Current Unix timestamp in seconds.
///
/// If the system clock is before the Unix epoch (clock skew), this falls back
/// to `Duration::ZERO` rather than panicking, yielding a timestamp of 0.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

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
    pub ignore_title: bool,
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

// --- Migration System ---

/// A database migration with a version number and apply function.
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// All migrations in order. Each runs exactly once.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: "
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
                match_field TEXT NOT NULL,
                pattern TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER NOT NULL,
                is_idle INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_start_time ON activity_logs(start_time);
        ",
    },
    Migration {
        version: 2,
        name: "add_category_id_to_activity_logs",
        sql: "
            ALTER TABLE activity_logs ADD COLUMN category_id INTEGER REFERENCES categories(id);
        ",
    },
    Migration {
        version: 3,
        name: "add_settings_table",
        sql: "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ",
    },
    Migration {
        version: 4,
        name: "add_ignore_title_to_rules",
        sql: "
            ALTER TABLE rules ADD COLUMN ignore_title INTEGER NOT NULL DEFAULT 0;
        ",
    },
];

/// Ensure the schema_migrations table exists.
fn ensure_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )?;
    Ok(())
}

/// Get the highest applied migration version, or 0 if none.
fn get_current_version(conn: &Connection) -> Result<i64> {
    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    ensure_migrations_table(conn)?;
    let current = get_current_version(conn)?;

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        println!(
            "[Database] Running migration {}: {}",
            migration.version, migration.name
        );
        conn.execute_batch(migration.sql)?;

        let now = now_unix();

        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            (migration.version, migration.name, now),
        )?;
    }

    Ok(())
}

/// Initialize the SQLite database with migration-based schema management.
///
/// Returns a thread-safe database connection wrapped in Arc<Mutex>.
pub fn init_database(db_path: &PathBuf) -> Result<DbConnection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    run_migrations(&conn)?;
    println!("[Database] Initialized database at {:?}", db_path);
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn get_last_activity_log(conn: &Connection) -> Result<Option<ActivityLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, process_name, window_title, start_time, end_time, is_idle, category_id
         FROM activity_logs 
         ORDER BY id DESC 
         LIMIT 1",
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
                insert_activity_log(
                    conn,
                    process_name,
                    window_title,
                    is_idle,
                    timestamp,
                    category_id,
                )?;
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
            insert_activity_log(
                conn,
                process_name,
                window_title,
                is_idle,
                timestamp,
                category_id,
            )?;
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
    let conn = db_conn
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
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
         ORDER BY start_time ASC",
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
    conn.execute(
        "UPDATE activity_logs SET category_id = NULL WHERE category_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

// --- Rule CRUD ---

pub fn get_rules(conn: &Connection) -> Result<Vec<Rule>> {
    let mut stmt = conn.prepare(
        "SELECT id, category_id, match_field, pattern, ignore_title FROM rules ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Rule {
            id: row.get(0)?,
            category_id: row.get(1)?,
            match_field: row.get(2)?,
            pattern: row.get(3)?,
            ignore_title: row.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect()
}

pub fn create_rule(
    conn: &Connection,
    category_id: i64,
    match_field: &str,
    pattern: &str,
    ignore_title: bool,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO rules (category_id, match_field, pattern, ignore_title) VALUES (?1, ?2, ?3, ?4)",
        (category_id, match_field, pattern, ignore_title as i64),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_rule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;
    Ok(())
}

// --- Categorization Logic ---

pub fn find_category(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
) -> Result<Option<i64>> {
    let rules = get_rules(conn)?;
    for rule in rules {
        let match_target = if rule.match_field == "process" {
            process_name
        } else {
            window_title
        };

        if match_target
            .to_lowercase()
            .contains(&rule.pattern.to_lowercase())
        {
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

/// Delete activity logs older than the given number of days.
///
/// Returns the number of rows deleted.
pub fn cleanup_old_data(conn: &Connection, retention_days: i64) -> Result<usize> {
    if retention_days <= 0 {
        return Ok(0);
    }
    let cutoff = now_unix() - (retention_days * 86400);
    let deleted = conn.execute("DELETE FROM activity_logs WHERE end_time < ?1", [cutoff])?;
    if deleted > 0 {
        println!(
            "[Database] Cleaned up {} old activity logs (retention: {} days)",
            deleted, retention_days
        );
    }
    Ok(deleted)
}

// --- Settings ---

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create an in-memory database using the migration system.
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run_migrations(&conn).expect("Migrations should succeed");
        conn
    }

    // --- Schema tests ---

    #[test]
    fn test_init_database_creates_schema() {
        let conn = setup_test_db();
        // Verify all tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"activity_logs".to_string()));
        assert!(tables.contains(&"categories".to_string()));
        assert!(tables.contains(&"rules".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"schema_migrations".to_string()));
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = setup_test_db();
        let v1 = get_current_version(&conn).unwrap();
        // Running migrations again should be a no-op
        run_migrations(&conn).unwrap();
        let v2 = get_current_version(&conn).unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v2, 4); // We have 4 migrations
    }

    #[test]
    fn test_migration_versions_recorded() {
        let conn = setup_test_db();
        let mut stmt = conn
            .prepare("SELECT version, name FROM schema_migrations ORDER BY version")
            .unwrap();
        let migrations: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(migrations.len(), 4);
        assert_eq!(migrations[0].0, 1);
        assert_eq!(migrations[0].1, "initial_schema");
        assert_eq!(migrations[1].0, 2);
        assert_eq!(migrations[2].0, 3);
        assert_eq!(migrations[3].0, 4);
        assert_eq!(migrations[3].1, "add_ignore_title_to_rules");
    }

    // --- Activity log round-trip ---

    #[test]
    fn test_insert_and_get_activity_logs() {
        let conn = setup_test_db();
        insert_activity_log(&conn, "firefox", "GitHub", false, 1000, None).unwrap();
        insert_activity_log(&conn, "code", "main.rs", false, 1010, None).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].process_name, "firefox");
        assert_eq!(logs[1].process_name, "code");
    }

    // --- Coalescing tests ---

    #[test]
    fn test_coalesce_same_process_within_30s() {
        let conn = setup_test_db();
        let t = 1000;
        log_activity(&conn, "firefox", "GitHub", false, t).unwrap();
        log_activity(&conn, "firefox", "GitHub", false, t + 5).unwrap();
        log_activity(&conn, "firefox", "GitHub", false, t + 10).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1, "Should coalesce into a single entry");
        assert_eq!(logs[0].start_time, t);
        assert_eq!(logs[0].end_time, t + 10);
    }

    #[test]
    fn test_coalesce_same_process_after_30s_gap() {
        let conn = setup_test_db();
        let t = 1000;
        log_activity(&conn, "firefox", "GitHub", false, t).unwrap();
        // Gap of 31s — exceeds MAX_GAP_SECONDS
        log_activity(&conn, "firefox", "GitHub", false, t + 31).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 2, "Should create a new entry after 30s gap");
        assert_eq!(logs[0].start_time, t);
        assert_eq!(logs[1].start_time, t + 31);
    }

    #[test]
    fn test_coalesce_different_process_new_entry() {
        let conn = setup_test_db();
        let t = 1000;
        log_activity(&conn, "firefox", "GitHub", false, t).unwrap();
        log_activity(&conn, "code", "main.rs", false, t + 5).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 2, "Different process should create new entry");
        assert_eq!(logs[0].process_name, "firefox");
        assert_eq!(logs[1].process_name, "code");
    }

    #[test]
    fn test_coalesce_idle_state_change() {
        let conn = setup_test_db();
        let t = 1000;
        log_activity(&conn, "firefox", "GitHub", false, t).unwrap();
        // Same process but now idle
        log_activity(&conn, "firefox", "GitHub", true, t + 5).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 2, "Idle state change should create new entry");
        assert!(!logs[0].is_idle);
        assert!(logs[1].is_idle);
    }

    #[test]
    fn test_coalesce_boundary_exactly_30s() {
        let conn = setup_test_db();
        let t = 1000;
        log_activity(&conn, "firefox", "GitHub", false, t).unwrap();
        // Exactly 30s gap — should still coalesce (MAX_GAP_SECONDS is <=)
        log_activity(&conn, "firefox", "GitHub", false, t + 30).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1, "Exactly 30s gap should still coalesce");
        assert_eq!(logs[0].end_time, t + 30);
    }

    // --- find_category tests ---

    #[test]
    fn test_find_category_matches_process_name() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Browsers", "#ff0000").unwrap();
        create_rule(&conn, cat_id, "process", "firefox", false).unwrap();

        let result = find_category(&conn, "firefox", "Some Page").unwrap();
        assert_eq!(result, Some(cat_id));
    }

    #[test]
    fn test_find_category_case_insensitive() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Browsers", "#ff0000").unwrap();
        create_rule(&conn, cat_id, "process", "firefox", false).unwrap();

        let result = find_category(&conn, "Firefox", "Some Page").unwrap();
        assert_eq!(result, Some(cat_id), "Should match case-insensitively");
    }

    #[test]
    fn test_find_category_matches_window_title() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Development", "#00ff00").unwrap();
        create_rule(&conn, cat_id, "title", "github", false).unwrap();

        let result = find_category(&conn, "firefox", "GitHub - Pull Request").unwrap();
        assert_eq!(result, Some(cat_id));
    }

    #[test]
    fn test_find_category_no_match() {
        let conn = setup_test_db();
        create_category(&conn, "Browsers", "#ff0000").unwrap();
        // No rules created

        let result = find_category(&conn, "firefox", "Some Page").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_category_no_rules_at_all() {
        let conn = setup_test_db();
        let result = find_category(&conn, "firefox", "Some Page").unwrap();
        assert_eq!(result, None);
    }

    // --- reprocess_logs tests ---

    #[test]
    fn test_reprocess_logs_clears_and_reapplies() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Browsers", "#ff0000").unwrap();

        // Insert logs without category
        insert_activity_log(&conn, "firefox", "GitHub", false, 1000, None).unwrap();
        insert_activity_log(&conn, "code", "main.rs", false, 1010, None).unwrap();

        // Now add a rule for firefox
        create_rule(&conn, cat_id, "process", "firefox", false).unwrap();

        // Reprocess
        reprocess_logs(&conn).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(
            logs[0].category_id,
            Some(cat_id),
            "firefox should be categorized"
        );
        assert_eq!(
            logs[1].category_id, None,
            "code should remain uncategorized"
        );
    }

    // --- Category CRUD tests ---

    #[test]
    fn test_category_crud() {
        let conn = setup_test_db();

        // Create
        let id = create_category(&conn, "Work", "#0000ff").unwrap();
        assert!(id > 0);

        // Read
        let cats = get_categories(&conn).unwrap();
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].name, "Work");
        assert_eq!(cats[0].color, "#0000ff");

        // Delete
        delete_category(&conn, id).unwrap();
        let cats = get_categories(&conn).unwrap();
        assert_eq!(cats.len(), 0);
    }

    #[test]
    fn test_delete_category_cascades_rules() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Work", "#0000ff").unwrap();
        create_rule(&conn, cat_id, "process", "slack", false).unwrap();

        assert_eq!(get_rules(&conn).unwrap().len(), 1);

        delete_category(&conn, cat_id).unwrap();
        // Rules should be cascade-deleted
        assert_eq!(get_rules(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_delete_category_nullifies_activity_logs() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Work", "#0000ff").unwrap();
        insert_activity_log(&conn, "slack", "General", false, 1000, Some(cat_id)).unwrap();

        delete_category(&conn, cat_id).unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(
            logs[0].category_id, None,
            "category_id should be NULL after category deletion"
        );
    }

    // --- Rule CRUD tests ---

    #[test]
    fn test_rule_crud() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Dev", "#00ff00").unwrap();

        // Create
        let rule_id = create_rule(&conn, cat_id, "process", "code", false).unwrap();
        assert!(rule_id > 0);

        // Read
        let rules = get_rules(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].category_id, cat_id);
        assert_eq!(rules[0].match_field, "process");
        assert_eq!(rules[0].pattern, "code");

        // Delete
        delete_rule(&conn, rule_id).unwrap();
        assert_eq!(get_rules(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_rule_ignore_title() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Dev", "#00ff00").unwrap();
        create_rule(&conn, cat_id, "process", "code", true).unwrap();

        let rules = get_rules(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].ignore_title, "ignore_title should be true");
    }

    // --- Cleanup tests ---

    #[test]
    fn test_cleanup_old_data_deletes_correct_rows() {
        let conn = setup_test_db();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Old log (100 days ago)
        insert_activity_log(&conn, "old-app", "Old", false, now - 100 * 86400, None).unwrap();
        // Update end_time to also be old
        conn.execute(
            "UPDATE activity_logs SET end_time = ?1 WHERE process_name = 'old-app'",
            [now - 100 * 86400],
        )
        .unwrap();

        // Recent log (1 day ago)
        insert_activity_log(&conn, "new-app", "New", false, now - 86400, None).unwrap();

        let deleted = cleanup_old_data(&conn, 30).unwrap();
        assert_eq!(deleted, 1);

        let logs = get_activity_logs(&conn, 0, now + 1000).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].process_name, "new-app");
    }

    #[test]
    fn test_cleanup_zero_retention_does_nothing() {
        let conn = setup_test_db();
        insert_activity_log(&conn, "app", "Title", false, 1000, None).unwrap();

        let deleted = cleanup_old_data(&conn, 0).unwrap();
        assert_eq!(deleted, 0);

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1);
    }

    // --- Settings tests ---

    #[test]
    fn test_settings_get_set() {
        let conn = setup_test_db();

        // Initially empty
        assert_eq!(get_setting(&conn, "retention_days").unwrap(), None);

        // Set
        set_setting(&conn, "retention_days", "30").unwrap();
        assert_eq!(
            get_setting(&conn, "retention_days").unwrap(),
            Some("30".to_string())
        );

        // Upsert
        set_setting(&conn, "retention_days", "60").unwrap();
        assert_eq!(
            get_setting(&conn, "retention_days").unwrap(),
            Some("60".to_string())
        );
    }

    #[test]
    fn test_get_all_settings() {
        let conn = setup_test_db();
        set_setting(&conn, "retention_days", "30").unwrap();
        set_setting(&conn, "chart_top_n", "8").unwrap();

        let all = get_all_settings(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }
}
