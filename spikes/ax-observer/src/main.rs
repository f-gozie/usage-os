//! Spike ② — the event-driven capture model.
//!
//! Spike #1 proved AX returns real focused-window titles with Accessibility
//! alone. But it *polled* on a timer and pumped the run loop as a stopgap. This
//! spike builds the REAL architecture the Phase-1 `capture` trait impl will use,
//! and answers the open threading/observer questions from the feasibility audit:
//!
//!   R6  — a working threading model: drive AX + NSWorkspace on a run loop and
//!         marshal results to the async (Tokio) side without blocking it.
//!   R8  — observe `NSWorkspaceDidActivateApplicationNotification`; the callback
//!         fires on every app switch and yields the new app's pid/name/bundle.
//!   R9  — that observer is a Rust closure wrapped in `block2::RcBlock`; the
//!         returned token is retained for the observer's life, no UAF on switch.
//!   R10 — a per-PID `AXObserver` fires `AXFocusedWindowChanged` / `AXTitleChanged`;
//!         it is rebuilt when the frontmost app (PID) changes, and the title
//!         observer is re-pointed at the new focused window within an app.
//!   R11 — event-driven, no polling: idle ≈ zero wakeups (measure with
//!         `powermetrics`); chatty title storms are debounced (dedupe).
//!   R13 — it all compiles AND runs on aarch64-apple-darwin (Apple Silicon).
//!
//! Shape:
//!   * The MAIN thread owns the run loop (`CFRunLoop::run()`). In the real app
//!     this maps onto Tauri's main-thread NSApplication run loop — we register
//!     our sources/observers during `setup` instead of calling `run()` ourselves.
//!   * A separate thread owns a real Tokio runtime — "the async side". Capture
//!     callbacks `send` `CaptureEvent`s to it over a `Send` channel; the send is
//!     non-blocking, so the run loop never stalls and the executor is never
//!     blocked (R6).
//!   * No network, no CGWindowList / Screen Recording. Accessibility only.
//!
//! Hard rules honored: no `unwrap()` / `expect()` / `panic!` in our logic — every
//! AX/observer outcome is a value we classify or a branch we handle.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use block2::RcBlock;
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
use objc2_application_services::{AXError, AXObserver, AXUIElement};
use objc2_core_foundation::{
    kCFBooleanTrue, kCFRunLoopCommonModes, CFBoolean, CFDictionary, CFRetained, CFRunLoop,
    CFRunLoopSource, CFString, CFType,
};
use objc2_foundation::NSNotification;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// ── Constants ────────────────────────────────────────────────────────────────

/// Trust-polling cadence while the user is in System Settings.
const TICK: Duration = Duration::from_millis(1200);

// AX attribute names (the kAX* constants are NOT re-exported — build CFStrings).
const ATTR_FOCUSED_WINDOW: &str = "AXFocusedWindow";
const ATTR_TITLE: &str = "AXTitle";

// AX notification names (likewise not re-exported).
const NOTIF_FOCUSED_WINDOW_CHANGED: &str = "AXFocusedWindowChanged";
const NOTIF_TITLE_CHANGED: &str = "AXTitleChanged";
const NOTIF_MAIN_WINDOW_CHANGED: &str = "AXMainWindowChanged";
const NOTIF_FOCUSED_UI_CHANGED: &str = "AXFocusedUIElementChanged";

/// Notifications registered on the *application* element. Window-switch signals;
/// `AXTitleChanged` is registered separately on the focused *window* (see
/// [`reregister_title_observer`]).
const APP_NOTIFICATIONS: &[&str] = &[
    NOTIF_FOCUSED_WINDOW_CHANGED,
    NOTIF_MAIN_WINDOW_CHANGED,
    NOTIF_FOCUSED_UI_CHANGED,
];

