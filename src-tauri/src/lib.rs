#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

// The typed repository layer (the lib's data boundary). `pub` so its API is reachable ahead of
// its consumers (otherwise not-yet-wired fns trip `dead_code` under `-D warnings`).
pub mod db;
// The observation boundary (hard rule 5): all native/objc2 code behind the `CaptureSource` trait.
pub mod capture;
// Turns raw capture signals into stored facts (site, project — D30).
mod enrich;
// The forward-only SQL migration runner; paired with the crate-root `migrations/` dir.
mod migrations;
// The pure read-time layer that turns a day's events into the view the dial renders (hard rule 6).
mod rollup;
// Installed-app catalog + offline icon extraction, isolated like the other native surfaces.
mod apps;

// The recap-narration seam (hard rule 5): a mockable `Narrator` + `build_recap` with a
// deterministic template fallback (D48). `pub` so its not-yet-wired API isn't dead-code.
pub mod ai;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri_specta::{collect_commands, Builder};

type DbState = Arc<Mutex<Connection>>;

/// Typed error contract for every command (replaces `Result<_, String>`).
/// Crosses to TS as a discriminated union the frontend can `switch` on `kind`.
#[derive(Debug, thiserror::Error, Serialize, specta::Type)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("database error: {0}")]
    Db(String),
    #[error("database lock was poisoned")]
    LockPoisoned,
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

/// Health of the background capture watcher (replaces an untyped `serde_json::Value`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WatcherStatus {
    pub consecutive_errors: u64,
    pub healthy: bool,
}

/// One persisted setting key/value (replaces the awkward `[string, string][]`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

/// Fetch activity logs for a `[start_time, end_time]` Unix-second range.
#[tauri::command]
#[specta::specta]
fn get_activity_stats(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<db::ActivityLog>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_activity_logs(&conn, start_time, end_time).map_err(AppError::from)
}

/// The category + project lookup maps the rollup reads off: `category_id -> meta` and
/// `project_id -> display name`.
type LookupMaps = (HashMap<i64, rollup::CategoryMeta>, HashMap<i64, String>);

/// The category + project lookup maps the three rollup commands share — one query each.
fn load_lookup_maps(conn: &Connection) -> Result<LookupMaps, AppError> {
    let categories = db::get_category_metas(conn)?
        .into_iter()
        .map(|(id, slug, name, color)| (id, rollup::CategoryMeta { slug, name, color }))
        .collect();
    let projects = db::get_projects(conn)?
        .into_iter()
        .map(|p| (p.id, p.display_name))
        .collect();
    Ok((categories, projects))
}

/// Build the Day view — per-axis category aggregates, category-runs, and the template
/// recap (D34) — for a `[start_time, end_time]` Unix-second range. Numbers are computed
/// in Rust (hard rule 6); the frontend only renders this.
#[tauri::command]
#[specta::specta]
fn get_day(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<rollup::DayView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let events = db::get_activity_logs(&conn, start_time, end_time)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    Ok(rollup::build_day_view(
        &events,
        &categories,
        &projects,
        start_time,
    ))
}

/// Build the Week view — 7 day-slices (each a mini-dial's runs + totals) plus week-level
/// aggregates (D34, hard rule 6). `day_starts` are the 7 local midnights (DST-correct,
/// computed by the frontend like `get_day`'s bounds); `week_end` is the exclusive end of
/// the last day. Each day's events are read for `[day_start, next_day_start | week_end)`.
#[tauri::command]
#[specta::specta]
fn get_week(
    db: State<DbState>,
    day_starts: Vec<i64>,
    week_end: i64,
) -> Result<rollup::WeekView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    let mut days = Vec::with_capacity(day_starts.len());
    for (i, &start) in day_starts.iter().enumerate() {
        let end = day_starts.get(i + 1).copied().unwrap_or(week_end);
        let events = db::get_activity_logs(&conn, start, end)?;
        days.push(rollup::build_day_slice(
            start,
            &events,
            &categories,
            &projects,
        ));
    }
    Ok(rollup::build_week_view(days))
}

