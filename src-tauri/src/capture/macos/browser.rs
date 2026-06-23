//! Browser front-tab inspection via Apple Events (port of `spikes/browser-url`).
//!
//! D8 is non-negotiable: an incognito/private window must never have its URL **or**
//! its title recorded. The Chromium ladder reads `mode of front window` FIRST; if
//! it isn't `"normal"` the caller drops both (C5/D8). Safari has no scriptable
//! private-window property, so the safe-default is to not read its URL (R18) — and,
//! because we can't *prove* it's non-private, we leave it as a non-browser here (its
//! AX title is kept; no URL). Static scripts via `/usr/bin/osascript` (C7); a denied
//! Automation grant (`-1743`) yields `Normal(None)` and we fall back (C6).

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

/// Inspect the front browser window: privacy state + URL. `mode` is read **before**
/// the URL (C5), so a private window is classified without ever touching its URL.
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
        // -1743 / spawn error: a browser, but we couldn't read it. Keep the AX title
        // (we can't prove it's private), no URL — the existing R18/C6 stance for URLs.
        None => BrowserUrl::Normal(None),
    }
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
