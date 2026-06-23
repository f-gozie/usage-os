//! Interim/fallback polling capture source.
//!
//! Wraps the v0.1.0 `active-win-pos-rs` + `user-idle` approach behind
//! [`CaptureSource`]. Cross-platform; yields the **app name + idle** only — NOT
//! window titles: `active-win-pos-rs` reads `kCGWindowName`, gated by Screen
//! Recording (capture standard C1, the permission we refuse). On macOS the
//! event-driven AX impl supersedes it; this stays as the non-macOS fallback.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::{note_capture_failure, note_capture_ok, CaptureSource, FocusEvent};
use crate::db::now_unix;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const IDLE_THRESHOLD_SECS: u64 = 180;

/// Polls the foreground app on a fixed interval, on its own thread.
pub struct PollingCapture {
    interval: Duration,
    idle_threshold_secs: u64,
}

impl Default for PollingCapture {
    fn default() -> Self {
        Self {
            interval: POLL_INTERVAL,
            idle_threshold_secs: IDLE_THRESHOLD_SECS,
        }
    }
}

impl CaptureSource for PollingCapture {
    fn start(self: Box<Self>, tx: Sender<FocusEvent>) {
        println!(
            "[Capture] polling source: every {}s, idle threshold {}s",
            self.interval.as_secs(),
            self.idle_threshold_secs
        );
        thread::spawn(move || loop {
            match active_focus(self.idle_threshold_secs) {
                Some(ev) => {
                    note_capture_ok();
                    if tx.send(ev).is_err() {
                        break; // consumer dropped — stop polling
                    }
                }
                None => note_capture_failure(),
            }
            thread::sleep(self.interval);
        });
    }
}

/// Read the current foreground app + idle state into a [`FocusEvent`].
fn active_focus(idle_threshold_secs: u64) -> Option<FocusEvent> {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => Some(FocusEvent {
            app_name: window.app_name,
            pid: window.process_id as i32,
            // Intentionally NOT window.title — that's CGWindowName (Screen-Recording
            // gated, capture standard C1). Titles come from AX on macOS.
            is_idle: is_user_idle(idle_threshold_secs),
            timestamp: now_unix(),
            ..FocusEvent::default()
        }),
        Err(e) => {
            eprintln!("[Capture] failed to get active window: {:?}", e);
            None
        }
    }
}

/// True if no input for `>= threshold_secs`. `user-idle` reads CoreGraphics
/// aggregate idle time — no permission (capture standard C10).
fn is_user_idle(threshold_secs: u64) -> bool {
    match user_idle::UserIdle::get_time() {
        Ok(t) => t.as_seconds() >= threshold_secs,
        Err(_) => false,
    }
}