/// Build the Timeline view — the day's category-runs, each with its inner app-switch
/// segments (D34) — for a `[start_time, end_time]` Unix-second range. Same read-model
/// inputs as `get_day`; numbers in Rust (hard rule 6).
#[tauri::command]
#[specta::specta]
fn get_timeline(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<rollup::TimelineView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let events = db::get_activity_logs(&conn, start_time, end_time)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    Ok(rollup::build_timeline(&events, &categories, &projects))
}

#[tauri::command]
#[specta::specta]
fn get_categories(db: State<DbState>) -> Result<Vec<db::Category>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_categories(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_category(db: State<DbState>, name: String, color: String) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_category(&conn, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn update_category(
    db: State<DbState>,
    id: i64,
    name: String,
    color: String,
) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::update_category(&conn, id, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_category(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_category(&conn, id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn get_rules(db: State<DbState>) -> Result<Vec<db::Rule>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_rules(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_rule(
    db: State<DbState>,
    category_id: i64,
    match_field: String,
    pattern: String,
    ignore_title: Option<bool>,
) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_rule(
        &conn,
        category_id,
        &match_field,
        &pattern,
        ignore_title.unwrap_or(false),
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_rule(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_rule(&conn, id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn reprocess_logs(db: State<DbState>) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::reprocess_logs(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn get_watcher_status() -> Result<WatcherStatus, AppError> {
    let errors = capture::get_error_count();
    Ok(WatcherStatus {
        consecutive_errors: errors,
        healthy: errors < 6,
    })
}

#[tauri::command]
#[specta::specta]
fn get_settings(db: State<DbState>) -> Result<Vec<Setting>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let rows = db::get_all_settings(&conn).map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(key, value)| Setting { key, value })
        .collect())
}

#[tauri::command]
#[specta::specta]
fn update_setting(db: State<DbState>, key: String, value: String) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::set_setting(&conn, &key, &value).map_err(AppError::from)
}

/// Persist the retention window **and** immediately prune anything older. Distinct from
/// the generic `update_setting` because retention has a side effect (deleting rows) that
/// a plain key/value setter must not carry. `days <= 0` = "keep forever" (a no-op prune).
/// Returns the number of rows deleted so the UI can confirm.
#[tauri::command]
#[specta::specta]
fn set_retention_days(db: State<DbState>, days: i64) -> Result<usize, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::set_setting(&conn, "data_retention_days", &days.to_string())?;
    db::cleanup_old_data(&conn, days).map_err(AppError::from)
}

// --- Exclusions (D8): thin passthroughs over the repository ---

#[tauri::command]
#[specta::specta]
fn get_exclusions(db: State<DbState>) -> Result<Vec<db::Exclusion>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_exclusions(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_exclusion(
    db: State<DbState>,
    match_type: String,
    pattern: String,
    mode: String,
) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_exclusion(&conn, &match_type, &pattern, &mode).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_exclusion(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_exclusion(&conn, id).map_err(AppError::from)
}

// --- Data ownership ---

/// Absolute path to the SQLite file, so the frontend can reveal it in Finder via the
/// `opener` plugin (path resolution lives in Rust — one source of truth).
#[tauri::command]
#[specta::specta]
fn get_database_path(app: tauri::AppHandle) -> Result<String, AppError> {
    db::get_db_path(&app)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(AppError::Db)
}

/// Export all events as RFC-4180 CSV to a file next to the DB (an app-owned dir that
/// already exists — no new fs scope), returning its absolute path for the frontend to
/// reveal. SQL + the file write stay in Rust (hard rule 4); nothing leaves the machine.
#[tauri::command]
#[specta::specta]
fn export_events_csv(app: tauri::AppHandle, db: State<DbState>) -> Result<String, AppError> {
    let db_path = db::get_db_path(&app).map_err(AppError::Db)?;
    let out = db_path.with_file_name(format!("usageos-export-{}.csv", db::now_unix()));
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let mut file = std::fs::File::create(&out).map_err(|e| AppError::Db(e.to_string()))?;
    db::export_events_csv(&conn, &mut file)?;
    Ok(out.to_string_lossy().into_owned())
}

/// Erase the captured record (events + derived projects/sites), preserving the user's
/// configuration (categories, rules, exclusions, settings). One transaction (see `db`).
#[tauri::command]
#[specta::specta]
fn delete_all_data(db: State<DbState>) -> Result<(), AppError> {
    let mut conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_all_data(&mut conn).map_err(AppError::from)
}

/// The installed-app catalog (name + icon data-URI) backing the UI's `AppIcon`. Reads
/// public app bundles and caches 64px PNGs next to the DB (an app-owned dir — no new
/// fs scope, no network; hard rule 1). Read-only; returns an empty list rather than
/// erroring when nothing scans (e.g. on CI Linux), so the UI just shows monograms.
#[tauri::command]
#[specta::specta]
fn list_installed_apps(app: tauri::AppHandle) -> Result<Vec<apps::InstalledApp>, AppError> {
    let db_path = db::get_db_path(&app).map_err(AppError::Db)?;
    let cache_dir = db_path.with_file_name("icon-cache");
    Ok(apps::list_installed(
        &apps::default_search_dirs(),
        &cache_dir,
    ))
}

/// Apps with tracked time that match no rule (they roll up as "Uncategorized"), for the
/// Settings list — all-time, ranked, trivial spans floored (see `db`). Numbers in Rust.
#[tauri::command]
#[specta::specta]
fn get_uncategorized_apps(db: State<DbState>) -> Result<Vec<db::UncategorizedApp>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_uncategorized_apps(&conn).map_err(AppError::from)
}

/// The single source of command registration. Both the runtime invoke handler
/// and the generated TS bindings come from this Builder, so they cannot disagree
/// (hard rule 2). Events stay empty until issue #211 is de-risked (commands-only).
fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        get_activity_stats,
        get_day,
        get_week,
        get_timeline,
        get_categories,
        create_category,
        update_category,
        delete_category,
        get_rules,
        create_rule,
        delete_rule,
        reprocess_logs,
        get_exclusions,
        create_exclusion,
        delete_exclusion,
        get_watcher_status,
        get_settings,
        update_setting,
        set_retention_days,
        get_database_path,
        export_events_csv,
        delete_all_data,
        list_installed_apps,
        get_uncategorized_apps,
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(|app| {
            let db_path = db::get_db_path(app.handle())?;

            let db_conn = db::init_database(&db_path)?;

            // Run data retention cleanup before starting the watcher.
            // A poisoned lock here is non-fatal: skip cleanup and continue startup.
            match db_conn.lock() {
                Ok(conn) => {
                    if let Ok(Some(days_str)) = db::get_setting(&conn, "data_retention_days") {
                        if let Ok(days) = days_str.parse::<i64>() {
                            match db::cleanup_old_data(&conn, days) {
                                Ok(deleted) if deleted > 0 => {
                                    println!("[Startup] Cleaned up {} old activity logs", deleted);
                                }
                                Err(e) => eprintln!("[Startup] Cleanup failed: {}", e),
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[Startup] Skipping cleanup, db lock poisoned: {}", e),
            }

            app.manage(db_conn.clone());

            // The source registers on this (main) thread — the macOS impl attaches to the main
            // CFRunLoop (D29) — while the consumer (the sole DB writer) drains on a dedicated
            // thread, since SQLite + git-shell enrichment block and must stay off the executor.
            let (tx, rx) = std::sync::mpsc::channel();
            capture::default_source().start(tx);
            std::thread::spawn(move || capture::consume(db_conn, rx));
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("[Fatal] Error while running tauri application: {}", e);
        std::process::exit(1);
    }
}

/// Codegen for the IPC bindings. Test-only, so the `expect()` doesn't violate hard rule 3.
/// The CI freshness gate regenerates this and fails if the committed `src/bindings.ts` drifts.
#[cfg(test)]
mod export_bindings {
    use specta_typescript::{BigIntExportBehavior, Typescript};

    #[test]
    fn export_bindings() {
        super::make_builder()
            .export(
                Typescript::new()
                    .bigint(BigIntExportBehavior::Number)
                    // @ts-nocheck: generated boilerplate trips strict `noUnusedLocals`; it's
                    // generated, not authored, so the freshness gate is its check, not the linter.
                    .header(
                        "// @ts-nocheck\n// @generated by tauri-specta — DO NOT EDIT. Run `cargo test export_bindings`.\n",
                    ),
                "../src/bindings.ts",
            )
            .expect("failed to export IPC bindings");
    }
}
