#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

// The typed repository layer (the lib's data boundary). `pub` so its API is reachable ahead of
// its consumers (otherwise not-yet-wired fns trip `dead_code` under `-D warnings`).
pub mod db;
// The observation boundary (hard rule 5): all native/objc2 code behind the `CaptureSource` trait.
pub mod capture;
// Turns raw capture signals into stored facts (site, project — D30).
mod enrich;
// The forward-only SQL migration runner; paired with the crate-root `migrations/` dir.
mod migrations;
// The pure read-time layer that turns a day's events into the view the dial renders (hard rule 6).
mod rollup;
// Installed-app catalog + offline icon extraction, isolated like the other native surfaces.
mod apps;
// macOS capture permissions (Accessibility + Automation) surfaced to onboarding + Settings.
mod permissions;
// The menubar tray icon: the mono Contexts mark + a now-triangle pointing at the current hour.
mod tray_icon;
// macOS NSPanel reclass so the menubar glance floats over full-screen Spaces (D56).
#[cfg(target_os = "macos")]
mod glance_panel;

// The recap-narration seam (hard rule 5): a mockable `Narrator` + `build_recap` with a
// deterministic template fallback (D48). `pub` so its not-yet-wired API isn't dead-code.
pub mod ai;

// Dev-only synthetic-history generator for the Phase-6 perf/stress harness. Behind the
// `perf` cargo feature so it never compiles into the shipped binary; seeds through the
// repository layer (hard rule 4) so the read path under test is the real one.
#[cfg(feature = "perf")]
pub mod perf;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Manager, PhysicalPosition, Position, Rect, Size, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_specta::{collect_commands, Builder};

type DbState = Arc<Mutex<Connection>>;

/// Typed error contract for every command (replaces `Result<_, String>`).
/// Crosses to TS as a discriminated union the frontend can `switch` on `kind`.
#[derive(Debug, thiserror::Error, Serialize, specta::Type)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("database error: {0}")]
    Db(String),
    #[error("database lock was poisoned")]
    LockPoisoned,
    #[error("autostart error: {0}")]
    Autostart(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

/// Health of the background capture watcher (replaces an untyped `serde_json::Value`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WatcherStatus {
    pub consecutive_errors: u64,
    pub healthy: bool,
}

/// One persisted setting key/value (replaces the awkward `[string, string][]`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

/// Fetch activity logs for a `[start_time, end_time]` Unix-second range.
#[tauri::command]
#[specta::specta]
fn get_activity_stats(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<db::ActivityLog>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_activity_logs(&conn, start_time, end_time).map_err(AppError::from)
}

/// The category + project lookup maps the rollup reads off: `category_id -> meta` and
/// `project_id -> display name`.
type LookupMaps = (HashMap<i64, rollup::CategoryMeta>, HashMap<i64, String>);

/// The category + project lookup maps the three rollup commands share — one query each.
fn load_lookup_maps(conn: &Connection) -> Result<LookupMaps, AppError> {
    let categories = db::get_category_metas(conn)?
        .into_iter()
        .map(|(id, slug, name, color)| (id, rollup::CategoryMeta { slug, name, color }))
        .collect();
    let projects = db::get_projects(conn)?
        .into_iter()
        .map(|p| (p.id, p.display_name))
        .collect();
    Ok((categories, projects))
}

/// Build the Day view — per-axis category aggregates, category-runs, and the template
/// recap (D34) — for a `[start_time, end_time]` Unix-second range. Numbers are computed
/// in Rust (hard rule 6); the frontend only renders this.
#[tauri::command]
#[specta::specta]
fn get_day(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<rollup::DayView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let events = db::get_activity_logs(&conn, start_time, end_time)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    Ok(rollup::build_day_view(
        &events,
        &categories,
        &projects,
        start_time,
    ))
}

