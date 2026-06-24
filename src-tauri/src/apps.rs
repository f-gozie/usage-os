//! Installed-app catalog + icons — so the UI can show real app branding (the
//! Timeline switch rows, the dial legend, Settings) instead of bare squares.
//!
//! **Offline, no new permission, not in the data path** (hard rule 1): it only reads
//! public `.app` bundles under the standard app directories (incl. Chrome/Brave PWA
//! folders) and extracts each icon's loose `.icns` to a cached 64px PNG via `sips`
//! (the same subprocess posture as `enrich`'s `git`). Every failure degrades to
//! `icon: None` — the UI falls back to a monogram — so there is no `unwrap`/`expect`/
//! `panic` here (hard rule 3). On non-macOS (CI Linux) the app dirs are absent and
//! `sips` is missing, so this simply returns an empty/iconless catalog.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// One user-facing application and (best-effort) its icon as a data-URI PNG.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct InstalledApp {
    /// Display name — the bundle's file stem (e.g. "Visual Studio Code", "Nudge — …").
    pub name: String,
    /// `data:image/png;base64,…`, or `None` when no icon could be extracted.
    pub icon: Option<String>,
}

/// `.icns` stems that conventionally name the *app* icon (vs document icons).
const APP_ICON_HINTS: &[&str] = &["appicon", "app", "icon", "electron"];
/// PNG target edge (px). Matches the dogfood-proven `sips -Z 64` (~5–15 KB).
const ICON_PX: &str = "64";

/// The standard places macOS apps live, including Chrome/Brave **PWA** folders
/// (`*.localized`) — which is where installed web apps like Nudge keep their icon.
pub fn default_search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(&home).join("Applications"));
    }
    dirs
}

/// Enumerate apps under `search_dirs` (recursing one level into `*.localized` PWA
/// folders), extract each icon to `cache_dir`, and return the catalog sorted by name.
/// De-duplicates by lowercased name (the first bundle wins).
pub fn list_installed(search_dirs: &[PathBuf], cache_dir: &Path) -> Vec<InstalledApp> {
    let _ = std::fs::create_dir_all(cache_dir);
    let mut apps: Vec<InstalledApp> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for dir in search_dirs {
        collect_dir(dir, cache_dir, &mut apps, &mut seen, true);
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn collect_dir(
    dir: &Path,
    cache_dir: &Path,
    apps: &mut Vec<InstalledApp>,
    seen: &mut HashSet<String>,
    recurse_localized: bool,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return; // absent / unreadable dir (e.g. on CI Linux) — skip, not fatal.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = fname.strip_suffix(".app") {
            if !seen.insert(stem.to_lowercase()) {
                continue; // already have an app by this name
            }
            apps.push(InstalledApp {
                name: stem.to_string(),
                icon: extract_icon(&path, stem, cache_dir),
            });
        } else if recurse_localized && fname.ends_with(".localized") && path.is_dir() {
            // Chrome/Brave PWA containers — one level deep only.
            collect_dir(&path, cache_dir, apps, seen, false);
        }
    }
}

/// Extract `app`'s icon to a cached 64px PNG and return it as a data-URI, or `None`.
fn extract_icon(app: &Path, stem: &str, cache_dir: &Path) -> Option<String> {
    let icns = best_icns(app, stem)?;
    let png = cache_dir.join(format!("{}.png", cache_key(stem)));
    if !is_cached_fresh(&png, &icns) {
        let ok = Command::new("sips")
            .args(["-s", "format", "png", "-Z", ICON_PX])
            .arg(&icns)
            .arg("--out")
            .arg(&png)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            return None; // sips absent (non-macOS) or unreadable icns
        }
    }
    let bytes = std::fs::read(&png).ok()?;
    Some(format!("data:image/png;base64,{}", b64(&bytes)))
}

/// Pick the bundle's *app* icon among its `.icns` files: prefer one whose stem matches
/// the app name or a conventional app-icon name; otherwise the largest (app icons carry
/// the most resolutions, so they're typically biggest — document icons are smaller).
fn best_icns(app: &Path, stem: &str) -> Option<PathBuf> {
    let res = app.join("Contents/Resources");
    let entries = std::fs::read_dir(&res).ok()?;
    let want = stem.to_lowercase();
    let mut best: Option<(PathBuf, i32, u64)> = None; // (path, score, size)
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("icns") {
            continue;
        }
        let fstem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let mut score = 0;
        if fstem == want {
            score += 3;
        }
        if APP_ICON_HINTS.contains(&fstem.as_str()) {
            score += 2;
        }
        let better = match &best {
            None => true,
            Some((_, bscore, bsize)) => score > *bscore || (score == *bscore && size > *bsize),
        };
        if better {
            best = Some((path, score, size));
        }
    }
    best.map(|(p, _, _)| p)
}

/// Cache filename stem: keep it filesystem-safe and stable per app name.
fn cache_key(stem: &str) -> String {
    stem.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// A cached PNG is reusable iff it exists and is at least as new as its source `.icns`.
fn is_cached_fresh(png: &Path, icns: &Path) -> bool {
    let (Ok(p), Ok(i)) = (std::fs::metadata(png), std::fs::metadata(icns)) else {
        return false;
    };
    match (p.modified(), i.modified()) {
        (Ok(pm), Ok(im)) => pm >= im,
        _ => false,
    }
}

/// Standard base64 (RFC 4648) — hand-rolled to avoid a dependency (the project keeps
/// its supply chain auditable, hard rule 1). PNG bytes → ASCII; no panics.
fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0);
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn b64_matches_known_vectors() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64(b"foob"), "Zm9vYg==");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn enumerates_apps_and_recurses_pwa_folders() {
        let root = std::env::temp_dir().join(format!("usageos-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let appdir = root.join("Applications");
        // A normal app + a PWA inside a *.localized container.
        write(&appdir.join("Foo.app/Contents/Resources/Foo.icns"), b"x");
        write(
            &appdir.join("Web Apps.localized/Bar.app/Contents/Resources/app.icns"),
            b"y",
        );
        let cache = root.join("cache");

        let apps = list_installed(std::slice::from_ref(&appdir), &cache);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "found top-level app: {names:?}");
        assert!(
            names.contains(&"Bar"),
            "recursed into PWA folder: {names:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn best_icns_prefers_name_match_over_largest() {
        let root = std::env::temp_dir().join(format!("usageos-icns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = root.join("Cursor.app");
        let res = app.join("Contents/Resources");
        write(&res.join("document.icns"), &[0u8; 5000]); // big, but a doc icon
        write(&res.join("Cursor.icns"), &[0u8; 100]); // small, but the app icon
        let pick = best_icns(&app, "Cursor").expect("an icns was chosen");
        assert_eq!(pick.file_name().unwrap().to_str().unwrap(), "Cursor.icns");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// End-to-end against the real `/Applications` (needs `sips`, macOS). Ignored so CI
    /// (Linux) skips it; run explicitly: `cargo test real_machine_icons -- --ignored --nocapture`.
    #[test]
    #[ignore = "hits real /Applications + sips; macOS-only"]
    fn real_machine_icons() {
        let cache = std::env::temp_dir().join("usageos-icon-cache-test");
        let apps = list_installed(&default_search_dirs(), &cache);
        let with_icons = apps.iter().filter(|a| a.icon.is_some()).count();
        println!("apps found: {}, with icons: {}", apps.len(), with_icons);
        assert!(apps.len() > 5, "should enumerate several apps");
        assert!(with_icons > 0, "should extract at least some real icons");
    }
}
