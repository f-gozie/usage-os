//! Project canonicalization + aliases (D30) and the site registry.
//!
//! Re-exported through `crate::db`. SQL stays in this repository layer (hard rule 4).

use super::*;
use rusqlite::{Connection, OptionalExtension, Result};

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
