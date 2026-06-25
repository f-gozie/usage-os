//! Key/value settings and the full data-ownership wipe.
//!
//! Re-exported through `crate::db`. SQL stays in this repository layer (hard rule 4).

use rusqlite::{Connection, Result};

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

// --- Data ownership: wipe ---

/// Wipe the captured record — events plus the registries derived purely from them
/// (projects + their aliases via cascade, sites) — in a single transaction. **Preserves**
/// user configuration: categories (`categories`), rules, exclusions, settings, and the
/// migration ledger. The capture writer shares this connection's `Mutex`, so it can't
/// interleave; its in-memory open-span id simply becomes a no-op `UPDATE` after the wipe
/// (the next focus change opens a fresh span). No `VACUUM` — freed pages are reclaimed
/// lazily; immediate reclaim isn't worth rewriting the whole file here.
pub fn delete_all_data(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM activity_logs", [])?;
    tx.execute("DELETE FROM project_aliases", [])?;
    tx.execute("DELETE FROM projects", [])?;
    tx.execute("DELETE FROM sites", [])?;
    tx.execute("DELETE FROM recap_cache", [])?; // captured-derived (D52)
    tx.commit()?;
    Ok(())
}
