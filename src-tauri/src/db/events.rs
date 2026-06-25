//! Activity-log writes/reads (the redesign's "events").
//!
//! Re-exported through `crate::db` — every `db::insert_event(..)` etc. resolves
//! unchanged. SQL stays in this repository layer (hard rule 4).

use super::*;
use rusqlite::{Connection, Result};
use std::borrow::Cow;
use std::io::Write;

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

/// Insert a fresh open span (`start_time` == `end_time` == `timestamp`; the capture machine
/// extends `end_time` via [`set_span_end`]). The caller blanks private fields — this does not filter.
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

/// Set a span's `end_time` by id — the only mutation the capture write path needs.
/// The consumer state machine owns the open span, so there's no "find the last row"
/// guesswork here (see `capture::consume`).
pub fn set_span_end(conn: &Connection, id: i64, end_time: i64) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs SET end_time = ?1 WHERE id = ?2",
        (end_time, id),
    )?;
    Ok(())
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
    // Half-open `[start_time, end_time)`: select every span that *overlaps* the window (not just
    // those that start in it), then clip each to the window below. A span crossing the boundary
    // (e.g. across local midnight) is thereby counted only for its in-window portion on each day,
    // instead of whole on the start day and absent from the next.
    let sql = format!(
        "SELECT {ACTIVITY_LOG_COLUMNS} FROM activity_logs
         WHERE start_time < ?2 AND end_time > ?1
         ORDER BY start_time ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([start_time, end_time], row_to_activity_log)?;

    let mut logs = Vec::new();
    for row in rows {
        let mut log = row?;
        log.start_time = log.start_time.max(start_time);
        log.end_time = log.end_time.min(end_time);
        logs.push(log);
    }
    Ok(logs)
}

/// Trivial uncategorized spans below this (active seconds, all-time) are hidden from the
/// Settings list to keep it calm — a sub-minute glance isn't worth sorting.
const UNCATEGORIZED_FLOOR_SECS: i64 = 60;

/// Apps with tracked time but no matching rule (`category_id IS NULL`), grouped by app
/// with their all-time active total and last-seen time. Excludes idle and trivial
/// (<`UNCATEGORIZED_FLOOR_SECS`) spans; ranked by total desc. Powers the Settings
/// "Uncategorized" list — sorting one writes a rule + reprocess fixes every past day.
pub fn get_uncategorized_apps(conn: &Connection) -> Result<Vec<UncategorizedApp>> {
    let mut stmt = conn.prepare(
        "SELECT process_name,
                SUM(end_time - start_time) AS total_secs,
                MAX(end_time) AS last_seen
         FROM activity_logs
         WHERE category_id IS NULL AND is_idle = 0
         GROUP BY process_name
         HAVING total_secs >= ?1
         ORDER BY total_secs DESC",
    )?;
    let rows = stmt.query_map([UNCATEGORIZED_FLOOR_SECS], |row| {
        Ok(UncategorizedApp {
            process_name: row.get(0)?,
            total_secs: row.get(1)?,
            last_seen: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// Escape SQLite LIKE metacharacters so a rule pattern matches as a literal substring — keeping
/// the bulk `reprocess_logs` in step with live `find_category` (which uses `.contains`). Without
/// it a `%`/`_` in a pattern (e.g. a window-title rule) would act as a wildcard in one path only.
fn like_escape(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Recategorize every stored event from the current rules (retroactive — D44). Runs in ONE
/// transaction, so a mid-reprocess failure can't leave history half-recategorized. Matching
/// mirrors [`find_category`] exactly: case-insensitive literal substring, first rule (by id) wins
/// (only still-`NULL` rows are touched as rules are applied in id order).
pub fn reprocess_logs(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE activity_logs SET category_id = NULL", [])?;
    for rule in get_rules(&tx)? {
        let pattern = format!("%{}%", like_escape(&rule.pattern));
        // `column` is a fixed literal (not user input) chosen by the match field — safe to inline.
        let column = if rule.match_field == "process" {
            "process_name"
        } else {
            "window_title"
        };
        let sql = format!(
            "UPDATE activity_logs SET category_id = ?1
             WHERE category_id IS NULL AND lower({column}) LIKE lower(?2) ESCAPE '\\'"
        );
        tx.execute(&sql, (rule.category_id, &pattern))?;
    }
    tx.commit()?;
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

// --- Data ownership: export ---

/// Map an `io::Error` (from writing the CSV) into a `rusqlite::Error` so the export
/// can share the repository's `Result` type; the underlying message is preserved.
fn io_to_db_err(e: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
}

/// Escape one CSV field per RFC 4180: wrap in double-quotes and double any internal
/// quote when the field contains `"`, `,`, CR or LF. Borrows when no escaping is needed.
fn csv_field(s: &str) -> Cow<'_, str> {
    if s.contains(['"', ',', '\n', '\r']) {
        Cow::Owned(format!("\"{}\"", s.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(s)
    }
}

/// An optional text column: escaped when present, an empty field when absent.
fn csv_opt(v: Option<&str>) -> Cow<'_, str> {
    v.map(csv_field).unwrap_or(Cow::Borrowed(""))
}

/// Stream every event to `w` as RFC-4180 CSV: a header row then one row per event,
/// oldest first. Reuses [`ACTIVITY_LOG_COLUMNS`] + [`row_to_activity_log`] so the export
/// shape can never drift from [`ActivityLog`]. Rows are written as they're read (flat
/// memory). Returns the number of data rows written.
pub fn export_events_csv<W: Write>(conn: &Connection, w: &mut W) -> Result<usize> {
    writeln!(
        w,
        "id,process_name,window_title,start_time,end_time,is_idle,category_id,url,site,project_id,project_abstain_reason,is_private"
    )
    .map_err(io_to_db_err)?;

    let sql = format!("SELECT {ACTIVITY_LOG_COLUMNS} FROM activity_logs ORDER BY start_time ASC");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut count = 0usize;
    while let Some(row) = rows.next()? {
        let e = row_to_activity_log(row)?;
        let num = |v: Option<i64>| v.map(|n| n.to_string()).unwrap_or_default();
        writeln!(
            w,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            e.id,
            csv_field(&e.process_name),
            csv_field(&e.window_title),
            e.start_time,
            e.end_time,
            e.is_idle as i64,
            num(e.category_id),
            csv_opt(e.url.as_deref()),
            csv_opt(e.site.as_deref()),
            num(e.project_id),
            csv_opt(e.project_abstain_reason.as_deref()),
            e.is_private as i64,
        )
        .map_err(io_to_db_err)?;
        count += 1;
    }
    Ok(count)
}
