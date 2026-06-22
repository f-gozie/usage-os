//! Spike #5 — project inference accuracy + the abstain threshold (R23/R26/R27).
//!
//! Spikes ①–④ proved we can *capture* the three project signals — window title
//! (①), browser URL (③), terminal cwd (④). This spike asks the different,
//! quality question: **how accurately can we turn those signals into a "project",
//! and when must we abstain?**
//!
//! For a calm rear-view mirror, a WRONG project label erodes trust more than a
//! missing one — so the design choice this spike settles is the **abstain
//! threshold**: emit a project only on an unambiguous, high-precision signal;
//! otherwise mark the activity *unassigned* rather than guess.
//!
//! It runs the heuristic over a corpus of REAL signals captured from this machine
//! (terminal cwds resolved via `git`, and representative browser tab URLs / editor
//! titles), and prints, per signal, the inferred project or the reason it abstained
//! — plus a summary. No network, no storage. No `unwrap()`/`expect()`/`panic!`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::process::Command;

fn main() {
    println!("project-inference spike — measure precision + the abstain threshold (R23/R26/R27)\n");

    println!("── Terminal cwds (resolved live via git) ─────────────────────────────");
    let cwds = [
        "/Users/favour/Documents/projects/combined/eyemark_frontend",
        "/Users/favour/Documents/projects/usage_os",
        "/Users/favour/Documents/projects/nudge",
        "/", // the 6 shells sitting at root — must abstain
    ];
    let mut results = Vec::new();
    for c in cwds {
        let inf = infer_cwd(c);
        print_row(&format!("cwd {c}"), &inf);
        results.push(inf);
    }

    println!("\n── Browser tab URLs (normal windows only — incognito already excluded) ─");
    let urls = [
        "https://github.com/usenudgeai/nudge",
        "https://github.com/f-gozie/usage-os/pull/6",
        "https://github.com/notifications",
        "http://localhost:3002/",
        "file:///Users/favour/Documents/projects/nudge/context/mockups/insights-share-card-adaptive.html",
        "https://www.youtube.com/watch?v=abc123",
        "https://x.com/Blue_Footy",
        "https://mail.google.com/mail/u/0/",
        "https://docs.google.com/document/d/xyz/edit",
        "https://www.google.com/search?q=rust+proc_pidinfo",
        "https://eu.posthog.com/project/123/dashboard",
        "https://favour.grafana.net/d/abc/overview",
        "https://dash.cloudflare.com/zone/dns",
        "https://appstoreconnect.apple.com/apps",
    ];
    for u in urls {
        let inf = infer_url(u);
        print_row(&format!("url {}", truncate(u, 52)), &inf);
        results.push(inf);
    }

    println!("\n── Editor / app window titles ────────────────────────────────────────");
    let titles = [
        "Browser Tab — nudge", // Cursor (Spike ② captured exactly this)
        "main.rs — usage_os",  // editor title carrying the folder
        "Claude",              // an app, no project
        "Spotify Premium",     // an app, no project
    ];
    for t in titles {
        let inf = infer_title(t);
        print_row(&format!("title {t:?}"), &inf);
        results.push(inf);
    }

    summarize(&results);
}

// ── Inference outcome ────────────────────────────────────────────────────────

enum Inference {
    Project {
        id: String,
        confidence: Conf,
        source: &'static str,
    },
    Abstain {
        kind: AbstainKind,
        reason: String,
    },
}

#[derive(Clone, Copy)]
enum Conf {
    High,
    Medium,
}

impl Conf {
    fn label(self) -> &'static str {
        match self {
            Conf::High => "HIGH",
            Conf::Medium => "MED",
        }
    }
}

/// Why a signal produced no project — the distinction matters downstream.
#[derive(Clone, Copy, PartialEq)]
enum AbstainKind {
    /// Genuinely no project signal (general browsing, a shell at `/`, an app).
    NoSignal,
    /// Clearly work, but the project is ambiguous (a dev dashboard, localhost).
    /// Phase 2 could *correlate* these to the concurrently-active project; the
    /// spike abstains rather than guess.
    Ambiguous,
}

// ── The heuristic ────────────────────────────────────────────────────────────

