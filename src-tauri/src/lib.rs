#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

// `db` is the typed repository layer (the lib's data boundary). It's `pub` so the
// repository API built ahead of its command/capture consumers (Phase 1.2+) is
// reachable — otherwise not-yet-wired fns would trip `dead_code` under `-D warnings`.
pub mod db;
// `capture` is the observation boundary (hard rule 5): all native/objc2 code lives
// behind the `CaptureSource` trait; tests use a fake. `pub` for the same reason as `db`.
pub mod capture;
// `enrich` turns raw capture signals into stored facts (site, project — D30).
// Cross-platform and CI-testable; consumed by `capture::process_focus_event`.
mod enrich;
// `migrations` is the forward-only SQL migration runner (per-file `.sql`, applied in
// a transaction, checksum-guarded). Paired with the crate-root `migrations/` dir.
mod migrations;
// `rollup` is the pure read-time layer that turns a day's events into the view the
// dial renders (per-axis aggregates + context-runs + template recap — D34, hard rule 6).
mod rollup;

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

/// Build the Day view — per-axis context aggregates, context-runs, and the template
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
    let contexts: HashMap<i64, rollup::ContextMeta> = db::get_context_metas(&conn)?
        .into_iter()
        .map(|(id, slug, name)| (id, rollup::ContextMeta { slug, name }))
        .collect();
    let projects: HashMap<i64, String> = db::get_projects(&conn)?
        .into_iter()
        .map(|p| (p.id, p.display_name))
        .collect();
    Ok(rollup::build_day_view(&events, &contexts, &projects))
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
    let contexts: HashMap<i64, rollup::ContextMeta> = db::get_context_metas(&conn)?
        .into_iter()
        .map(|(id, slug, name)| (id, rollup::ContextMeta { slug, name }))
        .collect();
    let projects: HashMap<i64, String> = db::get_projects(&conn)?
        .into_iter()
        .map(|p| (p.id, p.display_name))
        .collect();
    let mut days = Vec::with_capacity(day_starts.len());
    for (i, &start) in day_starts.iter().enumerate() {
        let end = day_starts.get(i + 1).copied().unwrap_or(week_end);
        let events = db::get_activity_logs(&conn, start, end)?;
        days.push(rollup::build_day_slice(
            start, &events, &contexts, &projects,
        ));
    }
    Ok(rollup::build_week_view(days))
}

/// Build the Timeline view — the day's context-runs, each with its inner app-switch
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
    let contexts: HashMap<i64, rollup::ContextMeta> = db::get_context_metas(&conn)?
        .into_iter()
        .map(|(id, slug, name)| (id, rollup::ContextMeta { slug, name }))
        .collect();
    let projects: HashMap<i64, String> = db::get_projects(&conn)?
        .into_iter()
        .map(|p| (p.id, p.display_name))
        .collect();
    Ok(rollup::build_timeline(&events, &contexts, &projects))
}

#[tauri::command]
#[specta::specta]
fn get_contexts(db: State<DbState>) -> Result<Vec<db::Context>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_contexts(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_context(db: State<DbState>, name: String, color: String) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_context(&conn, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn update_context(
    db: State<DbState>,
    id: i64,
    name: String,
    color: String,
) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::update_context(&conn, id, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_context(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_context(&conn, id).map_err(AppError::from)
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
    context_id: i64,
    match_field: String,
    pattern: String,
    ignore_title: Option<bool>,
) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_rule(
        &conn,
        context_id,
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
/// configuration (contexts, rules, exclusions, settings). One transaction (see `db`).
#[tauri::command]
#[specta::specta]
fn delete_all_data(db: State<DbState>) -> Result<(), AppError> {
    let mut conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_all_data(&mut conn).map_err(AppError::from)
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
        get_contexts,
        create_context,
        update_context,
        delete_context,
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

            // Capture: the source registers on THIS (main) thread — the macOS impl
            // attaches its observers to the main CFRunLoop (D29) — while the
            // consumer drains on a dedicated thread (SQLite + git-shell enrichment
            // block, so they must stay off the async executor; R57).
            let (tx, rx) = std::sync::mpsc::channel();
            capture::default_source().start(tx);
            // The consumer is the sole DB writer: it owns the open span, self-ticks to
            // extend it during sustained single-window work, and gates on idle.
            std::thread::spawn(move || capture::consume(db_conn, rx));
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("[Fatal] Error while running tauri application: {}", e);
        std::process::exit(1);
    }
}

/// Codegen for the IPC bindings. Lives in a test (debug-only) path, so the
/// `expect()` here does NOT violate hard rule 3 (Pattern 8). The freshness gate
/// (CI) regenerates this and fails if the committed `src/bindings.ts` drifts.
#[cfg(test)]
mod export_bindings {
    use specta_typescript::{BigIntExportBehavior, Typescript};

    #[test]
    fn export_bindings() {
        super::make_builder()
            .export(
                Typescript::new()
                    .bigint(BigIntExportBehavior::Number)
                    // @ts-nocheck: the generated file emits events/channel boilerplate
                    // that trips the app's strict `noUnusedLocals`; it's generated, not
                    // authored, so we don't lint it (the freshness gate is its check).
                    .header(
                        "// @ts-nocheck\n// @generated by tauri-specta — DO NOT EDIT. Run `cargo test export_bindings`.\n",
                    ),
                "../src/bindings.ts",
            )
            .expect("failed to export IPC bindings");
    }
}
