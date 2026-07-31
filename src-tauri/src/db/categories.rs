//! Categories + rules CRUD and the categorization match logic.
//!
//! Re-exported through `crate::db`. SQL stays in this repository layer (hard rule 4).

use super::*;
use rusqlite::{Connection, Result};

// --- Category CRUD (legacy `categories` table; see D46) ---

pub fn get_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt =
        conn.prepare("SELECT id, slug, name, color FROM categories ORDER BY name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            slug: row.get(1)?,
            name: row.get(2)?,
            color: row.get(3)?,
        })
    })?;
    rows.collect()
}

/// One category's rollup identity: `(id, slug, name, color)`.
pub type CategoryMetaRow = (i64, Option<String>, String, String);

/// Category identity for the rollup. `slug` (e.g. "deep") maps to a colour token in the UI;
/// `None` for a user-created category, which renders its hex `color` instead. Kept separate
/// from [`get_categories`] so the slug stays out of the legacy IPC shape.
pub fn get_category_metas(conn: &Connection) -> Result<Vec<CategoryMetaRow>> {
    let mut stmt = conn.prepare("SELECT id, slug, name, color FROM categories")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
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

/// Rename / recolour a category (name + colour only — `slug` is immutable canonical
/// identity, never edited). The Settings editor uses this for both canonical and
/// user-created categories.
pub fn update_category(conn: &Connection, id: i64, name: &str, color: &str) -> Result<()> {
    conn.execute(
        "UPDATE categories SET name = ?1, color = ?2 WHERE id = ?3",
        (name, color, id),
    )?;
    Ok(())
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

/// Precedence tier for a rule's match field: the narrower the signal, the earlier it is
/// consulted (D70). A `site` rule names one host, a `title` rule one window, a `process`
/// rule a whole app — so `youtube.com → Entertainment` beats `Chrome → Browsing` no matter
/// which was written first. Within a tier, the lowest rule id still wins.
pub(crate) fn match_field_tier(match_field: &str) -> u8 {
    match match_field {
        "site" => 0,
        "title" => 1,
        _ => 2, // "process"
    }
}

/// Rules in evaluation order: by precedence tier, then by id.
fn rules_by_precedence(conn: &Connection) -> Result<Vec<Rule>> {
    let mut rules = get_rules(conn)?;
    rules.sort_by_key(|r| (match_field_tier(&r.match_field), r.id));
    Ok(rules)
}

/// Does `host` fall under the site `pattern`? True for the host itself and for its
/// subdomains — `youtube.com` matches `www.youtube.com`. Deliberately NOT a substring
/// test: `"netflix.com".contains("x.com")` is true, which would let one `x.com` rule
/// swallow unrelated hosts. Both sides are lowercased by the caller.
pub(crate) fn host_matches(host: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches('.');
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

/// The category for one event, or `None` if no rule claims it. `site` is the parsed host
/// (`None` for non-browser events).
pub fn find_category(
    conn: &Connection,
    process_name: &str,
    window_title: &str,
    site: Option<&str>,
) -> Result<Option<i64>> {
    for rule in rules_by_precedence(conn)? {
        // An empty/whitespace pattern would `.contains` everything — skip it so a stray empty
        // rule can't swallow every event (defense-in-depth; the UI also rejects empty patterns).
        if rule.pattern.trim().is_empty() {
            continue;
        }
        let pattern = rule.pattern.to_lowercase();

        let matched = match rule.match_field.as_str() {
            "site" => site.is_some_and(|host| host_matches(&host.to_lowercase(), &pattern)),
            "title" => window_title.to_lowercase().contains(&pattern),
            _ => process_name.to_lowercase().contains(&pattern),
        };

        if matched {
            return Ok(Some(rule.category_id));
        }
    }
    Ok(None)
}