/// cwd → project: the highest-precision signal. A git repo's **remote** is the
/// canonical project id (`owner/repo`) — better than the folder name across
/// renames/forks; fall back to the folder name when there's no remote.
fn infer_cwd(path: &str) -> Inference {
    let Some(toplevel) = git(path, &["rev-parse", "--show-toplevel"]) else {
        return Inference::Abstain {
            kind: AbstainKind::NoSignal,
            reason: "not inside a git repo".to_string(),
        };
    };

    if let Some(remote) = git(path, &["remote", "get-url", "origin"]) {
        if let Some(id) = remote_to_id(&remote) {
            return Inference::Project {
                id,
                confidence: Conf::High,
                source: "cwd-git-remote",
            };
        }
    }
    Inference::Project {
        id: basename(&toplevel).to_string(),
        confidence: Conf::High,
        source: "cwd-folder",
    }
}

/// browser URL → project, or an explicit abstain.
fn infer_url(url: &str) -> Inference {
    let (scheme, host, path) = parse_url(url);

    // Local project file (e.g. a mockup opened from the repo).
    if scheme == "file" {
        if let Some(p) = project_from_path(&path) {
            return Inference::Project {
                id: p,
                confidence: Conf::Medium,
                source: "local-file",
            };
        }
        return abstain(AbstainKind::NoSignal, "local file, no project segment");
    }

    // GitHub owner/repo (R26) — the high-precision browser signal.
    if host.ends_with("github.com") {
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        match (segs.first(), segs.get(1)) {
            (Some(owner), Some(repo)) if !GITHUB_NON_REPO.contains(owner) => {
                return Inference::Project {
                    id: format!("{owner}/{repo}"),
                    confidence: Conf::High,
                    source: "github-url",
                };
            }
            _ => return abstain(AbstainKind::NoSignal, "github non-repo page"),
        }
    }

    // Local dev server — clearly work, but which project? Ambiguous.
    if host == "localhost" || host == "127.0.0.1" || host.starts_with("localhost:") {
        return abstain(AbstainKind::Ambiguous, "local dev server — project unknown");
    }

    // Dev-tool dashboards: work, but project-ambiguous (whose posthog/grafana?).
    if DEV_DASHBOARDS.iter().any(|d| host.ends_with(d)) {
        return abstain(AbstainKind::Ambiguous, "dev dashboard — project-ambiguous");
    }

    // Known general browsing → definitely no project.
    if GENERAL_HOSTS.iter().any(|d| host.ends_with(d)) {
        return abstain(AbstainKind::NoSignal, "general browsing");
    }

    abstain(AbstainKind::NoSignal, "no recognized project signal")
}

/// Editor titles often carry the folder (`<file> — <project>`). Lossy → Medium,
/// and never for bare app names.
fn infer_title(title: &str) -> Inference {
    // Split on em dash or hyphen-with-spaces; the trailing chunk is the candidate.
    let parts: Vec<&str> = title
        .split(" — ")
        .flat_map(|p| p.split(" - "))
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.len() < 2 {
        return abstain(AbstainKind::NoSignal, "bare app name / no project in title");
    }
    let candidate = parts[parts.len() - 1];
    if APP_NAMES.iter().any(|a| candidate.eq_ignore_ascii_case(a)) {
        return abstain(AbstainKind::NoSignal, "trailing token is an app name");
    }
    Inference::Project {
        id: candidate.to_string(),
        confidence: Conf::Medium,
        source: "window-title",
    }
}

// ── Vocabularies ─────────────────────────────────────────────────────────────

const GITHUB_NON_REPO: &[&str] = &[
    "settings",
    "notifications",
    "search",
    "marketplace",
    "sponsors",
    "orgs",
    "topics",
    "features",
    "about",
    "pricing",
    "new",
    "login",
    "join",
    "explore",
    "codespaces",
    "dashboard",
];

/// Project-ambiguous work tools — abstain (Phase 2 may correlate them).
const DEV_DASHBOARDS: &[&str] = &[
    "posthog.com",
    "grafana.net",
    "grafana.com",
    "cloudflare.com",
    "appstoreconnect.apple.com",
    "platform.claude.com",
    "console.anthropic.com",
    "vercel.com",
    "sentry.io",
    "supabase.com",
];

