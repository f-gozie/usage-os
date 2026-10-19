//! Read-time rollup: turn a day's raw events into the view the dial renders. Pure (no DB/IO),
//! so it's trivially testable and cheap to re-run; this is where "numbers are computed in Rust"
//! lives for the dial (hard rule 6). See D34.
//!
//! Two shapes come out: **per-axis aggregates** (`categories`) — plain sums per category that
//! feed the ledger/legend/stats/centre; and **category-runs** (`runs`) — continuous stretches of
//! one category with the project split inside, feeding the dial arcs + timeline.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::{ActivityLog, CoverageGap};

/// An idle/untracked gap at least this long ends a category-run. Dogfood-tunable; see D34a.
const IDLE_GAP_ENDS_RUN_SECS: i64 = 5 * 60;

/// Excursion-absorb: a brief detour into another category, sandwiched by the same category, folds
/// into the surrounding run (still shown as a segment in the expand). A detour whose wall-clock
/// span exceeds this stays its own run. Kept below `IDLE_GAP_ENDS_RUN_SECS`. Dogfood-tunable; see D34a.
const ABSORB_SECS: i64 = 180;
/// Backstop: a run may absorb at most this percent of its wall-clock as detours before it splits,
/// so the dial never draws a falsely-unbroken focus stretch. Dogfood-tunable; see D34a.
const MAX_ABSORB_FRACTION_PCT: i64 = 15;

/// Slug + display name for a category, looked up by category id. The dial maps `slug`
/// to a colour token (`--c-<slug>`). Internal — does not cross the IPC boundary.
pub struct CategoryMeta {
    pub slug: Option<String>,
    pub name: String,
    /// Stored hex colour — rendered for user-created (slug-less) categories; canonical
    /// categories colour by their slug token instead.
    pub color: String,
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
    /// Hex colour for a user-created category; `None` for canonical (coloured by slug) or
    /// uncategorized.
    pub color: Option<String>,
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
    /// Hex colour for a user-created category; `None` for canonical / uncategorized.
    pub category_color: Option<String>,
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
    /// Stretches of this day when UsageOS wasn't running, so the dial can show a blind spot
    /// instead of implying an idle one. Empty for a day we have no coverage record for.
    pub gaps: Vec<CoverageGap>,
    /// Seconds of this day nobody was watching — the sum of `gaps`, computed in Rust (rule 6).
    pub unwatched_secs: i64,
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
    /// Seconds of this day UsageOS wasn't running. 0 also means "we have no coverage record
    /// for this day" — callers distinguish the two with `coverage_known`.
    pub unwatched_secs: i64,
    /// Whether coverage was being recorded at all on this day. False for days that predate the
    /// coverage log: we cannot say what happened, and must not imply failure.
    pub coverage_known: bool,
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
    /// Hex colour for a user-created category segment; `None` for canonical / uncategorized.
    pub category_color: Option<String>,
    /// Resolved project name, or `None` when none was inferred.
    pub project: Option<String>,
    /// The focused-window title for this stretch — `None` when empty or blanked for a private
    /// window (D8). Shown in the Timeline expand so a non-project stretch still says what it was.
    pub title: Option<String>,
    pub secs: i64,
}

/// A category-run plus its inner app-switch segments — one expandable Timeline row (D34).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TimelineRun {
    pub category_slug: String,
    pub category_name: String,
    /// Hex colour for a user-created category; `None` for canonical / uncategorized.
    pub category_color: Option<String>,
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
    /// Seconds of this day UsageOS wasn't running, so an empty timeline can say which of the
    /// two reasons it is (computed in Rust, hard rule 6).
    pub unwatched_secs: i64,
}

fn duration(event: &ActivityLog) -> i64 {
    (event.end_time - event.start_time).max(0)
}

/// Resolve an event to its `(key, name, color)`. The key is the colour/identity used to
/// coalesce runs and aggregate slices: a canonical category uses its `slug` (→ theme token);
/// a user-created (slug-less) category uses a per-id key `cat-<id>` so distinct user categories
/// never collide with each other or with truly-uncategorized time, and carries its hex `color`
/// for the frontend to render. Only `category_id = None`/unknown is the neutral "other".
fn category_of(
    event: &ActivityLog,
    categories: &HashMap<i64, CategoryMeta>,
) -> (String, String, Option<String>) {
    match event
        .category_id
        .and_then(|id| categories.get(&id).map(|m| (id, m)))
    {
        Some((id, meta)) => match &meta.slug {
            Some(slug) => (slug.clone(), meta.name.clone(), None),
            None => (
                format!("cat-{id}"),
                meta.name.clone(),
                Some(meta.color.clone()),
            ),
        },
        None => (OTHER_SLUG.to_string(), OTHER_NAME.to_string(), None),
    }
}

/// Build the Day view from a day's events plus category/project name lookups. `day_start`
/// is the day's local midnight (Unix secs, owned by the caller like `get_day`) — used only
/// to phrase the recap's time-of-day ("in the morning"), never for bucketing.
pub fn build_day_view(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
    day_start: i64,
    gaps: Vec<CoverageGap>,
    now: i64,
) -> DayView {
    let (active_secs, idle_secs, category_slices) = aggregate_categories(events, categories);
    let runs = build_runs(events, categories, projects);
    let unwatched_secs = crate::db::unwatched_secs(&gaps);
    let facts = compute_recap_facts(&DayInput {
        events,
        categories,
        slices: &category_slices,
        runs: &runs,
        gaps: &gaps,
        day_start,
        now,
        active_secs,
    });
    let recap = render_template_recap(&facts);

    DayView {
        active_secs,
        idle_secs,
        categories: category_slices,
        runs,
        recap,
        gaps,
        unwatched_secs,
    }
}

/// Aggregate a day's events into `(active_secs, idle_secs, category_slices)` — the slices
/// sorted longest-first (slug as tie-breaker). Shared by the Day view and the recap-facts
/// builder so both see identical totals.
fn aggregate_categories(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
) -> (i64, i64, Vec<CategorySlice>) {
    let mut active_secs = 0;
    let mut idle_secs = 0;
    // key -> (name, secs, color); name/color kept for display, secs accumulated.
    let mut totals: HashMap<String, (String, i64, Option<String>)> = HashMap::new();

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
        let (slug, name, color) = category_of(event, categories);
        let entry = totals.entry(slug).or_insert((name, 0, color));
        entry.1 += secs;
    }

    let mut category_slices: Vec<CategorySlice> = totals
        .into_iter()
        .map(|(slug, (name, secs, color))| CategorySlice {
            slug,
            name,
            secs,
            pct: if active_secs > 0 {
                secs as f64 / active_secs as f64 * 100.0
            } else {
                0.0
            },
            color,
        })
        .collect();
    // Deterministic order: longest first, slug as the tie-breaker.
    category_slices.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.slug.cmp(&b.slug)));

    (active_secs, idle_secs, category_slices)
}

