//! Browser front-tab inspection via Apple Events.
//!
//! D8 invariant: an incognito/private window must never have its URL OR title recorded — the
//! Chromium ladder reads `mode of front window` first and drops both if it isn't `"normal"`.
//! Safari has no scriptable private-window property, so we treat it as a non-browser (keep its
//! AX title, read no URL) rather than risk leaking a private one — the R18 safe default.

use std::process::Command;

const OSASCRIPT: &str = "/usr/bin/osascript";

/// What the front browser window is. Drives whether the caller keeps the AX title.
#[derive(Debug, PartialEq, Eq)]
pub enum BrowserUrl {
    /// Not a scriptable Chromium browser (incl. Safari) — keep the AX title, no URL.
    NotBrowser,
    /// A normal (non-private) window: the front-tab URL if it could be read.
    Normal(Option<String>),
    /// A private/incognito window — the caller MUST omit BOTH the URL and the title.
    Private,
}

/// Browser bundle id → its exact AppleScript application name (a wrong name silently fails).
/// Chromium-family only; Safari is intentionally absent (the R18 safe default).
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

/// Inspect the front browser window: privacy state + URL. `mode` is read before the URL, so a
/// private window is classified without ever touching its URL (D8).
pub fn inspect(bundle_id: &str) -> BrowserUrl {
    let Some(app) = chromium_app_name(bundle_id) else {
        return BrowserUrl::NotBrowser;
    };
    let script = format!(
        "tell application \"{app}\"\n\
         if (count of windows) is 0 then return \"NOWIN\"\n\
         set m to mode of front window\n\
         if m is not \"normal\" then return \"PRIVATE\"\n\
         return \"URL\t\" & (URL of active tab of front window)\n\
         end tell"
    );
    match run_osa(&script).as_deref() {
        Some("PRIVATE") => BrowserUrl::Private,
        Some(s) => match s.strip_prefix("URL\t") {
            Some(u) => BrowserUrl::Normal(Some(u.to_string())),
            None => BrowserUrl::Normal(None), // NOWIN / unexpected — a normal browser, no URL
        },
        // Denied Automation (-1743) / spawn error: a browser we couldn't read. Keep the AX title
        // (can't prove it's private), no URL — the R18 safe default.
        None => BrowserUrl::Normal(None),
    }
}

/// Run a static AppleScript; `Some(stdout)` on success, `None` on any failure (e.g. a
/// denied Automation grant — fall back, never retry).
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
