//! Capture: the boundary that observes the active app/window (hard rule 5).
//!
//! [`CaptureSource`] produces [`FocusEvent`]s; [`consume`] is the single writer — a state machine
//! that owns the one open span, applies sensitive handling (D8) and project inference (D30), and
//! self-ticks to extend the span and detect idle. All native code lives behind this trait. Tests
//! use [`FakeCapture`]; production uses [`macos::MacosCapture`] on macOS, else [`PollingCapture`].

mod fake;
#[cfg(target_os = "macos")]
mod macos;
mod polling;

pub use fake::FakeCapture;
pub use polling::PollingCapture;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use rusqlite::Connection;

use crate::db::{self, DbConnection, ExclusionMode, NewEvent};
use crate::enrich::{self, ProjectAssignment, ProjectSignals};

/// A focus change marshaled from the capture source to the consumer thread (owned/`Send`).
/// Polling fills app only; the macOS impl adds title, url, and cwd. Idle is not a per-event
/// signal — the consumer is the sole source of idle truth (see D39).
#[derive(Debug, Clone, Default)]
pub struct FocusEvent {
    pub app_name: String,
    pub window_title: Option<String>,
    /// Browser front-tab URL (never set for incognito/private — D8).
    pub url: Option<String>,
    /// Terminal shell cwd — the ephemeral project signal, resolved at capture time.
    pub cwd: Option<String>,
    /// The source detected a private context (e.g. an incognito browser window) —
    /// record time + app only, never the title/url (D8).
    pub is_private: bool,
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

// ── Span state machine (the consumer is the SOLE writer) ─────────────────────────

/// How often the consumer wakes (with no events) to re-check idle and extend the open span —
/// the resolution of tail time and the max lost on a crash.
const TICK_SECS: i64 = 20;
/// Default input-idle gate (secs): past it with no input, the span closes. Generous enough that
/// scroll/trackpad keep ordinary reading alive. Dogfood-tunable; see D39.
const GATE_SECS: i64 = 120;
/// Longer idle gate (secs) for surfaces where you watch work happen with little input (an AI
/// agent, a long build). Bounded — a real walk-away still closes the span. Dogfood-tunable; see D39.
const PATIENT_GATE_SECS: i64 = 600;
/// Hard cap on a single span's length (secs): a span active past this is split into a fresh span
/// (same window). This makes the read-path lower-bound scan provably complete — no stored span
/// outlives `db::MAX_SPAN_LOOKBACK_SECS`, so `get_activity_logs`'s bounded scan can't miss an
/// overlapping span (D58). 12 h is far longer than any real continuous session yet well under the
/// 2-day read lookback; the split is invisible in the UI (read-time segmentation re-coalesces
/// adjacent same-window spans).
const MAX_OPEN_SPAN_SECS: i64 = 12 * 3600;
/// Process-name substrings (case-insensitive) that earn `PATIENT_GATE_SECS` — decoupled from
/// categorization (a browser "researching" is not a patient surface).
const PATIENT_APPS: &[&str] = &[
    "Cursor", "Code", "Xcode", "iTerm", "Terminal", "Warp", "Ghostty", "Zed", "Claude",
];

/// The idle gate for `app`: the longer patient gate for agent/dev surfaces, else default.
fn gate_secs_for(app: &str) -> i64 {
    let app = app.to_lowercase();
    if PATIENT_APPS.iter().any(|p| app.contains(&p.to_lowercase())) {
        PATIENT_GATE_SECS
    } else {
        GATE_SECS
    }
}

/// The resolved identity of a focused window — what we write and compare for
/// coalescing. Already privacy-sanitized (title/url blanked when private).
#[derive(Debug, Clone)]
struct Focus {
    app: String,
    title: String,
    url: Option<String>,
    site: Option<String>,
    project_id: Option<i64>,
    project_abstain_reason: Option<String>,
    is_private: bool,
}

impl Focus {
    /// Same window doing the same thing → coalesce. Includes `project_id` so a terminal `cd` to a
    /// different repo (same app + title) opens a new span under the new project (D30).
    fn same_window(&self, other: &Focus) -> bool {
        self.app == other.app
            && self.title == other.title
            && self.url == other.url
            && self.is_private == other.is_private
            && self.project_id == other.project_id
    }
}

/// The one span currently being written, held in memory by the sole writer.
struct OpenSpan {
    id: i64,
    focus: Focus,
    /// Span start (Unix secs) — used to cap span length (see `MAX_OPEN_SPAN_SECS`).
    start: i64,
    end: i64,
}

/// The consumer's mutable state. `last_focus` lets us reopen the *same* window after an
/// idle-close (macOS fires no event on return to an unchanged window); cleared after an
/// excluded app (nothing to resume).
#[derive(Default)]
struct SpanState {
    current: Option<OpenSpan>,
    last_focus: Option<Focus>,
}

/// Exclusion/enrichment outcome for one event (D8/D30).
enum Resolved {
    /// Excluded — drop entirely; close any open span, clear `last_focus`.
    Excluded,
    /// A trackable focus (possibly private/blanked).
    Track(Focus),
}

/// Frontmost apps that mean "away," not "in use" (the lock screen / screensaver). Input-idle
/// reads ~0 while locked, so the idle gate is blind — these are the reliable away signal, treated
/// like an excluded app (close the span, drop, don't resume). See D41.
const AWAY_APPS: &[&str] = &["loginwindow", "ScreenSaverEngine"];

fn is_away_app(app: &str) -> bool {
    AWAY_APPS.iter().any(|a| a.eq_ignore_ascii_case(app))
}

fn resolve_focus(conn: &Connection, ev: &FocusEvent) -> rusqlite::Result<Resolved> {
    // Locked screen / screensaver → away: close the open span and drop (see D41).
    if is_away_app(&ev.app_name) {
        return Ok(Resolved::Excluded);
    }
    let title = ev.window_title.as_deref().unwrap_or("");
    let site = ev.url.as_deref().and_then(enrich::parse_site);

    let exclusion = db::match_exclusion(conn, &ev.app_name, title, site.as_deref())?;
    if matches!(exclusion, Some(ExclusionMode::Exclude)) {
        return Ok(Resolved::Excluded);
    }
    // Private if the source flagged it (incognito) or a Private exclusion matched.
    if ev.is_private || matches!(exclusion, Some(ExclusionMode::Private)) {
        return Ok(Resolved::Track(Focus {
            app: ev.app_name.clone(),
            title: String::new(), // D8: omit title/url/site, skip project
            url: None,
            site: None,
            project_id: None,
            project_abstain_reason: None,
            is_private: true,
        }));
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
        ProjectAssignment::Abstain(reason) => (None, Some(reason.to_string())),
    };
    Ok(Resolved::Track(Focus {
        app: ev.app_name.clone(),
        title: title.to_string(),
        url: ev.url.clone(),
        site,
        project_id,
        project_abstain_reason,
        is_private: false,
    }))
}

/// Insert a fresh open span for `focus`, starting at `ts`. Category is computed here.
fn open_span(conn: &Connection, focus: &Focus, ts: i64) -> rusqlite::Result<OpenSpan> {
    let category_id = match db::find_category(conn, &focus.app, &focus.title) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("[Capture] category lookup failed: {}", e);
            None
        }
    };
    let id = db::insert_event(
        conn,
        &NewEvent {
            process_name: &focus.app,
            window_title: &focus.title,
            url: focus.url.as_deref(),
            site: focus.site.as_deref(),
            project_id: focus.project_id,
            project_abstain_reason: focus.project_abstain_reason.as_deref(),
            is_private: focus.is_private,
            is_idle: false,
            category_id,
            timestamp: ts,
        },
    )?;
    Ok(OpenSpan {
        id,
        focus: focus.clone(),
        start: ts,
        end: ts,
    })
}

