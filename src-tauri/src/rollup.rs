//! Read-time rollup: turn a day's raw events into the view the dial renders.
//!
//! Pure (no DB, no IO) so it's trivially unit-testable and cheap to re-run — the
//! command layer does the repository reads and hands us events + lookups. This is
//! where the "numbers are computed in Rust" rule (hard rule 6) lives for the dial.
//!
//! Two independent shapes come out (D34):
//! - **Per-axis aggregates** (`categories`) — plain sums by category; robust to any
//!   segmentation, they feed the ledger / legend / stats / dial centre.
//! - **Category-runs** (`runs`) — continuous stretches of one category, with the
//!   project split as inside-detail; they feed the dial arcs + (later) the timeline.
//!   Project-hopping never fragments a run; off-project time counts to its category.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::ActivityLog;

/// D34a — an idle or untracked gap at least this long ends a category-run. Placeholder:
/// the run-segmentation thresholds are tuned against real captured days (the session
/// explorer, M3) and then locked; the run/expand shape holds regardless of the value.
const IDLE_GAP_ENDS_RUN_SECS: i64 = 5 * 60;

/// D34a excursion-absorb: a brief detour into another category, sandwiched by the same category,
/// folds into the surrounding run (the detour still shows as a segment in the Timeline expand).
/// A detour whose **wall-clock span** exceeds this stays its own run. Raised 90→180s after
/// dogfooding (D41): real switches cluster at 90–300s, so 90s left too many as separate blocks.
/// Still a tunable knob; kept below `IDLE_GAP_ENDS_RUN_SECS` so the two stay distinct.
const ABSORB_SECS: i64 = 180;
/// Backstop on accumulation: a run may absorb at most this percent of its wall-clock as detours;
/// once it's more interrupted than this it splits, so the dial never draws a falsely-unbroken
/// focus stretch. Paired with a local-dominance check (host active ≥ excursion). Dogfood-tunable.
const MAX_ABSORB_FRACTION_PCT: i64 = 15;

/// Slug + display name for a category, looked up by category id. The dial maps `slug`
/// to a colour token (`--c-<slug>`). Internal — does not cross the IPC boundary.
pub struct CategoryMeta {
    pub slug: Option<String>,
    pub name: String,
}

/// Fallbacks for an event whose category has no slug (a user-created category) or no
/// category at all (no rule matched). The frontend maps this slug to a neutral token.
const OTHER_SLUG: &str = "other";
const OTHER_NAME: &str = "Uncategorized";
/// The canonical Deep-work category slug (the Week view's "deepest day" + per-day deep total).
const DEEP_SLUG: &str = "deep";
/// Shown for active time with no resolved project (D30/D34 — never a guessed project).
const NO_PROJECT: &str = "No project";

/// One category's total share of the active day (the ledger/legend/stats unit).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CategorySlice {
    pub slug: String,
    pub name: String,
    pub secs: i64,
    pub pct: f64,
}

/// A project's share of time *inside* a category-run (shown as a text line, never a bar).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectSlice {
    pub name: String,
    pub secs: i64,
}

/// A continuous stretch of one category — a dial arc, click-to-inspect.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CategoryRun {
    pub category_slug: String,
    pub category_name: String,
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
    pub categories: Vec<CategorySlice>,
    pub runs: Vec<CategoryRun>,
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
    pub runs: Vec<CategoryRun>,
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

/// One focused-window event inside a category-run — the Timeline's click-to-expand detail.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineSegment {
    pub start: i64,
    pub end: i64,
    pub app: String,
    /// The segment's own category. After excursion-absorb (D34a) a run may contain an absorbed
    /// detour of a *different* category, so each segment carries its own — the expand stays honest.
    pub category_slug: String,
    pub category_name: String,
    /// Resolved project name, or `None` when none was inferred (the UI shows "—").
    pub project: Option<String>,
    pub secs: i64,
}

