use rusqlite::{Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub type DbConnection = Arc<Mutex<Connection>>;

/// Current Unix timestamp in seconds.
///
/// If the system clock is before the Unix epoch (clock skew), this falls back
/// to `Duration::ZERO` rather than panicking, yielding a timestamp of 0.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Activity log entry representing a time block of app usage.
///
/// Conceptually the redesign's "event" (the table rename `activity_logs`→`events`
/// is deferred to the UI rewrite to avoid IPC churn). The enrichment fields
/// (`url`/`site`/`project_id`/`project_abstain_reason`/`is_private`) are written by
/// the capture + enrichment layers (Phase 1.2+); current capture leaves them empty.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ActivityLog {
    pub id: i64,
    pub process_name: String,
    pub window_title: String,
    pub start_time: i64,
    pub end_time: i64,
    pub is_idle: bool,
    pub category_id: Option<i64>,
    /// Browser URL for this span, if any. Omitted for private/incognito (D8).
    pub url: Option<String>,
    /// Parsed host/site from `url` (e.g. "github.com").
    pub site: Option<String>,
    /// Resolved project (D30). `None` = unassigned.
    pub project_id: Option<i64>,
    /// Why no project was assigned: `None` when assigned, else "no-signal" |
    /// "ambiguous". Phase 2 correlates "ambiguous" spans (never "no-signal").
    pub project_abstain_reason: Option<String>,
    /// A "private" app (D8): time counts, but title/url are not recorded.
    pub is_private: bool,
}

/// A canonical project (D30). Keyed on the git remote `owner/repo` (or folder name
/// when a repo has no remote); folder/title/url aliases resolve to it via
/// `project_aliases`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Project {
    pub id: i64,
    pub canonical_key: String,
    pub display_name: String,
    pub remote_url: Option<String>,
    pub created_at: i64,
}

/// A browser site registry entry. `kind` distinguishes general browsing from
/// project-ambiguous tooling (D30): "general" | "dashboard" | "project-host" | "unknown".
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Site {
    pub id: i64,
    pub host: String,
    pub display_name: Option<String>,
    pub kind: String,
    pub created_at: i64,
}

/// A sensitive-handling rule (D8): match an app/site/title and either drop the
/// event (`mode = "exclude"`) or record time without title/url (`mode = "private"`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Exclusion {
    pub id: i64,
    pub match_type: String,
    pub pattern: String,
    pub mode: String,
    pub created_at: i64,
}

/// How a matched event should be handled (D8). `Exclude` wins over `Private`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ExclusionMode {
    /// Drop the event entirely — never write it.
    Exclude,
    /// Record the time span + app, but omit title/url.
    Private,
}

/// A fully-enriched event to insert (the write path the capture/enrichment layers
/// use, Phase 1.2+). The legacy `insert_activity_log`/`log_activity` path stays for
/// the current watcher and writes empty enrichment fields.
#[derive(Debug, Clone, Default)]
pub struct NewEvent<'a> {
    pub process_name: &'a str,
    pub window_title: &'a str,
    pub url: Option<&'a str>,
    pub site: Option<&'a str>,
    pub project_id: Option<i64>,
    pub project_abstain_reason: Option<&'a str>,
    pub is_private: bool,
    pub is_idle: bool,
    pub category_id: Option<i64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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

/// Initialize the SQLite database with migration-based schema management.
///
/// Returns a thread-safe database connection wrapped in Arc<Mutex>.
pub fn init_database(db_path: &PathBuf) -> Result<DbConnection> {
    let mut conn = Connection::open(db_path)?;
    // WAL lets the dial read while capture writes (R57); persistent in the file
    // header, so it's set once. foreign_keys must be enabled per-connection.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    crate::migrations::run_migrations(&mut conn)?;
    println!("[Database] Initialized database at {:?}", db_path);
    Ok(Arc::new(Mutex::new(conn)))
}

/// The columns that map to an [`ActivityLog`], in struct field order.
const ACTIVITY_LOG_COLUMNS: &str = "id, process_name, window_title, start_time, end_time, \
     is_idle, category_id, url, site, project_id, project_abstain_reason, is_private";