/// Definitely-not-a-project hosts.
const GENERAL_HOSTS: &[&str] = &[
    "youtube.com",
    "x.com",
    "twitter.com",
    "google.com",
    "reddit.com",
    "ycombinator.com",
    "stackoverflow.com",
    "claude.ai",
    "chatgpt.com",
    "linkedin.com",
    "facebook.com",
    "instagram.com",
];

const APP_NAMES: &[&str] = &[
    "Claude",
    "Spotify",
    "Spotify Premium",
    "Slack",
    "Finder",
    "Notion",
    "WhatsApp",
    "Google Chrome",
    "Brave",
];

// ── Output ───────────────────────────────────────────────────────────────────

fn abstain(kind: AbstainKind, reason: &str) -> Inference {
    Inference::Abstain {
        kind,
        reason: reason.to_string(),
    }
}

fn print_row(label: &str, inf: &Inference) {
    match inf {
        Inference::Project {
            id,
            confidence,
            source,
        } => {
            println!(
                "  ✅ {label:<54} → PROJECT {id:<26} [{} {}]",
                confidence.label(),
                source
            );
        }
        Inference::Abstain { kind, reason } => {
            let tag = match kind {
                AbstainKind::NoSignal => "abstain·no-signal",
                AbstainKind::Ambiguous => "abstain·ambiguous",
            };
            println!("  ·  {label:<54} → {tag:<18} ({reason})");
        }
    }
}

fn summarize(results: &[Inference]) {
    let mut assigned = 0usize;
    let mut no_signal = 0usize;
    let mut ambiguous = 0usize;
    let mut projects: Vec<String> = Vec::new();

    for r in results {
        match r {
            Inference::Project { id, .. } => {
                assigned += 1;
                if !projects.contains(id) {
                    projects.push(id.clone());
                }
            }
            Inference::Abstain { kind, .. } => match kind {
                AbstainKind::NoSignal => no_signal += 1,
                AbstainKind::Ambiguous => ambiguous += 1,
            },
        }
    }

    let total = results.len();
    println!("\n── Summary ───────────────────────────────────────────────────────────");
    println!("  signals:            {total}");
    println!("  → assigned project: {assigned}");
    println!("  → abstain (no signal, correctly unassigned): {no_signal}");
    println!("  → abstain (ambiguous — work, project unknown): {ambiguous}");
    projects.sort();
    println!("  distinct projects:  {}", projects.join(", "));
    println!(
        "\n  Abstain threshold: emit a project only at HIGH (cwd-git-remote, github-url) or\n  \
         MEDIUM (local-file, window-title) confidence AND unambiguous. Everything else stays\n  \
         UNASSIGNED — a wrong label costs more trust than a gap (R27)."
    );
}

// ── Plumbing ─────────────────────────────────────────────────────────────────

/// Run a `git -C <dir> ...` and return trimmed stdout on success.
fn git(dir: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}

/// `git@github.com:f-gozie/usage-os.git` or `https://github.com/f-gozie/usage-os`
/// → `f-gozie/usage-os`. Take the last two path-ish segments, minus `.git`.
fn remote_to_id(remote: &str) -> Option<String> {
    let norm = remote.trim().trim_end_matches(".git").replace(':', "/");
    let segs: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    let n = segs.len();
    if n >= 2 {
        Some(format!("{}/{}", segs[n - 2], segs[n - 1]))
    } else {
        None
    }
}

/// Split a URL into (scheme, host, path). `file://` URLs have an empty host.
fn parse_url(url: &str) -> (String, String, String) {
    let Some((scheme, rest)) = url.split_once("://") else {
        return (String::new(), String::new(), url.to_string());
    };
    if scheme == "file" {
        return (scheme.to_string(), String::new(), rest.to_string());
    }
    match rest.split_once('/') {
        Some((host, path)) => (scheme.to_string(), host.to_string(), format!("/{path}")),
        None => (scheme.to_string(), rest.to_string(), String::new()),
    }
}

/// Extract a project name from a local file path: the segment after `projects/`.
fn project_from_path(path: &str) -> Option<String> {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let i = segs.iter().position(|&s| s == "projects")?;
    segs.get(i + 1).map(|s| s.to_string())
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