static SEQ: AtomicU64 = AtomicU64::new(1);
fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::Relaxed)
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    println!("ax-observer spike ② — event-driven capture (NSWorkspace + AXObserver on a run loop)");
    println!("Accessibility only, no Screen Recording. Ctrl-C to stop.\n");

    // Step 1: be AX-trusted (reuses Spike #1's flow).
    ensure_trusted();

    // The Send channel between the capture side (this/main thread) and the
    // async side (the consumer thread's Tokio runtime).
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<CaptureEvent>();

    // Spawn "the async side". In the real app this is the existing Tokio runtime
    // under Tauri; here it's a dedicated runtime on its own thread.
    let consumer = thread::spawn(move || run_consumer(rx));

    // The capture side owns the run loop. In Tauri this is the main NSApplication
    // run loop and we'd register into it during setup rather than call run().
    let Some(run_loop) = CFRunLoop::current() else {
        eprintln!("fatal: could not obtain the current CFRunLoop");
        return;
    };

    let state = Rc::new(RefCell::new(CaptureState {
        current: None,
        tx,
        run_loop: run_loop.clone(),
    }));

    // Install the observer for whoever is frontmost right now (before any switch).
    if let Some(app) = frontmost() {
        state.borrow_mut().switch_to(app);
    }

    // R8/R9: observe app-activation with a block-backed observer. Keep both the
    // block and the returned token alive for the whole run (they live in `main`,
    // and run() never returns, so they persist for the process lifetime).
    let activation_block = {
        let state = Rc::clone(&state);
        RcBlock::new(move |notif: NonNull<NSNotification>| {
            state.borrow_mut().on_activation(notif);
        })
    };
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    // SAFETY: valid notification name + block; queue=nil delivers on this (the
    // posting/main) thread, where all our AX state lives, so the non-Send block
    // is sound. The returned token is retained below for the observer's life.
    let _activation_token = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceDidActivateApplicationNotification),
            None,
            None,
            &activation_block,
        )
    };

    println!(
        "pid={} — capturing. Switch apps and change window titles to exercise it.",
        std::process::id()
    );
    println!(
        "Idle wakeups: in another terminal run\n  \
         sudo powermetrics --samplers tasks -i 1000 -n 5 | grep -i ax-observer\n\
         while NOT touching the machine — an event-driven capture should sit near 0 wakeups/s.\n"
    );

    // R6/R11: block on the run loop. Callbacks fire here on the main thread and
    // marshal events to the async side. When idle, the loop sleeps (no polling).
    CFRunLoop::run();

    // Not reached in normal use (run() blocks until the process exits); kept so
    // the keep-alive handles have an explicit, readable end of scope.
    drop(_activation_token);
    drop(activation_block);
    drop(state);
    let _ = consumer.join();
}

// ── The async side (consumer) ────────────────────────────────────────────────

/// Build a real Tokio runtime on this thread and drain the capture channel.
fn run_consumer(rx: UnboundedReceiver<CaptureEvent>) {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("fatal: failed to build Tokio runtime: {e}");
            return;
        }
    };
    rt.block_on(consume(rx));
}

/// The async consumer: purely reactive — it `await`s the channel and never
/// busy-loops, so the whole process is idle when nothing is happening.
async fn consume(mut rx: UnboundedReceiver<CaptureEvent>) {
    println!("▸ tokio consumer up — capture callbacks marshal here over a Send channel.\n");
    let mut count: u64 = 0;
    while let Some(ev) = rx.recv().await {
        count += 1;
        println!("{}", ev.format());
    }
    println!("\n▸ channel closed after {count} events.");
}

// ── Capture state (lives on the run-loop thread only) ────────────────────────

/// Single-threaded capture state, owned via `Rc<RefCell<…>>` and mutated only
/// from the run-loop thread (the activation block and AX callbacks).
struct CaptureState {
    current: Option<AppObserver>,
    tx: UnboundedSender<CaptureEvent>,
    run_loop: CFRetained<CFRunLoop>,
}

