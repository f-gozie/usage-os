//! macOS-native permission queries/requests.
//!
//! - **Accessibility** via the AX trust API (`AXIsProcessTrusted` / `…WithOptions`).
//! - **Automation** via `AEDeterminePermissionToAutomateTarget` (`ask_user = false`) — reads the
//!   TCC-Automation grant silently, without sending a real Apple Event. The consent *prompt* is
//!   raised separately, by sending an actual Apple Event to a running browser: the wildcard
//!   determination call does not reliably surface the dialog (notably on macOS 26). See
//!   [`request_automation`].
//!
//! Verified on-device (D32/D33); CI compiles this but never runs it.

use std::os::raw::c_void;

use objc2_application_services::{
    kAXTrustedCheckOptionPrompt, AXIsProcessTrusted, AXIsProcessTrustedWithOptions,
};
use objc2_core_foundation::{kCFBooleanTrue, CFBoolean, CFDictionary, CFRetained, CFString};

use super::{
    aggregate_automation, AutomationRequest, PermissionState, SettingsPane, AE_AUTHORIZED,
    AE_NOT_PERMITTED,
};

// ── Accessibility (AX trust) ─────────────────────────────────────────────────

/// Whether this process is AX-trusted (no prompt). The same check capture runs at start.
pub(crate) fn accessibility_trusted() -> bool {
    // SAFETY: argless C call.
    unsafe { AXIsProcessTrusted() }
}

/// Show the system Accessibility trust prompt (adds UsageOS to the list; non-blocking).
pub(crate) fn prompt_accessibility_trust() {
    // SAFETY: framework-guaranteed constant string.
    let key: &CFString = unsafe { kAXTrustedCheckOptionPrompt };
    let Some(value) = (unsafe { kCFBooleanTrue }) else {
        return;
    };
    let value: &CFBoolean = value;
    let options: CFRetained<CFDictionary<CFString, CFBoolean>> =
        CFDictionary::from_slices(&[key], &[value]);
    let untyped: &CFDictionary = {
        let ptr = CFRetained::as_ptr(&options).cast::<CFDictionary>();
        // SAFETY: same layout (generics are PhantomData); `options` outlives this borrow.
        unsafe { ptr.as_ref() }
    };
    // SAFETY: a valid CFDictionary whose key/value types match the option.
    let _ = unsafe { AXIsProcessTrustedWithOptions(Some(untyped)) };
}

/// Prompt for Accessibility, then open its Settings pane so the user lands on the toggle.
pub(crate) fn request_accessibility() {
    prompt_accessibility_trust();
    open_settings(SettingsPane::Accessibility);
}

// ── Automation (AppleEvents TCC) ─────────────────────────────────────────────

#[repr(C)]
struct AEDesc {
    descriptor_type: u32,     // DescType (FourCharCode)
    data_handle: *mut c_void, // AEDataStorage (opaque handle)
}

// FourCharCodes are big-endian packed.
const TYPE_APPLICATION_BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
const TYPE_WILDCARD: u32 = u32::from_be_bytes(*b"****");

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AECreateDesc(
        type_code: u32,
        data_ptr: *const c_void,
        data_size: isize,
        result: *mut AEDesc,
    ) -> i16;
    fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
    fn AEDeterminePermissionToAutomateTarget(
        target: *const AEDesc,
        event_class: u32,
        event_id: u32,
        ask_user_if_needed: u8,
    ) -> i32;
}

/// Chromium-family browsers we read URLs from (mirrors `capture::macos::browser`). Safari is
/// excluded by design (R18 — no scriptable private-window state).
const CHROMIUM_BUNDLES: &[&str] = &[
    "com.google.Chrome",
    "com.brave.Browser",
    "com.microsoft.edgemac",
    "company.thebrowser.Browser",
    "com.vivaldi.Vivaldi",
    "com.operasoftware.Opera",
];

/// `procNotFound` from `AEDeterminePermissionToAutomateTarget` — the target browser isn't running.
const AE_PROC_NOT_FOUND: i32 = -600;

/// Read the Automation grant for one bundle id, silently (`ask_user = false` — never prompts).
/// Returns the raw OSStatus (0 authorized, -1743 denied, -1744 undetermined, -600 not running).
/// The consent *prompt* is raised separately by [`prompt_automation_consent`]; the wildcard
/// determination call here does not reliably surface the dialog (notably on macOS 26).
fn automate_status(bundle_id: &str) -> i32 {
    let bytes = bundle_id.as_bytes();
    let mut target = AEDesc {
        descriptor_type: 0,
        data_handle: std::ptr::null_mut(),
    };
    // SAFETY: AECreateDesc copies the bundle-id bytes into a descriptor we own and dispose below.
    let created = unsafe {
        AECreateDesc(
            TYPE_APPLICATION_BUNDLE_ID,
            bytes.as_ptr().cast::<c_void>(),
            bytes.len() as isize,
            &mut target,
        )
    };
    if created != 0 {
        return created as i32;
    }
    // SAFETY: `target` is a descriptor we just created; the wildcards ask about automation in
    // general; ask_user = 0 so this only reads the grant and never prompts.
    let status =
        unsafe { AEDeterminePermissionToAutomateTarget(&target, TYPE_WILDCARD, TYPE_WILDCARD, 0) };
    // SAFETY: dispose the descriptor created above.
    unsafe { AEDisposeDesc(&mut target) };
    status
}

/// Send a real, benign Apple Event to a *running* Chromium browser to raise the macOS Automation
/// consent prompt — the only dependable trigger (the wildcard determination above does not surface
/// the dialog). Targets by bundle id (`application id`) so no display-name map is needed, and
/// `count windows` is the smallest scriptable event that still requires the grant and reads
/// nothing we keep. The caller only invokes this for browsers it has already confirmed running, so
/// the `tell` never auto-launches one.
fn prompt_automation_consent(bundle_id: &str) {
    let script = format!("tell application id \"{bundle_id}\" to count windows");
    let _ = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output();
}

pub(crate) fn automation_state() -> PermissionState {
    aggregate_automation(CHROMIUM_BUNDLES.iter().map(|&b| automate_status(b)))
}

pub(crate) fn request_automation() -> AutomationRequest {
    // The Automation list row appears only once we send a real Apple Event to a *running* browser,
    // so poke each running, not-yet-decided one — that raises the "UsageOS wants to control
    // <browser>" prompt. The silent query tells us which are running (procNotFound for the rest) so
    // we never auto-launch one; already-granted / already-denied browsers are left to the Settings
    // pane. If none is running there is nothing to prompt — the caller surfaces that to the user.
    let mut any_running = false;
    for bundle in CHROMIUM_BUNDLES {
        match automate_status(bundle) {
            AE_PROC_NOT_FOUND => {}
            AE_AUTHORIZED | AE_NOT_PERMITTED => any_running = true,
            _ => {
                any_running = true;
                prompt_automation_consent(bundle);
            }
        }
    }
    open_settings(SettingsPane::Automation);
    if any_running {
        AutomationRequest::Prompted
    } else {
        AutomationRequest::NoBrowserRunning
    }
}

// ── System Settings deep-link ────────────────────────────────────────────────

/// Open a System Settings → Privacy pane. Best-effort (`open` fails silently if the URL scheme
/// shifts across macOS versions — verified on-device).
pub(crate) fn open_settings(pane: SettingsPane) {
    let anchor = match pane {
        SettingsPane::Accessibility => "Privacy_Accessibility",
        SettingsPane::Automation => "Privacy_Automation",
    };
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
    let _ = std::process::Command::new("/usr/bin/open").arg(url).spawn();
}
