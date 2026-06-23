//! Read-time rollup: turn a day's raw events into the view the dial renders.
//!
//! Pure (no DB, no IO) so it's trivially unit-testable and cheap to re-run — the
//! command layer does the repository reads and hands us events + lookups. This is
//! where the "numbers are computed in Rust" rule (hard rule 6) lives for the dial.
//!
//! Two independent shapes come out (D34):
//! - **Per-axis aggregates** (`contexts`) — plain sums by context; robust to any
//!   segmentation, they feed the ledger / legend / stats / dial centre.
//! - **Context-runs** (`runs`) — continuous stretches of one context, with the
//!   project split as inside-detail; they feed the dial arcs + (later) the timeline.
//!   Project-hopping never fragments a run; off-project time counts to its context.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::ActivityLog;

/// D34a — an idle or untracked gap at least this long ends a context-run. Placeholder:
/// the run-segmentation thresholds are tuned against real captured days (the session
/// explorer, M3) and then locked; the run/expand shape holds regardless of the value.
const IDLE_GAP_ENDS_RUN_SECS: i64 = 5 * 60;

/// Slug + display name for a context, looked up by category id. The dial maps `slug`
/// to a colour token (`--c-<slug>`). Internal — does not cross the IPC boundary.
pub struct ContextMeta {
    pub slug: Option<String>,
    pub name: String,
}

/// Fallbacks for an event whose context has no slug (a user-created context) or no
/// context at all (no rule matched). The frontend maps this slug to a neutral token.
const OTHER_SLUG: &str = "other";
const OTHER_NAME: &str = "Uncategorized";
/// The canonical Deep-work context slug (the Week view's "deepest day" + per-day deep total).
const DEEP_SLUG: &str = "deep";
/// Shown for active time with no resolved project (D30/D34 — never a guessed project).
const NO_PROJECT: &str = "No project";

/// One context's total share of the active day (the ledger/legend/stats unit).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ContextSlice {
    pub slug: String,
    pub name: String,
    pub secs: i64,
    pub pct: f64,
}

/// A project's share of time *inside* a context-run (shown as a text line, never a bar).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectSlice {
    pub name: String,
    pub secs: i64,
}

/// A continuous stretch of one context — a dial arc, click-to-inspect.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ContextRun {
    pub context_slug: String,
    pub context_name: String,
    /// Span bounds (Unix secs); the arc is drawn start→end.
    pub start: i64,
    pub end: i64,
    /// Active seconds within the run (≤ end−start when small gaps are bridged).
    pub secs: i64,
    pub projects: Vec<ProjectSlice>,
    pub apps: Vec<String>,
}

/// The day's recap. `generated_by` is "template" here; the on-device Foundation
/// Models prose (Phase 3) will reuse the same facts behind the `ai` trait.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Recap {
    pub text: String,
    pub generated_by: String,
}

/// Everything the Day view needs, computed from one day of events.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DayView {
    pub active_secs: i64,
    pub idle_secs: i64,
    pub contexts: Vec<ContextSlice>,
    pub runs: Vec<ContextRun>,
    pub recap: Recap,
}

/// One day's compact summary for the Week view: a mini-dial's arcs plus the two totals
/// the week summary needs. `deep_secs` is carried so "deepest day" is a Rust number (rule 6).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DaySlice {
    /// Local midnight (Unix secs) — the mini-dial's angular origin.
    pub day_start: i64,
    pub active_secs: i64,
    pub deep_secs: i64,
    pub runs: Vec<ContextRun>,
}

/// Everything the Week view needs: 7 day-slices + week-level aggregates (numbers in Rust).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WeekView {
    pub days: Vec<DaySlice>,
    pub total_active_secs: i64,
    pub avg_active_secs: i64,
    /// Index into `days` of the day with the most Deep-work time, or `None` if there was none.
    pub deepest_day: Option<i64>,
}

/// One focused-window event inside a context-run — the Timeline's click-to-expand detail.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineSegment {
    pub start: i64,
    pub end: i64,
    pub app: String,
    /// Resolved project name, or `None` when none was inferred (the UI shows "—").
    pub project: Option<String>,
    pub secs: i64,
}