/// Narrate the day's recap for a `[start_time, end_time]` range with the on-device Foundation
/// Models sidecar, falling back to the deterministic template (D48) on any failure (hard rule
/// 6 / C5). Async + lazy by design (D11): `get_day` already returns the instant template
/// recap, so this never blocks the day load — the UI shows the template immediately, then
/// upgrades the recap card in place when this resolves. Numbers are still computed in Rust;
/// the model only phrases them.
///
/// Cached (D52): each day is narrated once. The cache key is a content fingerprint of the
/// day's facts, so a frozen past day is an instant hit (no spawn, no battery) and a rule
/// reprocess that changes the facts produces a new key and re-narrates exactly once. Today
/// regenerates as its facts grow (settle-on-open; a manual refresh re-runs it). Only real
/// model recaps are cached — never the template fallback.
#[tauri::command]
#[specta::specta]
async fn get_recap(
    db: State<'_, DbState>,
    narrator: State<'_, ai::sidecar::SidecarNarrator>,
    start_time: i64,
    end_time: i64,
) -> Result<rollup::Recap, AppError> {
    // Read + aggregate under the lock, fingerprint the facts, and check the cache — then DROP
    // the lock before the await (a std Mutex guard must never cross the ~seconds model call).
    let (facts, fingerprint, cached) = {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        let events = db::get_activity_logs(&conn, start_time, end_time)?;
        let (categories, projects) = load_lookup_maps(&conn)?;
        let facts = rollup::build_recap_facts(&events, &categories, &projects, start_time);
        let fingerprint = rollup::recap_fingerprint(&facts);
        let cached = db::get_cached_recap(&conn, start_time, &fingerprint)?;
        (facts, fingerprint, cached)
    };

    // Hit: the exact same facts were already narrated — return instantly, no model call.
    if let Some((text, generated_by)) = cached {
        return Ok(rollup::Recap { text, generated_by });
    }

    // Miss: narrate off the lock, then cache ONLY a real model recap (never the template — a
    // cold/unavailable model must retry on the next open instead of caching a placeholder).
    let recap = ai::build_recap(narrator.inner(), &facts).await;
    if recap.generated_by == ai::GENERATED_BY_MODEL {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        db::put_cached_recap(
            &conn,
            start_time,
            &fingerprint,
            &recap.text,
            &recap.generated_by,
        )?;
    }
    Ok(recap)
}

/// Build the Week view — 7 day-slices (each a mini-dial's runs + totals) plus week-level
/// aggregates (D34, hard rule 6). `day_starts` are the 7 local midnights (DST-correct,
/// computed by the frontend like `get_day`'s bounds); `week_end` is the exclusive end of
/// the last day. Each day's events are read for `[day_start, next_day_start | week_end)`.
#[tauri::command]
#[specta::specta]
fn get_week(
    db: State<DbState>,
    day_starts: Vec<i64>,
    week_end: i64,
) -> Result<rollup::WeekView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    let mut days = Vec::with_capacity(day_starts.len());
    for (i, &start) in day_starts.iter().enumerate() {
        let end = day_starts.get(i + 1).copied().unwrap_or(week_end);
        let events = db::get_activity_logs(&conn, start, end)?;
        days.push(rollup::build_day_slice(
            start,
            &events,
            &categories,
            &projects,
        ));
    }
    Ok(rollup::build_week_view(days))
}

/// Build the Timeline view — the day's category-runs, each with its inner app-switch
/// segments (D34) — for a `[start_time, end_time]` Unix-second range. Same read-model
/// inputs as `get_day`; numbers in Rust (hard rule 6).
#[tauri::command]
#[specta::specta]
fn get_timeline(
    db: State<DbState>,
    start_time: i64,
    end_time: i64,
) -> Result<rollup::TimelineView, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let events = db::get_activity_logs(&conn, start_time, end_time)?;
    let (categories, projects) = load_lookup_maps(&conn)?;
    Ok(rollup::build_timeline(&events, &categories, &projects))
}

