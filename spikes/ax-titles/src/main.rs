//! Spike #1 — AX focused-window titles WITHOUT Screen Recording.
//!
//! The make-or-break question for the UsageOS redesign:
//!
//!   Does the macOS Accessibility (AX) API return a real, non-empty
//!   focused-window TITLE for Chromium/Electron apps, editors, terminals, and
//!   native apps when ONLY Accessibility is granted and Screen Recording is
//!   explicitly OFF?
//!
//! Today the app reads titles via `active-win-pos-rs`, which falls back to
//! CGWindowList — and CGWindowList needs Screen Recording, so titles come back
//! empty. The whole redesign (recap, day-dial, local narration) assumes we can
//! read the focused window title with Accessibility ALONE. This binary proves
//! (or disproves) that.
//!
//! What it does, once per ~1.5s tick:
//!   1. Ensure the process is AX-trusted (prompt + poll if not).
//!   2. Ask the system-wide AX element for "AXFocusedApplication" — the app with
//!      keyboard focus RIGHT NOW. This is a synchronous AX query, so it tracks app
//!      switches without a running run loop (unlike NSWorkspace::frontmostApplication,
//!      which goes stale in a CLI that never pumps its run loop).
//!   3. From that app element, copy "AXFocusedWindow", then "AXTitle".
//!   4. Print one classified line: REAL("...") / EMPTY / NIL / AXERR(<variant>).
//!
//! Hard rules honored:
//!   * No network. No CGWindowList / Screen Recording path. Ever.
//!   * No unwrap()/expect()/panic! in the logic — every AX outcome is a value
//!     we classify and print, including the "expected error" cases
//!     (NoValue / AttributeUnsupported / CannotComplete / APIDisabled / ...).

use std::ptr::NonNull;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use objc2_app_kit::NSRunningApplication;
use objc2_application_services::{AXError, AXUIElement};
use objc2_core_foundation::{
    kCFBooleanTrue, CFBoolean, CFDictionary, CFRetained, CFString, CFType,
};

const TICK: Duration = Duration::from_millis(1500);

fn main() {
    println!("ax-titles spike — proving AX focused-window titles work without Screen Recording");
    println!("(Ctrl-C to stop. Switch between apps to exercise each one.)\n");

    // ── Step 1: make sure we are AX-trusted ────────────────────────────────
    ensure_trusted();

    // ── Steps 2–4: poll forever, one classified line per tick ──────────────
    loop {
        let trusted = is_trusted();

        // Ask the Accessibility server directly which application has focus right
        // now. The system-wide element + "AXFocusedApplication" is a synchronous
        // query, so it tracks app switches without a running run loop — the reason
        // NSWorkspace::frontmostApplication stayed frozen on the launching app.
        // SAFETY: plain FFI; returns an owned system-wide AXUIElement.
        let system = unsafe { AXUIElement::new_system_wide() };

        match copy_attr(&system, "AXFocusedApplication") {
            AttrValue::Object(app_obj) => {
                // The focused application is itself an AXUIElement.
                let app_el: &AXUIElement = {
                    let ptr = CFRetained::as_ptr(&app_obj).cast::<AXUIElement>();
                    // SAFETY: AXFocusedApplication yields a valid AXUIElementRef,
                    // valid for the lifetime of `app_obj`.
                    unsafe { ptr.as_ref() }
                };

                let pid = focused_pid(app_el);
                let (name, bundle) = app_identity(pid);
                let title = focused_window_title(app_el);

                println!(
                    "{ts}  trusted={trusted}  app={name:<22} bundle={bundle:<34} pid={pid:<7} title={title}",
                    ts = now_hms(),
                    trusted = trusted,
                    name = truncate(&name, 22),
                    bundle = truncate(&bundle, 34),
                    pid = pid,
                    title = title.describe(),
                );
            }
            AttrValue::Absent => println!(
                "{ts}  trusted={trusted}  <no focused application (AXFocusedApplication absent)>",
                ts = now_hms(),
                trusted = trusted,
            ),
            AttrValue::Err(name) => println!(
                "{ts}  trusted={trusted}  AXFocusedApplication AXERR({name})",
                ts = now_hms(),
                trusted = trusted,
                name = name,
            ),
        }
        sleep(TICK);
    }
}