impl CaptureState {
    /// Block callback for `NSWorkspaceDidActivateApplicationNotification`.
    fn on_activation(&mut self, _notif: NonNull<NSNotification>) {
        // The activation we're handling IS the now-frontmost app. (Reading
        // NSWorkspaceApplicationKey from the notification's userInfo would be a
        // hair more precise under rapid switching — noted in the README.)
        if let Some(app) = frontmost() {
            self.switch_to(app);
        }
    }

    /// Tear down the previous app's observer and stand up a fresh one for `app`.
    fn switch_to(&mut self, app: FrontApp) {
        if let Some(cur) = &self.current {
            if cur.ctx.pid == app.pid {
                return; // re-activation of the same app — nothing to rebuild.
            }
        }

        // Drop the old observer first: its Drop removes the run-loop source, so
        // no stale callback can fire against the freed context (R9 — no UAF).
        self.current = None;

        match install_observer(&app, self.tx.clone(), &self.run_loop) {
            Some(obs) => {
                // Seed the dedupe key and emit the initial state for the new app.
                let title = focused_window_title(&obs.ctx.app_element);
                obs.ctx.last_title_key.replace(Some(title.dedupe_key()));
                let _ = self.tx.send(CaptureEvent {
                    seq: next_seq(),
                    kind: EventKind::Activated,
                    app_name: app.name.clone(),
                    bundle_id: app.bundle_id.clone(),
                    pid: app.pid,
                    notif: "NSWorkspaceDidActivateApplication".to_string(),
                    title,
                    coalesced: 0,
                });
                self.current = Some(obs);
            }
            None => {
                let _ = self.tx.send(CaptureEvent {
                    seq: next_seq(),
                    kind: EventKind::ObserverFailed,
                    app_name: app.name.clone(),
                    bundle_id: app.bundle_id.clone(),
                    pid: app.pid,
                    notif: "AXObserverCreate".to_string(),
                    title: TitleResult::Nil,
                    coalesced: 0,
                });
            }
        }
    }
}

// ── Per-application AXObserver ───────────────────────────────────────────────

/// One application's observer: the `AXObserver`, its run-loop source, and the
/// boxed callback context whose stable address is the AX `refcon`.
struct AppObserver {
    // Field order matters only for tidiness; the Drop impl below removes the
    // source up-front so no callback can fire mid-teardown regardless of order.
    observer: CFRetained<AXObserver>,
    source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
    ctx: Box<CallbackContext>,
}

impl Drop for AppObserver {
    fn drop(&mut self) {
        // Remove the source FIRST (we're on the run-loop thread; this is
        // synchronous) so the boxed `ctx` — the live refcon target — can't be
        // freed out from under a pending callback.
        self.run_loop
            .remove_source(Some(&self.source), unsafe { kCFRunLoopCommonModes });
        let _ = &self.observer; // released on field drop; AX also auto-detaches.
    }
}

/// Data the AX C callback reaches through `refcon`. Mutated only on the
/// run-loop thread, so interior mutability is single-threaded and sound.
struct CallbackContext {
    tx: UnboundedSender<CaptureEvent>,
    pid: i32,
    app_name: String,
    bundle_id: String,
    app_element: CFRetained<AXUIElement>,
    /// The window we currently have an `AXTitleChanged` registration on.
    focused_window: RefCell<Option<CFRetained<AXUIElement>>>,
    /// Last emitted title key, for dedupe debounce.
    last_title_key: RefCell<Option<String>>,
    /// Count of chatty duplicates suppressed since the last emit.
    coalesced: Cell<u32>,
}