/// Compute just the day's [`RecapFacts`] from its events — the same aggregation
/// [`build_day_view`] runs, minus the template recap. The lazy `get_recap` command feeds
/// these to the Foundation Models narrator ([`crate::ai::build_recap`]), which falls back to
/// the template (D48) on any failure (hard rule 6 / C5).
pub(crate) fn build_recap_facts(
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
    day_start: i64,
    gaps: &[CoverageGap],
    now: i64,
) -> RecapFacts {
    let (active_secs, _idle_secs, category_slices) = aggregate_categories(events, categories);
    let runs = build_runs(events, categories, projects);
    compute_recap_facts(&DayInput {
        events,
        categories,
        slices: &category_slices,
        runs: &runs,
        gaps,
        day_start,
        now,
        active_secs,
    })
}

/// One day's slice for the Week grid: per-day active + deep totals and the dial arcs. Reuses
/// `build_runs`; no recap/ledger (the Week view doesn't show them). `day_start` is passed
/// through as the mini-dial origin (the caller owns local day bounds, like `get_day`).
pub fn build_day_slice(
    day_start: i64,
    events: &[ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    projects: &HashMap<i64, String>,
    unwatched_secs: i64,
    coverage_known: bool,
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
        unwatched_secs,
        coverage_known,
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

// ── Calendar: the Month and Year rungs (D73) ─────────────────────────────────

/// Everything the Month grid needs: one slice per day plus the two facts that are statements,
/// not scores. Deliberately no "busiest day" and no category share — a superlative and a
/// percentage are the first step toward a scoreboard, and this product cut scoring on purpose.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct MonthView {
    pub days: Vec<DaySlice>,
    pub total_active_secs: i64,
    /// Days with any tracked activity at all.
    pub tracked_days: i64,
    /// Days UsageOS was not running for essentially the whole of — the count the user is owed.
    pub unrecorded_days: i64,
}

/// One day at Year scale: a total, the category split that draws its stacked bar, and whether
/// anything was watching. No runs — a 24 h dial is illegible at a year cell's ~29px, so the
/// Year rung deliberately drops "when" rather than drawing it too small to read.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct YearDay {
    pub day_start: i64,
    pub active_secs: i64,
    pub categories: Vec<CategorySlice>,
    pub unwatched_secs: i64,
    pub coverage_known: bool,
}

/// A year of days. No totals, no averages, no best month: this is a field to look at, and
/// "long-range trends" is the part of the vision's out-of-scope line that still binds.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct YearView {
    pub days: Vec<YearDay>,
}

/// A day counts as unrecorded when essentially none of it was watched — the same threshold the
/// recap uses to decide the day has no story rather than a partial one.
pub const DAY_UNRECORDED_SECS: i64 = 23 * 3600;

/// Month aggregates from the day slices (numbers in Rust, hard rule 6). Pure.
pub fn build_month_view(days: Vec<DaySlice>) -> MonthView {
    let total_active_secs: i64 = days.iter().map(|d| d.active_secs).sum();
    let tracked_days = days.iter().filter(|d| d.active_secs > 0).count() as i64;
    let unrecorded_days = days
        .iter()
        .filter(|d| d.coverage_known && d.unwatched_secs >= DAY_UNRECORDED_SECS)
        .count() as i64;
    MonthView {
        days,
        total_active_secs,
        tracked_days,
        unrecorded_days,
    }
}

/// Turn the repository's per-day/per-category totals into the Year view.
///
/// `totals` is indexed by position in `day_starts`; `unwatched` and `known` are parallel to it.
/// Pure, so the whole shape is testable without a database.
pub fn build_year_view(
    day_starts: &[i64],
    totals: &[crate::db::DayCategoryTotal],
    categories: &HashMap<i64, CategoryMeta>,
    unwatched: &[i64],
    known: &[bool],
) -> YearView {
    let mut days: Vec<YearDay> = day_starts
        .iter()
        .enumerate()
        .map(|(i, &day_start)| YearDay {
            day_start,
            active_secs: 0,
            categories: Vec::new(),
            unwatched_secs: unwatched.get(i).copied().unwrap_or(0),
            coverage_known: known.get(i).copied().unwrap_or(false),
        })
        .collect();

    for total in totals {
        let Some(day) = days.get_mut(total.day) else {
            continue;
        };
        let (slug, name, color) = match total.category_id.and_then(|id| categories.get(&id)) {
            Some(meta) => (
                meta.slug.clone().unwrap_or_default(),
                meta.name.clone(),
                meta.slug.is_none().then(|| meta.color.clone()),
            ),
            None => (String::new(), OTHER_NAME.to_string(), None),
        };
        day.active_secs += total.secs;
        day.categories.push(CategorySlice {
            slug,
            name,
            secs: total.secs,
            pct: 0.0,
            color,
        });
    }

    // Percentages last, so each is a share of that day's own total, and longest-first so the
    // stacked bar is stable and the dominant category reads first.
    for day in &mut days {
        for slice in &mut day.categories {
            slice.pct = if day.active_secs > 0 {
                slice.secs as f64 / day.active_secs as f64 * 100.0
            } else {
                0.0
            };
        }
        day.categories
            .sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.slug.cmp(&b.slug)));
    }

    YearView { days }
}

/// In-progress raw run: consecutive same-category events accumulated as segments. Coalesced
/// into a [`SegRun`]; the absorb pass (D34a) then merges brief sandwiched excursions.
struct RawRunBuilder {
    slug: String,
    name: String,
    color: Option<String>,
    start: i64,
    end: i64,
    segments: Vec<TimelineSegment>,
}

