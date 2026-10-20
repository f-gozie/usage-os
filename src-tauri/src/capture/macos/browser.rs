//! Browser front-tab inspection via Apple Events.
//!
//! D8 invariant: an incognito/private window must never have its URL OR title recorded — the
//! Chromium ladder reads `mode of front window` first and drops both if it isn't `"normal"`.
//! A browser window that can't be proven non-private records neither either (D76): Safari and
//! Firefox have no scriptable private-window property, and a Chromium browser whose script fails
//! (Automation denied) can't be asked. The app and the time still count.

use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Command;

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSBundle, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;

const OSASCRIPT: &str = "/usr/bin/osascript";

/// What the front browser window is. Drives whether the caller keeps the AX title.
#[derive(Debug, PartialEq, Eq)]
pub enum BrowserUrl {
    /// Not a browser — keep the AX title, no URL.
    NotBrowser,
    /// A proven normal (non-private) window. `title` and `url` come from the same script call
    /// that checked the window, so they can't belong to a different one (D76); the caller uses
    /// this title, not the one Accessibility read a moment earlier.
    Normal { url: String, title: String },
    /// A private/incognito window — the caller MUST omit BOTH the URL and the title.
    Private,
    /// A browser window that can't be proven non-private — the caller MUST omit BOTH the URL and
    /// the title, but it isn't marked private: nothing says it was.
    Unverified,
}

/// Browser bundle id → its exact AppleScript application name (a wrong name silently fails).
/// Chromium-family only: the browsers that can say whether the front window is private.
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
        return if is_unverifiable_browser(bundle_id) || opens_web_pages(bundle_id) {
            BrowserUrl::Unverified
        } else {
            BrowserUrl::NotBrowser
        };
    };
    let script = format!(
        "tell application \"{app}\"\n\
         if (count of windows) is 0 then return \"NOWIN\"\n\
         set w to front window\n\
         if mode of w is not \"normal\" then return \"PRIVATE\"\n\
         set t to active tab of w\n\
         return \"URL\t\" & (URL of t) & \"\t\" & (title of t)\n\
         end tell"
    );
    classify(run_osa(&script).as_deref())
}

thread_local! {
    /// Bundle id → whether macOS lists it as opening web pages. Asked once per app; a browser
    /// installed later arrives under a new id, so it is still asked about.
    static OPENS_WEB_PAGES: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

/// Any app that macOS lists as opening both web links and HTML files, other than a terminal, is a
/// browser. A list of names can't keep up with new browsers (Comet, Dia, …); this can.
fn opens_web_pages(bundle_id: &str) -> bool {
    if super::terminal::is_terminal(bundle_id) {
        return false;
    }
    if let Some(known) = OPENS_WEB_PAGES.with(|c| c.borrow().get(bundle_id).copied()) {
        return known;
    }
    let found = handlers_of_web_pages().iter().any(|id| id == bundle_id);
    OPENS_WEB_PAGES.with(|c| c.borrow_mut().insert(bundle_id.to_string(), found));
    found
}

/// Bundle ids of the apps registered to open `https` links AND `public.html` files.
fn handlers_of_web_pages() -> Vec<String> {
    let workspace = NSWorkspace::sharedWorkspace();
    let ids = |apps: &objc2_foundation::NSArray<NSURL>| -> Vec<String> {
        apps.iter()
            .filter_map(|app| NSBundle::bundleWithURL(&app)?.bundleIdentifier())
            .map(|id| id.to_string())
            .collect()
    };
    let Some(link) = NSURL::URLWithString(&NSString::from_str("https://example.com")) else {
        return Vec::new();
    };
    let Some(html) = UTType::typeWithIdentifier(&NSString::from_str("public.html")) else {
        return Vec::new();
    };
    let opens_html = ids(&workspace.URLsForApplicationsToOpenContentType(&html));
    ids(&workspace.URLsForApplicationsToOpenURL(&link))
        .into_iter()
        .filter(|id| opens_html.contains(id))
        .collect()
}

/// Browsers with no scriptable private-window flag, so a window's privacy can never be proven.
/// A backstop for the registry check above, in case macOS can't answer it.
fn is_unverifiable_browser(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.apple.Safari"
            | "com.apple.SafariTechnologyPreview"
            | "org.mozilla.firefox"
            | "org.mozilla.firefoxdeveloperedition"
            | "org.mozilla.nightly"
            | "org.torproject.torbrowser"
            | "app.zen-browser.zen"
            | "com.kagi.kagimacOS"
            | "com.duckduckgo.macos.browser"
            | "org.chromium.Chromium"
    )
}

