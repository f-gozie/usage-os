//! Interim/fallback polling capture source.
//!
//! Wraps the v0.1.0 `active-win-pos-rs` approach behind [`CaptureSource`].
//! Cross-platform; yields the **app name** only — NOT window titles:
//! `active-win-pos-rs` reads `kCGWindowName`, gated by Screen Recording (capture
//! standard C1, the permission we refuse). Idle is the consumer's job (D39), not the
//! source's. On macOS the event-driven AX impl supersedes it; this stays as the
//! non-macOS fallback.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::{note_capture_failure, note_capture_ok, CaptureSource, FocusEvent};
use crate::db::now_unix;

const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Polls the foreground app on a fixed interval, on its own thread.
pub struct PollingCapture {
    interval: Duration,
}

impl Default for PollingCapture {
    fn default() -> Self {
        Self {
            interval: POLL_INTERVAL,
        }
    }
}

impl CaptureSource for PollingCapture {
    fn start(self: Box<Self>, tx: Sender<FocusEvent>) {
        println!(
            "[Capture] polling source: every {}s",
            self.interval.as_secs()
        );
        thread::spawn(move || loop {
            match active_focus() {
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

/// Read the current foreground app into a [`FocusEvent`].
fn active_focus() -> Option<FocusEvent> {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => Some(FocusEvent {
            app_name: window.app_name,
            pid: window.process_id as i32,
            // Intentionally NOT window.title — that's CGWindowName (Screen-Recording
            // gated, capture standard C1). Titles come from AX on macOS.
            timestamp: now_unix(),
            ..FocusEvent::default()
        }),
        Err(e) => {
            eprintln!("[Capture] failed to get active window: {:?}", e);
            None
        }
    }
}