/// Create an `AXObserver` for `app`, register the window/title notifications,
/// and attach its run-loop source. `None` if the observer can't be created
/// (e.g. the app exited between activation and here).
fn install_observer(
    app: &FrontApp,
    tx: UnboundedSender<CaptureEvent>,
    run_loop: &CFRetained<CFRunLoop>,
) -> Option<AppObserver> {
    // SAFETY: plain FFI; pid comes from a live NSRunningApplication.
    let app_element = unsafe { AXUIElement::new_application(app.pid) };

    // Create the observer bound to our C callback.
    let mut raw: *mut AXObserver = ptr::null_mut();
    let out: NonNull<*mut AXObserver> = NonNull::from(&mut raw);
    // SAFETY: `observer_callback` matches AXObserverCallback; `out` is live.
    let err = unsafe { AXObserver::create(app.pid, Some(observer_callback), out) };
    if err != AXError::Success {
        return None;
    }
    let observer_ptr = NonNull::new(raw)?;
    // SAFETY: AXObserverCreate follows the Create rule (+1 retained) — take it.
    let observer: CFRetained<AXObserver> = unsafe { CFRetained::from_raw(observer_ptr) };

    // The boxed context: its heap address is stable across the Box move into
    // AppObserver, so it is a valid long-lived `refcon`.
    let ctx = Box::new(CallbackContext {
        tx,
        pid: app.pid,
        app_name: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        app_element: app_element.clone(),
        focused_window: RefCell::new(None),
        last_title_key: RefCell::new(None),
        coalesced: Cell::new(0),
    });
    let refcon = ptr::addr_of!(*ctx) as *mut c_void;

    // App-level notifications (window switches).
    for name in APP_NOTIFICATIONS {
        let cf = CFString::from_str(name);
        // SAFETY: valid observer/element/name; refcon outlives the observer.
        let _ = unsafe { observer.add_notification(&app_element, &cf, refcon) };
    }

    // Title-change notification on the *current* focused window.
    if let Some(win) = copy_focused_window(&app_element) {
        let cf = CFString::from_str(NOTIF_TITLE_CHANGED);
        // SAFETY: as above; `win` is a valid window element.
        let _ = unsafe { observer.add_notification(&win, &cf, refcon) };
        *ctx.focused_window.borrow_mut() = Some(win);
    }

    // Attach to the run loop in common modes so events still arrive during
    // tracking/modal loops.
    // SAFETY: framework run-loop-source getter.
    let source = unsafe { observer.run_loop_source() };
    run_loop.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });

    Some(AppObserver {
        observer,
        source,
        run_loop: run_loop.clone(),
        ctx,
    })
}

/// Move the `AXTitleChanged` registration from the previous focused window to
/// the current one. Called when a window-switch notification fires.
fn reregister_title_observer(observer: &AXObserver, ctx: &CallbackContext, refcon: *mut c_void) {
    let title_name = CFString::from_str(NOTIF_TITLE_CHANGED);

    if let Some(old) = ctx.focused_window.borrow_mut().take() {
        // SAFETY: `old` was a window we registered; removing is always safe.
        let _ = unsafe { observer.remove_notification(&old, &title_name) };
    }

    if let Some(win) = copy_focused_window(&ctx.app_element) {
        // SAFETY: valid observer/window/name; refcon outlives the observer.
        let _ = unsafe { observer.add_notification(&win, &title_name, refcon) };
        *ctx.focused_window.borrow_mut() = Some(win);
    }
}

/// The AX observer C callback. Runs on the run-loop (main) thread.
///
/// # Safety
/// `refcon` must point at a live `CallbackContext` (it does — the boxed context
/// in the `AppObserver` that owns this observer), and the AX pointers are valid
/// for the duration of the call per the AX contract.
unsafe extern "C-unwind" fn observer_callback(
    observer: NonNull<AXObserver>,
    _element: NonNull<AXUIElement>,
    notification: NonNull<CFString>,
    refcon: *mut c_void,
) {
    if refcon.is_null() {
        return;
    }
    // SAFETY: refcon is the address of a live, boxed CallbackContext, and we
    // only ever take a shared ref + use interior mutability (no &mut aliasing).
    let ctx: &CallbackContext = unsafe { &*(refcon as *const CallbackContext) };
    // SAFETY: AX hands us a valid notification-name CFString for this call.
    let name = unsafe { notification.as_ref() }.to_string();

    // On a window switch, re-point the title observer at the new window.
    if name == NOTIF_FOCUSED_WINDOW_CHANGED || name == NOTIF_MAIN_WINDOW_CHANGED {
        // SAFETY: AX hands us a valid observer ref for this call.
        let obs = unsafe { observer.as_ref() };
        reregister_title_observer(obs, ctx, refcon);
    }

    emit_title(ctx, &name);
}

