//! Event-driven macOS capture (Accessibility only; no Screen Recording). See D29.
//!
//! NSWorkspace `didActivateApplication` fires on app switches; a per-PID `AXObserver` watches
//! `AXFocusedWindowChanged` + `AXTitleChanged`. All native state lives on the **main run loop**
//! (`start` runs in Tauri's `setup`, attaches to `CFRunLoop::current()`, then leaks the `!Send`
//! keep-alives for the process lifetime). Each event reads the AX title (+ url/cwd for
//! browsers/terminals) and sends a [`FocusEvent`]. Every AX outcome is a classified value — no
//! `unwrap`/`expect`/`panic` (hard rule 3).

mod browser;
mod terminal;

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::mpsc::Sender;

use block2::RcBlock;
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
use objc2_application_services::{AXError, AXObserver, AXUIElement};
use objc2_core_foundation::{
    kCFRunLoopCommonModes, CFRetained, CFRunLoop, CFRunLoopSource, CFString, CFType,
};
use objc2_foundation::NSNotification;

use super::{note_capture_failure, note_capture_ok, CaptureSource, FocusEvent};
use crate::db::now_unix;

// AX attribute + notification names (the kAX* constants are NOT re-exported).
const ATTR_FOCUSED_WINDOW: &str = "AXFocusedWindow";
const ATTR_TITLE: &str = "AXTitle";
const NOTIF_FOCUSED_WINDOW_CHANGED: &str = "AXFocusedWindowChanged";
const NOTIF_TITLE_CHANGED: &str = "AXTitleChanged";
const NOTIF_MAIN_WINDOW_CHANGED: &str = "AXMainWindowChanged";
const NOTIF_FOCUSED_UI_CHANGED: &str = "AXFocusedUIElementChanged";

const APP_NOTIFICATIONS: &[&str] = &[
    NOTIF_FOCUSED_WINDOW_CHANGED,
    NOTIF_MAIN_WINDOW_CHANGED,
    NOTIF_FOCUSED_UI_CHANGED,
];

/// The event-driven macOS capture source. A zero-config marker; all native state
/// is created inside [`CaptureSource::start`] on the main thread.
#[derive(Default)]
pub struct MacosCapture;

impl MacosCapture {
    pub fn new() -> Self {
        Self
    }
}

impl CaptureSource for MacosCapture {
    /// MUST be called on the main thread (Tauri `setup`) — the observers attach to
    /// the current (main) `CFRunLoop`, which Tauri pumps.
    fn start(self: Box<Self>, tx: Sender<FocusEvent>) {
        prompt_trust_if_needed();

        let Some(run_loop) = CFRunLoop::current() else {
            eprintln!("[Capture] fatal: no current CFRunLoop; capture disabled");
            return;
        };

        let state = Rc::new(RefCell::new(CaptureState {
            current: None,
            tx,
            run_loop: run_loop.clone(),
        }));

        // Install the observer for whoever is frontmost right now.
        if let Some(app) = frontmost() {
            state.borrow_mut().switch_to(app);
        }

        // Observe app activation. The block captures an Rc clone of state.
        let activation_block = {
            let state = Rc::clone(&state);
            RcBlock::new(move |notif: NonNull<NSNotification>| {
                state.borrow_mut().on_activation(notif);
            })
        };
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: valid notification name + block; queue=nil delivers on the main
        // thread, where all AX state lives, so the non-Send block is sound.
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
                None,
                &activation_block,
            )
        };

        println!("[Capture] macOS event-driven capture registered on the main run loop");

        // These objc2 values are !Send and must outlive `start` for the process lifetime, so
        // leak them — the block holds the only path to `state` and its live AXObserver (D33).
        std::mem::forget(activation_block);
        std::mem::forget(token);
        std::mem::forget(state);
    }
}

// ── Capture state (main-thread only) ─────────────────────────────────────────

struct CaptureState {
    current: Option<AppObserver>,
    tx: Sender<FocusEvent>,
    run_loop: CFRetained<CFRunLoop>,
}

impl CaptureState {
    fn on_activation(&mut self, _notif: NonNull<NSNotification>) {
        if let Some(app) = frontmost() {
            self.switch_to(app);
        }
    }

