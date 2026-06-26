//! The typed SQLite repository layer.
//!
//! This module is split into domain submodules (`events`, `projects`, `categories`,
//! `exclusions`, `settings`) that are flattened back out via `pub use` below — so every
//! existing `crate::db::X` path (functions AND types) resolves unchanged. All SQL lives
//! here in the repository layer (hard rule 4); the rest of the app calls typed functions.

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;

mod categories;
mod events;
mod exclusions;
mod projects;
mod recap;
mod settings;

pub use categories::*;
pub use events::*;
pub use exclusions::*;
pub use projects::*;
pub use recap::*;
pub use settings::*;

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

/// An app with tracked time that matches no rule — its `category_id` is NULL, so it
/// rolls up as "Uncategorized" (Other). Surfaced in Settings so the user can sort it
/// into a category. `total_secs`/`last_seen` are all-time (retention-bounded): sorting
/// it once re-sorts every past day it appears on (read-time segmentation, D40).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct UncategorizedApp {
    pub process_name: String,
    pub total_secs: i64,
    pub last_seen: i64,
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

/// A fully-enriched event to insert (the write path the capture state machine uses).
/// `insert_activity_log` is a simpler bare-app+title insert kept for tests/utilities.
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

/// A category (the redesign's noun for the legacy `categories` table — the SQL table
/// and column names stay `categories`/`category_id` per D31; only the IPC surface is
/// renamed). `slug` carries the canonical identity (`deep`|`research`|`comms`|`breaks`)
/// the UI maps to a colour token `--c-<slug>`; `None` = a user-created category (it
/// supplies its own `color`). Exposing `slug` lets the editor protect the canonical
/// four from deletion and colour their swatches from the theme-aware token.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Category {
    pub id: i64,
    pub slug: Option<String>,
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
pub(crate) const ACTIVITY_LOG_COLUMNS: &str =
    "id, process_name, window_title, start_time, end_time, \
     is_idle, category_id, url, site, project_id, project_abstain_reason, is_private";