// ── Trust handling ─────────────────────────────────────────────────────────

/// `AXIsProcessTrusted()` — no prompt, just a status read.
fn is_trusted() -> bool {
    // SAFETY: plain C call, no arguments, no pointers.
    unsafe { objc2_application_services::AXIsProcessTrusted() }
}

/// If not already trusted, fire the system prompt once, then poll until the
/// user grants Accessibility. Never panics — worst case it loops, which is the
/// intended behavior while the user is in System Settings.
fn ensure_trusted() {
    if is_trusted() {
        println!("AX trust: already granted.\n");
        return;
    }

    println!("AX trust: NOT granted yet — requesting (a system prompt should appear).");
    println!("Grant this binary under System Settings → Privacy & Security → Accessibility,");
    println!("then return here. Polling every 1.5s until trust is granted...\n");

    prompt_for_trust();

    let mut waited = Duration::ZERO;
    while !is_trusted() {
        sleep(TICK);
        waited += TICK;
        // Gentle heartbeat so the dev knows we are still waiting.
        if waited.as_secs() % 6 == 0 {
            println!(
                "  ...still waiting for Accessibility ({}s)",
                waited.as_secs()
            );
        }
    }
    println!("\nAX trust: granted. Starting capture.\n");
}

/// `AXIsProcessTrustedWithOptions({ kAXTrustedCheckOptionPrompt: true })`.
/// Building the options dictionary by hand because the attribute-name and
/// option-key handling lives in CoreFoundation, not in safe re-exports.
fn prompt_for_trust() {
    // kAXTrustedCheckOptionPrompt is a `&'static CFString` static.
    // SAFETY: it is a constant string the framework guarantees to exist.
    let key: &CFString = unsafe { objc2_application_services::kAXTrustedCheckOptionPrompt };

    // kCFBooleanTrue is `Option<&'static CFBoolean>`.
    let value: &CFBoolean = match unsafe { kCFBooleanTrue } {
        Some(b) => b,
        None => {
            // Should never happen, but per project rules we do not unwrap/panic
            // in logic. Fall back to the no-options check (no prompt).
            println!("  (could not obtain kCFBooleanTrue; falling back to silent trust check)");
            let _ = is_trusted();
            return;
        }
    };

    // Typed dictionary, then hand the AX call a base `&CFDictionary`.
    let options: CFRetained<CFDictionary<CFString, CFBoolean>> =
        CFDictionary::from_slices(&[key], &[value]);

    // `CFDictionary<K, V>` and the untyped `CFDictionary` share an identical
    // layout (the generics are PhantomData), so reborrowing the pointer as the
    // untyped base type is sound.
    let untyped: &CFDictionary = {
        let ptr = CFRetained::as_ptr(&options).cast::<CFDictionary>();
        // SAFETY: same layout; `options` (and thus the allocation) outlives this borrow.
        unsafe { ptr.as_ref() }
    };

    // SAFETY: `untyped` is a valid CFDictionary whose key/value types match what
    // the option expects (CFString → CFBoolean).
    let _ = unsafe { objc2_application_services::AXIsProcessTrustedWithOptions(Some(untyped)) };
}

// ── Focused application identity ─────────────────────────────────────────────

/// Pid of the focused-application AX element, or -1 if AX can't report it.
fn focused_pid(app_el: &AXUIElement) -> i32 {
    let mut pid: libc::pid_t = 0;
    // SAFETY: `app_el` is valid; the call writes the pid into our local.
    let err = unsafe { app_el.pid(NonNull::from(&mut pid)) };
    if err == AXError::Success {
        pid
    } else {
        -1
    }
}

/// Resolve a pid to (localized name, bundle id) via a direct per-pid
/// NSRunningApplication lookup — NOT NSWorkspace::frontmostApplication — so it
/// stays fresh without a run loop. Returns placeholders if the pid isn't a
/// running app or AX couldn't report it.
fn app_identity(pid: i32) -> (String, String) {
    if pid <= 0 {
        return ("<unknown>".to_string(), "<none>".to_string());
    }
    // Safe accessor in objc2-app-kit 0.3.2; None if the pid isn't a running app.
    match NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        Some(app) => {
            let name = app
                .localizedName()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            let bundle = app
                .bundleIdentifier()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            (name, bundle)
        }
        None => ("<unknown>".to_string(), "<none>".to_string()),
    }
}

