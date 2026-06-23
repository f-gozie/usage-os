//! Capture: the boundary that observes the active app/window (hard rule 5, D22).
//!
//! [`CaptureSource`] produces [`FocusEvent`]s; [`consume`] drains them into the
//! repository via [`process_focus_event`], applying D8 sensitive handling and D30
//! project inference. All platform/native code lives behind this trait — nothing
//! above `capture/` imports objc2. Tests use [`FakeCapture`]; production uses the
//! event-driven [`macos::MacosCapture`] on macOS, else [`PollingCapture`].

mod fake;
#[cfg(target_os = "macos")]
mod macos;
mod polling;

pub use fake::FakeCapture;
pub use polling::PollingCapture;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};

use rusqlite::Connection;

use crate::db::{self, DbConnection, ExclusionMode, NewEvent};
use crate::enrich::{self, ProjectAssignment, ProjectSignals};

/// A focus change marshaled from the capture side to the consumer thread. All
/// fields are owned/`Send` (D29). The signals are filled by whichever source is
/// active: polling sets app + idle only; the macOS impl adds title, url, and cwd.
#[derive(Debug, Clone, Default)]
pub struct FocusEvent {
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub window_title: Option<String>,
    /// Browser front-tab URL (never set for incognito/private — D8).
    pub url: Option<String>,
    /// Terminal shell cwd — the ephemeral project signal, resolved at capture time.
    pub cwd: Option<String>,
    pub is_idle: bool,
    pub timestamp: i64,
}

/// A producer of [`FocusEvent`]s. Each impl owns its execution model: polling
/// spawns a thread; the macOS impl registers run-loop observers. `start` is called
/// on the **main thread** during Tauri setup (the macOS impl attaches its observers
/// to the main `CFRunLoop`); the impl keeps whatever it needs alive for the process
/// lifetime.
pub trait CaptureSource: Send {
    fn start(self: Box<Self>, tx: Sender<FocusEvent>);
}

// ── Health (drives the get_watcher_status command) ───────────────────────────

const ERROR_THRESHOLD: u64 = 6;
static CONSECUTIVE_ERRORS: AtomicU64 = AtomicU64::new(0);

/// Consecutive capture (window-detection) failures, for the frontend status.
pub fn get_error_count() -> u64 {
    CONSECUTIVE_ERRORS.load(Ordering::Relaxed)
}

/// Record a successful detection — resets the failure counter.
pub fn note_capture_ok() {
    CONSECUTIVE_ERRORS.store(0, Ordering::Relaxed);
}

/// Record a detection failure; warn once when the threshold is first crossed.
pub fn note_capture_failure() {
    let count = CONSECUTIVE_ERRORS.fetch_add(1, Ordering::Relaxed) + 1;
    if count == ERROR_THRESHOLD {
        eprintln!(
            "[Capture] Warning: {} consecutive window-detection failures. \
             Check permissions (macOS: Accessibility).",
            count
        );
    }
}

// ── Source selection ─────────────────────────────────────────────────────────

/// The production capture source for this platform: the event-driven macOS impl
/// where available, else the cross-platform polling fallback.
pub fn default_source() -> Box<dyn CaptureSource> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosCapture::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(PollingCapture::default())
    }
}

// ── Consumer (runs on a dedicated thread — SQLite + git shell are blocking) ───

/// Drain capture events into the repository until the channel closes. Runs on its
/// own `std::thread` (R57 direction): per-event processing does blocking SQLite
/// writes and a `git` shell (project inference), neither of which may run on the
/// async executor.
pub fn consume(db_conn: DbConnection, rx: Receiver<FocusEvent>) {
    println!("[Capture] consumer thread up");
    for ev in rx {
        match db_conn.lock() {
            Ok(conn) => {
                if let Err(e) = process_focus_event(&conn, &ev) {
                    eprintln!("[Capture] failed to write event: {}", e);
                }
            }
            Err(e) => eprintln!("[Capture] db lock poisoned, dropping event: {}", e),
        }
    }
    println!("[Capture] channel closed; consumer stopped");
}