/// Read the current focused-window title and, unless it's a chatty duplicate,
/// emit a `CaptureEvent`. Debounce = drop identical consecutive titles (the
/// dominant `AXTitleChanged` storm), counting them as `coalesced`.
fn emit_title(ctx: &CallbackContext, notif_name: &str) {
    let title = focused_window_title(&ctx.app_element);
    let key = title.dedupe_key();

    {
        let mut last = ctx.last_title_key.borrow_mut();
        if last.as_deref() == Some(key.as_str()) {
            ctx.coalesced.set(ctx.coalesced.get() + 1);
            return;
        }
        *last = Some(key);
    }

    let coalesced = ctx.coalesced.replace(0);
    let _ = ctx.tx.send(CaptureEvent {
        seq: next_seq(),
        kind: kind_for(notif_name),
        app_name: ctx.app_name.clone(),
        bundle_id: ctx.bundle_id.clone(),
        pid: ctx.pid,
        notif: notif_name.to_string(),
        title,
        coalesced,
    });
}

// ── Events (cross the thread boundary) ───────────────────────────────────────

#[derive(Clone, Copy)]
enum EventKind {
    Activated,
    FocusedWindowChanged,
    TitleChanged,
    Other,
    ObserverFailed,
}

impl EventKind {
    fn label(self) -> &'static str {
        match self {
            EventKind::Activated => "ACTIVATED",
            EventKind::FocusedWindowChanged => "FOCUS-WIN",
            EventKind::TitleChanged => "TITLE",
            EventKind::Other => "OTHER",
            EventKind::ObserverFailed => "OBS-FAIL",
        }
    }
}

fn kind_for(notif: &str) -> EventKind {
    match notif {
        NOTIF_TITLE_CHANGED => EventKind::TitleChanged,
        NOTIF_FOCUSED_WINDOW_CHANGED | NOTIF_MAIN_WINDOW_CHANGED | NOTIF_FOCUSED_UI_CHANGED => {
            EventKind::FocusedWindowChanged
        }
        _ => EventKind::Other,
    }
}

/// A capture event marshaled to the async side. All fields are owned/`Send`.
struct CaptureEvent {
    seq: u64,
    kind: EventKind,
    app_name: String,
    bundle_id: String,
    pid: i32,
    notif: String,
    title: TitleResult,
    coalesced: u32,
}

impl CaptureEvent {
    fn format(&self) -> String {
        let coalesced = if self.coalesced > 0 {
            format!("  (coalesced {})", self.coalesced)
        } else {
            String::new()
        };
        format!(
            "[#{seq:<4} {ts}] {kind:<10} app={app:<20} bundle={bundle:<30} pid={pid:<7} via={notif:<26} title={title}{coalesced}",
            seq = self.seq,
            ts = now_hms(),
            kind = self.kind.label(),
            app = truncate(&self.app_name, 20),
            bundle = truncate(&self.bundle_id, 30),
            pid = self.pid,
            notif = truncate(&self.notif, 26),
            title = self.title.describe(),
        )
    }
}

// ── Frontmost application ────────────────────────────────────────────────────

struct FrontApp {
    name: String,
    bundle_id: String,
    pid: i32,
}