// ── AX focused-window title read ─────────────────────────────────────────────

/// The outcome of trying to read the focused-window title for one pid.
enum TitleResult {
    /// A non-empty title string.
    Real(String),
    /// AXTitle resolved to an empty string ("").
    Empty,
    /// The focused window or its title was present but the value was not a
    /// CFString (or was absent where the call still "succeeded").
    Nil,
    /// An AX error code at some step. Carries the human name of the variant.
    AxErr(&'static str),
}

impl TitleResult {
    fn describe(&self) -> String {
        match self {
            TitleResult::Real(s) => format!("REAL({:?})", s),
            TitleResult::Empty => "EMPTY".to_string(),
            TitleResult::Nil => "NIL".to_string(),
            TitleResult::AxErr(name) => format!("AXERR({name})"),
        }
    }
}

fn focused_window_title(app_el: &AXUIElement) -> TitleResult {
    // app element → AXFocusedWindow
    let focused = match copy_attr(app_el, "AXFocusedWindow") {
        AttrValue::Object(obj) => obj,
        AttrValue::Absent => return TitleResult::Nil,
        AttrValue::Err(name) => return TitleResult::AxErr(name),
    };

    // The focused window is itself an AXUIElement. Reinterpret the CFType as one.
    // SAFETY: AXFocusedWindow always yields an AXUIElementRef when present; the
    // pointer is valid for the lifetime of `focused`.
    let window_el: &AXUIElement = {
        let ptr = CFRetained::as_ptr(&focused).cast::<AXUIElement>();
        unsafe { ptr.as_ref() }
    };

    // window element → AXTitle
    match copy_attr(window_el, "AXTitle") {
        AttrValue::Object(obj) => match obj.downcast_ref::<CFString>() {
            Some(cf) => {
                let s = cf.to_string();
                if s.is_empty() {
                    TitleResult::Empty
                } else {
                    TitleResult::Real(s)
                }
            }
            // AXTitle existed but wasn't a string — treat as NIL.
            None => TitleResult::Nil,
        },
        AttrValue::Absent => TitleResult::Nil,
        AttrValue::Err(name) => TitleResult::AxErr(name),
    }
}

/// Result of copying a single AX attribute.
enum AttrValue {
    /// Success with a CF object value.
    Object(CFRetained<CFType>),
    /// Success but the out-pointer was left null (no value).
    Absent,
    /// An AX error code; carries the variant name.
    Err(&'static str),
}

/// Copy one AX attribute by name. Centralizes the unsafe out-pointer dance and
/// the AXError → name classification so callers stay readable.
fn copy_attr(element: &AXUIElement, attr: &str) -> AttrValue {
    let attr_name = CFString::from_str(attr);

    // Out-param: AXUIElementCopyAttributeValue writes a `*const CFType` here.
    let mut raw: *const CFType = std::ptr::null();
    let out: NonNull<*const CFType> = NonNull::from(&mut raw);

    // SAFETY: `element` and `attr_name` are valid; `out` points at a live local.
    let err: AXError = unsafe { element.copy_attribute_value(&attr_name, out) };

    if err != AXError::Success {
        return AttrValue::Err(ax_error_name(err));
    }

    // Success but null → no value present.
    let Some(ptr) = NonNull::new(raw.cast_mut()) else {
        return AttrValue::Absent;
    };

    // SAFETY: AX returned a +1-retained CF object (Copy semantics). Taking
    // ownership means it is released when the CFRetained is dropped.
    let value: CFRetained<CFType> = unsafe { CFRetained::from_raw(ptr) };
    AttrValue::Object(value)
}

/// Map an `AXError` to a stable, human-readable variant name for logging.
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
        // AXError is a newtype over i32; anything else is genuinely unexpected.
        _ => "Unknown",
    }
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

/// HH:MM:SS in local-ish terms — we only need a relative clock for the log, so
/// derive it from the Unix timestamp without pulling in a date crate.
fn now_hms() -> String {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };
    let s = secs % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}
