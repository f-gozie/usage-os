mod watcher;
mod db;

use tauri::{State, Manager};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

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
    let conn = db.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    db::get_activity_logs(&conn, start_time, end_time)
        .map_err(|e| format!("Database error: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_activity_stats])
        .setup(|app| {
            let db_path = db::get_db_path(&app.handle())
                .expect("Failed to get database path");
            
            let db_conn = db::init_database(&db_path)
                .expect("Failed to initialize database");
            
            app.manage(db_conn.clone());
            
            tauri::async_runtime::spawn(watcher::start_watcher(db_conn));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