    /// Tear down the previous app's observer and stand up a fresh one for `app`.
    fn switch_to(&mut self, app: FrontApp) {
        if let Some(cur) = &self.current {
            if cur.ctx.pid == app.pid {
                return; // re-activation of the same app
            }
        }
        // Drop the old observer first: its Drop removes the run-loop source, so no
        // stale callback can fire against the freed context (no UAF).
        self.current = None;

        match install_observer(&app, self.tx.clone(), &self.run_loop) {
            Some(obs) => {
                // Seed dedupe + emit the initial state for the new app.
                let title = focused_window_title(&obs.ctx.app_element);
                *obs.ctx.last_title.borrow_mut() = Some(title.clone());
                obs.ctx.emit_with_title(title);
                note_capture_ok();
                self.current = Some(obs);
            }
            None => note_capture_failure(),
        }
    }
}

// ── Per-application AXObserver ───────────────────────────────────────────────

struct AppObserver {
    observer: CFRetained<AXObserver>,
    source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
    ctx: Box<CallbackContext>,
}

impl Drop for AppObserver {
    fn drop(&mut self) {
        // Remove the source FIRST (we're on the run-loop thread; synchronous) so
        // the boxed `ctx` — the live refcon target — can't be freed under a
        // pending callback.
        self.run_loop
            .remove_source(Some(&self.source), unsafe { kCFRunLoopCommonModes });
        let _ = &self.observer; // released on field drop; AX also auto-detaches.
    }
}

/// Data the AX C callback reaches through `refcon`. Mutated only on the main thread.
struct CallbackContext {
    tx: Sender<FocusEvent>,
    pid: i32,
    app_name: String,
    bundle_id: String,
    app_element: CFRetained<AXUIElement>,
    focused_window: RefCell<Option<CFRetained<AXUIElement>>>,
    /// Last emitted title, for dedupe debounce (drops chatty duplicates).
    last_title: RefCell<Option<Option<String>>>,
}

impl CallbackContext {
    /// Read the current title; emit unless it's an unchanged duplicate.
    fn emit(&self) {
        let title = focused_window_title(&self.app_element);
        {
            let mut last = self.last_title.borrow_mut();
            if last.as_ref() == Some(&title) {
                return;
            }
            *last = Some(title.clone());
        }
        self.emit_with_title(title);
    }

    /// Build and send a [`FocusEvent`] for the current app + title, adding the front-tab url
    /// (browsers) and cwd (terminals). An incognito/private window drops BOTH title and url (D8).
    fn emit_with_title(&self, title: Option<String>) {
        let mut title = title;
        let mut url = None;
        let mut is_private = false;
        match browser::inspect(&self.bundle_id) {
            browser::BrowserUrl::Private => {
                title = None; // D8: never record an incognito window's title or url
                is_private = true;
            }
            browser::BrowserUrl::Normal(u) => url = u,
            browser::BrowserUrl::NotBrowser => {}
        }
        let cwd = terminal::front_cwd(&self.bundle_id, self.pid);
        let _ = self.tx.send(FocusEvent {
            app_name: self.app_name.clone(),
            window_title: title,
            url,
            cwd,
            is_private,
            timestamp: now_unix(),
        });
    }
}

