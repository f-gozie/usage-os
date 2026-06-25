//! Cross-platform fallback capture source (`active-win-pos-rs` behind [`CaptureSource`]).
//!
//! Yields the app name only — not titles: that would need `kCGWindowName` (Screen Recording),
//! the permission we refuse. On macOS the event-driven AX impl supersedes it.

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
            // Not window.title — that's CGWindowName (Screen-Recording gated); titles come from AX.
            timestamp: now_unix(),
            ..FocusEvent::default()
        }),
        Err(e) => {
            eprintln!("[Capture] failed to get active window: {:?}", e);
            None
        }
    }
}