/// A context-run plus its inner app-switch segments — one expandable Timeline row (D34).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineRun {
    pub context_slug: String,
    pub context_name: String,
    pub start: i64,
    pub end: i64,
    pub secs: i64,
    pub projects: Vec<ProjectSlice>,
    pub apps: Vec<String>,
    pub segments: Vec<TimelineSegment>,
}

/// Everything the Timeline view needs: the day's context-runs, each with its segments.
/// The "Away" idle gaps and the now-marker are derived on the frontend from run bounds.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineView {
    pub runs: Vec<TimelineRun>,
}

fn duration(event: &ActivityLog) -> i64 {
    (event.end_time - event.start_time).max(0)
}

/// Resolve an event to its (slug, name), falling back to the neutral "other" context.
fn context_of(event: &ActivityLog, contexts: &HashMap<i64, ContextMeta>) -> (String, String) {
    match event.category_id.and_then(|id| contexts.get(&id)) {
        Some(meta) => (
            meta.slug.clone().unwrap_or_else(|| OTHER_SLUG.to_string()),
            meta.name.clone(),
        ),
        None => (OTHER_SLUG.to_string(), OTHER_NAME.to_string()),
    }
}

fn project_of(event: &ActivityLog, projects: &HashMap<i64, String>) -> String {
    event
        .project_id
        .and_then(|id| projects.get(&id).cloned())
        .unwrap_or_else(|| NO_PROJECT.to_string())
}