/// Handle one focus change: coalesce into the open span if it's the same window, else close the
/// open span at the switch and open a new one. Excluded → close + drop.
///
/// `idle_secs` is the input-idle at arrival. Past the gate, an arriving event is background churn
/// on an unattended window (a live title reads as a *new* window since the title is part of the
/// identity), so it would spawn phantom spans — instead we close the span, remember the window for
/// `on_tick` to resume, and neither extend nor open. Real input resets idle, so activity resumes.
fn on_focus(
    conn: &Connection,
    state: &mut SpanState,
    ev: &FocusEvent,
    idle_secs: i64,
) -> rusqlite::Result<()> {
    let focus = match resolve_focus(conn, ev)? {
        Resolved::Excluded => {
            if let Some(open) = state.current.take() {
                db::set_span_end(conn, open.id, ev.timestamp)?;
            }
            state.last_focus = None;
            return Ok(());
        }
        Resolved::Track(focus) => focus,
    };

    // Gate on the app we'd keep (open span, else the last window) — patient apps get the longer
    // gate, exactly like `on_tick`. Past it, drop the event as unattended churn.
    let gate = state
        .current
        .as_ref()
        .map(|open| open.focus.app.as_str())
        .or_else(|| state.last_focus.as_ref().map(|f| f.app.as_str()))
        .map_or(GATE_SECS, gate_secs_for);
    if idle_secs >= gate {
        state.current = None;
        state.last_focus = Some(focus);
        return Ok(());
    }

    // Same window re-fire (e.g. a duplicate title event) → extend in place (no-op if time didn't
    // advance, so a burst of identical events doesn't rewrite the row).
    if let Some(open) = state.current.as_mut() {
        if open.focus.same_window(&focus) {
            if ev.timestamp > open.end {
                db::set_span_end(conn, open.id, ev.timestamp)?;
                open.end = ev.timestamp;
            }
            state.last_focus = Some(focus);
            return Ok(());
        }
    }
    // Switched: close the old span at the switch instant, open a new one.
    if let Some(open) = state.current.take() {
        db::set_span_end(conn, open.id, ev.timestamp)?;
    }
    state.current = Some(open_span(conn, &focus, ev.timestamp)?);
    state.last_focus = Some(focus);
    Ok(())
}