#[tauri::command]
#[specta::specta]
fn get_categories(db: State<DbState>) -> Result<Vec<db::Category>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_categories(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_category(db: State<DbState>, name: String, color: String) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_category(&conn, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn update_category(
    db: State<DbState>,
    id: i64,
    name: String,
    color: String,
) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::update_category(&conn, id, &name, &color).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_category(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_category(&conn, id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn get_rules(db: State<DbState>) -> Result<Vec<db::Rule>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_rules(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_rule(
    db: State<DbState>,
    category_id: i64,
    match_field: String,
    pattern: String,
    ignore_title: Option<bool>,
) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_rule(
        &conn,
        category_id,
        &match_field,
        &pattern,
        ignore_title.unwrap_or(false),
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_rule(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_rule(&conn, id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn reprocess_logs(db: State<DbState>) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::reprocess_logs(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn get_watcher_status() -> Result<WatcherStatus, AppError> {
    let errors = capture::get_error_count();
    Ok(WatcherStatus {
        consecutive_errors: errors,
        healthy: errors < 6,
    })
}

#[tauri::command]
#[specta::specta]
fn get_settings(db: State<DbState>) -> Result<Vec<Setting>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let rows = db::get_all_settings(&conn).map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(key, value)| Setting { key, value })
        .collect())
}

#[tauri::command]
#[specta::specta]
fn update_setting(db: State<DbState>, key: String, value: String) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::set_setting(&conn, &key, &value).map_err(AppError::from)
}

/// Persist the retention window **and** immediately prune anything older. Distinct from
/// the generic `update_setting` because retention has a side effect (deleting rows) that
/// a plain key/value setter must not carry. `days <= 0` = "keep forever" (a no-op prune).
/// Returns the number of rows deleted so the UI can confirm.
#[tauri::command]
#[specta::specta]
fn set_retention_days(db: State<DbState>, days: i64) -> Result<usize, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::set_setting(&conn, "data_retention_days", &days.to_string())?;
    db::cleanup_old_data(&conn, days).map_err(AppError::from)
}

// --- Exclusions (D8): thin passthroughs over the repository ---

#[tauri::command]
#[specta::specta]
fn get_exclusions(db: State<DbState>) -> Result<Vec<db::Exclusion>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_exclusions(&conn).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn create_exclusion(
    db: State<DbState>,
    match_type: String,
    pattern: String,
    mode: String,
) -> Result<i64, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::create_exclusion(&conn, &match_type, &pattern, &mode).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
fn delete_exclusion(db: State<DbState>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_exclusion(&conn, id).map_err(AppError::from)
}

// --- Data ownership ---

/// Absolute path to the SQLite file, so the frontend can reveal it in Finder via the
/// `opener` plugin (path resolution lives in Rust — one source of truth).
#[tauri::command]
#[specta::specta]
fn get_database_path(app: tauri::AppHandle) -> Result<String, AppError> {
    db::get_db_path(&app)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(AppError::Db)
}

/// Export all events as RFC-4180 CSV to a file next to the DB (an app-owned dir that
/// already exists — no new fs scope), returning its absolute path for the frontend to
/// reveal. SQL + the file write stay in Rust (hard rule 4); nothing leaves the machine.
#[tauri::command]
#[specta::specta]
fn export_events_csv(app: tauri::AppHandle, db: State<DbState>) -> Result<String, AppError> {
    let db_path = db::get_db_path(&app).map_err(AppError::Db)?;
    let out = db_path.with_file_name(format!("usageos-export-{}.csv", db::now_unix()));
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let mut file = std::fs::File::create(&out).map_err(|e| AppError::Db(e.to_string()))?;
    db::export_events_csv(&conn, &mut file)?;
    Ok(out.to_string_lossy().into_owned())
}

/// Erase the captured record (events + derived projects/sites), preserving the user's
/// configuration (categories, rules, exclusions, settings). One transaction (see `db`).
#[tauri::command]
#[specta::specta]
fn delete_all_data(db: State<DbState>) -> Result<(), AppError> {
    let mut conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::delete_all_data(&mut conn).map_err(AppError::from)
}

/// The installed-app catalog (name + icon data-URI) backing the UI's `AppIcon`. Reads
/// public app bundles and caches 64px PNGs next to the DB (an app-owned dir — no new
/// fs scope, no network; hard rule 1). Read-only; returns an empty list rather than
/// erroring when nothing scans (e.g. on CI Linux), so the UI just shows monograms.
#[tauri::command]
#[specta::specta]
fn list_installed_apps(app: tauri::AppHandle) -> Result<Vec<apps::InstalledApp>, AppError> {
    let db_path = db::get_db_path(&app).map_err(AppError::Db)?;
    let cache_dir = db_path.with_file_name("icon-cache");
    Ok(apps::list_installed(
        &apps::default_search_dirs(),
        &cache_dir,
    ))
}

/// Apps with tracked time that match no rule (they roll up as "Uncategorized"), for the
/// Settings list — all-time, ranked, trivial spans floored (see `db`). Numbers in Rust.
#[tauri::command]
#[specta::specta]
fn get_uncategorized_apps(db: State<DbState>) -> Result<Vec<db::UncategorizedApp>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    db::get_uncategorized_apps(&conn).map_err(AppError::from)
}

/// Snapshot the two macOS capture permissions (Accessibility + Automation) for onboarding +
/// Settings. Non-macOS builds report accessibility=true / automation=NotApplicable.
#[tauri::command]
#[specta::specta]
fn get_permissions() -> Result<permissions::Permissions, AppError> {
    Ok(permissions::status())
}

/// Prompt for Accessibility and open its System Settings → Privacy pane.
#[tauri::command]
#[specta::specta]
fn request_accessibility() -> Result<(), AppError> {
    permissions::request_accessibility();
    Ok(())
}

/// Trigger the Automation consent prompt for running browsers and open its Privacy pane. Returns
/// whether a browser was running to prompt, so the UI can guide the "open a browser first" case.
#[tauri::command]
#[specta::specta]
fn request_automation() -> Result<permissions::AutomationRequest, AppError> {
    Ok(permissions::request_automation())
}

/// Open a specific System Settings → Privacy pane (Accessibility or Automation).
#[tauri::command]
#[specta::specta]
fn open_settings_pane(pane: permissions::SettingsPane) -> Result<(), AppError> {
    permissions::open_settings(pane);
    Ok(())
}

/// Show + focus the main window, hiding the glance popover (its "Open UsageOS" affordance).
#[tauri::command]
#[specta::specta]
fn show_main_window(app: AppHandle) -> Result<(), AppError> {
    show_main(&app);
    if let Some(glance) = app.get_webview_window("glance") {
        let _ = glance.hide();
    }
    Ok(())
}

/// Quit UsageOS entirely (stops background tracking) — the glance popover's "Quit".
#[tauri::command]
#[specta::specta]
fn quit_app(app: AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

/// Whether UsageOS starts at login. Reads the LaunchAgent itself — the system is the single
/// source of truth, there is no settings row to drift (D68).
#[tauri::command]
#[specta::specta]
fn get_launch_at_login(app: AppHandle) -> Result<bool, AppError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Autostart(e.to_string()))
}

/// Register / unregister the start-at-login LaunchAgent (the Settings + onboarding toggle).
#[tauri::command]
#[specta::specta]
fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), AppError> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| AppError::Autostart(e.to_string()))
}