fn install_observer(
    app: &FrontApp,
    tx: Sender<FocusEvent>,
    run_loop: &CFRetained<CFRunLoop>,
) -> Option<AppObserver> {
    // SAFETY: plain FFI; pid comes from a live NSRunningApplication.
    let app_element = unsafe { AXUIElement::new_application(app.pid) };

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

    let ctx = Box::new(CallbackContext {
        tx,
        pid: app.pid,
        app_name: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        app_element: app_element.clone(),
        focused_window: RefCell::new(None),
        last_title: RefCell::new(None),
    });
    let refcon = ptr::addr_of!(*ctx) as *mut c_void;

    for name in APP_NOTIFICATIONS {
        let cf = CFString::from_str(name);
        // SAFETY: valid observer/element/name; refcon outlives the observer.
        let _ = unsafe { observer.add_notification(&app_element, &cf, refcon) };
    }

    if let Some(win) = copy_focused_window(&app_element) {
        let cf = CFString::from_str(NOTIF_TITLE_CHANGED);
        // SAFETY: as above; `win` is a valid window element.
        let _ = unsafe { observer.add_notification(&win, &cf, refcon) };
        *ctx.focused_window.borrow_mut() = Some(win);
    }

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

/// Move the `AXTitleChanged` registration to the now-focused window.
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

/// The AX observer C callback. Runs on the main (run-loop) thread.
///
/// # Safety
/// `refcon` points at the live, boxed `CallbackContext` owned by this observer's
/// `AppObserver`; the AX pointers are valid for the call per the AX contract.
unsafe extern "C-unwind" fn observer_callback(
    observer: NonNull<AXObserver>,
    _element: NonNull<AXUIElement>,
    notification: NonNull<CFString>,
    refcon: *mut c_void,
) {
    if refcon.is_null() {
        return;
    }
    // SAFETY: refcon is a live, boxed CallbackContext; shared ref + interior
    // mutability only (no &mut aliasing).
    let ctx: &CallbackContext = unsafe { &*(refcon as *const CallbackContext) };
    // SAFETY: AX hands us a valid notification-name CFString for this call.
    let name = unsafe { notification.as_ref() }.to_string();

    if name == NOTIF_FOCUSED_WINDOW_CHANGED || name == NOTIF_MAIN_WINDOW_CHANGED {
        // SAFETY: AX hands us a valid observer ref for this call.
        let obs = unsafe { observer.as_ref() };
        reregister_title_observer(obs, ctx, refcon);
    }
    ctx.emit();
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
        .unwrap_or_default();
    Some(FrontApp {
        name,
        bundle_id,
        pid: app.processIdentifier(),
    })
}

// ── AX title read ────────────────────────────────────────────────────────────

/// The focused-window title, or `None` (no window / empty / AX error).
fn focused_window_title(app_el: &AXUIElement) -> Option<String> {
    let focused = copy_attr(app_el, ATTR_FOCUSED_WINDOW)?;
    let window_el: &AXUIElement = {
        let ptr = CFRetained::as_ptr(&focused).cast::<AXUIElement>();
        // SAFETY: AXFocusedWindow yields a valid AXUIElementRef, valid for `focused`.
        unsafe { ptr.as_ref() }
    };
    let obj = copy_attr(window_el, ATTR_TITLE)?;
    let cf = obj.downcast_ref::<CFString>()?;
    let s = sanitize(&cf.to_string());
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Copy `AXFocusedWindow` as an owned AXUIElement.
fn copy_focused_window(app_el: &AXUIElement) -> Option<CFRetained<AXUIElement>> {
    let obj = copy_attr(app_el, ATTR_FOCUSED_WINDOW)?;
    let ptr = CFRetained::as_ptr(&obj).cast::<AXUIElement>();
    // SAFETY: the value is an AXUIElementRef; +1 it so it outlives `obj`.
    Some(unsafe { CFRetained::retain(ptr) })
}

/// Strip control/bidi/zero-width marks, then trim.
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c,
                    '\u{200B}'..='\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}'
                    | '\u{FEFF}')
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Copy a single AX attribute as an owned CF object, or `None` (absent / error).
fn copy_attr(element: &AXUIElement, attr: &str) -> Option<CFRetained<CFType>> {
    let attr_name = CFString::from_str(attr);
    let mut raw: *const CFType = ptr::null();
    let out: NonNull<*const CFType> = NonNull::from(&mut raw);
    // SAFETY: `element` and `attr_name` are valid; `out` points at a live local.
    let err = unsafe { element.copy_attribute_value(&attr_name, out) };
    if err != AXError::Success {
        return None;
    }
    let ptr = NonNull::new(raw.cast_mut())?;
    // SAFETY: AX returned a +1-retained CF object (Copy semantics); take it.
    Some(unsafe { CFRetained::from_raw(ptr) })
}

// ── Trust ────────────────────────────────────────────────────────────────────

/// If Accessibility isn't granted, prompt once (non-blocking) and continue — capture
/// degrades to no-titles until granted (full priming is Phase 4, see D21).
fn prompt_trust_if_needed() {
    if crate::permissions::accessibility_trusted() {
        return;
    }
    eprintln!(
        "[Capture] Accessibility not granted — capture is degraded (no titles). \
         Grant UsageOS under System Settings → Privacy & Security → Accessibility."
    );
    crate::permissions::prompt_accessibility_trust();
}