/// Build an [`ActivityLog`] from a row selected with [`ACTIVITY_LOG_COLUMNS`].
fn row_to_activity_log(row: &rusqlite::Row) -> Result<ActivityLog> {
    Ok(ActivityLog {
        id: row.get(0)?,
        process_name: row.get(1)?,
        window_title: row.get(2)?,
        start_time: row.get(3)?,
        end_time: row.get(4)?,
        is_idle: row.get::<_, i64>(5)? != 0,
        category_id: row.get(6)?,
        url: row.get(7)?,
        site: row.get(8)?,
        project_id: row.get(9)?,
        project_abstain_reason: row.get(10)?,
        is_private: row.get::<_, i64>(11)? != 0,
    })
}

pub fn get_last_activity_log(conn: &Connection) -> Result<Option<ActivityLog>> {
    let sql = format!("SELECT {ACTIVITY_LOG_COLUMNS} FROM activity_logs ORDER BY id DESC LIMIT 1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(row_to_activity_log(row)?))
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

/// Insert a fully-enriched event (the capture/enrichment write path, Phase 1.2+).
///
/// `start_time` and `end_time` both start at `timestamp`; the coalescing logic in
/// [`log_activity`] extends `end_time`. For private events (D8), the caller must omit
/// `window_title`/`url`/`site` and set `is_private` — this function does not filter.
pub fn insert_event(conn: &Connection, event: &NewEvent) -> Result<i64> {
    conn.execute(
        "INSERT INTO activity_logs
            (process_name, window_title, start_time, end_time, is_idle, category_id,
             url, site, project_id, project_abstain_reason, is_private)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            event.process_name,
            event.window_title,
            event.timestamp,
            event.is_idle as i64,
            event.category_id,
            event.url,
            event.site,
            event.project_id,
            event.project_abstain_reason,
            event.is_private as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_last_activity_end_time(conn: &Connection, id: i64, timestamp: i64) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs SET end_time = ?1 WHERE id = ?2",
        (timestamp, id),
    )?;
    Ok(())
}

/// How close two consecutive spans must be (seconds) to coalesce into one. Larger
/// gaps start a fresh entry, so closing the app for hours doesn't inflate a span.
const MAX_COALESCE_GAP_SECONDS: i64 = 30;

/// Log a focus span with smart coalescing — the capture write path (Phase 1.2).
///
/// If the last entry matches this one (app/title/idle/private/url) and the gap is
/// `<= MAX_COALESCE_GAP_SECONDS`, extends its `end_time`; otherwise inserts a new
/// span. The category is (re)computed from app+title here, so callers leave
/// `ev.category_id` unset. Callers own sensitive handling (D8): for a private span
/// pass `is_private = true` with `window_title`/`url` already omitted — this fn
/// does not filter. `project_id`/`project_abstain_reason`/`site` flow through from
/// the enrichment pass.
pub fn log_focus(conn: &Connection, ev: &NewEvent) -> Result<()> {
    let category_id = match find_category(conn, ev.process_name, ev.window_title) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("[Capture] category lookup failed: {}", e);
            None
        }
    };

    if let Some(last) = get_last_activity_log(conn)? {
        let gap = ev.timestamp - last.end_time;
        let is_same = last.process_name == ev.process_name
            && last.window_title == ev.window_title
            && last.is_idle == ev.is_idle
            && last.is_private == ev.is_private
            && last.url.as_deref() == ev.url;
        if is_same && gap <= MAX_COALESCE_GAP_SECONDS {
            update_last_activity_end_time(conn, last.id, ev.timestamp)?;
            return Ok(());
        }
    }

    insert_event(
        conn,
        &NewEvent {
            category_id,
            ..ev.clone()
        },
    )?;
    Ok(())
}

