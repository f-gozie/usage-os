//! Recap cache (D52): on-device AI recaps keyed by `(day_start, facts fingerprint)`, so a frozen
//! day is a one-time narration and a rule reprocess (new fingerprint) re-narrates exactly once.

use super::*;

/// Look up a cached AI recap by its facts fingerprint. A hit means the exact same facts were
/// already narrated — return the stored `(text, generated_by)` with no model call. A miss (new
/// day, or the day's facts changed via a rule reprocess → new fingerprint) returns `None` so the
/// caller regenerates.
pub fn get_cached_recap(
    conn: &Connection,
    day_start: i64,
    fingerprint: &str,
) -> Result<Option<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT text, generated_by FROM recap_cache WHERE day_start = ?1 AND fingerprint = ?2",
    )?;
    let mut rows = stmt.query((day_start, fingerprint))?;
    if let Some(row) = rows.next()? {
        Ok(Some((row.get(0)?, row.get(1)?)))
    } else {
        Ok(None)
    }
}

/// Store a freshly-narrated AI recap under its facts fingerprint (upsert). Callers cache ONLY
/// model recaps, never the template fallback — so a cold/unavailable model retries on the next
/// open instead of caching a placeholder.
pub fn put_cached_recap(
    conn: &Connection,
    day_start: i64,
    fingerprint: &str,
    text: &str,
    generated_by: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO recap_cache (day_start, fingerprint, text, generated_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(day_start, fingerprint) DO UPDATE SET
            text = excluded.text,
            generated_by = excluded.generated_by,
            created_at = excluded.created_at",
        (day_start, fingerprint, text, generated_by, now_unix()),
    )?;
    Ok(())
}
