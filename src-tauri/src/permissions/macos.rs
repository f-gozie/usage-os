//! macOS-native permission queries/requests.
//!
//! - **Accessibility** via the AX trust API (`AXIsProcessTrusted` / `…WithOptions`).
//! - **Automation** via `AEDeterminePermissionToAutomateTarget` — the only API that reads the
//!   TCC-Automation grant WITHOUT sending a real Apple Event: `ask_user = false` queries
//!   silently, `true` shows the system consent prompt without launching the target.
//!
//! Verified on-device (D32/D33); CI compiles this but never runs it.

use std::os::raw::c_void;

use objc2_application_services::{
    kAXTrustedCheckOptionPrompt, AXIsProcessTrusted, AXIsProcessTrustedWithOptions,
};
use objc2_core_foundation::{kCFBooleanTrue, CFBoolean, CFDictionary, CFRetained, CFString};

use super::{aggregate_automation, PermissionState, SettingsPane};

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

/// Query (`ask_user = false`) or request (`true`) Automation permission for one bundle id.
/// Returns the raw OSStatus (0 authorized, -1743 denied, -1744 undetermined, -600 not running).
fn automate_status(bundle_id: &str, ask_user: bool) -> i32 {
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
    // general; ask_user=false never prompts, =true shows the consent prompt without launching.
    let status = unsafe {
        AEDeterminePermissionToAutomateTarget(&target, TYPE_WILDCARD, TYPE_WILDCARD, ask_user as u8)
    };
    // SAFETY: dispose the descriptor created above.
    unsafe { AEDisposeDesc(&mut target) };
    status
}

pub(crate) fn automation_state() -> PermissionState {
    aggregate_automation(CHROMIUM_BUNDLES.iter().map(|b| automate_status(b, false)))
}

pub(crate) fn request_automation() {
    // Prompts each browser whose grant is still undetermined; already-decided ones are a no-op,
    // and a not-running browser simply isn't prompted here (it asks naturally on first read).
    for bundle in CHROMIUM_BUNDLES {
        let _ = automate_status(bundle, true);
    }
    open_settings(SettingsPane::Automation);
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