/// Turn one [`FocusEvent`] into a stored span. Order matters: sensitive handling
/// (D8) is applied first — an `Exclude` match is dropped entirely; a `Private`
/// match records time + app but omits title/url/site and skips project inference.
/// Otherwise the url is parsed to a site and the live signals are resolved to a
/// project (D30) before the coalescing write.
pub fn process_focus_event(conn: &Connection, ev: &FocusEvent) -> rusqlite::Result<()> {
    let title = ev.window_title.as_deref().unwrap_or("");
    let site = ev.url.as_deref().and_then(enrich::parse_site);

    match db::match_exclusion(conn, &ev.app_name, title, site.as_deref())? {
        Some(ExclusionMode::Exclude) => return Ok(()), // D8: never written
        Some(ExclusionMode::Private) => {
            return db::log_focus(
                conn,
                &NewEvent {
                    process_name: &ev.app_name,
                    window_title: "", // D8: omit title
                    is_private: true,
                    is_idle: ev.is_idle,
                    timestamp: ev.timestamp,
                    ..Default::default()
                },
            );
        }
        None => {}
    }

    let (project_id, project_abstain_reason) = match enrich::infer_project(
        conn,
        &ProjectSignals {
            cwd: ev.cwd.as_deref(),
            url: ev.url.as_deref(),
            title: ev.window_title.as_deref(),
        },
    )? {
        ProjectAssignment::Assigned(id) => (Some(id), None),
        ProjectAssignment::Abstain(reason) => (None, Some(reason)),
    };

    db::log_focus(
        conn,
        &NewEvent {
            process_name: &ev.app_name,
            window_title: title,
            url: ev.url.as_deref(),
            site: site.as_deref(),
            project_id,
            project_abstain_reason,
            is_private: false,
            is_idle: ev.is_idle,
            category_id: None,
            timestamp: ev.timestamp,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActivityLog;

    /// In-memory DB on the real migration chain.
    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        db::run_migrations(&conn).expect("migrations");
        conn
    }

    fn focus(app: &str, title: Option<&str>, ts: i64) -> FocusEvent {
        FocusEvent {
            app_name: app.to_string(),
            pid: 1,
            window_title: title.map(|s| s.to_string()),
            timestamp: ts,
            ..Default::default()
        }
    }

    fn all_logs(conn: &Connection) -> Vec<ActivityLog> {
        db::get_activity_logs(conn, 0, i64::MAX).unwrap()
    }

    #[test]
    fn normal_event_is_written() {
        let conn = test_db();
        process_focus_event(&conn, &focus("Code", Some("main.rs"), 1000)).unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[0].window_title, "main.rs");
        assert!(!logs[0].is_private);
    }

    #[test]
    fn excluded_app_is_dropped() {
        let conn = test_db();
        db::create_exclusion(&conn, "app", "1Password", "exclude").unwrap();
        process_focus_event(&conn, &focus("1Password", Some("Vault"), 1000)).unwrap();
        assert_eq!(
            all_logs(&conn).len(),
            0,
            "excluded event must never be written"
        );
    }

    #[test]
    fn private_app_records_time_without_title() {
        let conn = test_db();
        db::create_exclusion(&conn, "app", "Banking", "private").unwrap();
        process_focus_event(&conn, &focus("Banking App", Some("Acct 1234"), 1000)).unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].is_private);
        assert_eq!(
            logs[0].window_title, "",
            "title must be omitted for private (D8)"
        );
        assert_eq!(
            logs[0].process_name, "Banking App",
            "time + app still recorded"
        );
    }

    #[test]
    fn repeated_event_coalesces() {
        let conn = test_db();
        process_focus_event(&conn, &focus("Code", Some("main.rs"), 1000)).unwrap();
        process_focus_event(&conn, &focus("Code", Some("main.rs"), 1005)).unwrap();
        process_focus_event(&conn, &focus("Code", Some("main.rs"), 1010)).unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1, "identical consecutive spans coalesce");
        assert_eq!(logs[0].end_time, 1010);
    }

    #[test]
    fn browser_event_sets_site_and_project() {
        let conn = test_db();
        let ev = FocusEvent {
            app_name: "Google Chrome".to_string(),
            pid: 2,
            window_title: Some("usenudgeai/nudge".to_string()),
            url: Some("https://github.com/usenudgeai/nudge".to_string()),
            timestamp: 1000,
            ..Default::default()
        };
        process_focus_event(&conn, &ev).unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].site.as_deref(), Some("github.com"));
        assert!(
            logs[0].project_id.is_some(),
            "github url should resolve a project"
        );
        assert_eq!(logs[0].project_abstain_reason, None);
    }

    #[test]
    fn ambiguous_url_persists_abstain_reason() {
        let conn = test_db();
        let ev = FocusEvent {
            app_name: "Google Chrome".to_string(),
            pid: 2,
            window_title: Some("Grafana".to_string()),
            url: Some("https://acme.grafana.net/d/x".to_string()),
            timestamp: 1000,
            ..Default::default()
        };
        process_focus_event(&conn, &ev).unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs[0].project_id, None);
        assert_eq!(logs[0].project_abstain_reason.as_deref(), Some("ambiguous"));
    }

    #[test]
    fn fake_source_feeds_the_spine() {
        // The whole capture spine, exercised without a Mac (hard rule 5).
        let conn = test_db();
        let events = vec![
            focus("Code", Some("main.rs"), 1000),
            focus("Slack", Some("general"), 1100),
        ];
        let (tx, rx) = std::sync::mpsc::channel::<FocusEvent>();
        Box::new(FakeCapture::new(events)).start(tx);
        for ev in rx {
            process_focus_event(&conn, &ev).unwrap();
        }
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[1].process_name, "Slack");
    }
}
