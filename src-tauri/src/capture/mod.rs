//! Capture: the boundary that observes the active app/window (hard rule 5, D22).
//!
//! [`CaptureSource`] produces [`FocusEvent`]s; [`run`] drains them into the
//! repository via [`process_focus_event`]. All platform/native code lives behind
//! this trait — nothing above `capture/` imports objc2. Tests use [`FakeCapture`];
//! production uses [`PollingCapture`] today and the event-driven macOS impl (1.2b).

mod fake;
mod polling;

pub use fake::FakeCapture;
pub use polling::PollingCapture;

use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::Connection;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::db::{self, DbConnection, ExclusionMode};

/// A focus change marshaled from the capture side to the async consumer. All
/// fields are owned/`Send` (D29). `url` is `None` until the macOS impl (1.2b);
/// `site`/project are filled by the later enrichment pass.
#[derive(Debug, Clone)]
pub struct FocusEvent {
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub is_idle: bool,
    pub timestamp: i64,
}

/// A producer of [`FocusEvent`]s. Each impl owns its execution model: polling
/// spawns a task; the macOS impl registers run-loop observers. Capture runs for
/// the whole process — there is no stop (the app captures while it is open).
pub trait CaptureSource: Send {
    /// Begin producing events into `tx`. Consumes `self`; the impl keeps whatever
    /// it needs alive for the process lifetime.
    fn start(self: Box<Self>, tx: UnboundedSender<FocusEvent>);
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

// ── Source selection (1.2b swaps in the event-driven macOS impl) ─────────────

/// The production capture source for this platform. Today: polling everywhere;
/// 1.2b returns the event-driven `MacosCapture` under `#[cfg(target_os = "macos")]`.
pub fn default_source() -> Box<dyn CaptureSource> {
    Box::new(PollingCapture::default())
}

// ── Consumer ─────────────────────────────────────────────────────────────────

/// Run capture: start `source`, then drain its events into the repository until
/// the channel closes. Spawned on the Tokio runtime during Tauri setup.
pub async fn run(db_conn: DbConnection, source: Box<dyn CaptureSource>) {
    println!("[Capture] starting capture runner");
    let (tx, mut rx) = unbounded_channel::<FocusEvent>();
    source.start(tx);

    while let Some(ev) = rx.recv().await {
        match db_conn.lock() {
            Ok(conn) => {
                if let Err(e) = process_focus_event(&conn, &ev) {
                    eprintln!("[Capture] failed to write event: {}", e);
                }
            }
            Err(e) => eprintln!("[Capture] db lock poisoned, dropping event: {}", e),
        }
    }
    println!("[Capture] channel closed; capture runner stopped");
}

/// Turn one [`FocusEvent`] into a stored span. Applies sensitive handling (D8)
/// before writing: an `Exclude` match is dropped entirely; a `Private` match
/// records time + app but omits the title/url (R58). The normal path coalesces.
pub fn process_focus_event(conn: &Connection, ev: &FocusEvent) -> rusqlite::Result<()> {
    let title = ev.window_title.as_deref().unwrap_or("");
    // No site in 1.2a (polling carries no url); 1.2b parses url→site and passes it.
    let site: Option<&str> = None;

    match db::match_exclusion(conn, &ev.app_name, title, site)? {
        Some(ExclusionMode::Exclude) => Ok(()), // D8: never written
        Some(ExclusionMode::Private) => db::log_focus(
            conn,
            &ev.app_name,
            None, // omit title (D8)
            None, // omit url
            None,
            ev.is_idle,
            true,
            ev.timestamp,
        ),
        None => db::log_focus(
            conn,
            &ev.app_name,
            ev.window_title.as_deref(),
            ev.url.as_deref(),
            site,
            ev.is_idle,
            false,
            ev.timestamp,
        ),
    }
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
            bundle_id: None,
            pid: 1,
            window_title: title.map(|s| s.to_string()),
            url: None,
            is_idle: false,
            timestamp: ts,
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
    fn fake_source_feeds_the_spine() {
        // The whole capture spine, exercised without a Mac (hard rule 5).
        let conn = test_db();
        let events = vec![
            focus("Code", Some("main.rs"), 1000),
            focus("Chrome", Some("GitHub"), 1100),
        ];
        let (tx, mut rx) = unbounded_channel::<FocusEvent>();
        Box::new(FakeCapture::new(events)).start(tx);
        while let Ok(ev) = rx.try_recv() {
            process_focus_event(&conn, &ev).unwrap();
        }
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[1].process_name, "Chrome");
    }
}