/// Build the Day view from a day's events plus context/project name lookups.
pub fn build_day_view(
    events: &[ActivityLog],
    contexts: &HashMap<i64, ContextMeta>,
    projects: &HashMap<i64, String>,
) -> DayView {
    let mut active_secs = 0;
    let mut idle_secs = 0;
    // slug -> (name, secs); name kept for display, secs accumulated.
    let mut totals: HashMap<String, (String, i64)> = HashMap::new();

    for event in events {
        let secs = duration(event);
        if secs == 0 {
            continue;
        }
        if event.is_idle {
            idle_secs += secs;
            continue;
        }
        active_secs += secs;
        let (slug, name) = context_of(event, contexts);
        let entry = totals.entry(slug).or_insert((name, 0));
        entry.1 += secs;
    }

    let mut context_slices: Vec<ContextSlice> = totals
        .into_iter()
        .map(|(slug, (name, secs))| ContextSlice {
            slug,
            name,
            secs,
            pct: if active_secs > 0 {
                secs as f64 / active_secs as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();
    // Deterministic order: longest first, slug as the tie-breaker.
    context_slices.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.slug.cmp(&b.slug)));

    let runs = build_runs(events, contexts, projects);
    let recap = render_template_recap(active_secs, &context_slices, &runs);

    DayView {
        active_secs,
        idle_secs,
        contexts: context_slices,
        runs,
        recap,
    }
}

/// One day's slice for the Week grid: per-day active + deep totals and the dial arcs. Reuses
/// `build_runs`; no recap/ledger (the Week view doesn't show them). `day_start` is passed
/// through as the mini-dial origin (the caller owns local day bounds, like `get_day`).
pub fn build_day_slice(
    day_start: i64,
    events: &[ActivityLog],
    contexts: &HashMap<i64, ContextMeta>,
    projects: &HashMap<i64, String>,
) -> DaySlice {
    let mut active_secs = 0;
    let mut deep_secs = 0;
    for event in events {
        let secs = duration(event);
        if secs == 0 || event.is_idle {
            continue;
        }
        active_secs += secs;
        if context_of(event, contexts).0 == DEEP_SLUG {
            deep_secs += secs;
        }
    }
    DaySlice {
        day_start,
        active_secs,
        deep_secs,
        runs: build_runs(events, contexts, projects),
    }
}

/// Assemble the Week view from its day slices: total + average active time and the deepest
/// (most Deep-work) day. Average is over the slice count (a 7-day week → ÷7, matching the
/// design). `deepest_day` is the argmax of `deep_secs`, only when some deep work exists.
pub fn build_week_view(days: Vec<DaySlice>) -> WeekView {
    let total_active_secs: i64 = days.iter().map(|d| d.active_secs).sum();
    let divisor = days.len().max(1) as i64;
    let deepest_day = days
        .iter()
        .enumerate()
        .filter(|(_, d)| d.deep_secs > 0)
        .max_by_key(|(_, d)| d.deep_secs)
        .map(|(i, _)| i as i64);
    WeekView {
        total_active_secs,
        avg_active_secs: total_active_secs / divisor,
        deepest_day,
        days,
    }
}

/// In-progress timeline-run accumulator: like `RunBuilder`, but retains each event as a
/// segment (the expand detail) and derives the project breakdown from those segments.
struct TimelineRunBuilder {
    slug: String,
    name: String,
    start: i64,
    end: i64,
    secs: i64,
    apps: Vec<String>,
    segments: Vec<TimelineSegment>,
}

impl TimelineRunBuilder {
    fn new(event: &ActivityLog, slug: String, name: String, project: Option<String>) -> Self {
        let mut builder = TimelineRunBuilder {
            slug,
            name,
            start: event.start_time,
            end: event.end_time,
            secs: 0,
            apps: Vec::new(),
            segments: Vec::new(),
        };
        builder.add(event, project);
        builder
    }

    fn add(&mut self, event: &ActivityLog, project: Option<String>) {
        self.end = self.end.max(event.end_time);
        let secs = duration(event);
        self.secs += secs;
        if !self.apps.iter().any(|a| a == &event.process_name) {
            self.apps.push(event.process_name.clone());
        }
        self.segments.push(TimelineSegment {
            start: event.start_time,
            end: event.end_time,
            app: event.process_name.clone(),
            project,
            secs,
        });
    }

    fn finish(self) -> TimelineRun {
        // Project breakdown from the segments (unresolved → the neutral "No project").
        let mut totals: HashMap<String, i64> = HashMap::new();
        for seg in &self.segments {
            let name = seg
                .project
                .clone()
                .unwrap_or_else(|| NO_PROJECT.to_string());
            *totals.entry(name).or_insert(0) += seg.secs;
        }
        let mut projects: Vec<ProjectSlice> = totals
            .into_iter()
            .map(|(name, secs)| ProjectSlice { name, secs })
            .collect();
        projects.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name)));
        TimelineRun {
            context_slug: self.slug,
            context_name: self.name,
            start: self.start,
            end: self.end,
            secs: self.secs,
            projects,
            apps: self.apps,
            segments: self.segments,
        }
    }
}

/// Build the Timeline view: the day's context-runs (same segmentation as `build_runs` — a
/// context change or a gap ≥ `IDLE_GAP_ENDS_RUN_SECS` ends a run) but with every event kept
/// as a segment for the click-to-expand detail (D34). The current segmentation only — this
/// view is the tool to evaluate the D34a excursion-absorb / sustained-shift refinements.
pub fn build_timeline(
    events: &[ActivityLog],
    contexts: &HashMap<i64, ContextMeta>,
    projects: &HashMap<i64, String>,
) -> TimelineView {
    let mut active: Vec<&ActivityLog> = events
        .iter()
        .filter(|e| !e.is_idle && duration(e) > 0)
        .collect();
    active.sort_by_key(|e| e.start_time);

    let mut runs: Vec<TimelineRun> = Vec::new();
    let mut current: Option<TimelineRunBuilder> = None;

    for event in active {
        let (slug, name) = context_of(event, contexts);
        let project = event.project_id.and_then(|id| projects.get(&id).cloned());

        match current.take() {
            Some(mut run)
                if run.slug == slug && event.start_time - run.end < IDLE_GAP_ENDS_RUN_SECS =>
            {
                run.add(event, project);
                current = Some(run);
            }
            Some(run) => {
                runs.push(run.finish());
                current = Some(TimelineRunBuilder::new(event, slug, name, project));
            }
            None => current = Some(TimelineRunBuilder::new(event, slug, name, project)),
        }
    }
    if let Some(run) = current {
        runs.push(run.finish());
    }
    TimelineView { runs }
}

