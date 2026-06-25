//! Sensitive-handling exclusions (D8): CRUD + the match logic.
//!
//! Re-exported through `crate::db`. SQL stays in this repository layer (hard rule 4).

use super::*;
use rusqlite::{Connection, Result};

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
        // An empty/whitespace pattern would `.contains` everything — a single one in `exclude`
        // mode would silently drop every capture (a tracking blackout), or in `private` mode
        // blank every title/url. Skip it (the UI also rejects empty patterns).
        if ex.pattern.trim().is_empty() {
            continue;
        }
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
