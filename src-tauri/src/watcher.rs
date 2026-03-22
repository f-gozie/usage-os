use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use crate::db::{self, DbConnection};

const POLL_INTERVAL_SECS: u64 = 5;
const IDLE_THRESHOLD_SECS: u64 = 180;

/// Information about the currently active window.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub process_name: String,
    pub window_title: String,
    pub is_idle: bool,
}

/// Get information about the currently active window.
///
/// Returns None if unable to query the OS (e.g., missing permissions).
pub fn get_active_window() -> Option<WindowInfo> {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => {
            let is_idle = is_user_idle();
            Some(WindowInfo {
                process_name: window.app_name,
                window_title: window.title,
                is_idle,
            })
        }
        Err(e) => {
            eprintln!("[Watcher Error] Failed to get active window: {:?}", e);
            eprintln!("[Watcher] On macOS, you may need to grant Accessibility permissions:");
            eprintln!("[Watcher] System Preferences > Privacy & Security > Accessibility");
            None
        }
    }
}

/// Check if the user is currently idle.
///
/// Returns true if no mouse/keyboard activity for >= IDLE_THRESHOLD_SECS.
pub fn is_user_idle() -> bool {
    match user_idle::UserIdle::get_time() {
        Ok(idle_time) => idle_time.as_seconds() >= IDLE_THRESHOLD_SECS,
        Err(_) => false,
    }
}

/// Get current Unix timestamp in seconds.
fn get_current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

/// Get current Unix timestamp in seconds (public for testing).
pub fn get_current_timestamp_pub() -> i64 {
    get_current_timestamp()
}

/// Start the background watcher that polls active window info.
///
/// Runs indefinitely, polling every POLL_INTERVAL_SECS seconds.
/// Logs activity to the database with automatic coalescing.
pub async fn start_watcher(db_conn: DbConnection) {
    println!("[Watcher] Starting background watcher...");
    println!("[Watcher] Polling interval: {} seconds", POLL_INTERVAL_SECS);
    println!("[Watcher] Idle threshold: {} seconds ({} minutes)", IDLE_THRESHOLD_SECS, IDLE_THRESHOLD_SECS / 60);
    
    let mut interval = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    let mut tick_count = 0;

    loop {
        interval.tick().await;
        tick_count += 1;

        if tick_count % 12 == 1 {
            println!("[Watcher] Still running... (tick #{})", tick_count);
        }

        if let Some(info) = get_active_window() {
            let timestamp = get_current_timestamp();
            
            if let Err(e) = db::log_activity_safe(
                &db_conn,
                &info.process_name,
                &info.window_title,
                info.is_idle,
                timestamp,
            ) {
                eprintln!("[Watcher] Failed to log activity: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_timestamp_reasonable() {
        let ts = get_current_timestamp();
        // Should be after 2024-01-01 and before 2030-01-01
        assert!(ts > 1_704_067_200, "Timestamp should be after 2024");
        assert!(ts < 1_893_456_000, "Timestamp should be before 2030");
    }
}