// ── Menubar tray + glance popover ─────────────────────────────────────────────

/// Show + focus the main window (tray "Open" + the glance "Open UsageOS").
fn show_main(app: &AppHandle) {
    // Regular first, then show — the other way round the window can appear without focus.
    set_dock_visible(app, true);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Regular = Dock icon + Cmd-Tab while the dashboard is open; Accessory = menu-bar only (D68).
/// Best-effort: a policy that fails to apply leaves the Dock icon, nothing worse.
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn set_dock_visible(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        let policy = if visible {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        if let Err(e) = app.set_activation_policy(policy) {
            eprintln!("[Dock] failed to set activation policy: {e}");
        }
    }
}

/// Place the glance popover centred under the tray icon. Coordinates are physical; the tray
/// `rect` is the icon's frame in the menubar. (Multi-display placement is verified on-device.)
fn position_glance(window: &WebviewWindow, rect: &Rect) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let (tray_x, tray_y) = match rect.position {
        Position::Physical(p) => (f64::from(p.x), f64::from(p.y)),
        Position::Logical(p) => (p.x * scale, p.y * scale),
    };
    let (tray_w, tray_h) = match rect.size {
        Size::Physical(s) => (f64::from(s.width), f64::from(s.height)),
        Size::Logical(s) => (s.width * scale, s.height * scale),
    };
    let win_w = window
        .outer_size()
        .map(|s| f64::from(s.width))
        .unwrap_or(336.0 * scale);
    // Centre under the tray icon, in the tray rect's own coordinate space — which is already on
    // the correct display. (An earlier `monitor_from_point` clamp mis-resolved the display and
    // threw the popover onto the wrong screen; the raw tray coords are authoritative.)
    let x = tray_x + tray_w / 2.0 - win_w / 2.0;
    let y = tray_y + tray_h;
    let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
}