/// In-progress run accumulator (kept out of the public surface).
struct RunBuilder {
    slug: String,
    name: String,
    start: i64,
    end: i64,
    secs: i64,
    projects: HashMap<String, i64>,
    apps: Vec<String>,
}

impl RunBuilder {
    fn new(event: &ActivityLog, slug: String, name: String, project: String) -> Self {
        let mut builder = RunBuilder {
            slug,
            name,
            start: event.start_time,
            end: event.end_time,
            secs: 0,
            projects: HashMap::new(),
            apps: Vec::new(),
        };
        builder.add(event, project);
        builder
    }

    fn add(&mut self, event: &ActivityLog, project: String) {
        self.end = self.end.max(event.end_time);
        self.secs += duration(event);
        *self.projects.entry(project).or_insert(0) += duration(event);
        if !self.apps.iter().any(|a| a == &event.process_name) {
            self.apps.push(event.process_name.clone());
        }
    }

    fn finish(self) -> ContextRun {
        let mut projects: Vec<ProjectSlice> = self
            .projects
            .into_iter()
            .map(|(name, secs)| ProjectSlice { name, secs })
            .collect();
        // Longest first; name as the tie-breaker so "No project" doesn't float by hash.
        projects.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name)));
        ContextRun {
            context_slug: self.slug,
            context_name: self.name,
            start: self.start,
            end: self.end,
            secs: self.secs,
            projects,
            apps: self.apps,
        }
    }
}

/// Coalesce active events into context-runs. A run ends when the context changes or a
/// gap (idle/untracked) of at least `IDLE_GAP_ENDS_RUN_SECS` opens. Project changes
/// never split a run — the project breakdown lives inside it (D34).
fn build_runs(
    events: &[ActivityLog],
    contexts: &HashMap<i64, ContextMeta>,
    projects: &HashMap<i64, String>,
) -> Vec<ContextRun> {
    let mut active: Vec<&ActivityLog> = events
        .iter()
        .filter(|e| !e.is_idle && duration(e) > 0)
        .collect();
    active.sort_by_key(|e| e.start_time);

    let mut runs: Vec<ContextRun> = Vec::new();
    let mut current: Option<RunBuilder> = None;

    for event in active {
        let (slug, name) = context_of(event, contexts);
        let project = project_of(event, projects);

        match current.take() {
            Some(mut run)
                if run.slug == slug && event.start_time - run.end < IDLE_GAP_ENDS_RUN_SECS =>
            {
                run.add(event, project);
                current = Some(run);
            }
            Some(run) => {
                runs.push(run.finish());
                current = Some(RunBuilder::new(event, slug, name, project));
            }
            None => current = Some(RunBuilder::new(event, slug, name, project)),
        }
    }
    if let Some(run) = current {
        runs.push(run.finish());
    }
    runs
}

/// Format a duration the way the UI reads it: "4h 15m" / "45m" / "30s".
fn human_secs(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{secs}s")
    }
}

/// A plain, honest, deterministic recap. The Foundation Models prose (Phase 3) will
/// replace the phrasing; this is the always-available template (and the fallback).
fn render_template_recap(
    active_secs: i64,
    contexts: &[ContextSlice],
    runs: &[ContextRun],
) -> Recap {
    let text = if active_secs == 0 || contexts.is_empty() {
        "No activity tracked today yet.".to_string()
    } else {
        let top = &contexts[0];
        let mut text = if contexts.len() == 1 {
            format!(
                "{} tracked, all {}.",
                human_secs(active_secs),
                top.name.to_lowercase()
            )
        } else {
            format!(
                "{} tracked. {} led at {}.",
                human_secs(active_secs),
                top.name,
                human_secs(top.secs)
            )
        };
        if let Some(project) = leading_project(runs, active_secs) {
            text.push_str(&format!(" Most of it on {project}."));
        }
        text
    };

    Recap {
        text,
        generated_by: "template".to_string(),
    }
}

