#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

// `db` is the typed repository layer (the lib's data boundary). It's `pub` so the
// repository API built ahead of its command/capture consumers (Phase 1.2+) is
// reachable — otherwise not-yet-wired fns would trip `dead_code` under `-D warnings`.
pub mod db;
mod watcher;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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
    let errors = watcher::get_error_count();
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

/// The single source of command registration. Both the runtime invoke handler
/// and the generated TS bindings come from this Builder, so they cannot disagree
/// (hard rule 2). Events stay empty until issue #211 is de-risked (commands-only).
fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        get_activity_stats,
        get_categories,
        create_category,
        delete_category,
        get_rules,
        create_rule,
        delete_rule,
        reprocess_logs,
        get_watcher_status,
        get_settings,
        update_setting,
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

            tauri::async_runtime::spawn(watcher::start_watcher(db_conn));
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
