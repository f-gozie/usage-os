//! Project inference (D30) — port of `spikes/project-inference`, made
//! persistence-aware. Turns capture signals (terminal cwd, browser url, window
//! title) into a `project_id` via the repository, or an explicit abstain.
//!
//! The canonical identity is the git remote `owner/repo` (folder name as fallback);
//! weak signals (folder/title/file names) are reconciled to an existing project by
//! a `"name"` alias before a new project is created, so one project never fragments
//! (the spike's headline finding). A wrong label costs more trust than a gap, so we
//! assign only on a confident, unambiguous signal — else persist the abstain *kind*
//! (`no-signal` vs `ambiguous`) for later correlation.

use std::process::Command;

use rusqlite::{Connection, Result};

use crate::db;

/// The live signals available for one focus span.
#[derive(Debug, Default, Clone)]
pub struct ProjectSignals<'a> {
    pub cwd: Option<&'a str>,
    pub url: Option<&'a str>,
    pub title: Option<&'a str>,
}

/// What inference resolved to, ready to persist on the event.
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectAssignment {
    /// Resolved (or created) project id.
    Assigned(i64),
    /// No project; the reason persists for Phase-2 correlation:
    /// `"ambiguous"` (work, project unknown) or `"no-signal"`.
    Abstain(&'static str),
}

/// A resolved-but-not-yet-persisted project, plus how to reconcile it.
struct Candidate {
    canonical_key: String,
    display_name: String,
    remote_url: Option<String>,
    /// Basename to register/look up under the `"name"` alias (folder/repo name).
    name_alias: Option<String>,
    /// Weak signals (title/file/folder name) reconcile to an existing project by
    /// `name_alias` before treating `canonical_key` as new.
    weak: bool,
}

enum Classified {
    Project(Candidate),
    Ambiguous,
    NoSignal,
}

/// Infer and persist the project for a span. Tries signals in precedence order —
/// cwd (anchor) → url → title — assigning on the first confident hit; otherwise
/// abstains, preferring `ambiguous` if any signal was work-but-project-unknown.
pub fn infer_project(conn: &Connection, sig: &ProjectSignals) -> Result<ProjectAssignment> {
    if let Some(cwd) = sig.cwd {
        if let Classified::Project(c) = classify_cwd(cwd) {
            return Ok(ProjectAssignment::Assigned(resolve(conn, &c)?));
        }
    }

    let mut saw_ambiguous = false;
    if let Some(url) = sig.url {
        match classify_url(url) {
            Classified::Project(c) => return Ok(ProjectAssignment::Assigned(resolve(conn, &c)?)),
            Classified::Ambiguous => saw_ambiguous = true,
            Classified::NoSignal => {}
        }
    }

    if let Some(title) = sig.title {
        if let Classified::Project(c) = classify_title(title) {
            return Ok(ProjectAssignment::Assigned(resolve(conn, &c)?));
        }
    }

    Ok(ProjectAssignment::Abstain(if saw_ambiguous {
        "ambiguous"
    } else {
        "no-signal"
    }))
}

/// Persist a candidate to a `project_id`. Weak candidates first try to reconcile
/// to an existing project by their `"name"` alias (D30 — no fragmentation).
fn resolve(conn: &Connection, c: &Candidate) -> Result<i64> {
    if c.weak {
        if let Some(name) = &c.name_alias {
            if let Some(pid) = db::find_project_by_alias(conn, "name", name)? {
                return Ok(pid);
            }
        }
    }
    let aliases: Vec<(&str, &str)> = c
        .name_alias
        .as_deref()
        .map(|n| vec![("name", n)])
        .unwrap_or_default();
    db::resolve_or_create_project(
        conn,
        &c.canonical_key,
        &c.display_name,
        c.remote_url.as_deref(),
        &aliases,
    )
}

// ── Classification (pure + git shell; no DB) ─────────────────────────────────

/// cwd → project: the highest-precision signal. The git remote `owner/repo` is the
/// canonical id (stable across renames/forks); fall back to the folder name.
fn classify_cwd(path: &str) -> Classified {
    let Some(toplevel) = git(path, &["rev-parse", "--show-toplevel"]) else {
        return Classified::NoSignal; // not inside a git repo
    };
    let folder = basename(&toplevel).to_string();

    if let Some(remote) = git(path, &["remote", "get-url", "origin"]) {
        if let Some(id) = remote_to_id(&remote) {
            let display = basename(&id).to_string();
            return Classified::Project(Candidate {
                canonical_key: id,
                display_name: display,
                remote_url: Some(remote),
                name_alias: Some(folder),
                weak: false,
            });
        }
    }
    // No remote → folder name is the canonical key.
    Classified::Project(Candidate {
        canonical_key: folder.clone(),
        display_name: folder.clone(),
        remote_url: None,
        name_alias: Some(folder),
        weak: false,
    })
}

/// browser URL → project, or an abstain (ambiguous vs no-signal).
fn classify_url(url: &str) -> Classified {
    let (scheme, host, path) = parse_url(url);

    if scheme == "file" {
        return match project_from_path(&path) {
            Some(name) => Classified::Project(Candidate {
                canonical_key: name.clone(),
                display_name: name.clone(),
                remote_url: None,
                name_alias: Some(name),
                weak: true,
            }),
            None => Classified::NoSignal,
        };
    }

    if host.ends_with("github.com") {
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        match (segs.first(), segs.get(1)) {
            (Some(owner), Some(repo)) if !GITHUB_NON_REPO.contains(owner) => {
                let id = format!("{owner}/{repo}");
                return Classified::Project(Candidate {
                    canonical_key: id.clone(),
                    display_name: (*repo).to_string(),
                    remote_url: Some(format!("https://github.com/{id}")),
                    name_alias: Some((*repo).to_string()),
                    weak: false,
                });
            }
            _ => return Classified::NoSignal, // github non-repo page
        }
    }

    if host == "localhost" || host == "127.0.0.1" || host.starts_with("localhost:") {
        return Classified::Ambiguous; // local dev server — project unknown
    }
    if DEV_DASHBOARDS.iter().any(|d| host.ends_with(d)) {
        return Classified::Ambiguous; // dev dashboard — project-ambiguous
    }
    if GENERAL_HOSTS.iter().any(|d| host.ends_with(d)) {
        return Classified::NoSignal; // general browsing
    }
    Classified::NoSignal
}

/// Editor titles often carry the folder (`<file> — <project>`). Lossy → weak, and
/// never for bare app names.
fn classify_title(title: &str) -> Classified {
    let parts: Vec<&str> = title
        .split(" — ")
        .flat_map(|p| p.split(" - "))
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.len() < 2 {
        return Classified::NoSignal; // bare app name / no project in title
    }
    let candidate = parts[parts.len() - 1];
    if APP_NAMES.iter().any(|a| candidate.eq_ignore_ascii_case(a)) {
        return Classified::NoSignal;
    }
    Classified::Project(Candidate {
        canonical_key: candidate.to_string(),
        display_name: candidate.to_string(),
        remote_url: None,
        name_alias: Some(candidate.to_string()),
        weak: true,
    })
}

// ── Vocabularies (seeded from the spike; data-driven later) ──────────────────

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

// ── Plumbing ─────────────────────────────────────────────────────────────────

/// Run `git -C <dir> ...`, returning trimmed stdout on success. Shells out (the
/// consumer runs on a dedicated thread, so this never blocks the async executor).
fn git(dir: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `git@github.com:f-gozie/usage-os.git` or `https://github.com/f-gozie/usage-os`
/// → `f-gozie/usage-os` (last two path-ish segments, minus `.git`).
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
pub(crate) fn parse_url(url: &str) -> (String, String, String) {
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

/// Project name from a local file path: the segment after `projects/`.
fn project_from_path(path: &str) -> Option<String> {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let i = segs.iter().position(|&s| s == "projects")?;
    segs.get(i + 1).map(|s| s.to_string())
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        db::run_migrations(&conn).expect("migrations");
        conn
    }

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// Create a throwaway git repo with a remote; returns its path. Caller removes it.
    fn temp_git_repo(remote: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "usageos_enrich_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
        };
        run(&["init"]);
        run(&["remote", "add", "origin", remote]);
        dir
    }

    fn key_of(conn: &Connection, id: i64) -> String {
        db::get_project(conn, id).unwrap().unwrap().canonical_key
    }

    #[test]
    fn cwd_git_remote_is_canonical() {
        let conn = test_db();
        let repo = temp_git_repo("git@github.com:f-gozie/usage-os.git");
        let sig = ProjectSignals {
            cwd: Some(repo.to_str().unwrap()),
            ..Default::default()
        };
        let a = infer_project(&conn, &sig).unwrap();
        let ProjectAssignment::Assigned(id) = a else {
            panic!("expected Assigned, got {a:?}");
        };
        assert_eq!(key_of(&conn, id), "f-gozie/usage-os");
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn cwd_and_title_do_not_fragment() {
        // The repo folder basename must reconcile a later title signal to the SAME
        // project (D30). temp dir basename == the folder name we register.
        let conn = test_db();
        let repo = temp_git_repo("git@github.com:f-gozie/usage-os.git");
        let folder = repo.file_name().unwrap().to_str().unwrap().to_string();

        let from_cwd = infer_project(
            &conn,
            &ProjectSignals {
                cwd: Some(repo.to_str().unwrap()),
                ..Default::default()
            },
        )
        .unwrap();
        let from_title = infer_project(
            &conn,
            &ProjectSignals {
                title: Some(&format!("main.rs — {folder}")),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(from_cwd, from_title, "same project, not fragmented");
        assert_eq!(db::get_projects(&conn).unwrap().len(), 1);
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn github_url_assigns_owner_repo() {
        let conn = test_db();
        let a = infer_project(
            &conn,
            &ProjectSignals {
                url: Some("https://github.com/usenudgeai/nudge"),
                ..Default::default()
            },
        )
        .unwrap();
        let ProjectAssignment::Assigned(id) = a else {
            panic!("expected Assigned");
        };
        assert_eq!(key_of(&conn, id), "usenudgeai/nudge");
    }

    #[test]
    fn github_non_repo_abstains() {
        let conn = test_db();
        assert_eq!(
            infer_project(
                &conn,
                &ProjectSignals {
                    url: Some("https://github.com/notifications"),
                    ..Default::default()
                }
            )
            .unwrap(),
            ProjectAssignment::Abstain("no-signal")
        );
    }

    #[test]
    fn localhost_and_dashboards_are_ambiguous() {
        let conn = test_db();
        for url in [
            "http://localhost:3002/",
            "https://eu.posthog.com/project/1",
            "https://favour.grafana.net/d/x",
        ] {
            assert_eq!(
                infer_project(
                    &conn,
                    &ProjectSignals {
                        url: Some(url),
                        ..Default::default()
                    }
                )
                .unwrap(),
                ProjectAssignment::Abstain("ambiguous"),
                "{url}"
            );
        }
    }

    #[test]
    fn general_browsing_is_no_signal() {
        let conn = test_db();
        for url in [
            "https://www.youtube.com/watch?v=x",
            "https://mail.google.com/mail/u/0/",
            "https://x.com/someone",
        ] {
            assert_eq!(
                infer_project(
                    &conn,
                    &ProjectSignals {
                        url: Some(url),
                        ..Default::default()
                    }
                )
                .unwrap(),
                ProjectAssignment::Abstain("no-signal"),
                "{url}"
            );
        }
    }

    #[test]
    fn file_url_assigns_project_segment() {
        let conn = test_db();
        let a = infer_project(
            &conn,
            &ProjectSignals {
                url: Some("file:///Users/x/Documents/projects/nudge/mockup.html"),
                ..Default::default()
            },
        )
        .unwrap();
        let ProjectAssignment::Assigned(id) = a else {
            panic!("expected Assigned");
        };
        assert_eq!(key_of(&conn, id), "nudge");
    }

    #[test]
    fn bare_app_title_is_no_signal() {
        let conn = test_db();
        assert_eq!(
            infer_project(
                &conn,
                &ProjectSignals {
                    title: Some("Spotify Premium"),
                    ..Default::default()
                }
            )
            .unwrap(),
            ProjectAssignment::Abstain("no-signal")
        );
    }

    #[test]
    fn remote_to_id_handles_ssh_and_https() {
        assert_eq!(
            remote_to_id("git@github.com:f-gozie/usage-os.git").as_deref(),
            Some("f-gozie/usage-os")
        );
        assert_eq!(
            remote_to_id("https://github.com/usenudgeai/nudge").as_deref(),
            Some("usenudgeai/nudge")
        );
    }
}