/// A category-run plus its inner app-switch segments — one expandable Timeline row (D34).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineRun {
    pub category_slug: String,
    pub category_name: String,
    pub start: i64,
    pub end: i64,
    pub secs: i64,
    pub projects: Vec<ProjectSlice>,
    pub apps: Vec<String>,
    pub segments: Vec<TimelineSegment>,
}

/// Everything the Timeline view needs: the day's category-runs, each with its segments.
/// The "Away" idle gaps and the now-marker are derived on the frontend from run bounds.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineView {
    pub runs: Vec<TimelineRun>,
}

fn duration(event: &ActivityLog) -> i64 {
    (event.end_time - event.start_time).max(0)
}

/// Resolve an event to its (slug, name), falling back to the neutral "other" category.
fn category_of(event: &ActivityLog, categories: &HashMap<i64, CategoryMeta>) -> (String, String) {
    match event.category_id.and_then(|id| categories.get(&id)) {
        Some(meta) => (
            meta.slug.clone().unwrap_or_else(|| OTHER_SLUG.to_string()),
            meta.name.clone(),
        ),
        None => (OTHER_SLUG.to_string(), OTHER_NAME.to_string()),
    }
}

/// Build the Day view from a day's events plus category/project name lookups.
pub fn build_day_view(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
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
        let (slug, name) = category_of(event, categories);
        let entry = totals.entry(slug).or_insert((name, 0));
        entry.1 += secs;
    }

    let mut category_slices: Vec<CategorySlice> = totals
        .into_iter()
        .map(|(slug, (name, secs))| CategorySlice {
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
    category_slices.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.slug.cmp(&b.slug)));

    let runs = build_runs(events, categories, projects);
    let recap = render_template_recap(active_secs, &category_slices, &runs);

    DayView {
        active_secs,
        idle_secs,
        categories: category_slices,
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
    categories: &HashMap<i64, CategoryMeta>,
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
        if category_of(event, categories).0 == DEEP_SLUG {
            deep_secs += secs;
        }
    }
    DaySlice {
        day_start,
        active_secs,
        deep_secs,
        runs: build_runs(events, categories, projects),
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

/// In-progress raw run: consecutive same-category events accumulated as segments. Coalesced
/// into a [`SegRun`]; the absorb pass (D34a) then merges brief sandwiched excursions.
struct RawRunBuilder {
    slug: String,
    name: String,
    start: i64,
    end: i64,
    segments: Vec<TimelineSegment>,
}

impl RawRunBuilder {
    fn new(event: &ActivityLog, slug: String, name: String, project: Option<String>) -> Self {
        let mut builder = RawRunBuilder {
            slug,
            name,
            start: event.start_time,
            end: event.end_time,
            segments: Vec::new(),
        };
        builder.add(event, project);
        builder
    }

    fn add(&mut self, event: &ActivityLog, project: Option<String>) {
        self.end = self.end.max(event.end_time);
        self.segments.push(TimelineSegment {
            start: event.start_time,
            end: event.end_time,
            app: event.process_name.clone(),
            category_slug: self.slug.clone(),
            category_name: self.name.clone(),
            project,
            secs: duration(event),
        });
    }
}

/// A coalesced run carrying every segment (each with its own category). After the absorb pass a
/// run's segments may include a different-category detour; its reported `secs` / `projects` /
/// `apps` are always for the **host** category only (D34a) — the detour lives on its segment.
struct SegRun {
    slug: String,
    name: String,
    start: i64,
    end: i64,
    segments: Vec<TimelineSegment>,
}

impl SegRun {
    fn from_raw(raw: RawRunBuilder) -> Self {
        SegRun {
            slug: raw.slug,
            name: raw.name,
            start: raw.start,
            end: raw.end,
            segments: raw.segments,
        }
    }

    /// Active seconds whose segment category is the host category.
    fn host_active(&self) -> i64 {
        self.segments
            .iter()
            .filter(|s| s.category_slug == self.slug)
            .map(|s| s.secs)
            .sum()
    }

    /// Active seconds of absorbed (non-host-category) detours.
    fn absorbed(&self) -> i64 {
        self.segments
            .iter()
            .filter(|s| s.category_slug != self.slug)
            .map(|s| s.secs)
            .sum()
    }

    /// All active seconds in the run — the "excursion active" when this is a block run.
    fn total_active(&self) -> i64 {
        self.segments.iter().map(|s| s.secs).sum()
    }

    fn finish(self) -> TimelineRun {
        // Headline numbers are host-category only: a "Deep · 52m" run means 52m of Deep, and the
        // off-category detour never injects a phantom project slice (D34a, debate). The detour's
        // seconds live on its segment (the expand) and in the per-axis ledger.
        let host = self.slug.clone();
        let secs = self.host_active();
        let mut totals: HashMap<String, i64> = HashMap::new();
        let mut apps: Vec<String> = Vec::new();
        for seg in self.segments.iter().filter(|s| s.category_slug == host) {
            let name = seg
                .project
                .clone()
                .unwrap_or_else(|| NO_PROJECT.to_string());
            *totals.entry(name).or_insert(0) += seg.secs;
            if !apps.iter().any(|a| a == &seg.app) {
                apps.push(seg.app.clone());
            }
        }
        let mut projects: Vec<ProjectSlice> = totals
            .into_iter()
            .map(|(name, secs)| ProjectSlice { name, secs })
            .collect();
        projects.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name)));
        TimelineRun {
            category_slug: self.slug,
            category_name: self.name,
            start: self.start,
            end: self.end,
            secs,
            projects,
            apps,
            segments: self.segments,
        }
    }
}

