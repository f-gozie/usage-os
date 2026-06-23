//! Interim polling capture source.
//!
//! Wraps the v0.1.0 `active-win-pos-rs` + `user-idle` approach behind
//! [`CaptureSource`] so the app keeps capturing while the event-driven macOS impl
//! is built (1.2b). Cross-platform; it yields the **app name + idle** only — NOT
//! window titles: `active-win-pos-rs` reads `kCGWindowName`, which is gated by
//! Screen Recording (capture standard C1, the permission we refuse). Titles (and
//! url/cwd) arrive via AX in the macOS impl. Kept afterwards as the fallback.

use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tokio::time::interval;

use super::{note_capture_failure, note_capture_ok, CaptureSource, FocusEvent};
use crate::db::now_unix;

const POLL_INTERVAL_SECS: u64 = 5;
const IDLE_THRESHOLD_SECS: u64 = 180;

/// Polls the foreground app on a fixed interval.
pub struct PollingCapture {
    interval: Duration,
    idle_threshold_secs: u64,
}

impl Default for PollingCapture {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(POLL_INTERVAL_SECS),
            idle_threshold_secs: IDLE_THRESHOLD_SECS,
        }
    }
}

impl CaptureSource for PollingCapture {
    fn start(self: Box<Self>, tx: UnboundedSender<FocusEvent>) {
        println!(
            "[Capture] polling source: every {}s, idle threshold {}s",
            self.interval.as_secs(),
            self.idle_threshold_secs
        );
        tauri::async_runtime::spawn(async move {
            let mut ticker = interval(self.interval);
            loop {
                ticker.tick().await;
                match active_focus(self.idle_threshold_secs) {
                    Some(ev) => {
                        note_capture_ok();
                        if tx.send(ev).is_err() {
                            break; // consumer dropped — stop polling
                        }
                    }
                    None => note_capture_failure(),
                }
            }
        });
    }
}

/// Read the current foreground app + idle state into a [`FocusEvent`].
fn active_focus(idle_threshold_secs: u64) -> Option<FocusEvent> {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => Some(FocusEvent {
            app_name: window.app_name,
            bundle_id: None,
            pid: window.process_id as i32,
            // Intentionally NOT window.title — that's CGWindowName (Screen-Recording
            // gated, capture standard C1). Titles come from AX in 1.2b.
            window_title: None,
            url: None,
            is_idle: is_user_idle(idle_threshold_secs),
            timestamp: now_unix(),
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
