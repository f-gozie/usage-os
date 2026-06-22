#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod db;
mod watcher;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

type DbState = Arc<Mutex<Connection>>;

/// Tauri command to fetch activity logs for a given time range.
///
/// # Arguments
/// * `db` - Managed database connection state
/// * `start_time` - Unix timestamp for range start
/// * `end_time` - Unix timestamp for range end
///
/// # Returns
/// Vector of activity logs or error message
#[tauri::command]
fn get_activity_stats(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<db::ActivityLog>, String> {
    let conn = db
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    db::get_activity_logs(&conn, start_time, end_time).map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
fn get_categories(db: State<DbState>) -> Result<Vec<db::Category>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_categories(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_category(db: State<DbState>, name: String, color: String) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_category(&conn, &name, &color).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_category(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::delete_category(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_rules(db: State<DbState>) -> Result<Vec<db::Rule>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_rules(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_rule(
    db: State<DbState>,
    category_id: i64,
    match_field: String,
    pattern: String,
    ignore_title: Option<bool>,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_rule(
        &conn,
        category_id,
        &match_field,
        &pattern,
        ignore_title.unwrap_or(false),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_rule(db: State<DbState>, id: i64) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::delete_rule(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reprocess_logs(db: State<DbState>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::reprocess_logs(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_watcher_status() -> Result<serde_json::Value, String> {
    let errors = watcher::get_error_count();
    Ok(serde_json::json!({
        "consecutive_errors": errors,
        "healthy": errors < 6
    }))
}

#[tauri::command]
fn get_settings(db: State<DbState>) -> Result<Vec<(String, String)>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_all_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_setting(db: State<DbState>, key: String, value: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
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
            update_setting
        ])
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