/// The wall-clock tick: extend the open span while active, or close it when idle (away).
/// On a return from idle to the *same* window (no focus event) reopen it (decision A).
/// `idle_secs` is passed in so this is deterministically testable.
fn on_tick(
    conn: &Connection,
    state: &mut SpanState,
    now: i64,
    idle_secs: i64,
) -> rusqlite::Result<()> {
    // The gate depends on the app we'd extend or resume — agent/dev surfaces get the
    // longer patient gate so hands-off watching (an agent working) isn't dropped.
    let gate = state
        .current
        .as_ref()
        .map(|open| open.focus.app.as_str())
        .or_else(|| state.last_focus.as_ref().map(|f| f.app.as_str()))
        .map_or(GATE_SECS, gate_secs_for);
    if idle_secs >= gate {
        // Away: close the open span (leave its end at the last tick) but remember the
        // focus so we can resume the same window when the user comes back.
        state.current = None;
        return Ok(());
    }
    if let Some(open) = state.current.as_mut() {
        if now > open.end {
            db::set_span_end(conn, open.id, now)?;
            open.end = now;
            // Cap span length: split a marathon span into a fresh one at `now` so no stored span
            // outlives the read-path lookback (see MAX_OPEN_SPAN_SECS). The two pieces are
            // contiguous and the same window, so read-time segmentation shows one continuous run.
            if now - open.start >= MAX_OPEN_SPAN_SECS {
                let focus = open.focus.clone();
                state.current = Some(open_span(conn, &focus, now)?);
            }
        }
    } else if let Some(focus) = state.last_focus.clone() {
        // Active again, same window, no focus event from macOS → resume from now.
        state.current = Some(open_span(conn, &focus, now)?);
    }
    Ok(())
}

fn current_idle_secs() -> i64 {
    user_idle::UserIdle::get_time()
        .map(|t| t.as_seconds() as i64)
        .unwrap_or(0)
}

// ── Consumer (the sole DB writer; SQLite + git shell are blocking, so own thread) ─