fn frontmost() -> Option<FrontApp> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let name = app
        .localizedName()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    let bundle_id = app
        .bundleIdentifier()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<none>".to_string());
    let pid = app.processIdentifier();
    Some(FrontApp {
        name,
        bundle_id,
        pid,
    })
}

// ── AX title read (ported from Spike #1, plus sanitize) ──────────────────────

/// The outcome of trying to read the focused-window title for one app element.
enum TitleResult {
    Real(String),
    Empty,
    Nil,
    AxErr(&'static str),
}

impl TitleResult {
    fn describe(&self) -> String {
        match self {
            TitleResult::Real(s) => format!("REAL({s:?})"),
            TitleResult::Empty => "EMPTY".to_string(),
            TitleResult::Nil => "NIL".to_string(),
            TitleResult::AxErr(name) => format!("AXERR({name})"),
        }
    }

    /// A stable key for dedupe debounce (NUL-prefixed so a real title equal to
    /// "EMPTY"/"NIL" can't collide with the sentinels).
    fn dedupe_key(&self) -> String {
        match self {
            TitleResult::Real(s) => s.clone(),
            TitleResult::Empty => "\u{0}EMPTY".to_string(),
            TitleResult::Nil => "\u{0}NIL".to_string(),
            TitleResult::AxErr(n) => format!("\u{0}ERR:{n}"),
        }
    }
}

fn focused_window_title(app_el: &AXUIElement) -> TitleResult {
    let focused = match copy_attr(app_el, ATTR_FOCUSED_WINDOW) {
        AttrValue::Object(obj) => obj,
        AttrValue::Absent => return TitleResult::Nil,
        AttrValue::Err(name) => return TitleResult::AxErr(name),
    };

    let window_el: &AXUIElement = {
        let ptr = CFRetained::as_ptr(&focused).cast::<AXUIElement>();
        // SAFETY: AXFocusedWindow yields a valid AXUIElementRef, valid for `focused`.
        unsafe { ptr.as_ref() }
    };

    match copy_attr(window_el, ATTR_TITLE) {
        AttrValue::Object(obj) => match obj.downcast_ref::<CFString>() {
            Some(cf) => {
                let s = sanitize(&cf.to_string());
                if s.is_empty() {
                    TitleResult::Empty
                } else {
                    TitleResult::Real(s)
                }
            }
            None => TitleResult::Nil,
        },
        AttrValue::Absent => TitleResult::Nil,
        AttrValue::Err(name) => TitleResult::AxErr(name),
    }
}

/// Copy `AXFocusedWindow` and hand back the window as an owned AXUIElement.
fn copy_focused_window(app_el: &AXUIElement) -> Option<CFRetained<AXUIElement>> {
    match copy_attr(app_el, ATTR_FOCUSED_WINDOW) {
        AttrValue::Object(obj) => {
            let ptr = CFRetained::as_ptr(&obj).cast::<AXUIElement>();
            // SAFETY: the value is an AXUIElementRef; +1 it so it outlives `obj`.
            Some(unsafe { CFRetained::retain(ptr) })
        }
        _ => None,
    }
}

/// Strip control chars and bidi/zero-width formatting marks, then trim. Directly
/// addresses Spike #1's finding (WhatsApp leaked a stray LTR mark in its title).
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c,
                    '\u{200B}'..='\u{200F}'   // ZWSP … RLM
                    | '\u{202A}'..='\u{202E}' // LRE … RLO
                    | '\u{2066}'..='\u{2069}' // LRI … PDI
                    | '\u{FEFF}') // BOM / ZWNBSP
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Result of copying a single AX attribute.
enum AttrValue {
    Object(CFRetained<CFType>),
    Absent,
    Err(&'static str),
}

fn copy_attr(element: &AXUIElement, attr: &str) -> AttrValue {
    let attr_name = CFString::from_str(attr);

    let mut raw: *const CFType = ptr::null();
    let out: NonNull<*const CFType> = NonNull::from(&mut raw);

    // SAFETY: `element` and `attr_name` are valid; `out` points at a live local.
    let err: AXError = unsafe { element.copy_attribute_value(&attr_name, out) };

    if err != AXError::Success {
        return AttrValue::Err(ax_error_name(err));
    }

    let Some(ptr) = NonNull::new(raw.cast_mut()) else {
        return AttrValue::Absent;
    };

    // SAFETY: AX returned a +1-retained CF object (Copy semantics); take it.
    let value: CFRetained<CFType> = unsafe { CFRetained::from_raw(ptr) };
    AttrValue::Object(value)
}

fn ax_error_name(err: AXError) -> &'static str {
    match err {
        AXError::Success => "Success",
        AXError::Failure => "Failure",
        AXError::IllegalArgument => "IllegalArgument",
        AXError::InvalidUIElement => "InvalidUIElement",
        AXError::InvalidUIElementObserver => "InvalidUIElementObserver",
        AXError::CannotComplete => "CannotComplete",
        AXError::AttributeUnsupported => "AttributeUnsupported",
        AXError::ActionUnsupported => "ActionUnsupported",
        AXError::NotificationUnsupported => "NotificationUnsupported",
        AXError::NotImplemented => "NotImplemented",
        AXError::NotificationAlreadyRegistered => "NotificationAlreadyRegistered",
        AXError::NotificationNotRegistered => "NotificationNotRegistered",
        AXError::APIDisabled => "APIDisabled",
        AXError::NoValue => "NoValue",
        AXError::ParameterizedAttributeUnsupported => "ParameterizedAttributeUnsupported",
        AXError::NotEnoughPrecision => "NotEnoughPrecision",
        _ => "Unknown",
    }
}