/// Coalesce active events into raw single-category runs: a run ends when the category changes or
/// a gap ≥ `IDLE_GAP_ENDS_RUN_SECS` opens; project changes never split (D34).
fn raw_runs(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
) -> Vec<SegRun> {
    let mut active: Vec<&ActivityLog> = events
        .iter()
        .filter(|e| !e.is_idle && duration(e) > 0)
        .collect();
    active.sort_by_key(|e| e.start_time);

    let mut runs: Vec<SegRun> = Vec::new();
    let mut current: Option<RawRunBuilder> = None;

    for event in active {
        let (slug, name) = category_of(event, categories);
        let project = event.project_id.and_then(|id| projects.get(&id).cloned());

        match current.take() {
            Some(mut run)
                if run.slug == slug && event.start_time - run.end < IDLE_GAP_ENDS_RUN_SECS =>
            {
                run.add(event, project);
                current = Some(run);
            }
            Some(run) => {
                runs.push(SegRun::from_raw(run));
                current = Some(RawRunBuilder::new(event, slug, name, project));
            }
            None => current = Some(RawRunBuilder::new(event, slug, name, project)),
        }
    }
    if let Some(run) = current {
        runs.push(SegRun::from_raw(run));
    }
    runs
}

/// Fold brief sandwiched excursions into the surrounding category-run (D34a). A maximal
/// contiguous block of non-X runs flanked by category X on both sides is absorbed into one X run
/// when ALL hold: the block's wall-clock span ≤ `ABSORB_SECS`; the host's active time ≥ the
/// excursion's (local dominance — a tiny host can't masquerade); and the run's total absorbed
/// time stays ≤ `MAX_ABSORB_FRACTION_PCT`% of its wall-clock (the accumulation backstop). The
/// detour's events stay as segments (with their real category) for the expand. Iterated to a
/// fixpoint, so `X | a | X | b | X` collapses left-to-right.
fn absorb_excursions(mut runs: Vec<SegRun>) -> Vec<SegRun> {
    loop {
        let mut target: Option<(usize, usize)> = None;
        for i in 0..runs.len() {
            let host = &runs[i].slug;
            // The maximal block of consecutive non-host runs after i, and its closing flanker j.
            let mut j = i + 1;
            while j < runs.len() && &runs[j].slug != host {
                j += 1;
            }
            // Need a non-empty block AND a closing flanker of the same category.
            if j >= runs.len() || j == i + 1 {
                continue;
            }
            // No idle gap ≥ threshold anywhere across the window (seams + internal).
            if (i..j).any(|k| runs[k + 1].start - runs[k].end >= IDLE_GAP_ENDS_RUN_SECS) {
                continue;
            }
            if runs[j - 1].end - runs[i + 1].start > ABSORB_SECS {
                continue; // excursion wall-clock span
            }
            let excursion_active: i64 = runs[i + 1..j].iter().map(SegRun::total_active).sum();
            if runs[i].host_active() + runs[j].host_active() < excursion_active {
                continue; // local dominance
            }
            let absorbed_after = runs[i].absorbed() + runs[j].absorbed() + excursion_active;
            let wall_after = runs[j].end - runs[i].start;
            if absorbed_after * 100 > wall_after * MAX_ABSORB_FRACTION_PCT {
                continue; // accumulation cap
            }
            target = Some((i, j));
            break;
        }
        match target {
            Some((i, j)) => {
                let name = runs[i].name.clone();
                let host = runs[i].slug.clone();
                let start = runs[i].start;
                let end = runs[j].end;
                let segments: Vec<TimelineSegment> =
                    runs.drain(i..=j).flat_map(|r| r.segments).collect();
                runs.insert(
                    i,
                    SegRun {
                        slug: host,
                        name,
                        start,
                        end,
                        segments,
                    },
                );
            }
            None => break,
        }
    }
    runs
}