/// Drain focus events and self-tick into the repository until the channel closes. The idle gate
/// is a wall-clock deadline checked on every wake (not the bare `recv_timeout` firing), so a
/// chatty event stream while you're away can't starve the gate and balloon the span.
pub fn consume(db_conn: DbConnection, rx: Receiver<FocusEvent>) {
    println!(
        "[Capture] consumer up (tick {}s, idle gate {}s)",
        TICK_SECS, GATE_SECS
    );
    let mut state = SpanState::default();
    let mut next_tick = db::now_unix() + TICK_SECS;

    loop {
        let wait = (next_tick - db::now_unix()).max(0) as u64;
        match rx.recv_timeout(Duration::from_secs(wait)) {
            Ok(ev) => {
                // Idle at event arrival, so a same-window churn while away can't extend the span.
                let idle = current_idle_secs();
                match db_conn.lock() {
                    Ok(conn) => {
                        if let Err(e) = on_focus(&conn, &mut state, &ev, idle) {
                            eprintln!("[Capture] focus write failed: {}", e);
                        }
                    }
                    Err(e) => eprintln!("[Capture] db lock poisoned: {}", e),
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = db::now_unix();
        if now >= next_tick {
            let idle = current_idle_secs();
            match db_conn.lock() {
                Ok(conn) => {
                    if let Err(e) = on_tick(&conn, &mut state, now, idle) {
                        eprintln!("[Capture] tick write failed: {}", e);
                    }
                }
                Err(e) => eprintln!("[Capture] db lock poisoned: {}", e),
            }
            next_tick = now + TICK_SECS;
        }
    }
    println!("[Capture] channel closed; consumer stopped");
}

/// Dev-only write-path probe for the Phase-6 churn stress test (behind the `perf` feature, never
/// shipped). Wraps the private span-state so a harness can drive the *real* `on_focus` write path
/// (resolve → privacy/exclusion → project inference → category lookup → insert/extend span)
/// without exposing the state machine. The producer→channel→consumer plumbing and idle gate are
/// out of scope here — this measures the per-event write cost the consumer pays under the lock.
#[cfg(feature = "perf")]
#[derive(Default)]
pub struct WriteProbe {
    state: SpanState,
}

#[cfg(feature = "perf")]
impl WriteProbe {
    /// Drive one synthetic focus event through the write path with `idle_secs = 0` (always
    /// active, so the idle gate never drops it — steady-state throughput, not gating behaviour).
    pub fn feed(
        &mut self,
        conn: &Connection,
        app: &str,
        title: &str,
        ts: i64,
    ) -> rusqlite::Result<()> {
        let ev = FocusEvent {
            app_name: app.to_string(),
            window_title: Some(title.to_string()),
            timestamp: ts,
            ..Default::default()
        };
        on_focus(conn, &mut self.state, &ev, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActivityLog;

    /// In-memory DB on the real migration chain.
    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::migrations::run_migrations(&mut conn).expect("migrations");
        conn
    }

    fn focus(app: &str, title: Option<&str>, ts: i64) -> FocusEvent {
        FocusEvent {
            app_name: app.to_string(),
            window_title: title.map(|s| s.to_string()),
            timestamp: ts,
            ..Default::default()
        }
    }

    fn all_logs(conn: &Connection) -> Vec<ActivityLog> {
        db::get_activity_logs(conn, 0, i64::MAX).unwrap()
    }

    /// Drive a focus event through the state machine (active: idle = 0).
    fn feed(conn: &Connection, state: &mut SpanState, ev: &FocusEvent) {
        on_focus(conn, state, ev, 0).unwrap();
    }

    #[test]
    fn first_event_opens_a_span() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[0].window_title, "main.rs");
        assert!(!logs[0].is_private);
    }

    #[test]
    fn switching_closes_previous_and_opens_new() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        feed(&conn, &mut st, &focus("Slack", Some("general"), 1090));
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[0].start_time, 1000);
        assert_eq!(logs[0].end_time, 1090, "previous span closed at the switch");
        assert_eq!(logs[1].process_name, "Slack");
        assert_eq!(logs[1].start_time, 1090);
    }

    #[test]
    fn same_window_event_extends_in_place() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1005));
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1010));
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1, "same window coalesces");
        assert_eq!(logs[0].end_time, 1010);
    }

    #[test]
    fn excluded_app_drops_and_closes_previous() {
        let conn = test_db();
        db::create_exclusion(&conn, "app", "1Password", "exclude").unwrap();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        feed(&conn, &mut st, &focus("1Password", Some("Vault"), 1050));
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1, "excluded event is never written");
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[0].end_time, 1050, "previous span closed at the switch");
        assert!(st.current.is_none() && st.last_focus.is_none());
    }

    #[test]
    fn away_app_closes_span_and_is_never_tracked() {
        // Locking the screen makes `loginwindow` frontmost; it must not accrue (the idle gate
        // is blind while locked — D41) and must not be resumed afterwards.
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        feed(&conn, &mut st, &focus("loginwindow", None, 1100)); // screen locked
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1, "the lock screen is never recorded");
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(
            logs[0].end_time, 1100,
            "the previous span closes at lock time"
        );
        assert!(
            st.current.is_none() && st.last_focus.is_none(),
            "away clears resume state — post-unlock work starts fresh"
        );
    }

    #[test]
    fn private_records_time_without_title_or_url() {
        let conn = test_db();
        db::create_exclusion(&conn, "app", "Banking", "private").unwrap();
        let mut st = SpanState::default();
        feed(
            &conn,
            &mut st,
            &focus("Banking App", Some("Acct 1234"), 1000),
        );
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].is_private);
        assert_eq!(logs[0].window_title, "", "title omitted for private (D8)");
        assert_eq!(logs[0].process_name, "Banking App");
    }

    #[test]
    fn browser_event_sets_site_and_project() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(
            &conn,
            &mut st,
            &FocusEvent {
                app_name: "Google Chrome".to_string(),
                window_title: Some("usenudgeai/nudge".to_string()),
                url: Some("https://github.com/usenudgeai/nudge".to_string()),
                timestamp: 1000,
                ..Default::default()
            },
        );
        let logs = all_logs(&conn);
        assert_eq!(logs[0].site.as_deref(), Some("github.com"));
        assert!(logs[0].project_id.is_some());
        assert_eq!(logs[0].project_abstain_reason, None);
    }

    #[test]
    fn ambiguous_url_persists_abstain_reason() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(
            &conn,
            &mut st,
            &FocusEvent {
                app_name: "Google Chrome".to_string(),
                window_title: Some("Grafana".to_string()),
                url: Some("https://acme.grafana.net/d/x".to_string()),
                timestamp: 1000,
                ..Default::default()
            },
        );
        let logs = all_logs(&conn);
        assert_eq!(logs[0].project_id, None);
        assert_eq!(logs[0].project_abstain_reason.as_deref(), Some("ambiguous"));
    }

    #[test]
    fn incognito_flag_blanks_title_and_url() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(
            &conn,
            &mut st,
            &FocusEvent {
                app_name: "Google Chrome".to_string(),
                window_title: Some("Secret Page".to_string()),
                url: Some("https://secret.example/x".to_string()),
                is_private: true,
                timestamp: 1000,
                ..Default::default()
            },
        );
        let logs = all_logs(&conn);
        assert!(logs[0].is_private);
        assert_eq!(logs[0].window_title, "");
        assert_eq!(logs[0].url, None);
        assert_eq!(logs[0].project_id, None);
    }

    #[test]
    fn title_churn_while_idle_does_not_inflate_or_spawn_phantoms() {
        // The user walks away; the focused window's title keeps changing (a build counter, a
        // "(3)" badge). Each churn reads as a new window (title is part of identity), so without
        // the on_focus idle gate it would spawn a chain of phantom spans and inflate the day.
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Slack", Some("general"), 1000)); // opens at 1000, active
        on_focus(
            &conn,
            &mut st,
            &focus("Slack", Some("(1) general"), 1200),
            200,
        )
        .unwrap();
        on_focus(
            &conn,
            &mut st,
            &focus("Slack", Some("(2) general"), 1400),
            400,
        )
        .unwrap();
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 1, "churn while away spawns no phantom spans");
        assert_eq!(logs[0].end_time, 1000, "the span did not accrue idle churn");
        assert!(st.current.is_none(), "the span is closed while away");
    }

    #[test]
    fn same_window_distinguishes_project() {
        // A `cd` to a different repo (same app + title, different inferred project) is NOT the
        // same window, so it opens a fresh span under the new project rather than extending (D30).
        let base = Focus {
            app: "iTerm".into(),
            title: "zsh".into(),
            url: None,
            site: None,
            project_id: Some(1),
            project_abstain_reason: None,
            is_private: false,
        };
        let mut other = base.clone();
        assert!(base.same_window(&other));
        other.project_id = Some(2);
        assert!(
            !base.same_window(&other),
            "a project change is a new window"
        );
    }

    #[test]
    fn tick_extends_open_span_while_active() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        on_tick(&conn, &mut st, 1020, 5).unwrap(); // active
        assert_eq!(all_logs(&conn)[0].end_time, 1020, "sustained work accrues");
    }

    #[test]
    fn tick_caps_marathon_span_into_contiguous_pieces() {
        // A span active past MAX_OPEN_SPAN_SECS is split so no stored span outlives the read-path
        // lookback (the pieces are the same window, so read-time segmentation shows one run).
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        on_tick(&conn, &mut st, 1000 + MAX_OPEN_SPAN_SECS - 20, 5).unwrap(); // within the cap
        assert_eq!(all_logs(&conn).len(), 1, "within the cap → one span");

        let split = 1000 + MAX_OPEN_SPAN_SECS;
        on_tick(&conn, &mut st, split, 5).unwrap(); // crosses the cap
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2, "past the cap → the span splits");
        assert_eq!(logs[0].end_time, split, "first piece closes at the split");
        assert_eq!(logs[1].start_time, split, "second piece is contiguous");
        assert_eq!(logs[0].process_name, logs[1].process_name, "same window");
    }

    #[test]
    fn tick_never_extends_backward() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        on_tick(&conn, &mut st, 990, 5).unwrap(); // clock skewed backward
        assert_eq!(all_logs(&conn)[0].end_time, 1000);
    }

    #[test]
    fn idle_closes_span_without_inflating_it() {
        let conn = test_db();
        let mut st = SpanState::default();
        // Slack is not a patient app — the default 120s gate applies.
        feed(&conn, &mut st, &focus("Slack", Some("general"), 1000));
        on_tick(&conn, &mut st, 1000 + GATE_SECS, GATE_SECS).unwrap(); // away
        assert!(st.current.is_none(), "the open span is closed when away");
        assert_eq!(
            all_logs(&conn)[0].end_time,
            1000,
            "the idle gap stays untracked"
        );
    }

    #[test]
    fn returns_to_same_window_after_idle_reopen_a_span() {
        // read → walk away → come back to the SAME window (macOS fires no event).
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Slack", Some("general"), 1000));
        on_tick(&conn, &mut st, 1200, GATE_SECS).unwrap(); // away → close
        on_tick(&conn, &mut st, 1230, 3).unwrap(); // active again, same window
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2, "a fresh span resumes the same window");
        assert_eq!(logs[1].process_name, "Slack");
        assert_eq!(
            logs[1].start_time, 1230,
            "resumes at now, not across the gap"
        );
    }

    #[test]
    fn agent_app_keeps_accruing_past_the_default_gate() {
        // Watching an agent work in a patient app: 5 min hands-off is under the 600s
        // gate, so the span keeps extending instead of being dropped at 120s.
        let conn = test_db();
        let mut st = SpanState::default();
        feed(
            &conn,
            &mut st,
            &focus("Claude", Some("usage-os — working"), 1000),
        );
        on_tick(&conn, &mut st, 1300, 300).unwrap(); // 5 min idle, still watching
        assert!(
            st.current.is_some(),
            "a patient app survives past the default gate"
        );
        assert_eq!(
            all_logs(&conn)[0].end_time,
            1300,
            "supervision time accrues"
        );
    }

    #[test]
    fn agent_app_still_closes_at_the_patient_gate() {
        // The patient gate is bounded: a true walk-away (>10 min) still closes the span,
        // so there is no phantom focus past the cap (honest-mirror trade-off).
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Claude", Some("usage-os"), 1000));
        on_tick(&conn, &mut st, 1000 + PATIENT_GATE_SECS, PATIENT_GATE_SECS).unwrap();
        assert!(
            st.current.is_none(),
            "the patient gate still closes on a walk-away"
        );
        assert_eq!(
            all_logs(&conn)[0].end_time,
            1000,
            "no phantom time past the cap"
        );
    }

    #[test]
    fn gate_is_per_app() {
        assert_eq!(gate_secs_for("Cursor"), PATIENT_GATE_SECS);
        assert_eq!(
            gate_secs_for("iTerm2"),
            PATIENT_GATE_SECS,
            "substring match"
        );
        assert_eq!(gate_secs_for("Claude"), PATIENT_GATE_SECS);
        assert_eq!(gate_secs_for("Slack"), GATE_SECS);
        assert_eq!(
            gate_secs_for("Google Chrome"),
            GATE_SECS,
            "research surfaces are not patient"
        );
    }

    #[test]
    fn fake_source_feeds_the_spine() {
        // The whole capture write path, exercised without a Mac (hard rule 5).
        let conn = test_db();
        let mut st = SpanState::default();
        let events = vec![
            focus("Code", Some("main.rs"), 1000),
            focus("Slack", Some("general"), 1100),
        ];
        let (tx, rx) = std::sync::mpsc::channel::<FocusEvent>();
        Box::new(FakeCapture::new(events)).start(tx);
        for ev in rx {
            feed(&conn, &mut st, &ev);
        }
        let logs = all_logs(&conn);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].process_name, "Code");
        assert_eq!(logs[1].process_name, "Slack");
    }
}