/// The single real project that clearly led the day (≥ 40% of active time), if any.
/// "No project" never qualifies — we don't narrate the absence of a project.
fn leading_project(runs: &[ContextRun], active_secs: i64) -> Option<String> {
    if active_secs <= 0 {
        return None;
    }
    let mut totals: HashMap<&str, i64> = HashMap::new();
    for run in runs {
        for project in &run.projects {
            if project.name != NO_PROJECT {
                *totals.entry(project.name.as_str()).or_insert(0) += project.secs;
            }
        }
    }
    totals
        .into_iter()
        .max_by_key(|(_, secs)| *secs)
        .filter(|(_, secs)| *secs * 100 >= active_secs * 40)
        .map(|(name, _)| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal event builder for tests.
    fn ev(
        start: i64,
        end: i64,
        app: &str,
        category_id: Option<i64>,
        project_id: Option<i64>,
    ) -> ActivityLog {
        ActivityLog {
            id: 0,
            process_name: app.to_string(),
            window_title: String::new(),
            start_time: start,
            end_time: end,
            is_idle: false,
            category_id,
            url: None,
            site: None,
            project_id,
            project_abstain_reason: None,
            is_private: false,
        }
    }

    fn idle(start: i64, end: i64) -> ActivityLog {
        let mut e = ev(start, end, "idle", None, None);
        e.is_idle = true;
        e
    }

    fn ctx_map() -> HashMap<i64, ContextMeta> {
        HashMap::from([
            (
                1,
                ContextMeta {
                    slug: Some("deep".into()),
                    name: "Deep work".into(),
                },
            ),
            (
                2,
                ContextMeta {
                    slug: Some("comms".into()),
                    name: "Comms".into(),
                },
            ),
        ])
    }

    fn proj_map() -> HashMap<i64, String> {
        HashMap::from([(1, "usageos".to_string()), (2, "nudge".to_string())])
    }

    #[test]
    fn aggregates_sum_per_context_and_exclude_idle() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // 10m deep
            ev(600, 900, "Slack", Some(2), None),   // 5m comms
            idle(900, 1200),                        // 5m idle
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.active_secs, 900);
        assert_eq!(day.idle_secs, 300);
        assert_eq!(day.contexts.len(), 2);
        assert_eq!(day.contexts[0].slug, "deep"); // longest first
        assert_eq!(day.contexts[0].secs, 600);
        assert!((day.contexts[0].pct - 66.666).abs() < 0.01);
    }

    #[test]
    fn same_context_coalesces_into_one_run_across_projects() {
        // Deep work bouncing usageos <-> nudge must stay ONE run (D34).
        let events = vec![
            ev(0, 300, "Cursor", Some(1), Some(1)),
            ev(300, 600, "Cursor", Some(1), Some(2)),
            ev(600, 900, "iTerm", Some(1), Some(1)),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), 1);
        let run = &day.runs[0];
        assert_eq!(run.context_slug, "deep");
        assert_eq!(run.start, 0);
        assert_eq!(run.end, 900);
        assert_eq!(run.apps, vec!["Cursor", "iTerm"]);
        assert_eq!(run.projects.len(), 2);
        assert_eq!(run.projects[0].name, "usageos"); // 600s vs 300s
        assert_eq!(run.projects[0].secs, 600);
    }

    #[test]
    fn context_change_splits_runs() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 900, "Slack", Some(2), None),
            ev(900, 1200, "Cursor", Some(1), Some(1)),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), 3);
        assert_eq!(day.runs[1].context_slug, "comms");
        assert_eq!(day.runs[1].projects[0].name, "No project");
    }

    #[test]
    fn large_gap_splits_same_context_into_two_runs() {
        // Deep, then a long idle/untracked gap, then deep again -> two runs.
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(
                600 + IDLE_GAP_ENDS_RUN_SECS,
                1200 + IDLE_GAP_ENDS_RUN_SECS,
                "Cursor",
                Some(1),
                Some(1),
            ),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), 2);
    }

    #[test]
    fn uncategorized_active_time_becomes_other_context() {
        let events = vec![ev(0, 600, "Unknown", None, None)];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.contexts[0].slug, "other");
        assert_eq!(day.contexts[0].name, "Uncategorized");
        assert_eq!(day.runs[0].context_slug, "other");
    }

    #[test]
    fn recap_is_empty_when_no_active_time() {
        let day = build_day_view(&[idle(0, 600)], &ctx_map(), &proj_map());
        assert_eq!(day.recap.text, "No activity tracked today yet.");
        assert_eq!(day.recap.generated_by, "template");
    }

    #[test]
    fn recap_names_the_leading_context_and_project() {
        let events = vec![
            ev(0, 3600, "Cursor", Some(1), Some(1)), // 1h deep on usageos
            ev(3600, 4200, "Slack", Some(2), None),  // 10m comms
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert!(day.recap.text.contains("Deep work led at"));
        assert!(day.recap.text.contains("Most of it on usageos."));
    }

    #[test]
    fn day_slice_sums_active_and_deep_and_builds_runs() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // 10m deep
            ev(600, 900, "Slack", Some(2), None),   // 5m comms
            idle(900, 1200),                        // ignored
        ];
        let slice = build_day_slice(42, &events, &ctx_map(), &proj_map());
        assert_eq!(slice.day_start, 42, "origin passed through");
        assert_eq!(slice.active_secs, 900);
        assert_eq!(slice.deep_secs, 600, "only the deep-work time");
        assert_eq!(slice.runs.len(), 2);
    }

    #[test]
    fn week_view_totals_average_and_deepest_day() {
        let day = |deep: i64| DaySlice {
            day_start: 0,
            active_secs: deep, // active == deep for this fixture
            deep_secs: deep,
            runs: vec![],
        };
        // 7 days; index 3 is the deepest (1800s).
        let week = build_week_view(vec![
            day(0),
            day(300),
            day(600),
            day(1800),
            day(600),
            day(0),
            day(900),
        ]);
        assert_eq!(week.total_active_secs, 4200);
        assert_eq!(week.avg_active_secs, 600, "4200 / 7");
        assert_eq!(week.deepest_day, Some(3));
    }

    #[test]
    fn week_view_has_no_deepest_day_without_deep_work() {
        let flat = DaySlice {
            day_start: 0,
            active_secs: 300,
            deep_secs: 0,
            runs: vec![],
        };
        let week = build_week_view(vec![flat.clone(), flat.clone(), flat]);
        assert_eq!(week.deepest_day, None);
        assert_eq!(week.avg_active_secs, 300);
    }

    #[test]
    fn timeline_keeps_segments_and_projects_within_a_run() {
        // Deep work bouncing usageos <-> nudge stays one run, with both events as segments.
        let events = vec![
            ev(0, 300, "Cursor", Some(1), Some(1)),
            ev(300, 600, "iTerm", Some(1), Some(2)),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 1);
        let run = &tl.runs[0];
        assert_eq!(run.segments.len(), 2);
        assert_eq!(run.segments[0].app, "Cursor");
        assert_eq!(run.segments[0].project.as_deref(), Some("usageos"));
        assert_eq!(run.segments[1].app, "iTerm");
        assert_eq!(run.secs, 600);
        assert_eq!(run.projects.len(), 2, "two projects inside the run");
    }

    #[test]
    fn timeline_splits_on_context_change_and_marks_no_project() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // deep
            ev(600, 900, "Slack", Some(2), None),   // comms → split
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 2);
        assert_eq!(tl.runs[1].context_slug, "comms");
        assert_eq!(
            tl.runs[1].segments[0].project, None,
            "unresolved project → None"
        );
    }

    #[test]
    fn timeline_splits_same_context_on_a_long_gap() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(
                600 + IDLE_GAP_ENDS_RUN_SECS,
                1200 + IDLE_GAP_ENDS_RUN_SECS,
                "Cursor",
                Some(1),
                Some(1),
            ),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 2, "a gap ≥ threshold ends the run");
    }
}
