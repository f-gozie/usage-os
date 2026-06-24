//! Forward-only SQL migration runner.
//!
//! Each migration is a numbered `.sql` file under `src-tauri/migrations/`, embedded at
//! compile time and applied exactly once inside a transaction. A checksum of each
//! applied migration is stored and re-verified on every boot: shipped migrations are
//! immutable, so editing one after release is a startup error, not silent drift. There
//! are no down-migrations by design — a local-first desktop DB only rolls forward.
//!
//! See `src-tauri/migrations/README.md` for the contributor rules.

use rusqlite::{Connection, Result};

use crate::db::now_unix;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// The ordered migration chain. Append new entries; never edit or reorder shipped ones.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: include_str!("../migrations/0001_initial_schema.sql"),
    },
    Migration {
        version: 2,
        name: "seed_canonical_contexts",
        sql: include_str!("../migrations/0002_seed_canonical_contexts.sql"),
    },
    Migration {
        version: 3,
        name: "seed_default_rules",
        sql: include_str!("../migrations/0003_seed_default_rules.sql"),
    },
    Migration {
        version: 4,
        name: "recategorize_claude_as_deep",
        sql: include_str!("../migrations/0004_recategorize_claude_as_deep.sql"),
    },
    Migration {
        version: 5,
        name: "drop_away_app_spans",
        sql: include_str!("../migrations/0005_drop_away_app_spans.sql"),
    },
];

/// FNV-1a (64-bit): a small, stable, dependency-free hash. Not cryptographic — its only
/// job is to catch an accidental edit to an already-applied migration.
fn checksum(sql: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in sql.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn ensure_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            checksum   TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )
}

fn current_version(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )
}

/// Refuse to start if an already-applied migration's SQL has changed since it ran.
fn verify_applied_checksums(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT version, name, checksum FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (version, name, stored) = row?;
        if let Some(migration) = MIGRATIONS.iter().find(|m| m.version == version) {
            if checksum(migration.sql) != stored {
                return Err(drift_error(format!(
                    "migration {version} ({name}) changed after it was applied — \
                     migrations are immutable once shipped; add a new migration instead"
                )));
            }
        }
    }
    Ok(())
}

/// Run all pending migrations, each atomically.
pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    ensure_migrations_table(conn)?;
    verify_applied_checksums(conn)?;
    let current = current_version(conn)?;

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        println!(
            "[Database] Running migration {}: {}",
            migration.version, migration.name
        );
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, ?4)",
            (
                migration.version,
                migration.name,
                checksum(migration.sql),
                now_unix(),
            ),
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// Build a `rusqlite::Error` carrying a human-readable message (for the drift guard).
fn drift_error(message: String) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
        Some(message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run_migrations(&mut conn).expect("migrations should succeed");
        conn
    }

    #[test]
    fn runs_to_latest_version() {
        let conn = fresh();
        let version = current_version(&conn).unwrap();
        assert_eq!(version, MIGRATIONS.last().unwrap().version);
    }

    #[test]
    fn is_idempotent() {
        let mut conn = fresh();
        let before = current_version(&conn).unwrap();
        run_migrations(&mut conn).unwrap(); // second run is a no-op
        assert_eq!(current_version(&conn).unwrap(), before);
    }

    #[test]
    fn records_version_name_and_checksum() {
        let conn = fresh();
        let (name, stored): (String, String) = conn
            .query_row(
                "SELECT name, checksum FROM schema_migrations WHERE version = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "initial_schema");
        assert_eq!(stored, checksum(MIGRATIONS[0].sql));
    }

    #[test]
    fn seeds_the_four_canonical_contexts() {
        let conn = fresh();
        let slugs: Vec<String> = conn
            .prepare("SELECT slug FROM categories WHERE slug IS NOT NULL ORDER BY slug")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(slugs, vec!["breaks", "comms", "deep", "research"]);
    }

    #[test]
    fn detects_drift_in_an_applied_migration() {
        let mut conn = fresh();
        // Simulate a shipped migration being edited after it was applied.
        conn.execute(
            "UPDATE schema_migrations SET checksum = 'tampered' WHERE version = 1",
            [],
        )
        .unwrap();
        let err = run_migrations(&mut conn).unwrap_err();
        assert!(err.to_string().contains("changed after it was applied"));
    }

    #[test]
    fn seeds_starter_rules_that_categorize_known_apps() {
        let conn = fresh();
        let deep: i64 = conn
            .query_row("SELECT id FROM categories WHERE slug = 'deep'", [], |r| {
                r.get(0)
            })
            .unwrap();
        // A starter rule maps the Cursor editor to Deep work.
        let matched = crate::db::find_category(&conn, "Cursor", "").unwrap();
        assert_eq!(matched, Some(deep));
    }

    #[test]
    fn claude_app_recategorized_to_deep_work() {
        // 0004 re-points the Claude rule from Research to Deep work (supervising an
        // agent is deep work). The claude.ai web tab is unaffected — it matches browser
        // process rules, not this one.
        let conn = fresh();
        let deep: i64 = conn
            .query_row("SELECT id FROM categories WHERE slug = 'deep'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let matched = crate::db::find_category(&conn, "Claude", "").unwrap();
        assert_eq!(matched, Some(deep), "Claude app should now be Deep work");
    }
}