// ── Trust handling (ported from Spike #1) ────────────────────────────────────

fn is_trusted() -> bool {
    // SAFETY: plain C call, no arguments, no pointers.
    unsafe { objc2_application_services::AXIsProcessTrusted() }
}

fn ensure_trusted() {
    if is_trusted() {
        println!("AX trust: already granted.\n");
        return;
    }

    println!("AX trust: NOT granted yet — requesting (a system prompt should appear).");
    println!("Grant this binary under System Settings → Privacy & Security → Accessibility,");
    println!("then return here. Polling until trust is granted...\n");

    prompt_for_trust();

    let mut waited = Duration::ZERO;
    while !is_trusted() {
        thread::sleep(TICK);
        waited += TICK;
        if waited.as_secs() % 6 == 0 {
            println!(
                "  ...still waiting for Accessibility ({}s)",
                waited.as_secs()
            );
        }
    }
    println!("\nAX trust: granted. Starting capture.\n");
}

fn prompt_for_trust() {
    // SAFETY: framework-guaranteed constant string.
    let key: &CFString = unsafe { objc2_application_services::kAXTrustedCheckOptionPrompt };

    let value: &CFBoolean = match unsafe { kCFBooleanTrue } {
        Some(b) => b,
        None => {
            println!("  (could not obtain kCFBooleanTrue; falling back to silent trust check)");
            let _ = is_trusted();
            return;
        }
    };

    let options: CFRetained<CFDictionary<CFString, CFBoolean>> =
        CFDictionary::from_slices(&[key], &[value]);

    let untyped: &CFDictionary = {
        let ptr = CFRetained::as_ptr(&options).cast::<CFDictionary>();
        // SAFETY: same layout (generics are PhantomData); `options` outlives this.
        unsafe { ptr.as_ref() }
    };

    // SAFETY: a valid CFDictionary whose key/value types match the option.
    let _ = unsafe { objc2_application_services::AXIsProcessTrustedWithOptions(Some(untyped)) };
}

// ── Small helpers ────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// HH:MM:SS derived from the Unix timestamp — a relative clock for the log.
fn now_hms() -> String {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };
    let s = secs % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}