/// Log activity with smart coalescing — the legacy app+title write path (kept so
/// existing callers/tests are unchanged). Delegates to [`log_focus`] with no url
/// and `is_private = false`.
pub fn log_activity(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
    is_idle: bool,
    timestamp: i64,
) -> Result<()> {
    log_focus(
        conn,
        &NewEvent {
            process_name,
            window_title,
            is_idle,
            timestamp,
            ..Default::default()
        },
    )
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
    let sql = format!(
        "SELECT {ACTIVITY_LOG_COLUMNS} FROM activity_logs
         WHERE start_time >= ?1 AND start_time <= ?2
         ORDER BY start_time ASC"
    );
    let mut stmt = conn.prepare(&sql)?;

    let logs = stmt.query_map([start_time, end_time], row_to_activity_log)?;

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

/// Context identity for the rollup: `(id, slug, name)`. `slug` (e.g. "deep") maps to a
/// colour token in the UI; `None` for a user-created context. Kept separate from
/// [`get_categories`] so the slug stays out of the legacy IPC `Category` shape.
pub fn get_context_metas(conn: &Connection) -> Result<Vec<(i64, Option<String>, String)>> {
    let mut stmt = conn.prepare("SELECT id, slug, name FROM categories")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
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

// --- Projects (D30) ---

/// Look up a project id by one of its aliases (folder / title / github-url).
pub fn find_project_by_alias(
    conn: &Connection,
    alias_kind: &str,
    alias_value: &str,
) -> Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT project_id FROM project_aliases WHERE alias_kind = ?1 AND alias_value = ?2",
    )?;
    let mut rows = stmt.query((alias_kind, alias_value))?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

/// Attach an alias to a project. Idempotent: a `(kind, value)` already present (for
/// this or any project) is left untouched, so canonicalization never fragments.
pub fn add_project_alias(
    conn: &Connection,
    project_id: i64,
    alias_kind: &str,
    alias_value: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO project_aliases (project_id, alias_kind, alias_value)
         VALUES (?1, ?2, ?3)",
        (project_id, alias_kind, alias_value),
    )?;
    Ok(())
}

/// Resolve a project by its canonical key (git remote `owner/repo`, or folder-name
/// fallback), creating it if absent, and attach any `aliases` either way. This is the
/// single entry point the inference layer (Phase 1.2) calls so the same project never
/// fragments into several (D30). Returns the project id.
pub fn resolve_or_create_project(
    conn: &Connection,
    canonical_key: &str,
    display_name: &str,
    remote_url: Option<&str>,
    aliases: &[(&str, &str)],
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM projects WHERE canonical_key = ?1",
            [canonical_key],
            |row| row.get(0),
        )
        .optional()?;

    let project_id = match existing {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO projects (canonical_key, display_name, remote_url, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                (canonical_key, display_name, remote_url, now_unix()),
            )?;
            conn.last_insert_rowid()
        }
    };

    for (kind, value) in aliases {
        add_project_alias(conn, project_id, kind, value)?;
    }

    Ok(project_id)
}

