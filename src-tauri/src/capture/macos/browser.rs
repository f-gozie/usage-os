//! Browser front-tab URL via Apple Events (port of `spikes/browser-url`).
//!
//! D8 is non-negotiable: an incognito/private URL must never be read. The Chromium
//! ladder reads `mode of front window` FIRST and returns nothing unless it is
//! `"normal"` (C5). Safari has no scriptable private-window property, so the
//! safe-default is to **not** read its URL at all (R18). Static scripts via
//! `/usr/bin/osascript` (process-isolated, no injection surface — C7); a denied
//! Automation grant (`-1743`) just yields `None` and we fall back (C6).

use std::process::Command;

const OSASCRIPT: &str = "/usr/bin/osascript";

/// Browser **bundle id** → its AppleScript application name, for Chromium-family
/// browsers only. Names matter: Brave is `"Brave Browser"`, Edge is
/// `"Microsoft Edge"` — hardcoding the wrong name silently fails (R15). Safari is
/// intentionally absent (D8 safe-default, R18).
fn chromium_app_name(bundle_id: &str) -> Option<&'static str> {
    match bundle_id {
        "com.google.Chrome" | "com.google.Chrome.canary" => Some("Google Chrome"),
        "com.brave.Browser" => Some("Brave Browser"),
        "com.microsoft.edgemac" => Some("Microsoft Edge"),
        "company.thebrowser.Browser" => Some("Arc"),
        "com.vivaldi.Vivaldi" => Some("Vivaldi"),
        "com.operasoftware.Opera" => Some("Opera"),
        _ => None,
    }
}

/// Front-tab URL for a browser app, or `None`. Returns `None` (no osascript) for
/// non-browsers and Safari. For Chromium browsers: `None` for incognito/private
/// windows (D8 — checked before the URL is read), no-window, or a denied/failed
/// Automation call (C6).
pub fn front_tab_url(bundle_id: &str) -> Option<String> {
    let app = chromium_app_name(bundle_id)?;
    let script = format!(
        "tell application \"{app}\"\n\
         if (count of windows) is 0 then return \"NOWIN\"\n\
         set m to mode of front window\n\
         if m is not \"normal\" then return \"PRIVATE\"\n\
         return \"URL\t\" & (URL of active tab of front window)\n\
         end tell"
    );
    let out = run_osa(&script)?;
    out.strip_prefix("URL\t").map(|u| u.to_string())
}

/// Run a static AppleScript; `Some(stdout)` on success, `None` on any failure
/// (incl. `-1743` denial — fall back, never retry; C6).
fn run_osa(script: &str) -> Option<String> {
    let out = Command::new(OSASCRIPT)
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}