/// When the glance last auto-hid on focus loss — used to debounce a tray click that lands in the
/// same instant (the click can deliver the panel's focus-loss before the tray handler runs).
static LAST_GLANCE_HIDE: Mutex<Option<Instant>> = Mutex::new(None);

fn note_glance_hidden() {
    if let Ok(mut t) = LAST_GLANCE_HIDE.lock() {
        *t = Some(Instant::now());
    }
}

/// True if the glance auto-hid within the last ~200ms — i.e. this tray click is the same one that
/// dismissed it, so re-showing would immediately re-open it.
fn glance_just_auto_hidden() -> bool {
    LAST_GLANCE_HIDE
        .lock()
        .ok()
        .and_then(|t| *t)
        .is_some_and(|t| t.elapsed() < Duration::from_millis(200))
}

/// Left-click the tray icon: toggle the glance popover (created lazily, hidden on focus loss).
fn toggle_glance(app: &AppHandle, rect: Rect) {
    if let Some(window) = app.get_webview_window("glance") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else if !glance_just_auto_hidden() {
            position_glance(&window, &rect);
            let _ = window.show();
        }
        return;
    }
    match WebviewWindowBuilder::new(app, "glance", WebviewUrl::App("index.html#/glance".into()))
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        // Transparent window: the visible chrome is the solid themed CSS card (Glance.tsx); the
        // window's level/rounding/float-over-fullscreen are owned by the NSPanel reclass (D56).
        .transparent(true)
        .visible(false)
        .inner_size(336.0, 488.0)
        .build()
    {
        Ok(window) => {
            // Reclass to a non-activating NSPanel so it floats over full-screen Spaces without
            // activating the app (D56). Never `set_focus()` — that would activate UsageOS.
            // The rounded shell is a solid themed CSS card (not a system frost — it must follow the
            // app's own paper/warm/black theme, not macOS light/dark); the transparent window shows
            // the desktop in the corners and the NSPanel draws the native shadow.
            #[cfg(target_os = "macos")]
            glance_panel::configure(&window);
            position_glance(&window, &rect);
            let _ = window.show();
            let handle = window.clone();
            window.on_window_event(move |event| {
                // Dismiss on click-away. A non-activating panel may resolve focus differently; if
                // this is unreliable on-device, fall back to an AppKit outside-click monitor.
                if let WindowEvent::Focused(false) = event {
                    let _ = handle.hide();
                    note_glance_hidden();
                }
            });
        }
        Err(e) => eprintln!("[Tray] failed to open the glance popover: {e}"),
    }
}

/// Build the menubar tray: left-click toggles the glance popover; right-click shows a small
/// menu (Open / Quit). The app keeps running + tracking when the main window is closed — it
/// exits only via Quit.
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open UsageOS").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit UsageOS").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&open, &quit]).build()?;

    let builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("UsageOS")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon_as_template(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_glance(tray.app_handle(), rect);
            }
        });
    // The tray wears the mono Contexts mark with a now-triangle at the current hour (a template
    // image, so macOS tints it for the light/dark menu bar). Falls back to the app icon if a frame
    // can't be decoded.
    let builder = if let Some(icon) = tray_icon::current() {
        builder.icon(icon)
    } else if let Some(icon) = app.default_window_icon().cloned() {
        builder.icon(icon)
    } else {
        builder
    };
    builder.build(app)?;
    spawn_tray_updater(app.clone());
    Ok(())
}