pub fn get_project(conn: &Connection, id: i64) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, canonical_key, display_name, remote_url, created_at FROM projects WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Project {
            id: row.get(0)?,
            canonical_key: row.get(1)?,
            display_name: row.get(2)?,
            remote_url: row.get(3)?,
            created_at: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, canonical_key, display_name, remote_url, created_at
         FROM projects ORDER BY display_name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            canonical_key: row.get(1)?,
            display_name: row.get(2)?,
            remote_url: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

/// Delete a project. Activity logs that referenced it become `unassigned`
/// (`project_id = NULL`); aliases cascade. Mirrors [`delete_category`].
pub fn delete_project(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs SET project_id = NULL WHERE project_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    Ok(())
}

// --- Sites ---

/// Insert a site by host, or update its metadata if the host already exists.
/// Returns the site id.
pub fn resolve_or_create_site(
    conn: &Connection,
    host: &str,
    display_name: Option<&str>,
    kind: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO sites (host, display_name, kind, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(host) DO UPDATE SET
            display_name = COALESCE(excluded.display_name, sites.display_name),
            kind = excluded.kind",
        (host, display_name, kind, now_unix()),
    )?;
    let id: i64 = conn.query_row("SELECT id FROM sites WHERE host = ?1", [host], |row| {
        row.get(0)
    })?;
    Ok(id)
}

pub fn set_site_kind(conn: &Connection, host: &str, kind: &str) -> Result<()> {
    conn.execute("UPDATE sites SET kind = ?1 WHERE host = ?2", (kind, host))?;
    Ok(())
}

pub fn get_sites(conn: &Connection) -> Result<Vec<Site>> {
    let mut stmt = conn
        .prepare("SELECT id, host, display_name, kind, created_at FROM sites ORDER BY host ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Site {
            id: row.get(0)?,
            host: row.get(1)?,
            display_name: row.get(2)?,
            kind: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

// --- Exclusions (D8) ---

pub fn get_exclusions(conn: &Connection) -> Result<Vec<Exclusion>> {
    let mut stmt = conn.prepare(
        "SELECT id, match_type, pattern, mode, created_at FROM exclusions ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Exclusion {
            id: row.get(0)?,
            match_type: row.get(1)?,
            pattern: row.get(2)?,
            mode: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn create_exclusion(
    conn: &Connection,
    match_type: &str,
    pattern: &str,
    mode: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO exclusions (match_type, pattern, mode, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        (match_type, pattern, mode, now_unix()),
    )?;
    // INSERT OR IGNORE yields rowid 0 on a duplicate; resolve the real id by key.
    let id: i64 = conn.query_row(
        "SELECT id FROM exclusions WHERE match_type = ?1 AND pattern = ?2 AND mode = ?3",
        (match_type, pattern, mode),
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn delete_exclusion(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM exclusions WHERE id = ?1", [id])?;
    Ok(())
}

/// Decide how a captured event should be handled (D8). Checks every exclusion rule
/// against the app name, window title, and (optional) site, returning the strongest
/// match: `Exclude` (drop) wins over `Private` (time only). Matching is
/// case-insensitive substring, consistent with [`find_category`].
pub fn match_exclusion(
    conn: &Connection,
    app: &str,
    title: &str,
    site: Option<&str>,
) -> Result<Option<ExclusionMode>> {
    let app = app.to_lowercase();
    let title = title.to_lowercase();
    let site = site.map(|s| s.to_lowercase());

    let mut result: Option<ExclusionMode> = None;
    for ex in get_exclusions(conn)? {
        let target = match ex.match_type.as_str() {
            "app" => Some(app.as_str()),
            "title" => Some(title.as_str()),
            "site" => site.as_deref(),
            _ => None,
        };
        let Some(target) = target else { continue };
        if !target.contains(&ex.pattern.to_lowercase()) {
            continue;
        }
        let mode = match ex.mode.as_str() {
            "exclude" => ExclusionMode::Exclude,
            "private" => ExclusionMode::Private,
            _ => continue,
        };
        // Exclude is strictly stronger than Private; short-circuit on it.
        if mode == ExclusionMode::Exclude {
            return Ok(Some(ExclusionMode::Exclude));
        }
        result = Some(ExclusionMode::Private);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create an in-memory database using the migration system. The starter rules
    /// (migration 3) are cleared so rules-engine tests control their own rule set;
    /// the seed itself is verified in `crate::migrations`.
    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("Failed to open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::migrations::run_migrations(&mut conn).expect("Migrations should succeed");
        conn.execute("DELETE FROM rules", []).unwrap();
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
        // Redesign data-model tables (D30/D8).
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"project_aliases".to_string()));
        assert!(tables.contains(&"sites".to_string()));
        assert!(tables.contains(&"exclusions".to_string()));
    }

    // (Migration-runner behaviour — versioning, idempotency, checksums, drift — is
    // tested in `crate::migrations`.)

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
        // The 4 canonical contexts are seeded by migration 2, so assert against that
        // baseline rather than an empty table.
        let baseline = get_categories(&conn).unwrap().len();

        // Create
        let id = create_category(&conn, "Work", "#0000ff").unwrap();
        assert!(id > 0);

        // Read
        let cats = get_categories(&conn).unwrap();
        assert_eq!(cats.len(), baseline + 1);
        let work = cats
            .iter()
            .find(|c| c.name == "Work")
            .expect("Work category");
        assert_eq!(work.color, "#0000ff");

        // Delete
        delete_category(&conn, id).unwrap();
        assert_eq!(get_categories(&conn).unwrap().len(), baseline);
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

    // --- Project canonicalization tests (D30) ---

    #[test]
    fn test_resolve_or_create_project_dedups_by_canonical_key() {
        let conn = setup_test_db();
        let first = resolve_or_create_project(
            &conn,
            "f-gozie/usage-os",
            "usage-os",
            Some("git@github.com:f-gozie/usage-os.git"),
            &[],
        )
        .unwrap();
        let second =
            resolve_or_create_project(&conn, "f-gozie/usage-os", "usage-os", None, &[]).unwrap();
        assert_eq!(
            first, second,
            "same canonical_key must resolve to one project"
        );
        assert_eq!(get_projects(&conn).unwrap().len(), 1);
    }

    #[test]
    fn test_project_does_not_fragment_across_signals() {
        // The headline D30 finding: the same project arrives as both the git remote
        // and the folder/title name. Resolving via the canonical key then looking up
        // by a folder alias must land on the SAME project — never two.
        let conn = setup_test_db();
        let id = resolve_or_create_project(
            &conn,
            "f-gozie/usage-os",
            "usage-os",
            None,
            &[("folder", "usage_os"), ("title", "usage_os")],
        )
        .unwrap();

        assert_eq!(
            find_project_by_alias(&conn, "folder", "usage_os").unwrap(),
            Some(id)
        );
        assert_eq!(
            find_project_by_alias(&conn, "title", "usage_os").unwrap(),
            Some(id)
        );
        assert_eq!(
            get_projects(&conn).unwrap().len(),
            1,
            "one project, not fragmented"
        );
    }

    #[test]
    fn test_add_project_alias_is_idempotent() {
        let conn = setup_test_db();
        let id = resolve_or_create_project(&conn, "owner/repo", "repo", None, &[]).unwrap();
        add_project_alias(&conn, id, "folder", "repo").unwrap();
        add_project_alias(&conn, id, "folder", "repo").unwrap(); // no-op, no error
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_aliases", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_find_project_by_alias_none() {
        let conn = setup_test_db();
        assert_eq!(
            find_project_by_alias(&conn, "folder", "nope").unwrap(),
            None
        );
    }

    #[test]
    fn test_delete_project_nullifies_events_and_cascades_aliases() {
        let conn = setup_test_db();
        let id =
            resolve_or_create_project(&conn, "owner/repo", "repo", None, &[("folder", "repo")])
                .unwrap();
        insert_event(
            &conn,
            &NewEvent {
                process_name: "code",
                window_title: "main.rs",
                project_id: Some(id),
                timestamp: 1000,
                ..Default::default()
            },
        )
        .unwrap();

        delete_project(&conn, id).unwrap();

        // Aliases cascade-deleted.
        let alias_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_aliases", [], |r| r.get(0))
            .unwrap();
        assert_eq!(alias_count, 0);
        // Event survives but is now unassigned.
        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].project_id, None);
    }

    #[test]
    fn test_get_project_by_id() {
        let conn = setup_test_db();
        let id =
            resolve_or_create_project(&conn, "owner/repo", "repo", Some("https://x"), &[]).unwrap();
        let p = get_project(&conn, id).unwrap().unwrap();
        assert_eq!(p.canonical_key, "owner/repo");
        assert_eq!(p.display_name, "repo");
        assert_eq!(p.remote_url, Some("https://x".to_string()));
        assert!(get_project(&conn, 9999).unwrap().is_none());
    }

    // --- Site registry tests ---

    #[test]
    fn test_resolve_or_create_site_is_idempotent_on_host() {
        let conn = setup_test_db();
        let a =
            resolve_or_create_site(&conn, "github.com", Some("GitHub"), "project-host").unwrap();
        let b = resolve_or_create_site(&conn, "github.com", None, "project-host").unwrap();
        assert_eq!(a, b);
        assert_eq!(get_sites(&conn).unwrap().len(), 1);
        // display_name preserved when later passed as None (COALESCE).
        assert_eq!(
            get_sites(&conn).unwrap()[0].display_name,
            Some("GitHub".to_string())
        );
    }

    #[test]
    fn test_set_site_kind() {
        let conn = setup_test_db();
        resolve_or_create_site(&conn, "grafana.net", None, "unknown").unwrap();
        set_site_kind(&conn, "grafana.net", "dashboard").unwrap();
        assert_eq!(get_sites(&conn).unwrap()[0].kind, "dashboard");
    }

    // --- Exclusion tests (D8) ---

    #[test]
    fn test_exclusion_crud_and_unique() {
        let conn = setup_test_db();
        let id = create_exclusion(&conn, "app", "1Password", "exclude").unwrap();
        assert!(id > 0);
        // Duplicate (match_type, pattern, mode) resolves to the same row, no error.
        let dup = create_exclusion(&conn, "app", "1Password", "exclude").unwrap();
        assert_eq!(id, dup);
        assert_eq!(get_exclusions(&conn).unwrap().len(), 1);

        delete_exclusion(&conn, id).unwrap();
        assert_eq!(get_exclusions(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_match_exclusion_by_app_title_site() {
        let conn = setup_test_db();
        create_exclusion(&conn, "app", "Banking", "exclude").unwrap();
        create_exclusion(&conn, "title", "password", "private").unwrap();
        create_exclusion(&conn, "site", "mybank.com", "exclude").unwrap();

        // case-insensitive substring on app
        assert_eq!(
            match_exclusion(&conn, "My Banking App", "Home", None).unwrap(),
            Some(ExclusionMode::Exclude)
        );
        // title match → private
        assert_eq!(
            match_exclusion(&conn, "Chrome", "Reset Password", None).unwrap(),
            Some(ExclusionMode::Private)
        );
        // site match
        assert_eq!(
            match_exclusion(&conn, "Chrome", "Home", Some("app.mybank.com")).unwrap(),
            Some(ExclusionMode::Exclude)
        );
        // no match
        assert_eq!(
            match_exclusion(&conn, "Slack", "general", Some("slack.com")).unwrap(),
            None
        );
    }

    #[test]
    fn test_match_exclusion_exclude_beats_private() {
        let conn = setup_test_db();
        // Same app matched by both a private and an exclude rule — exclude must win.
        create_exclusion(&conn, "app", "Notes", "private").unwrap();
        create_exclusion(&conn, "title", "secret", "exclude").unwrap();
        assert_eq!(
            match_exclusion(&conn, "Notes", "My secret journal", None).unwrap(),
            Some(ExclusionMode::Exclude)
        );
    }

    // --- Enriched event write/read tests ---

    #[test]
    fn test_insert_event_round_trips_enrichment_fields() {
        let conn = setup_test_db();
        let project_id = resolve_or_create_project(&conn, "owner/repo", "repo", None, &[]).unwrap();
        insert_event(
            &conn,
            &NewEvent {
                process_name: "Chrome",
                window_title: "owner/repo: PR #1",
                url: Some("https://github.com/owner/repo/pull/1"),
                site: Some("github.com"),
                project_id: Some(project_id),
                project_abstain_reason: None,
                is_private: false,
                is_idle: false,
                category_id: None,
                timestamp: 1000,
            },
        )
        .unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1);
        let e = &logs[0];
        assert_eq!(
            e.url.as_deref(),
            Some("https://github.com/owner/repo/pull/1")
        );
        assert_eq!(e.site.as_deref(), Some("github.com"));
        assert_eq!(e.project_id, Some(project_id));
        assert_eq!(e.project_abstain_reason, None);
        assert!(!e.is_private);
        assert_eq!(e.start_time, 1000);
        assert_eq!(e.end_time, 1000);
    }

    #[test]
    fn test_insert_event_persists_abstain_reason_and_private() {
        let conn = setup_test_db();
        // An ambiguous (dashboard) span: no project, reason persisted for Phase-2 correlation.
        insert_event(
            &conn,
            &NewEvent {
                process_name: "Chrome",
                window_title: "Grafana",
                site: Some("grafana.net"),
                project_abstain_reason: Some("ambiguous"),
                timestamp: 1000,
                ..Default::default()
            },
        )
        .unwrap();
        // A private span: time recorded, title omitted by the caller, flag set.
        insert_event(
            &conn,
            &NewEvent {
                process_name: "1Password",
                window_title: "",
                is_private: true,
                timestamp: 1010,
                ..Default::default()
            },
        )
        .unwrap();

        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].project_id, None);
        assert_eq!(logs[0].project_abstain_reason.as_deref(), Some("ambiguous"));
        assert!(logs[1].is_private);
        assert_eq!(logs[1].window_title, "");
    }

    #[test]
    fn test_legacy_insert_leaves_enrichment_fields_empty() {
        // The current watcher path must keep working and write empty enrichment.
        let conn = setup_test_db();
        insert_activity_log(&conn, "firefox", "GitHub", false, 1000, None).unwrap();
        let logs = get_activity_logs(&conn, 0, 2000).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].url, None);
        assert_eq!(logs[0].site, None);
        assert_eq!(logs[0].project_id, None);
        assert_eq!(logs[0].project_abstain_reason, None);
        assert!(!logs[0].is_private);
    }
}