/// Build an [`ActivityLog`] from a row selected with [`ACTIVITY_LOG_COLUMNS`].
pub(crate) fn row_to_activity_log(row: &rusqlite::Row) -> Result<ActivityLog> {
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

    /// Insert a span with an explicit duration (the bare `insert_activity_log` helper
    /// makes zero-length spans, which the uncategorized SUM would never surface).
    fn span(conn: &Connection, app: &str, start: i64, secs: i64, cat: Option<i64>, idle: bool) {
        conn.execute(
            "INSERT INTO activity_logs (process_name, window_title, start_time, end_time, is_idle, category_id)
             VALUES (?1, '', ?2, ?3, ?4, ?5)",
            rusqlite::params![app, start, start + secs, idle as i64, cat],
        )
        .unwrap();
    }

    #[test]
    fn uncategorized_apps_groups_ranks_and_floors() {
        let conn = setup_test_db();
        span(&conn, "Obsidian", 1000, 120, None, false);
        span(&conn, "Obsidian", 2000, 180, None, false); // → 300 total, last_seen 2180
        span(&conn, "TablePlus", 3000, 90, None, false);
        span(&conn, "Tiny", 4000, 30, None, false); // below the 60s floor → hidden
        span(&conn, "Cursor", 5000, 600, Some(1), false); // categorized → excluded
        span(&conn, "idlewatch", 6000, 600, None, true); // idle → excluded

        let apps = get_uncategorized_apps(&conn).unwrap();
        let names: Vec<&str> = apps.iter().map(|a| a.process_name.as_str()).collect();
        assert_eq!(names, vec!["Obsidian", "TablePlus"]); // ranked by total desc
        assert_eq!(apps[0].total_secs, 300);
        assert_eq!(apps[0].last_seen, 2180);
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

    #[test]
    fn get_activity_logs_clips_spans_crossing_the_window_boundary() {
        let conn = setup_test_db();
        // A 1200s span [99_400, 100_600] that crosses a day boundary at 100_000.
        span(&conn, "Cursor", 99_400, 1200, Some(1), false);
        // Day 1 = [90_000, 100_000): sees only the pre-boundary 600s, clipped at the end.
        let day1 = get_activity_logs(&conn, 90_000, 100_000).unwrap();
        assert_eq!(day1.len(), 1);
        assert_eq!(day1[0].start_time, 99_400);
        assert_eq!(day1[0].end_time, 100_000, "clipped at the window end");
        // Day 2 = [100_000, 110_000): sees the post-boundary 600s, clipped at the start —
        // so the span is counted once per day, not whole on day 1 and absent from day 2.
        let day2 = get_activity_logs(&conn, 100_000, 110_000).unwrap();
        assert_eq!(day2.len(), 1);
        assert_eq!(day2[0].start_time, 100_000, "clipped at the window start");
        assert_eq!(day2[0].end_time, 100_600);
    }

    #[test]
    fn get_activity_logs_lower_bounds_the_scan_at_max_span_lookback() {
        // The Phase-6 perf guard: the overlap scan is floored at `start - MAX_SPAN_LOOKBACK_SECS`
        // (2 days) so `idx_start_time` does a bounded range scan instead of walking all history.
        // A span overlapping the window but starting before that floor is dropped — no real span
        // lives that long (the idle gate closes spans within minutes), so this only excludes the
        // impossible while keeping the read O(window).
        let conn = setup_test_db();
        let win_start = 10_000_000;
        let win_end = win_start + 86_400;
        // (a) Starts 1 day before the window, overlaps into it → INCLUDED, clipped at the start.
        span(
            &conn,
            "Cursor",
            win_start - 86_400,
            86_400 + 100,
            Some(1),
            false,
        );
        // (b) Starts 3 days before (beyond the 2-day floor) but still overlaps → DROPPED by the guard.
        span(
            &conn,
            "Ghost",
            win_start - 3 * 86_400,
            3 * 86_400 + 50,
            Some(1),
            false,
        );

        let logs = get_activity_logs(&conn, win_start, win_end).unwrap();
        let apps: Vec<&str> = logs.iter().map(|l| l.process_name.as_str()).collect();
        assert_eq!(
            apps,
            vec!["Cursor"],
            "within-lookback kept; beyond-lookback dropped"
        );
        assert_eq!(logs[0].start_time, win_start, "clipped at the window start");
        assert_eq!(logs[0].end_time, win_start + 100);
    }

    // (Span coalescing / close-on-switch / idle behaviour now lives in the capture
    // state machine — see `crate::capture` tests.)

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
    fn empty_pattern_rule_does_not_match_everything() {
        // A stray empty-pattern rule must NOT swallow every event (`.contains("")` is always
        // true). Both find_category and reprocess_logs must skip it.
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Junk", "#000000").unwrap();
        create_rule(&conn, cat_id, "process", "   ", false).unwrap(); // whitespace pattern
        assert_eq!(find_category(&conn, "firefox", "anything").unwrap(), None);
        span(&conn, "firefox", 1000, 60, None, false);
        reprocess_logs(&conn).unwrap();
        let logs = get_activity_logs(&conn, 0, 100_000).unwrap();
        assert_eq!(
            logs[0].category_id, None,
            "empty pattern matched nothing on reprocess"
        );
    }

    #[test]
    fn reprocess_treats_like_wildcards_as_literal_text() {
        // A pattern containing `%`/`_` must match literally (like find_category's `.contains`),
        // not as a SQL LIKE wildcard. "50%" should match "50% off" but not "5000 off".
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Deals", "#ff0000").unwrap();
        create_rule(&conn, cat_id, "title", "50%", false).unwrap();
        span(&conn, "browser", 1000, 60, None, false); // title '' — set below
        conn.execute(
            "UPDATE activity_logs SET window_title = '50% off' WHERE start_time = 1000",
            [],
        )
        .unwrap();
        span(&conn, "browser", 2000, 60, None, false);
        conn.execute(
            "UPDATE activity_logs SET window_title = '5000 off' WHERE start_time = 2000",
            [],
        )
        .unwrap();
        reprocess_logs(&conn).unwrap();
        let logs = get_activity_logs(&conn, 0, 100_000).unwrap();
        let by_start: std::collections::HashMap<i64, Option<i64>> =
            logs.iter().map(|l| (l.start_time, l.category_id)).collect();
        assert_eq!(
            by_start[&1000],
            Some(cat_id),
            "literal '50%' matches '50% off'"
        );
        assert_eq!(
            by_start[&2000], None,
            "'%' is not a wildcard — '5000 off' must not match"
        );
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
        // The 4 canonical categories are seeded by migration 2, so assert against that
        // baseline rather than an empty table.
        let baseline = get_categories(&conn).unwrap().len();

        // Create
        let id = create_category(&conn, "Errands", "#0000ff").unwrap();
        assert!(id > 0);

        // Read
        let cats = get_categories(&conn).unwrap();
        assert_eq!(cats.len(), baseline + 1);
        let errands = cats
            .iter()
            .find(|c| c.name == "Errands")
            .expect("Errands category");
        assert_eq!(errands.color, "#0000ff");

        // Delete
        delete_category(&conn, id).unwrap();
        assert_eq!(get_categories(&conn).unwrap().len(), baseline);
    }

    #[test]
    fn test_delete_category_cascades_rules() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Errands", "#0000ff").unwrap();
        create_rule(&conn, cat_id, "process", "slack", false).unwrap();

        assert_eq!(get_rules(&conn).unwrap().len(), 1);

        delete_category(&conn, cat_id).unwrap();
        // Rules should be cascade-deleted
        assert_eq!(get_rules(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_delete_category_nullifies_activity_logs() {
        let conn = setup_test_db();
        let cat_id = create_category(&conn, "Errands", "#0000ff").unwrap();
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

    // --- Category update test ---

    #[test]
    fn test_update_category_persists_name_and_color() {
        let conn = setup_test_db();
        let id = create_category(&conn, "Old", "#000000").unwrap();
        update_category(&conn, id, "New", "#ffffff").unwrap();
        let c = get_categories(&conn)
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
            .expect("category");
        assert_eq!(c.name, "New");
        assert_eq!(c.color, "#ffffff");
    }

    #[test]
    fn test_get_categories_exposes_canonical_slug() {
        let conn = setup_test_db();
        // The 4 canonical seeds (migration 2) carry slugs; a user category does not.
        assert!(get_categories(&conn)
            .unwrap()
            .iter()
            .any(|c| c.slug.as_deref() == Some("deep")));
        let id = create_category(&conn, "Mine", "#123456").unwrap();
        assert!(get_categories(&conn)
            .unwrap()
            .iter()
            .any(|c| c.id == id && c.slug.is_none()));
    }

    // --- Data ownership: CSV export + delete-all ---

    #[test]
    fn test_export_events_csv_header_and_rows() {
        let conn = setup_test_db();
        insert_activity_log(&conn, "firefox", "GitHub", false, 1000, None).unwrap();
        insert_activity_log(&conn, "code", "main.rs", false, 1010, None).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let n = export_events_csv(&conn, &mut buf).unwrap();
        assert_eq!(n, 2);
        let csv = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "header + 2 rows");
        assert!(lines[0].starts_with("id,process_name,window_title,"));
        assert_eq!(lines[0].split(',').count(), 12, "12 columns");
        assert!(lines[1].contains("firefox"));
        assert!(lines[2].contains("code"));
    }

    #[test]
    fn test_export_events_csv_escapes_special_chars() {
        let conn = setup_test_db();
        // A title with a comma, embedded quotes, and a newline — the load-bearing case.
        insert_activity_log(&conn, "chrome", "He said \"hi\", bye\nx", false, 1000, None).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        export_events_csv(&conn, &mut buf).unwrap();
        let csv = String::from_utf8(buf).unwrap();
        assert!(
            csv.contains("\"He said \"\"hi\"\", bye\nx\""),
            "field must be quoted with doubled internal quotes; got: {csv}"
        );
    }

    #[test]
    fn test_export_events_csv_empty_db_and_empty_optionals() {
        let conn = setup_test_db();
        // Empty DB → header only.
        let mut buf: Vec<u8> = Vec::new();
        let n = export_events_csv(&conn, &mut buf).unwrap();
        assert_eq!(n, 0);
        assert_eq!(String::from_utf8(buf).unwrap().lines().count(), 1);

        // A bare event leaves url/site/project/abstain/category empty (not "None").
        insert_activity_log(&conn, "code", "main.rs", false, 1000, None).unwrap();
        let mut buf2: Vec<u8> = Vec::new();
        export_events_csv(&conn, &mut buf2).unwrap();
        let body = String::from_utf8(buf2).unwrap();
        let row = body.lines().nth(1).expect("data row");
        let cols: Vec<&str> = row.split(',').collect();
        assert_eq!(cols.len(), 12);
        assert_eq!(cols[6], "", "category_id empty");
        assert_eq!(cols[7], "", "url empty");
        assert_eq!(cols[8], "", "site empty");
        assert_eq!(cols[9], "", "project_id empty");
        assert_eq!(cols[10], "", "abstain empty");
        assert_eq!(cols[11], "0", "is_private");
        assert!(!row.contains("None"));
    }

    #[test]
    fn test_delete_all_data_wipes_record_preserves_config() {
        let mut conn = setup_test_db();
        // Captured record: event + project(+alias) + site.
        let pid =
            resolve_or_create_project(&conn, "owner/repo", "repo", None, &[("folder", "repo")])
                .unwrap();
        insert_event(
            &conn,
            &NewEvent {
                process_name: "code",
                window_title: "x",
                project_id: Some(pid),
                timestamp: 1000,
                ..Default::default()
            },
        )
        .unwrap();
        resolve_or_create_site(&conn, "github.com", None, "project-host").unwrap();
        // Config that must survive.
        let ctx = create_category(&conn, "Mine", "#123456").unwrap();
        create_rule(&conn, ctx, "process", "zed", false).unwrap();
        create_exclusion(&conn, "app", "1Password", "exclude").unwrap();
        set_setting(&conn, "theme", "warm").unwrap();

        delete_all_data(&mut conn).unwrap();

        // Record gone (incl. cascaded aliases).
        assert!(get_activity_logs(&conn, 0, 9999).unwrap().is_empty());
        assert!(get_projects(&conn).unwrap().is_empty());
        let aliases: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_aliases", [], |r| r.get(0))
            .unwrap();
        assert_eq!(aliases, 0, "aliases cascade with projects");
        assert!(get_sites(&conn).unwrap().is_empty());
        // Config preserved.
        assert!(get_categories(&conn)
            .unwrap()
            .iter()
            .any(|c| c.name == "Mine"));
        assert_eq!(get_rules(&conn).unwrap().len(), 1);
        assert_eq!(get_exclusions(&conn).unwrap().len(), 1);
        assert_eq!(
            get_setting(&conn, "theme").unwrap(),
            Some("warm".to_string())
        );
    }
}