/// The single segmentation pass (D34a): raw category-runs → excursion-absorb → rich runs with
/// segments. `build_runs` (dial / week) projects these to `CategoryRun`; `build_timeline` returns
/// them whole — one source of truth, so the dial and Timeline can never disagree.
fn build_segmented_runs(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
) -> Vec<TimelineRun> {
    absorb_excursions(raw_runs(events, categories, projects))
        .into_iter()
        .map(SegRun::finish)
        .collect()
}

/// Build the Timeline view: the day's category-runs, each with its inner app-switch segments
/// (D34/D34a) — the same segmentation the dial uses, with the segments retained for expand.
pub fn build_timeline(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
) -> TimelineView {
    TimelineView {
        runs: build_segmented_runs(events, categories, projects),
    }
}

/// Category-runs for the dial arcs + week mini-dials: the segmented runs (D34a) minus their
/// per-segment detail. The dial and Timeline share one segmentation, so they cannot diverge.
fn build_runs(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
) -> Vec<CategoryRun> {
    build_segmented_runs(events, categories, projects)
        .into_iter()
        .map(|run| CategoryRun {
            category_slug: run.category_slug,
            category_name: run.category_name,
            start: run.start,
            end: run.end,
            secs: run.secs,
            projects: run.projects,
            apps: run.apps,
        })
        .collect()
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
    categories: &[CategorySlice],
    runs: &[CategoryRun],
) -> Recap {
    let text = if active_secs == 0 || categories.is_empty() {
        "No activity tracked today yet.".to_string()
    } else {
        let top = &categories[0];
        let mut text = if categories.len() == 1 {
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
fn leading_project(runs: &[CategoryRun], active_secs: i64) -> Option<String> {
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

    fn ctx_map() -> HashMap<i64, CategoryMeta> {
        HashMap::from([
            (
                1,
                CategoryMeta {
                    slug: Some("deep".into()),
                    name: "Deep work".into(),
                },
            ),
            (
                2,
                CategoryMeta {
                    slug: Some("comms".into()),
                    name: "Comms".into(),
                },
            ),
            (
                3,
                CategoryMeta {
                    slug: Some("research".into()),
                    name: "Research".into(),
                },
            ),
        ])
    }

    fn proj_map() -> HashMap<i64, String> {
        HashMap::from([(1, "usageos".to_string()), (2, "nudge".to_string())])
    }

    #[test]
    fn aggregates_sum_per_category_and_exclude_idle() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // 10m deep
            ev(600, 900, "Slack", Some(2), None),   // 5m comms
            idle(900, 1200),                        // 5m idle
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.active_secs, 900);
        assert_eq!(day.idle_secs, 300);
        assert_eq!(day.categories.len(), 2);
        assert_eq!(day.categories[0].slug, "deep"); // longest first
        assert_eq!(day.categories[0].secs, 600);
        assert!((day.categories[0].pct - 66.666).abs() < 0.01);
    }

    #[test]
    fn same_category_coalesces_into_one_run_across_projects() {
        // Deep work bouncing usageos <-> nudge must stay ONE run (D34).
        let events = vec![
            ev(0, 300, "Cursor", Some(1), Some(1)),
            ev(300, 600, "Cursor", Some(1), Some(2)),
            ev(600, 900, "iTerm", Some(1), Some(1)),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), 1);
        let run = &day.runs[0];
        assert_eq!(run.category_slug, "deep");
        assert_eq!(run.start, 0);
        assert_eq!(run.end, 900);
        assert_eq!(run.apps, vec!["Cursor", "iTerm"]);
        assert_eq!(run.projects.len(), 2);
        assert_eq!(run.projects[0].name, "usageos"); // 600s vs 300s
        assert_eq!(run.projects[0].secs, 600);
    }

    #[test]
    fn category_change_splits_runs() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 900, "Slack", Some(2), None),
            ev(900, 1200, "Cursor", Some(1), Some(1)),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), 3);
        assert_eq!(day.runs[1].category_slug, "comms");
        assert_eq!(day.runs[1].projects[0].name, "No project");
    }

    #[test]
    fn large_gap_splits_same_category_into_two_runs() {
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
    fn uncategorized_active_time_becomes_other_category() {
        let events = vec![ev(0, 600, "Unknown", None, None)];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        assert_eq!(day.categories[0].slug, "other");
        assert_eq!(day.categories[0].name, "Uncategorized");
        assert_eq!(day.runs[0].category_slug, "other");
    }

    #[test]
    fn recap_is_empty_when_no_active_time() {
        let day = build_day_view(&[idle(0, 600)], &ctx_map(), &proj_map());
        assert_eq!(day.recap.text, "No activity tracked today yet.");
        assert_eq!(day.recap.generated_by, "template");
    }

    #[test]
    fn recap_names_the_leading_category_and_project() {
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
    fn timeline_splits_on_category_change_and_marks_no_project() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // deep
            ev(600, 900, "Slack", Some(2), None),   // comms → split
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 2);
        assert_eq!(tl.runs[1].category_slug, "comms");
        assert_eq!(
            tl.runs[1].segments[0].project, None,
            "unresolved project → None"
        );
    }

    #[test]
    fn timeline_splits_same_category_on_a_long_gap() {
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

    // ── D34a excursion-absorb ────────────────────────────────────────────────────

    #[test]
    fn absorb_folds_a_sandwiched_brief_excursion() {
        // deep → 50s comms glance → deep becomes ONE deep run; the comms stays an inner segment,
        // the run's secs/projects are host-only, and the ledger still counts the 50s as Comms.
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),    // deep / usageos
            ev(600, 650, "Slack", Some(2), None),      // comms 50s, no project
            ev(650, 1200, "Cursor", Some(1), Some(1)), // deep / usageos
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 1, "the brief comms glance is absorbed");
        let run = &tl.runs[0];
        assert_eq!(run.category_slug, "deep");
        assert_eq!(
            (run.start, run.end),
            (0, 1200),
            "the arc spans the whole block"
        );
        assert_eq!(
            run.secs, 1150,
            "secs is host (deep) only — the 50s comms is excluded"
        );
        assert_eq!(
            run.segments.len(),
            3,
            "the comms stays a segment for the expand"
        );
        assert_eq!(
            run.projects.len(),
            1,
            "no phantom 'No project' from the comms"
        );
        assert_eq!(run.projects[0].name, "usageos");
        let comms = run.segments.iter().find(|s| s.app == "Slack").unwrap();
        assert_eq!(
            comms.category_slug, "comms",
            "the absorbed segment keeps its real category"
        );
        // Totals are independent of segmentation (D34): the 50s is still Comms in the ledger.
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        let comms_total = day
            .categories
            .iter()
            .find(|c| c.slug == "comms")
            .unwrap()
            .secs;
        assert_eq!(comms_total, 50);
    }

    #[test]
    fn absorb_clusters_consecutive_excursions() {
        // deep → comms 30s → research 20s → deep: the whole 50s cluster folds into one deep run.
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 630, "Slack", Some(2), None),
            ev(630, 650, "Chrome", Some(3), None),
            ev(650, 1200, "Cursor", Some(1), Some(1)),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 1);
        assert_eq!(tl.runs[0].category_slug, "deep");
        assert_eq!(tl.runs[0].segments.len(), 4);
    }

    #[test]
    fn absorb_skips_a_too_long_excursion() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 600 + ABSORB_SECS + 30, "Slack", Some(2), None), // > ABSORB_SECS
            ev(600 + ABSORB_SECS + 30, 1500, "Cursor", Some(1), Some(1)),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(
            tl.runs.len(),
            3,
            "a long detour is a real block, not absorbed"
        );
    }

    #[test]
    fn absorb_keeps_an_unsandwiched_edge_blip() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 640, "Slack", Some(2), None), // 40s blip at the end, no deep after
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(
            tl.runs.len(),
            2,
            "an unsandwiched edge blip stays its own block"
        );
    }

    #[test]
    fn absorb_local_dominance_blocks_a_tiny_host() {
        // deep 5s → Slack 80s → deep 5s: host (10s) < excursion (80s) → NOT absorbed.
        let events = vec![
            ev(0, 5, "Cursor", Some(1), Some(1)),
            ev(5, 85, "Slack", Some(2), None),
            ev(85, 90, "Cursor", Some(1), Some(1)),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(
            tl.runs.len(),
            3,
            "a tiny host can't absorb a dominant detour"
        );
    }

    #[test]
    fn absorb_cap_splits_an_over_interrupted_run() {
        // deep 100s → comms 80s → deep 100s: dominance ok (80<200), but absorbed 80 / wall 280
        // = 28.6% > 15% → the accumulation cap forbids it.
        let events = vec![
            ev(0, 100, "Cursor", Some(1), Some(1)),
            ev(100, 180, "Slack", Some(2), None),
            ev(180, 280, "Cursor", Some(1), Some(1)),
        ];
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(tl.runs.len(), 3, "the cap forbids a run that's >15% detour");
    }

    #[test]
    fn build_runs_is_the_timeline_projection() {
        // The dial's CategoryRuns are exactly the Timeline runs minus segments (one source).
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)),
            ev(600, 650, "Slack", Some(2), None), // absorbed into the deep run
            ev(650, 1200, "Cursor", Some(1), Some(1)),
            ev(1200, 1500, "Slack", Some(2), None), // tail comms — its own run
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map());
        let tl = build_timeline(&events, &ctx_map(), &proj_map());
        assert_eq!(day.runs.len(), tl.runs.len());
        for (cr, tr) in day.runs.iter().zip(tl.runs.iter()) {
            assert_eq!(cr.category_slug, tr.category_slug);
            assert_eq!((cr.start, cr.end, cr.secs), (tr.start, tr.end, tr.secs));
        }
    }
}
