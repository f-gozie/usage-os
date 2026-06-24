//! Capture: the boundary that observes the active app/window (hard rule 5, D22).
//!
//! [`CaptureSource`] produces [`FocusEvent`]s; [`consume`] is the single writer — a
//! state machine that owns the one open span in memory, applies D8 sensitive handling
//! and D30 project inference, and self-ticks (via `recv_timeout`) to extend the open
//! span and detect idle. All platform/native code lives behind this trait — nothing
//! above `capture/` imports objc2. Tests use [`FakeCapture`]; production uses the
//! event-driven [`macos::MacosCapture`] on macOS, else [`PollingCapture`].

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

/// A focus change marshaled from the capture side to the consumer thread. All
/// fields are owned/`Send` (D29). The signals are filled by whichever source is
/// active: polling sets app only; the macOS impl adds title, url, and cwd. Idle is
/// NOT a per-event signal — the consumer is the single source of idle truth (D39),
/// reading input-idle on each tick against the per-app gate.
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

/// How often the consumer wakes (when no events arrive) to re-check idle and extend
/// the open span — the resolution of tail time and the max lost on a crash.
const TICK_SECS: i64 = 20;
/// Default input-idle gate: at/over this many seconds with no input the open span is
/// closed and time stops accruing. Also the everyday reading allowance — scroll/trackpad
/// count as input, so genuine reading rarely trips it.
const GATE_SECS: i64 = 120;
/// A longer gate for apps where attentively watching with little input is the norm —
/// supervising an AI coding agent, a long build/test run. Bounded, never disabled: a
/// genuine walk-away still closes the span once idle crosses it, capping the over-count
/// (the honest-mirror trade-off). D34a dogfood-tunable starting value (D39, set by the
/// Codex+Opus debate).
const PATIENT_GATE_SECS: i64 = 600;
/// Process-name substrings (case-insensitive) that earn `PATIENT_GATE_SECS`: the
/// editor / terminal / agent surfaces where you watch work happen without touching the
/// keys. Mirrors the "deep work" starter rules (migration 0003) but is deliberately
/// decoupled from categorization — a browser doing "research" is not a patient surface.
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
    /// Same window doing the same thing → coalesce rather than open a new span.
    fn same_window(&self, other: &Focus) -> bool {
        self.app == other.app
            && self.title == other.title
            && self.url == other.url
            && self.is_private == other.is_private
    }
}

/// The one span currently being written, held in memory by the sole writer.
struct OpenSpan {
    id: i64,
    focus: Focus,
    end: i64,
}

/// The consumer's entire mutable state. `last_focus` lets us reopen the *same* window
/// after an idle-close (macOS emits no event when you return to an unchanged window —
/// debate decision A); it's cleared after an excluded app (nothing to resume).
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

/// Frontmost apps that mean "the user is away," not "an app in use": macOS swaps these in when
/// the screen locks / the screensaver runs. Input-idle reads ~0 while locked, so the idle gate
/// can't catch it (D41) — this is the reliable away signal, so we treat focus on them like an
/// excluded app: close the open span, drop, and don't resume.
const AWAY_APPS: &[&str] = &["loginwindow", "ScreenSaverEngine"];

fn is_away_app(app: &str) -> bool {
    AWAY_APPS.iter().any(|a| a.eq_ignore_ascii_case(app))
}

fn resolve_focus(conn: &Connection, ev: &FocusEvent) -> rusqlite::Result<Resolved> {
    // Locked screen / screensaver → away: close the open span and drop. The idle gate is blind
    // while locked (input-idle reads ~0), so the frontmost away-app is the signal we trust (D41).
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
        end: ts,
    })
}

/// Handle one focus change: coalesce into the open span if it's the same window, else
/// close the open span at the switch and open a new one. Excluded → close + drop.
fn on_focus(conn: &Connection, state: &mut SpanState, ev: &FocusEvent) -> rusqlite::Result<()> {
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

    // Same window re-fire (e.g. a duplicate title event) → just extend in place.
    if let Some(open) = state.current.as_mut() {
        if open.focus.same_window(&focus) {
            db::set_span_end(conn, open.id, ev.timestamp)?;
            open.end = ev.timestamp;
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

/// Drain focus events and self-tick into the repository until the channel closes. The
/// idle gate is driven by a **wall-clock deadline checked on every wake** (not by the
/// bare `recv_timeout` firing) — otherwise a chatty event stream (a live-updating tab
/// title) while you're away would starve the gate and balloon the span.
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
            Ok(ev) => match db_conn.lock() {
                Ok(conn) => {
                    if let Err(e) = on_focus(&conn, &mut state, &ev) {
                        eprintln!("[Capture] focus write failed: {}", e);
                    }
                }
                Err(e) => eprintln!("[Capture] db lock poisoned: {}", e),
            },
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
            pid: 1,
            window_title: title.map(|s| s.to_string()),
            timestamp: ts,
            ..Default::default()
        }
    }

    fn all_logs(conn: &Connection) -> Vec<ActivityLog> {
        db::get_activity_logs(conn, 0, i64::MAX).unwrap()
    }

    /// Drive a focus event through the state machine.
    fn feed(conn: &Connection, state: &mut SpanState, ev: &FocusEvent) {
        on_focus(conn, state, ev).unwrap();
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
    fn tick_extends_open_span_while_active() {
        let conn = test_db();
        let mut st = SpanState::default();
        feed(&conn, &mut st, &focus("Code", Some("main.rs"), 1000));
        on_tick(&conn, &mut st, 1020, 5).unwrap(); // active
        assert_eq!(all_logs(&conn)[0].end_time, 1020, "sustained work accrues");
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