/// What the Chromium script's output proves. Only an explicit normal-window answer keeps a
/// title; no window, a failure (Automation denied, a spawn error) or anything unexpected fails
/// closed, since none of them proves the title read a moment earlier wasn't a private window's.
fn classify(output: Option<&str>) -> BrowserUrl {
    match output {
        Some("PRIVATE") => BrowserUrl::Private,
        Some(s) => match s
            .strip_prefix("URL\t")
            .and_then(|rest| rest.split_once('\t'))
        {
            Some((url, title)) => BrowserUrl::Normal {
                url: url.to_string(),
                title: title.to_string(),
            },
            None => BrowserUrl::Unverified,
        },
        None => BrowserUrl::Unverified,
    }
}

/// Does this osascript stderr mean macOS denied the Apple Event (TCC Automation)?
/// Matched on the OSStatus, with the prose as belt-and-braces — macOS localises the message
/// but prints the code either way ("Not authorized to send Apple events … (-1743)").
fn stderr_is_automation_denial(stderr: &str) -> bool {
    stderr.contains("-1743") || stderr.contains("Not authorized to send Apple events")
}

/// Run a static AppleScript; `Some(stdout)` on success, `None` on any failure (fall back,
/// never retry). Every real outcome also feeds the Automation health latch: a success proves
/// the grant works; a -1743 on stderr proves macOS revoked it — the failure that used to be
/// indistinguishable from "no window open" and cost weeks of silent URL loss (D71).
fn run_osa(script: &str) -> Option<String> {
    let out = Command::new(OSASCRIPT)
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if out.status.success() {
        super::super::note_automation_ok();
        // Only the line ending: a trailing tab separates an empty tab title, so it has to stay.
        let stdout = String::from_utf8_lossy(&out.stdout);
        Some(stdout.trim_end_matches(['\n', '\r']).to_string())
    } else {
        if stderr_is_automation_denial(&String::from_utf8_lossy(&out.stderr)) {
            super::super::note_automation_denied();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, inspect, stderr_is_automation_denial, BrowserUrl};

    #[test]
    fn only_a_proven_normal_window_keeps_its_title() {
        assert_eq!(
            classify(Some("URL\thttps://github.com/x\tIssues · x")),
            BrowserUrl::Normal {
                url: "https://github.com/x".into(),
                title: "Issues · x".into()
            }
        );
        // A normal tab with no title still keeps its URL.
        assert_eq!(
            classify(Some("URL\thttps://github.com/x\t")),
            BrowserUrl::Normal {
                url: "https://github.com/x".into(),
                title: String::new()
            }
        );
        // No window left by the time the script ran: the title read earlier is unproven.
        assert_eq!(classify(Some("NOWIN")), BrowserUrl::Unverified);
        // A URL without the window's own title isn't enough either.
        assert_eq!(
            classify(Some("URL\thttps://github.com/x")),
            BrowserUrl::Unverified
        );
        assert_eq!(classify(Some("PRIVATE")), BrowserUrl::Private);
        // Automation denied or the script failed: can't prove it isn't private.
        assert_eq!(classify(None), BrowserUrl::Unverified);
        assert_eq!(classify(Some("something else")), BrowserUrl::Unverified);
    }

    #[test]
    fn a_browser_missing_from_every_list_is_still_found() {
        // Comet and Dia are on neither list; on a Mac that has them, macOS says they open web
        // pages. Skipped where they aren't installed.
        for id in ["ai.perplexity.comet", "company.thebrowser.dia"] {
            if super::handlers_of_web_pages().iter().any(|h| h == id) {
                assert_eq!(inspect(id), BrowserUrl::Unverified, "{id}");
            }
        }
        // The registry query itself works: Safari ships with every Mac.
        assert!(super::handlers_of_web_pages()
            .iter()
            .any(|h| h == "com.apple.Safari"));
        // A terminal that registers for web links keeps its titles.
        assert_eq!(inspect("com.googlecode.iterm2"), BrowserUrl::NotBrowser);
    }

    #[test]
    fn safari_and_firefox_windows_are_never_trusted() {
        // No osascript runs for these: they can't say whether a window is private.
        assert_eq!(inspect("com.apple.Safari"), BrowserUrl::Unverified);
        assert_eq!(inspect("org.mozilla.firefox"), BrowserUrl::Unverified);
        assert_eq!(inspect("com.microsoft.VSCode"), BrowserUrl::NotBrowser);
    }

    #[test]
    fn denial_stderr_is_recognised() {
        assert!(stderr_is_automation_denial(
            "execution error: Not authorized to send Apple events to Google Chrome. (-1743)"
        ));
        // Localised prose still carries the code.
        assert!(stderr_is_automation_denial("erreur d’exécution : (-1743)"));
    }

    #[test]
    fn other_failures_are_not_denials() {
        // A script/syntax error or a missing process must NOT latch the denial flag.
        assert!(!stderr_is_automation_denial(
            "execution error: Google Chrome got an error: Application isn’t running. (-600)"
        ));
        assert!(!stderr_is_automation_denial(
            "syntax error: Expected end of line (-2741)"
        ));
        assert!(!stderr_is_automation_denial(""));
    }
}
