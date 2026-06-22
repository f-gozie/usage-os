//! Spike ③ — browser front-tab URL capture + incognito/private exclusion.
//!
//! The UsageOS "site" axis wants the URL of the active browser tab — but D8 is
//! non-negotiable: an **incognito / private** URL must never be read, let alone
//! stored. This spike proves the mechanism and the privacy-safe fallback ladder,
//! Accessibility-independent (it uses **Apple Events** / Automation, a separate
//! permission), shelling out to `/usr/bin/osascript` with **static** scripts
//! (capture standard C7 — process-isolated, no injection surface):
//!
//!   R15 — Chromium front-tab URL via `URL of active tab of front window`.
//!   R16 — Safari URL via `URL of front document`.
//!   R17 — **incognito excluded by reading `mode of front window` FIRST** — and
//!         `mode` is actually *readable* (the sdef's "set only once" wording cast
//!         doubt). Any non-`"normal"` mode ⇒ skip the URL entirely.
//!   R18 — Safari Private Browsing has no scriptable property → **safe-default**:
//!         never emit a Safari URL we can't prove is non-private.
//!   R19 — Automation TCC is per-(client,target): the first query to each browser
//!         prompts once; denial returns `-1743` **permanently** → fall back, never
//!         retry. [capture standard C6]
//!   R21 — osascript shell-out latency is acceptable for event-driven capture
//!         (we query on app-switch, not in a hot loop).
//!
//! The fallback ladder (C6), privacy-safe at every rung:
//!   URL via Automation
//!     └─ incognito/private window      → skip (no URL)
//!     └─ denied (-1743) / unsupported  → (title-derived site, handled upstream)
//!          └─ app-level only
//!
//! This binary **only reads and prints** — it never stores. It touches no
//! network and no disk. Hard rule 3: no `unwrap()`/`expect()`/`panic!`.
//!
//! NOTE — frontmost-app detection is intentionally NOT here: each browser's
//! `front window` is queried directly, so there is no NSWorkspace/objc2 use and
//! no run-loop staleness. In the real app, Spike ②'s event-driven capture says
//! *when* a browser is frontmost; this query says *what* it's showing.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::collections::HashMap;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TICK: Duration = Duration::from_millis(1200);
const OSASCRIPT: &str = "/usr/bin/osascript";

#[derive(Clone, Copy, PartialEq)]
enum Engine {
    Chromium,
    /// Arc — Chromium-based scripting but a different window/space model; flagged
    /// in output for manual confirmation (R17 open question).
    ChromiumArc,
    Safari,
}

struct Browser {
    app: &'static str,
    engine: Engine,
}

/// AppleScript app names matter: Brave is `"Brave Browser"`, Edge is
/// `"Microsoft Edge"` — hardcoding `"Brave"` silently fails (R15).
const BROWSERS: &[Browser] = &[
    Browser {
        app: "Google Chrome",
        engine: Engine::Chromium,
    },
    Browser {
        app: "Brave Browser",
        engine: Engine::Chromium,
    },
    Browser {
        app: "Microsoft Edge",
        engine: Engine::Chromium,
    },
    Browser {
        app: "Arc",
        engine: Engine::ChromiumArc,
    },
    Browser {
        app: "Safari",
        engine: Engine::Safari,
    },
];

fn main() {
    println!("browser-url spike ③ — front-tab URL + incognito/private exclusion (Apple Events)");
    println!(
        "Reads only, never stores. The FIRST query to each browser prompts for Automation — \
         click Allow. Ctrl-C to stop.\n"
    );

    let running: Vec<&str> = BROWSERS
        .iter()
        .filter(|b| is_running(b.app))
        .map(|b| b.app)
        .collect();
    if running.is_empty() {
        println!("No supported browsers are running. Open Chrome / Brave / Safari and try again.");
    } else {
        println!("Running browsers detected: {}", running.join(", "));
    }
    println!("Switch tabs and open incognito / private windows to exercise each.\n");

    // Dedupe: print a browser's line only when its state changes.
    let mut last: HashMap<&'static str, String> = HashMap::new();

    loop {
        for b in BROWSERS {
            if !is_running(b.app) {
                continue;
            }
            let (res, ms) = query(b);
            let key = res.key();
            if last.get(b.app) == Some(&key) {
                continue;
            }
            last.insert(b.app, key);

            let tag = if matches!(b.engine, Engine::ChromiumArc) {
                "   (Arc: Chromium-family — verify mode/tab semantics [R17])"
            } else {
                ""
            };
            println!(
                "[{ts}] {app:<15} {desc}   ({ms} ms){tag}",
                ts = now_hms(),
                app = b.app,
                desc = res.describe(),
                ms = ms,
                tag = tag,
            );
        }
        sleep(TICK);
    }
}

// ── Querying ─────────────────────────────────────────────────────────────────