/// Keep the tray's now-triangle pointing at the right hour. A background thread that wakes every
/// 10 minutes (negligible — no busy timer, honours the idle-CPU discipline) and updates the icon
/// only when the local hour rolls over. The tray outlives the main window, so this runs for the
/// life of the process.
fn spawn_tray_updater(app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("tray-now-hand".into())
        .spawn(move || {
            let mut last = tray_icon::local_hour();
            loop {
                std::thread::sleep(Duration::from_secs(600));
                let hour = tray_icon::local_hour();
                if hour == last {
                    continue;
                }
                last = hour;
                if let (Some(tray), Some(icon)) =
                    (app.tray_by_id("main-tray"), tray_icon::frame(hour))
                {
                    let _ = tray.set_icon(Some(icon));
                    let _ = tray.set_icon_as_template(true);
                }
            }
        });
}

/// The single source of command registration. Both the runtime invoke handler
/// and the generated TS bindings come from this Builder, so they cannot disagree
/// Relaunch the app. Used after the updater downloads + installs a new version so the freshly
/// installed binary takes over. Diverges (`restart` replaces the process), so nothing runs after.
#[tauri::command]
#[specta::specta]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

/// (hard rule 2). Events stay empty until issue #211 is de-risked (commands-only).
fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        get_activity_stats,
        get_day,
        get_recap,
        get_week,
        get_timeline,
        get_categories,
        create_category,
        update_category,
        delete_category,
        get_rules,
        create_rule,
        delete_rule,
        reprocess_logs,
        get_exclusions,
        create_exclusion,
        delete_exclusion,
        get_watcher_status,
        get_settings,
        update_setting,
        set_retention_days,
        get_database_path,
        export_events_csv,
        delete_all_data,
        list_installed_apps,
        get_uncategorized_apps,
        get_permissions,
        request_accessibility,
        request_automation,
        open_settings_pane,
        show_main_window,
        quit_app,
        restart_app,
        get_launch_at_login,
        set_launch_at_login,
    ])
}