impl RawRunBuilder {
    fn new(
        event: &ActivityLog,
        slug: String,
        name: String,
        color: Option<String>,
        project: Option<String>,
    ) -> Self {
        let mut builder = RawRunBuilder {
            slug,
            name,
            color,
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
            category_color: self.color.clone(),
            project,
            title: Some(event.window_title.clone()).filter(|t| !t.is_empty()),
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
    color: Option<String>,
    start: i64,
    end: i64,
    segments: Vec<TimelineSegment>,
}

impl SegRun {
    fn from_raw(raw: RawRunBuilder) -> Self {
        SegRun {
            slug: raw.slug,
            name: raw.name,
            color: raw.color,
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
        // Headline numbers are host-category only: a "Deep · 52m" run means 52m of Deep; an
        // off-category detour never injects a phantom project slice (D34a). The detour's seconds
        // live on its segment (the expand) and in the per-axis ledger.
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
            category_color: self.color,
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
        let (slug, name, color) = category_of(event, categories);
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
                current = Some(RawRunBuilder::new(event, slug, name, color, project));
            }
            None => current = Some(RawRunBuilder::new(event, slug, name, color, project)),
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
                let color = runs[i].color.clone();
                let start = runs[i].start;
                let end = runs[j].end;
                let segments: Vec<TimelineSegment> =
                    runs.drain(i..=j).flat_map(|r| r.segments).collect();
                runs.insert(
                    i,
                    SegRun {
                        slug: host,
                        name,
                        color,
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
    unwatched_secs: i64,
) -> TimelineView {
    TimelineView {
        runs: build_segmented_runs(events, categories, projects),
        unwatched_secs,
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
            category_color: run.category_color,
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

/// A run shorter than this isn't worth calling out as "your longest stretch" — below it,
/// the recap omits the focus sentence rather than narrate a trivial 4-minute window.
const RECAP_MIN_FOCUS_SECS: i64 = 900; // 15 min

/// One category's contribution, named for the recap. (`name` is the display name.)
#[derive(Debug, Clone)]
pub struct CategoryFact {
    pub name: String,
    pub secs: i64,
}

/// The day's single longest continuous category stretch, with a rough time-of-day.
#[derive(Debug, Clone)]
pub struct FocusFact {
    pub secs: i64,
    /// A prepositional phrase ready to drop after "ran 1h 12m …" (e.g. "in the morning").
    pub when: &'static str,
}

/// The deterministic *facts* of a day — computed in Rust (hard rule 6). The app writes the recap
/// from these ([`render_template_recap`]); the on-device model is not called for it (D74).
#[derive(Debug, Clone, Default)]
pub struct RecapFacts {
    pub active_secs: i64,
    /// The biggest category by time, if any activity was tracked.
    pub leading: Option<CategoryFact>,
    /// The runner-up category, when a second one had time (drives "then X at …").
    pub second: Option<CategoryFact>,
    /// The one project that clearly led (≥40% of active time); never a guessed project.
    pub leading_project: Option<String>,
    /// The longest continuous stretch, only when it's substantial (≥ [`RECAP_MIN_FOCUS_SECS`]).
    pub longest_focus: Option<FocusFact>,
    /// Seconds of the day UsageOS wasn't running. A fact like any other: the recap states it
    /// plainly and never speculates about it.
    pub unwatched_secs: i64,
    /// When the day ran, from its first to its last real stretch of activity.
    pub span: Option<SpanFact>,
    /// Activity just after midnight that the night's sleep separates from the day proper.
    pub after_midnight_secs: Option<i64>,
    /// What each part of the day mostly went to, neighbouring parts that agree merged.
    pub parts: Vec<PartFact>,
    /// The apps and sites the top two categories mostly happened in.
    pub places: Vec<PlacesFact>,
    pub texture: Option<Texture>,
    /// The longest time away inside the span — never a stretch UsageOS wasn't running.
    pub longest_break: Option<BreakFact>,
}

/// Seconds since the day's local midnight, so the phrasing needs no time-zone library.
#[derive(Debug, Clone)]
pub struct SpanFact {
    pub from: i64,
    pub to: i64,
    /// The day isn't over yet: the recap says "so far", never "until".
    pub ongoing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartFact {
    /// "morning", or merged: "morning and afternoon".
    pub part: String,
    /// One category, or two: "Messaging and Browsing".
    pub what: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacesFact {
    pub category: String,
    pub places: Vec<Place>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub name: String,
    /// A website ("on x.com") rather than an app ("in Figma").
    pub is_site: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Texture {
    LongStretches,
    ShortPieces,
}

#[derive(Debug, Clone)]
pub struct BreakFact {
    pub secs: i64,
    /// Seconds since midnight when it began.
    pub from: i64,
}

// Day-shape thresholds (D74), tuned on `design/recap.html`.
/// A day needs this much activity before its start and end are worth saying.
const SPAN_MIN_ACTIVE_SECS: i64 = 3600;
/// Runs shorter than this don't move the start or end (a glance at the clock at 6am).
const SPAN_RUN_MIN_SECS: i64 = 5 * 60;
/// Activity ending before this, then a long gap, belongs to the night before.
const NIGHT_ENDS_SECS: i64 = 5 * 3600;
const OVERNIGHT_GAP_SECS: i64 = 3 * 3600;
const AFTER_MIDNIGHT_MIN_SECS: i64 = 15 * 60;
/// Morning, afternoon and evening, in seconds since midnight. Before 5am is the night's.
const PARTS: [(&str, i64, i64); 3] = [
    ("morning", 5 * 3600, 12 * 3600),
    ("afternoon", 12 * 3600, 17 * 3600),
    ("evening", 17 * 3600, 24 * 3600),
];
const PART_MIN_ACTIVE_SECS: i64 = 45 * 60;
/// One category is "mostly" a part at this share; two together at the second.
const PART_ONE_PCT: i64 = 55;
const PART_TWO_PCT: i64 = 65;
/// An app or site is named only with this much time, and this share of its category.
const PLACE_MIN_SECS: i64 = 20 * 60;
const PLACE_MIN_PCT: i64 = 20;
const LONG_STRETCH_SECS: i64 = 30 * 60;
const TEXTURE_MIN_ACTIVE_SECS: i64 = 90 * 60;
const TEXTURE_LONG_PCT: i64 = 60;
const TEXTURE_SHORT_PCT: i64 = 25;
const TEXTURE_SHORT_MIN_RUNS: usize = 8;
const BREAK_MIN_SECS: i64 = 75 * 60;

/// A rough, local time-of-day phrase for the recap, from the hour-of-day (0–23) of a run's
/// start. Deliberately coarse — an honest "in the morning", not a false-precision clock time.
fn time_of_day_phrase(hour: i64) -> &'static str {
    match hour {
        5..=7 => "early in the morning",
        8..=11 => "in the morning",
        12 => "around midday",
        13..=16 => "in the afternoon",
        17..=20 => "in the evening",
        _ => "at night",
    }
}

/// A clock time rounded to the half hour, with a coarse part of day: "around 8:30 in the
/// morning", "around noon", "around midnight". `secs` is seconds since midnight.
fn clock_phrase(secs: i64) -> String {
    let mins = (secs.clamp(0, 86_400) + 900) / 1800 * 30;
    let (hour, minute) = ((mins / 60) % 24, mins % 60);
    match (hour, minute) {
        (0, 0) => return "around midnight".to_string(),
        (12, 0) => return "around noon".to_string(),
        _ => {}
    }
    let h12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    let clock = if minute == 0 {
        h12.to_string()
    } else {
        format!("{h12}:30")
    };
    let part = match hour {
        0..=4 => "at night",
        5..=11 => "in the morning",
        12..=16 => "in the afternoon",
        17..=20 => "in the evening",
        _ => "at night",
    };
    format!("around {clock} {part}")
}

/// Everything the facts are computed from. One struct so the two callers can't drift.
struct DayInput<'a> {
    events: &'a [ActivityLog],
    categories: &'a HashMap<i64, CategoryMeta>,
    slices: &'a [CategorySlice],
    runs: &'a [CategoryRun],
    gaps: &'a [CoverageGap],
    /// The local midnight, so `t - day_start` is the local time of day.
    day_start: i64,
    now: i64,
    active_secs: i64,
}

fn compute_recap_facts(day: &DayInput) -> RecapFacts {
    let category = |c: &CategorySlice| CategoryFact {
        name: c.name.clone(),
        secs: c.secs,
    };
    let leading = day.slices.first().map(category);
    let second = day.slices.get(1).filter(|c| c.secs > 0).map(category);
    let longest_focus = day
        .runs
        .iter()
        .max_by_key(|r| r.secs)
        .filter(|r| r.secs >= RECAP_MIN_FOCUS_SECS)
        .map(|r| FocusFact {
            secs: r.secs,
            when: time_of_day_phrase((r.start - day.day_start) / 3600),
        });

    let mut active: Vec<&ActivityLog> = day
        .events
        .iter()
        .filter(|e| !e.is_idle && duration(e) > 0)
        .collect();
    active.sort_by_key(|e| e.start_time);

    // The night's tail: activity before 5am that a gap of 3h+ separates from the rest.
    let night_ends = day.day_start + NIGHT_ENDS_SECS;
    let tail_len = active
        .windows(2)
        .rposition(|w| {
            w[0].end_time <= night_ends && w[1].start_time - w[0].end_time >= OVERNIGHT_GAP_SECS
        })
        .map_or(0, |i| i + 1);
    let tail_secs: i64 = active[..tail_len].iter().map(|e| duration(e)).sum();
    let day_begins = active[..tail_len].last().map(|e| e.end_time);

    // The day's real stretches: past the tail, and long enough to move the start or end.
    let real: Vec<&CategoryRun> = day
        .runs
        .iter()
        .filter(|r| r.secs >= SPAN_RUN_MIN_SECS && !matches!(day_begins, Some(t) if r.start < t))
        .collect();

    let span = match (real.first(), real.last()) {
        (Some(first), Some(last)) if day.active_secs >= SPAN_MIN_ACTIVE_SECS => Some(SpanFact {
            from: first.start - day.day_start,
            to: last.end - day.day_start,
            ongoing: day.now >= day.day_start && day.now < day.day_start + 86_400,
        }),
        _ => None,
    };

    // Measured over every bit of activity, not just the runs that set the span: a few scattered
    // minutes in the middle of an afternoon mean it wasn't a break.
    let longest_break = match (real.first(), real.last()) {
        (Some(first), Some(last)) => {
            let mut away_from = first.start;
            let mut best: Option<BreakFact> = None;
            for e in &active[tail_len..] {
                if e.end_time <= first.start || e.start_time >= last.end {
                    continue;
                }
                let (from, to) = (away_from, e.start_time);
                let blind = day.gaps.iter().any(|g| g.start < to && g.end > from);
                if !blind
                    && to - from >= BREAK_MIN_SECS
                    && best.as_ref().is_none_or(|b| to - from > b.secs)
                {
                    best = Some(BreakFact {
                        secs: to - from,
                        from: from - day.day_start,
                    });
                }
                away_from = away_from.max(e.end_time);
            }
            best
        }
        _ => None,
    };

    let texture = (day.active_secs >= TEXTURE_MIN_ACTIVE_SECS).then(|| {
        let in_long: i64 = day
            .runs
            .iter()
            .filter(|r| r.secs >= LONG_STRETCH_SECS)
            .map(|r| r.secs)
            .sum();
        let pct = in_long * 100 / day.active_secs;
        if pct >= TEXTURE_LONG_PCT {
            Some(Texture::LongStretches)
        } else if pct <= TEXTURE_SHORT_PCT && day.runs.len() >= TEXTURE_SHORT_MIN_RUNS {
            Some(Texture::ShortPieces)
        } else {
            None
        }
    });

    RecapFacts {
        active_secs: day.active_secs,
        leading,
        second,
        leading_project: leading_project(day.runs, day.active_secs),
        longest_focus,
        unwatched_secs: crate::db::unwatched_secs(day.gaps),
        span,
        after_midnight_secs: (tail_secs >= AFTER_MIDNIGHT_MIN_SECS).then_some(tail_secs),
        parts: day_parts(&active, day.categories, day.day_start),
        places: day
            .slices
            .iter()
            .take(2)
            .filter(|c| c.secs > 0)
            .filter_map(|c| places_of(c, &active, day.categories))
            .collect(),
        texture: texture.flatten(),
        longest_break,
    }
}

/// What each part of the day mostly went to. A part with too little activity, or no clear
/// leader, says nothing rather than something vague.
fn day_parts(
    active: &[&ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
    day_start: i64,
) -> Vec<PartFact> {
    let mut parts: Vec<PartFact> = Vec::new();
    for (name, from, to) in PARTS {
        let (from, to) = (day_start + from, day_start + to);
        let mut totals: HashMap<String, i64> = HashMap::new();
        for e in active {
            let overlap = e.end_time.min(to) - e.start_time.max(from);
            if overlap > 0 {
                *totals.entry(category_of(e, categories).1).or_insert(0) += overlap;
            }
        }
        let total: i64 = totals.values().sum();
        if total < PART_MIN_ACTIVE_SECS {
            continue;
        }
        let mut ranked: Vec<(String, i64)> = totals.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let what = match ranked.as_slice() {
            [(one, secs), ..] if secs * 100 >= total * PART_ONE_PCT => one.clone(),
            [(one, a), (two, b), ..] if (a + b) * 100 >= total * PART_TWO_PCT => {
                format!("{one} and {two}")
            }
            _ => continue,
        };
        // Neighbouring parts that say the same thing merge: "afternoon and evening".
        match parts.last_mut() {
            Some(prev) if prev.what == what => prev.part = format!("{} and {name}", prev.part),
            _ => parts.push(PartFact {
                part: name.to_string(),
                what,
            }),
        }
    }
    parts
}

/// The one or two apps or sites a category mostly happened in. A site names the place when
/// there is one; a browser's own name never does (it says nothing the category doesn't).
fn places_of(
    slice: &CategorySlice,
    active: &[&ActivityLog],
    categories: &HashMap<i64, CategoryMeta>,
) -> Option<PlacesFact> {
    let browsers: Vec<&str> = active
        .iter()
        .filter(|e| e.site.is_some())
        .map(|e| e.process_name.as_str())
        .collect();
    let mut totals: HashMap<(String, bool), i64> = HashMap::new();
    for e in active {
        if category_of(e, categories).0 != slice.slug {
            continue;
        }
        let place = match e.site.as_deref().map(clean_label).filter(|s| !s.is_empty()) {
            Some(site) => (site, true),
            None if browsers.contains(&e.process_name.as_str()) => continue,
            None => (clean_label(&e.process_name), false),
        };
        if !place.0.is_empty() {
            *totals.entry(place).or_insert(0) += duration(e);
        }
    }
    let mut ranked: Vec<((String, bool), i64)> = totals
        .into_iter()
        .filter(|(_, secs)| *secs >= PLACE_MIN_SECS && secs * 100 >= slice.secs * PLACE_MIN_PCT)
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let places: Vec<Place> = ranked
        .into_iter()
        .take(2)
        .map(|((name, is_site), _)| Place { name, is_site })
        .collect();
    (!places.is_empty()).then(|| PlacesFact {
        category: slice.name.clone(),
        places,
    })
}

/// App names can carry invisible direction marks (WhatsApp's starts with U+200E); a recap
/// sentence must not.
fn clean_label(name: &str) -> String {
    name.chars()
        .filter(|c| {
            !matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// "a and b", or "a, b, and c".
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// "in Figma and Linear", "on x.com", "in Slack and on github.com" — apps are in, sites are on.
fn places_phrase(places: &[Place]) -> String {
    let words: Vec<String> = places
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let same_kind = i > 0 && places[i - 1].is_site == p.is_site;
            if same_kind {
                p.name.clone()
            } else {
                format!("{} {}", if p.is_site { "on" } else { "in" }, p.name)
            }
        })
        .collect();
    and_list(&words)
}

/// Below this much missing time the gap isn't worth mentioning in prose — a reboot or a short
/// crash doesn't change how the day reads.
const RECAP_GAP_MENTION_SECS: i64 = 30 * 60;

/// A whole day is "not recorded" only when essentially none of it was watched. Below this the
/// day still has a story; the missing stretch is a clause, not the headline.
const MOSTLY_UNWATCHED_SECS: i64 = 23 * 3600;

/// The recap: the shape of the day in plain sentences — when it ran, what each part went to,
/// where, and how the time came. Purely descriptive, never evaluative (the calm rear-view
/// mirror, not a coach: no "productive"/"focused"/"distracted", no comparison with other days).
pub(crate) fn render_template_recap(facts: &RecapFacts) -> Recap {
    let text = match &facts.leading {
        // Nothing tracked. Which of the two reasons it is matters more than anything else the
        // recap says: a day you rested and a day the recorder was dead are opposite facts.
        None if facts.unwatched_secs >= MOSTLY_UNWATCHED_SECS => {
            "UsageOS wasn’t running on this day, so there’s nothing to show.".to_string()
        }
        None => "No activity tracked today yet.".to_string(),
        Some(leading) => {
            let total = human_secs(facts.active_secs);
            let counting = facts
                .after_midnight_secs
                .map(|secs| format!(", counting {} just after midnight", human_secs(secs)))
                .unwrap_or_default();
            let mut out = vec![match &facts.span {
                Some(span) if span.ongoing => format!(
                    "You’ve been active since {}, {total} so far{counting}.",
                    clock_phrase(span.from)
                ),
                Some(span) => format!(
                    "You were active from {} until {}, {total} in all{counting}.",
                    clock_phrase(span.from),
                    clock_phrase(span.to)
                ),
                None => format!("{total} tracked."),
            }];

            if facts.parts.is_empty() {
                out.push(match &facts.second {
                    Some(second) => format!(
                        "{} led at {}, then {} at {}.",
                        leading.name,
                        human_secs(leading.secs),
                        second.name,
                        human_secs(second.secs),
                    ),
                    None => format!("All of it was {}.", leading.name),
                });
            } else {
                // "The morning went to Messaging, the afternoon to Work and Browsing, and …"
                let parts: Vec<String> = facts
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| match i {
                        0 => format!("The {} went to {}", p.part, p.what),
                        _ => format!("the {} to {}", p.part, p.what),
                    })
                    .collect();
                // Clauses take a comma even when there are two: a part's "what" may hold its own "and".
                let sentence = match parts.as_slice() {
                    [a, b] => format!("{a}, and {b}"),
                    _ => and_list(&parts),
                };
                out.push(format!("{sentence}."));
            }

            if !facts.places.is_empty() {
                let places: Vec<String> = facts
                    .places
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let verb = if i == 0 { " was" } else { "" };
                        format!("{}{verb} mostly {}", p.category, places_phrase(&p.places))
                    })
                    .collect();
                out.push(format!("{}.", places.join("; ")));
            }

            if let Some(focus) = &facts.longest_focus {
                let (secs, when) = (human_secs(focus.secs), focus.when);
                out.push(match facts.texture {
                    Some(Texture::ShortPieces) => {
                        format!("It came in short pieces; the longest ran {secs} {when}.")
                    }
                    Some(Texture::LongStretches) => {
                        format!("Most of it came in long stretches, the longest {secs} {when}.")
                    }
                    None => format!("Your longest stretch ran {secs} {when}."),
                });
            }
            if let Some(gap) = &facts.longest_break {
                out.push(format!(
                    "The longest break was {}, from {}.",
                    human_secs(gap.secs),
                    clock_phrase(gap.from)
                ));
            }
            if let Some(project) = &facts.leading_project {
                out.push(format!("Mostly on {project}."));
            }
            // Appended, never prepended: the day still had a shape, and this is a footnote about
            // the recorder, not a verdict on you. Stated plainly so the totals read as partial.
            if facts.unwatched_secs >= RECAP_GAP_MENTION_SECS {
                out.push(format!(
                    "UsageOS wasn’t running for {} of it, so this is only part of the day.",
                    human_secs(facts.unwatched_secs),
                ));
            }
            out.join(" ")
        }
    };

    Recap {
        text,
        generated_by: "template".to_string(),
    }
}

/// Spell a duration out in full words for the AI prompt ("4 hours 53 minutes") — never the "47m"
/// shorthand, which the model misreads as "47 million". Prompt-only; the UI keeps [`human_secs`].
fn human_secs_long(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let plural = |n: i64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    match (hours, minutes) {
        (0, 0) => plural(secs.max(0), "second"),
        (0, m) => plural(m, "minute"),
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}

/// Format [`RecapFacts`] into the prompt the Foundation Models sidecar narrates. Numbers are
/// pre-formatted, units spelled out, fields labeled ("category" vs "project") so the model can't
/// conflate them — it only phrases this, never computes (hard rule 6).
pub(crate) fn format_recap_prompt(facts: &RecapFacts) -> String {
    let mut lines =
        vec!["The day's facts, already computed — narrate only these, change nothing:".to_string()];
    lines.push(format!(
        "- Total active time: {}",
        human_secs_long(facts.active_secs)
    ));
    if let Some(leading) = &facts.leading {
        lines.push(format!(
            "- Leading category: {}, {}",
            leading.name,
            human_secs_long(leading.secs)
        ));
    }
    if let Some(second) = &facts.second {
        lines.push(format!(
            "- Runner-up category: {}, {}",
            second.name,
            human_secs_long(second.secs)
        ));
    }
    if let Some(focus) = &facts.longest_focus {
        lines.push(format!(
            "- Longest unbroken stretch: {}, {}",
            human_secs_long(focus.secs),
            focus.when
        ));
    }
    if let Some(project) = &facts.leading_project {
        lines.push(format!("- Main project: {project}"));
    }
    // Only when there is a real gap, so a normal day's prompt — and therefore its cached
    // narration (D52) — is byte-identical to before this shipped.
    if facts.unwatched_secs >= RECAP_GAP_MENTION_SECS {
        lines.push(format!(
            "- UsageOS was not running for {} of this day, so these totals cover only part of it. \
             Say so plainly at the end. Do not guess what happened during that time.",
            human_secs_long(facts.unwatched_secs)
        ));
    }
    lines.join("\n")
}

/// Bumped whenever the model's *input or behavior* changes in a way that should re-narrate
/// every day — the prompt format here, OR the Swift sidecar's instructions/temperature. The
/// recap fingerprint folds this in, so a bump invalidates every cached recap (D52): they
/// regenerate once, under the new version.
/// - 1: the initial shipped recap (D51).
/// - 2: instructions stop opening with "today"/the day name; narrate in the past tense.
/// - 3: voice tuned (eval-driven) — calm, human prose that drops the "leading category / runner-up / total active time" scaffolding, with faithfulness + verbatim-name guards.
pub(crate) const RECAP_CACHE_VERSION: u32 = 3;

/// A stable content fingerprint of a day's recap facts, the key for the recap cache (D52). It
/// hashes the EXACT prompt the model narrates (identical facts → identical key) plus the cache
/// version. Because the key *is* the content, invalidation is free: a rule reprocess that
/// changes a day's facts yields a new fingerprint and the old cached row is never matched.
/// The fingerprint must cover everything that determines the prose — if the narrator ever
/// takes input beyond this prompt, fold it in here too.
pub(crate) fn recap_fingerprint(facts: &RecapFacts) -> String {
    // FNV-1a (64-bit): small, stable, dependency-free — the same hash the migration runner uses.
    let input = format!("v{RECAP_CACHE_VERSION}\n{}", format_recap_prompt(facts));
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
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
    // Rank by time (name as a stable tie-break for determinism).
    let mut ranked: Vec<(&str, i64)> = totals.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let (name, secs) = *ranked.first()?;
    // "Mostly on X" needs a CLEAR leader: ≥40% of active time AND strictly ahead of the
    // runner-up — otherwise a 40/40 split would be narrated as if one project dominated.
    let tied = ranked.get(1).is_some_and(|&(_, second)| second == secs);
    if tied || secs * 100 < active_secs * 40 {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `now` long after the test day, so it reads as a finished day.
    const LATER: i64 = 10 * 86_400;

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
                    color: "#1B45BE".into(),
                },
            ),
            (
                2,
                CategoryMeta {
                    slug: Some("comms".into()),
                    name: "Comms".into(),
                    color: "#EAB308".into(),
                },
            ),
            (
                3,
                CategoryMeta {
                    slug: Some("research".into()),
                    name: "Research".into(),
                    color: "#1D9E75".into(),
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert_eq!(day.runs.len(), 2);
    }

    #[test]
    fn uncategorized_active_time_becomes_other_category() {
        let events = vec![ev(0, 600, "Unknown", None, None)];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert_eq!(day.categories[0].slug, "other");
        assert_eq!(day.categories[0].name, "Uncategorized");
        assert_eq!(day.runs[0].category_slug, "other");
        assert_eq!(day.categories[0].color, None);
    }

    #[test]
    fn user_categories_keep_their_identity_and_colour() {
        // Two distinct slug-less (user-created) categories + true uncategorized time. They must
        // NOT collapse into one "Uncategorized" bucket; each keeps its name + hex colour, and
        // only the truly-uncategorized event is "other". (Regression: slug=None used to map to
        // OTHER_SLUG, merging every custom category together and into Uncategorized.)
        let cats = HashMap::from([
            (
                10,
                CategoryMeta {
                    slug: None,
                    name: "Design".into(),
                    color: "#7A4FC2".into(),
                },
            ),
            (
                11,
                CategoryMeta {
                    slug: None,
                    name: "Reading".into(),
                    color: "#1D9E75".into(),
                },
            ),
        ]);
        let events = vec![
            ev(0, 600, "Figma", Some(10), None),    // Design
            ev(600, 1200, "Books", Some(11), None), // Reading
            ev(1200, 1500, "Unknown", None, None),  // truly uncategorized
        ];
        let day = build_day_view(&events, &cats, &proj_map(), 0, vec![], LATER);
        let by_name: HashMap<&str, &CategorySlice> = day
            .categories
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        assert_eq!(by_name.len(), 3, "three distinct buckets, none collapsed");
        assert_eq!(by_name["Design"].color.as_deref(), Some("#7A4FC2"));
        assert_eq!(by_name["Design"].secs, 600);
        assert_eq!(by_name["Reading"].color.as_deref(), Some("#1D9E75"));
        assert_eq!(by_name["Uncategorized"].slug, "other");
        assert_eq!(by_name["Uncategorized"].color, None);
        // Distinct runs too (a user category is its own arc, not a merged grey one).
        assert_eq!(day.runs.len(), 3);
        let design_run = day
            .runs
            .iter()
            .find(|r| r.category_name == "Design")
            .unwrap();
        assert_eq!(design_run.category_color.as_deref(), Some("#7A4FC2"));
    }

    #[test]
    fn recap_is_empty_when_no_active_time() {
        let day = build_day_view(&[idle(0, 600)], &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert_eq!(day.recap.text, "No activity tracked today yet.");
        assert_eq!(day.recap.generated_by, "template");
    }

    // ── Calendar rungs (D73) ────────────────────────────────────────────

    fn slice(active: i64, unwatched: i64, known: bool) -> DaySlice {
        DaySlice {
            day_start: 0,
            active_secs: active,
            deep_secs: 0,
            runs: vec![],
            unwatched_secs: unwatched,
            coverage_known: known,
        }
    }

    #[test]
    fn month_counts_tracked_and_unrecorded_days_separately() {
        let month = build_month_view(vec![
            slice(3600, 0, true),   // a normal day
            slice(0, 0, true),      // watched all day, you just didn't touch it
            slice(0, 86_400, true), // the app wasn't running
            slice(0, 0, false),     // predates coverage — unknowable, not a failure
        ]);
        assert_eq!(month.total_active_secs, 3600);
        assert_eq!(month.tracked_days, 1);
        assert_eq!(
            month.unrecorded_days, 1,
            "only the day we KNOW nobody was watching counts"
        );
    }

    #[test]
    fn a_partly_blind_day_is_not_counted_as_unrecorded() {
        // Six hours missing is a footnote on the day, not a missing day.
        let month = build_month_view(vec![slice(7200, 6 * 3600, true)]);
        assert_eq!(month.unrecorded_days, 0);
        assert_eq!(month.tracked_days, 1);
    }

    fn total(day: usize, category_id: Option<i64>, secs: i64) -> crate::db::DayCategoryTotal {
        crate::db::DayCategoryTotal {
            day,
            category_id,
            secs,
        }
    }

    #[test]
    fn year_splits_each_day_into_its_categories_longest_first() {
        let starts = [0i64, 86_400];
        let year = build_year_view(
            &starts,
            &[
                total(0, Some(1), 3600),
                total(0, Some(2), 7200),
                total(1, Some(1), 1800),
            ],
            &ctx_map(),
            &[0, 0],
            &[true, true],
        );

        assert_eq!(year.days.len(), 2);
        assert_eq!(year.days[0].active_secs, 10_800);
        assert_eq!(
            year.days[0].categories[0].secs, 7200,
            "the dominant category leads the stack"
        );
        assert_eq!(year.days[0].categories[1].secs, 3600);
        // Percentages are a share of that day, never of the year.
        assert!((year.days[0].categories[0].pct - 66.66).abs() < 0.1);
        assert_eq!(year.days[1].active_secs, 1800);
        assert!((year.days[1].categories[0].pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn year_keeps_a_day_with_nothing_on_it() {
        // An empty day still has to occupy its slot, or the calendar misaligns.
        let starts = [0i64, 86_400, 172_800];
        let year = build_year_view(
            &starts,
            &[total(2, Some(1), 60)],
            &ctx_map(),
            &[0; 3],
            &[true; 3],
        );
        assert_eq!(year.days.len(), 3);
        assert_eq!(year.days[0].active_secs, 0);
        assert!(year.days[0].categories.is_empty());
        assert_eq!(year.days[2].active_secs, 60);
    }

    #[test]
    fn year_carries_coverage_through_per_day() {
        let starts = [0i64, 86_400];
        let year = build_year_view(&starts, &[], &ctx_map(), &[86_400, 0], &[true, false]);
        assert_eq!(year.days[0].unwatched_secs, 86_400);
        assert!(year.days[0].coverage_known, "we know nobody was watching");
        assert_eq!(year.days[1].unwatched_secs, 0);
        assert!(
            !year.days[1].coverage_known,
            "a day before coverage existed must not read as a failure"
        );
    }

    #[test]
    fn year_labels_time_with_no_category_rather_than_dropping_it() {
        let starts = [0i64];
        let year = build_year_view(&starts, &[total(0, None, 900)], &ctx_map(), &[0], &[true]);
        assert_eq!(year.days[0].active_secs, 900);
        assert_eq!(year.days[0].categories[0].name, "Uncategorized");
    }

    #[test]
    fn an_unwatched_day_does_not_read_as_a_quiet_one() {
        // The Sep 2026 bug: the app was dead for the whole day, and the recap said the same
        // thing it says about a day you chose to rest. These are opposite facts.
        let whole_day = vec![CoverageGap {
            start: 0,
            end: 86_400,
        }];
        let day = build_day_view(&[], &ctx_map(), &proj_map(), 0, whole_day, LATER);
        assert_eq!(
            day.recap.text,
            "UsageOS wasn’t running on this day, so there’s nothing to show."
        );
        assert_eq!(day.unwatched_secs, 86_400);
    }

    #[test]
    fn a_quiet_but_watched_day_still_reads_as_quiet() {
        let day = build_day_view(&[idle(0, 600)], &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert_eq!(day.recap.text, "No activity tracked today yet.");
        assert_eq!(day.unwatched_secs, 0);
    }

    #[test]
    fn a_partly_unwatched_day_says_so_after_the_story_not_before() {
        let gaps = vec![CoverageGap {
            start: 40_000,
            end: 60_000,
        }];
        let day = build_day_view(
            &[ev(0, 7200, "Zed", Some(1), None)],
            &ctx_map(),
            &proj_map(),
            0,
            gaps,
            LATER,
        );
        assert!(
            day.recap.text.starts_with("You were active from"),
            "the day's own story leads: {}",
            day.recap.text
        );
        assert!(
            day.recap.text.ends_with("so this is only part of the day."),
            "the gap is a footnote, not the headline: {}",
            day.recap.text
        );
    }

    #[test]
    fn a_short_gap_is_not_worth_mentioning() {
        // A reboot or an update relaunch shouldn't editorialise the day.
        let gaps = vec![CoverageGap {
            start: 40_000,
            end: 40_400,
        }];
        let day = build_day_view(
            &[ev(0, 7200, "Zed", Some(1), None)],
            &ctx_map(),
            &proj_map(),
            0,
            gaps,
            LATER,
        );
        assert!(
            !day.recap.text.contains("wasn’t running"),
            "{}",
            day.recap.text
        );
    }

    #[test]
    fn the_model_prompt_is_unchanged_on_a_fully_watched_day() {
        // So every recap cached before coverage shipped (D52 keys on this prompt) still hits.
        let base = build_recap_facts(
            &[ev(0, 7200, "Zed", Some(1), None)],
            &ctx_map(),
            &proj_map(),
            0,
            &[],
            LATER,
        );
        let with_sliver = build_recap_facts(
            &[ev(0, 7200, "Zed", Some(1), None)],
            &ctx_map(),
            &proj_map(),
            0,
            &[CoverageGap {
                start: 50_000,
                end: 50_060,
            }],
            LATER,
        );
        assert_eq!(
            format_recap_prompt(&base),
            format_recap_prompt(&with_sliver)
        );
    }

    #[test]
    fn recap_names_leading_runner_up_focus_and_project() {
        let events = vec![
            ev(0, 3600, "Cursor", Some(1), Some(1)), // 1h deep on usageos
            ev(3600, 4200, "Slack", Some(2), None),  // 10m comms
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        let t = &day.recap.text;
        assert!(t.contains("Deep work led at"), "leading category: {t}");
        assert!(t.contains("then Comms at"), "runner-up category: {t}");
        assert!(t.contains("Your longest stretch ran"), "focus stretch: {t}");
        assert!(t.contains("Mostly on usageos."), "leading project: {t}");
    }

    #[test]
    fn recap_single_category_reads_all_and_omits_short_focus() {
        // One 12-minute category — under the 15-min focus floor, so no "longest stretch" line.
        let day = build_day_view(
            &[ev(0, 720, "Spotify", Some(2), None)],
            &ctx_map(),
            &proj_map(),
            0,
            vec![],
            LATER,
        );
        let t = &day.recap.text;
        assert_eq!(t, "12m tracked. All of it was Comms.", "single category");
        assert!(!t.contains("longest stretch"), "short day omits focus: {t}");
    }

    const H: i64 = 3600;

    fn on_site(start: i64, end: i64, browser: &str, site: &str, category_id: i64) -> ActivityLog {
        let mut e = ev(start, end, browser, Some(category_id), None);
        e.site = Some(site.to_string());
        e
    }

    /// Alternating 10-minute pieces of two categories, like a morning of email and reading.
    fn alternating(from: i64, to: i64) -> Vec<ActivityLog> {
        (from..to)
            .step_by(600)
            .enumerate()
            .map(|(i, t)| match i % 2 {
                0 => ev(t, t + 600, "Slack", Some(2), None),
                _ => on_site(t, t + 600, "Arc", "news.example.org", 3),
            })
            .collect()
    }

    #[test]
    fn the_recap_tells_the_shape_of_a_day_that_changed_halfway() {
        // A late night carried over, a morning of Comms and Research, then Deep work all evening.
        let mut events = vec![ev(0, H, "Cursor", Some(1), None)];
        events.extend(alternating(9 * H, 12 * H));
        events.push(ev(17 * H, 23 * H, "Cursor", Some(1), None));
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        let t = &day.recap.text;
        assert!(
            t.starts_with("You were active from around 9 in the morning until around 11 at night, 10h 0m in all, counting 1h 0m just after midnight."),
            "{t}"
        );
        assert!(
            t.contains("The morning went to Comms and Research, and the evening to Deep work."),
            "{t}"
        );
        assert!(t.contains("Deep work was mostly in Cursor;"), "{t}");
    }

    #[test]
    fn today_is_still_going_so_it_never_says_until() {
        let events = vec![ev(8 * H, 10 * H, "Cursor", Some(1), None)];
        let now = 10 * H + 600;
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], now);
        let t = &day.recap.text;
        assert!(
            t.starts_with("You’ve been active since around 8 in the morning, 2h 0m so far."),
            "{t}"
        );
        assert!(!t.contains("until"), "{t}");
    }

    #[test]
    fn a_long_break_is_named_but_never_across_a_stretch_nobody_watched() {
        let events = vec![
            ev(9 * H, 11 * H, "Cursor", Some(1), None),
            ev(14 * H, 16 * H, "Cursor", Some(1), None),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert!(
            day.recap
                .text
                .contains("The longest break was 3h 0m, from around 11 in the morning."),
            "{}",
            day.recap.text
        );

        let blind = vec![CoverageGap {
            start: 12 * H,
            end: 13 * H,
        }];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, blind, LATER);
        assert!(!day.recap.text.contains("break"), "{}", day.recap.text);
    }

    #[test]
    fn places_name_sites_not_browsers_and_drop_invisible_marks() {
        let events = vec![
            on_site(9 * H, 10 * H, "Arc", "github.com", 3),
            ev(10 * H, 11 * H, "\u{200E}WhatsApp", Some(2), None),
            ev(11 * H, 11 * H + 300, "Arc", Some(3), None), // a browser with no site: not a place
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        let t = &day.recap.text;
        assert!(t.contains("Research was mostly on github.com"), "{t}");
        assert!(t.contains("Comms mostly in WhatsApp."), "{t}");
        assert!(!t.contains("Arc"), "{t}");
        assert!(!t.contains('\u{200E}'), "{t}");
    }

    #[test]
    fn a_few_minutes_of_activity_split_a_break() {
        let events = vec![
            ev(9 * H, 11 * H, "Cursor", Some(1), None),
            ev(12 * H, 12 * H + 180, "Slack", Some(2), None), // too short to be a run of its own
            ev(14 * H, 16 * H, "Cursor", Some(1), None),
        ];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert!(
            day.recap
                .text
                .contains("The longest break was 1h 57m, from around noon."),
            "{}",
            day.recap.text
        );
    }

    #[test]
    fn neighbouring_parts_that_agree_merge() {
        let events = vec![ev(9 * H, 16 * H, "Cursor", Some(1), None)];
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert!(
            day.recap
                .text
                .contains("The morning and afternoon went to Deep work."),
            "{}",
            day.recap.text
        );
    }

    #[test]
    fn a_day_of_short_pieces_says_so() {
        // Eight 16-minute pieces, alternating: nothing near a long stretch.
        let events: Vec<ActivityLog> = (0..8)
            .map(|i| {
                let t = 9 * H + i * 960;
                ev(
                    t,
                    t + 960,
                    if i % 2 == 0 { "Cursor" } else { "Slack" },
                    Some(1 + i % 2),
                    None,
                )
            })
            .collect();
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        assert!(
            day.recap
                .text
                .contains("It came in short pieces; the longest ran 16m in the morning."),
            "{}",
            day.recap.text
        );
    }

    #[test]
    fn clock_phrase_rounds_to_the_half_hour() {
        assert_eq!(clock_phrase(0), "around midnight");
        assert_eq!(clock_phrase(12 * H), "around noon");
        assert_eq!(clock_phrase(8 * H + 20 * 60), "around 8:30 in the morning");
        assert_eq!(clock_phrase(13 * H + 10 * 60), "around 1 in the afternoon");
        assert_eq!(clock_phrase(23 * H + 50 * 60), "around midnight");
        assert_eq!(clock_phrase(H + 1800), "around 1:30 at night");
    }

    #[test]
    fn time_of_day_phrase_buckets_the_clock() {
        assert_eq!(time_of_day_phrase(9), "in the morning");
        assert_eq!(time_of_day_phrase(12), "around midday");
        assert_eq!(time_of_day_phrase(15), "in the afternoon");
        assert_eq!(time_of_day_phrase(19), "in the evening");
        assert_eq!(time_of_day_phrase(2), "at night");
    }

    #[test]
    fn day_slice_sums_active_and_deep_and_builds_runs() {
        let events = vec![
            ev(0, 600, "Cursor", Some(1), Some(1)), // 10m deep
            ev(600, 900, "Slack", Some(2), None),   // 5m comms
            idle(900, 1200),                        // ignored
        ];
        let slice = build_day_slice(42, &events, &ctx_map(), &proj_map(), 0, true);
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
            unwatched_secs: 0,
            coverage_known: true,
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
            unwatched_secs: 0,
            coverage_known: true,
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
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
        let day = build_day_view(&events, &ctx_map(), &proj_map(), 0, vec![], LATER);
        let tl = build_timeline(&events, &ctx_map(), &proj_map(), 0);
        assert_eq!(day.runs.len(), tl.runs.len());
        for (cr, tr) in day.runs.iter().zip(tl.runs.iter()) {
            assert_eq!(cr.category_slug, tr.category_slug);
            assert_eq!((cr.start, cr.end, cr.secs), (tr.start, tr.end, tr.secs));
        }
    }

    #[test]
    fn recap_fingerprint_is_stable_and_sensitive() {
        let base = RecapFacts {
            unwatched_secs: 0,
            active_secs: 17580,
            leading: Some(CategoryFact {
                name: "Work".into(),
                secs: 11400,
            }),
            second: Some(CategoryFact {
                name: "Browsing".into(),
                secs: 5400,
            }),
            leading_project: Some("usageos".into()),
            longest_focus: Some(FocusFact {
                secs: 4320,
                when: "in the morning",
            }),
            ..Default::default()
        };
        // Deterministic: identical facts → identical key (the cache-hit guarantee).
        assert_eq!(recap_fingerprint(&base), recap_fingerprint(&base));

        // Sensitive: a changed number → a new key, so a rule reprocess re-narrates the day.
        let mut changed = base.clone();
        changed.active_secs = 9999;
        assert_ne!(recap_fingerprint(&base), recap_fingerprint(&changed));

        // Sensitive: a renamed category → a new key (the prose would differ).
        let mut renamed = base.clone();
        renamed.leading = Some(CategoryFact {
            name: "Deep".into(),
            secs: 11400,
        });
        assert_ne!(recap_fingerprint(&base), recap_fingerprint(&renamed));
    }
}