/// The classified outcome of one browser query.
enum UrlResult {
    /// A real URL from a normal (non-private) window.
    Url(String),
    /// A Safari URL read **without** private-browsing enforcement — spike-only,
    /// loudly flagged. Production must add the R18 safe-default before reading.
    SafariUnenforced(String),
    /// A private/incognito window — URL deliberately NOT read (D8). Carries the
    /// `mode` value we saw (e.g. `"incognito"`).
    PrivateSkipped(String),
    /// The browser is running but has no windows/documents.
    NoWindow,
    /// Automation denied (`-1743`) — permanent; fall back, never retry.
    NotAuthorized,
    /// Anything else (unexpected AppleScript error).
    Error(String),
}

impl UrlResult {
    /// Dedupe key (so unchanged state isn't reprinted every tick).
    fn key(&self) -> String {
        match self {
            UrlResult::Url(u) => format!("url:{u}"),
            UrlResult::SafariUnenforced(u) => format!("safari:{u}"),
            UrlResult::PrivateSkipped(m) => format!("private:{m}"),
            UrlResult::NoWindow => "nowin".to_string(),
            UrlResult::NotAuthorized => "noauth".to_string(),
            UrlResult::Error(e) => format!("err:{e}"),
        }
    }

    fn describe(&self) -> String {
        match self {
            UrlResult::Url(u) => format!("URL  {u}"),
            UrlResult::SafariUnenforced(u) => format!(
                "URL  {u}   ⚠ private-detection NOT enforced (spike) — production safe-defaults to skip [R18]"
            ),
            UrlResult::PrivateSkipped(m) => {
                format!("SKIPPED-PRIVATE (mode={m}) — URL not read   ✅ D8")
            }
            UrlResult::NoWindow => "no windows".to_string(),
            UrlResult::NotAuthorized => {
                "NOT AUTHORIZED (-1743) → fall back to title-derived site   ✅ [R19/C6]".to_string()
            }
            UrlResult::Error(e) => format!("error: {e}"),
        }
    }
}

/// Query one browser, returning the classified result and the round-trip ms.
fn query(b: &Browser) -> (UrlResult, u128) {
    let script = match b.engine {
        Engine::Chromium | Engine::ChromiumArc => chromium_script(b.app),
        Engine::Safari => safari_script(),
    };
    let start = Instant::now();
    let res = run_osa(&script);
    let ms = start.elapsed().as_millis();
    let outcome = match res {
        Ok(s) => classify_ok(&s, b.engine),
        Err(e) => classify_err(&e),
    };
    (outcome, ms)
}

/// Chromium family: **mode first**, skip anything that isn't explicitly
/// `"normal"` (incognito, guest, …), only then read the URL. A literal tab
/// separates the tag from the value.
fn chromium_script(app: &str) -> String {
    format!(
        "tell application \"{app}\"\n\
         if (count of windows) is 0 then return \"NOWIN\"\n\
         set m to mode of front window\n\
         if m is not \"normal\" then return \"PRIVATE\t\" & m\n\
         return \"URL\t\" & (URL of active tab of front window)\n\
         end tell"
    )
}

/// Safari: WebKit document model. Read-only here and flagged unenforced — Safari
/// exposes no private-browsing property (R18), so production must add the
/// System-Events safe-default check before this URL may be used.
fn safari_script() -> String {
    "tell application \"Safari\"\n\
     if (count of documents) is 0 then return \"NOWIN\"\n\
     return \"URL\t\" & (URL of front document)\n\
     end tell"
        .to_string()
}

fn classify_ok(s: &str, engine: Engine) -> UrlResult {
    if s == "NOWIN" {
        return UrlResult::NoWindow;
    }
    if let Some(rest) = s.strip_prefix("PRIVATE\t") {
        return UrlResult::PrivateSkipped(rest.to_string());
    }
    if let Some(rest) = s.strip_prefix("URL\t") {
        return match engine {
            Engine::Safari => UrlResult::SafariUnenforced(rest.to_string()),
            _ => UrlResult::Url(rest.to_string()),
        };
    }
    UrlResult::Error(format!("unexpected output: {s:?}"))
}

fn classify_err(e: &str) -> UrlResult {
    if e.contains("-1743") {
        UrlResult::NotAuthorized
    } else {
        UrlResult::Error(e.to_string())
    }
}

// ── osascript plumbing ───────────────────────────────────────────────────────

/// Run a static AppleScript via `/usr/bin/osascript`. `Ok(stdout)` on success,
/// `Err(stderr)` otherwise (which `classify_err` maps to `-1743` / other).
fn run_osa(script: &str) -> Result<String, String> {
    match Command::new(OSASCRIPT).arg("-e").arg(script).output() {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        Err(e) => Err(format!("spawn failed: {e}")),
    }
}

/// `application "X" is running` — a Launch Services query: it does **not** send
/// an Apple Event, so it never prompts and never launches a non-running app.
fn is_running(app: &str) -> bool {
    let script = format!("application \"{app}\" is running");
    matches!(run_osa(&script).as_deref(), Ok("true"))
}

// ── Small helpers ────────────────────────────────────────────────────────────

fn now_hms() -> String {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };
    let s = secs % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}