/// Set once the menubar tray is built. The main-window close handler reads it to decide whether
/// hiding-on-close is safe (a reachable tray to reopen) or it should fall back to a normal close.
static TRAY_READY: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // The recap sidecar (D49 chunk C) is spawned via the shell plugin; the capability is
        // scoped to exactly the one named sidecar (capabilities/default.json).
        .plugin(tauri_plugin_shell::init())
        // Opt-in auto-update (D61/D67): the plugin only exposes check/download/install — the
        // actual check is gated in the frontend behind the `auto_update_enabled` setting, so no
        // network happens unless the user turned it on. Updates are ed25519-signed (pubkey in
        // tauri.conf.json); a tampered or unsigned update can't install.
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Start at login (D68): opt-in via Settings/onboarding. LaunchAgent mode — a plist in
        // ~/Library/LaunchAgents, no AppleScript/Automation. `--hidden` makes a login launch
        // start in menu-bar mode (no window, no Dock icon); see the setup hook.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .invoke_handler(builder.invoke_handler())
        .on_window_event(|window, event| {
            // Closing the main window HIDES it (tracking keeps running in the background); the
            // app exits only via the tray "Quit". Other windows (the glance popover) close
            // normally — its focus-loss handler hides it. Only swallow the close once the tray
            // exists to bring the window back — otherwise a tray-setup failure would strand the
            // app with its only window hidden and no Open/Quit path.
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if TRAY_READY.load(Ordering::Relaxed) {
                        api.prevent_close();
                        let _ = window.hide();
                        // No window left → drop the Dock icon too; the tray is the way back (D68).
                        set_dock_visible(window.app_handle(), false);
                    }
                }
            }
        })
        .setup(|app| {
            // The main window is `visible: false` in tauri.conf.json so a `--hidden` launch
            // (the start-at-login LaunchAgent) never flashes it: login starts straight into
            // menu-bar mode, a normal launch shows the window as before (D68).
            if std::env::args().any(|a| a == "--hidden") {
                set_dock_visible(app.handle(), false);
            } else {
                show_main(app.handle());
            }

            let db_path = db::get_db_path(app.handle())?;

            let db_conn = db::init_database(&db_path)?;

            // Run data retention cleanup before starting the watcher.
            // A poisoned lock here is non-fatal: skip cleanup and continue startup.
            match db_conn.lock() {
                Ok(conn) => {
                    if let Ok(Some(days_str)) = db::get_setting(&conn, "data_retention_days") {
                        if let Ok(days) = days_str.parse::<i64>() {
                            match db::cleanup_old_data(&conn, days) {
                                Ok(deleted) if deleted > 0 => {
                                    println!("[Startup] Cleaned up {} old activity logs", deleted);
                                }
                                Err(e) => eprintln!("[Startup] Cleanup failed: {}", e),
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[Startup] Skipping cleanup, db lock poisoned: {}", e),
            }

            app.manage(db_conn.clone());

            // Recap narrator: the FM sidecar behind `ai::Narrator`, managed for the lazy
            // `get_recap` command. Prewarm off the main thread — best-effort; the template
            // recap always covers a cold/missing model (D49/C5).
            app.manage(ai::sidecar::SidecarNarrator::new(app.handle().clone()));
            let warm_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                ai::sidecar::prewarm(&warm_handle).await;
            });

            // The source registers on this (main) thread — the macOS impl attaches to the main
            // CFRunLoop (D29) — while the consumer (the sole DB writer) drains on a dedicated
            // thread, since SQLite + git-shell enrichment block and must stay off the executor.
            let (tx, rx) = std::sync::mpsc::channel();
            capture::default_source().start(tx);
            std::thread::spawn(move || capture::consume(db_conn, rx));

            // Menubar tray + glance popover. Non-fatal: a tray failure shouldn't block launch —
            // but the main window keeps normal close-to-quit behavior until the tray is up (see
            // the close handler), so we never strand it behind a missing tray.
            match setup_tray(app.handle()) {
                Ok(()) => TRAY_READY.store(true, Ordering::Relaxed),
                Err(e) => eprintln!("[Startup] Tray setup failed: {}", e),
            }
            Ok(())
        })
        .build(tauri::generate_context!());

    match result {
        // Underscore-named params: only the macOS Reopen arm uses them (the variant doesn't
        // exist on other targets), and bare `_`s would trip unused-variable lints there.
        Ok(app) => app.run(|_app_handle, _event| {
            // While the window is closed there is no Dock icon, so re-opening the app from
            // Finder/Spotlight is the natural "bring it back" gesture — macOS activates this
            // process and fires Reopen instead of launching a second instance.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_main(_app_handle);
            }
        }),
        Err(e) => {
            eprintln!("[Fatal] Error while running tauri application: {}", e);
            std::process::exit(1);
        }
    }
}

/// Codegen for the IPC bindings. Test-only, so the `expect()` doesn't violate hard rule 3.
/// The CI freshness gate regenerates this and fails if the committed `src/bindings.ts` drifts.
#[cfg(test)]
mod export_bindings {
    use specta_typescript::{BigIntExportBehavior, Typescript};

    #[test]
    fn export_bindings() {
        super::make_builder()
            .export(
                Typescript::new()
                    .bigint(BigIntExportBehavior::Number)
                    // @ts-nocheck: generated boilerplate trips strict `noUnusedLocals`; it's
                    // generated, not authored, so the freshness gate is its check, not the linter.
                    .header(
                        "// @ts-nocheck\n// @generated by tauri-specta — DO NOT EDIT. Run `cargo test export_bindings`.\n",
                    ),
                "../src/bindings.ts",
            )
            .expect("failed to export IPC bindings");
    }
}
