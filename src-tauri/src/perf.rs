//! Synthetic-history generator for the Phase-6 perf/stress harness.
//!
//! Behind the `perf` cargo feature, so it never compiles into the shipped binary or runs in
//! the CI default lanes. It writes realistic spans **through the repository layer**
//! ([`crate::db::bulk_insert_events`]) — never raw SQL (hard rule 4) — so the read path under
//! test (`get_activity_logs` → the pure `rollup` builders) is exactly the production one.
//!
//! Realism that makes the rollup do real work (segmentation, excursion-absorb, projects):
//! short clustered spans across a ~12-app catalog spanning the five seeded categories, a
//! handful of projects/sites/urls, a morning-to-evening active window, and occasional gaps
//! (untracked idle, matching live capture — D38). The generator is **deterministic** (a seeded
//! SplitMix64 PRNG, no wall-clock/`rand`), so a given `SeedConfig` reproduces the same DB and
//! the timing numbers are comparable run-to-run.
//!
//! Two consumers: the in-crate `#[ignore]` timing harness ([`mod tests`]) and the `seed_db`
//! bin (a real on-disk WAL DB for the webview-render feel test + the later release soak).

use crate::db::{self, NewEvent};
use rusqlite::Connection;

/// Seconds in a day — the local-day stride between seeded days.
pub const SECS_PER_DAY: i64 = 86_400;

/// A fixed UTC midnight used as the most-recent seeded day's start: **2025-06-15 00:00:00 UTC**.
/// Fixed (not wall-clock) so seeds and the read windows over them are reproducible. The read
/// harness reads `[DEFAULT_END_DAY_START, +86400)` as "today".
pub const DEFAULT_END_DAY_START: i64 = 1_749_945_600;

/// How to generate a synthetic history. `days` of history end at `end_day_start` (the most
/// recent day) and extend backwards. The active window per day usually binds before
/// `max_spans_per_day` — that cap only guards against a pathological run.
#[derive(Debug, Clone)]
pub struct SeedConfig {
    pub days: i64,
    pub end_day_start: i64,
    pub max_spans_per_day: usize,
    pub seed: u64,
    /// Demo mode: lay down long single-category blocks (clean arcs) instead of the fragmented
    /// rapid-switch churn the perf harness wants. For screenshots/demos only.
    pub demo: bool,
}

/// What a seed run produced — reported by the bin and the timing harness.
#[derive(Debug, Clone, Copy)]
pub struct SeedStats {
    pub days: i64,
    pub events: usize,
    pub projects: usize,
}

// --- deterministic PRNG (SplitMix64) -----------------------------------------------------

/// A tiny, dependency-free deterministic PRNG. SplitMix64 is fine for synthetic data (we need
/// reproducibility, not cryptographic quality). Seeding the same value reproduces the run.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform index in `0..n` (callers pass `n > 0`; `n == 0` yields 0, never a panic).
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// A uniform `i64` in `[lo, hi)` (callers pass `hi > lo`; otherwise yields `lo`).
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            lo
        } else {
            lo + (self.next_u64() % (hi - lo) as u64) as i64
        }
    }

    /// True with probability `percent`/100.
    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
}

// --- the app catalog ---------------------------------------------------------------------

/// One app in the synthetic catalog. `slug` names the seeded category it rolls into (`""`
/// stays Uncategorized — exercises the no-rule path); `project_key` (editors/terminals) and
/// `url`/`site` (browsers) are fixed `'static` strings so every [`NewEvent`] borrows without
/// allocation. `weight` is the relative frequency in the per-day mix.
struct AppSpec {
    process: &'static str,
    slug: &'static str,
    titles: &'static [&'static str],
    url: Option<&'static str>,
    site: Option<&'static str>,
    project_key: Option<&'static str>,
    weight: u32,
}

/// Projects to create up-front (`canonical_key`, `display_name`, `remote_url`) — referenced by
/// `AppSpec::project_key` so editor/terminal spans carry a real `project_id` (D30).
const PROJECTS: &[(&str, &str, &str)] = &[
    (
        "usage-os",
        "usage_os",
        "https://github.com/f-gozie/usage-os",
    ),
    ("nudge", "nudge", "https://github.com/f-gozie/nudge"),
];

/// ~14 apps across the five seeded categories (deep / research / comms / breaks / personal),
/// plus one uncategorized. Weights skew toward a dev's day (editor + browser heaviest).
const CATALOG: &[AppSpec] = &[
    AppSpec {
        process: "Cursor",
        slug: "deep",
        titles: &[
            "usage_os — lib.rs",
            "usage_os — rollup.rs",
            "usage_os — perf.rs",
        ],
        url: None,
        site: None,
        project_key: Some("usage-os"),
        weight: 11,
    },
    AppSpec {
        process: "Cursor",
        slug: "deep",
        titles: &["nudge — App.tsx", "nudge — theme.ts"],
        url: None,
        site: None,
        project_key: Some("nudge"),
        weight: 5,
    },
    AppSpec {
        process: "iTerm2",
        slug: "deep",
        titles: &[
            "favour@mac: ~/projects/usage_os",
            "cargo test",
            "git status",
        ],
        url: None,
        site: None,
        project_key: Some("usage-os"),
        weight: 7,
    },
    AppSpec {
        process: "Code",
        slug: "deep",
        titles: &["nudge — server.ts", "nudge — routes.ts"],
        url: None,
        site: None,
        project_key: Some("nudge"),
        weight: 4,
    },
    AppSpec {
        process: "Figma",
        slug: "deep",
        titles: &["UsageOS — Day dial", "Bauhaus tokens"],
        url: None,
        site: None,
        project_key: None,
        weight: 3,
    },
    AppSpec {
        process: "Google Chrome",
        slug: "research",
        titles: &[
            "f-gozie/usage-os: Issues",
            "rusqlite — docs.rs",
            "SQLite query planner",
        ],
        url: Some("https://github.com/f-gozie/usage-os/issues"),
        site: Some("github.com"),
        project_key: None,
        weight: 10,
    },
    AppSpec {
        process: "Safari",
        slug: "research",
        titles: &["Foundation Models | Apple Developer", "Tauri v2 Guide"],
        url: Some("https://developer.apple.com/documentation/foundationmodels"),
        site: Some("developer.apple.com"),
        project_key: None,
        weight: 5,
    },
    AppSpec {
        process: "Notion",
        slug: "research",
        titles: &["Phase 6 — perf notes", "Roadmap"],
        url: Some("https://www.notion.so/workspace"),
        site: Some("notion.so"),
        project_key: None,
        weight: 3,
    },
    AppSpec {
        process: "Slack",
        slug: "comms",
        titles: &["#general", "#engineering", "DM — teammate"],
        url: None,
        site: None,
        project_key: None,
        weight: 7,
    },
    AppSpec {
        process: "WhatsApp",
        slug: "comms",
        titles: &["Family", "Friends"],
        url: None,
        site: None,
        project_key: None,
        weight: 4,
    },
    AppSpec {
        process: "Spotify",
        slug: "breaks",
        titles: &["Lofi beats", "Deep focus playlist"],
        url: None,
        site: None,
        project_key: None,
        weight: 5,
    },
    AppSpec {
        process: "Music",
        slug: "breaks",
        titles: &["Album — Artist"],
        url: None,
        site: None,
        project_key: None,
        weight: 2,
    },
    AppSpec {
        process: "Notes",
        slug: "personal",
        titles: &["Groceries", "Ideas"],
        url: None,
        site: None,
        project_key: None,
        weight: 3,
    },
    AppSpec {
        process: "Mail",
        slug: "personal",
        titles: &["Inbox (3)", "Receipts"],
        url: None,
        site: None,
        project_key: None,
        weight: 3,
    },
    AppSpec {
        process: "Preview",
        slug: "", // no rule → stays Uncategorized (exercises the category-less path)
        titles: &["invoice.pdf", "diagram.png"],
        url: None,
        site: None,
        project_key: None,
        weight: 2,
    },
];

// --- generation --------------------------------------------------------------------------

/// Seed `cfg.days` of synthetic history into `conn` (already migrated). Creates the projects,
/// maps the seeded categories, then generates and bulk-inserts each day in one transaction.
/// Returns the realized row count (the active window usually caps a day below
/// `max_spans_per_day`). Idempotent only on a fresh DB — call on a freshly-migrated connection.
pub fn seed(conn: &Connection, cfg: &SeedConfig) -> rusqlite::Result<SeedStats> {
    // Category slug -> id, from the seeded taxonomy (deep/research/comms/breaks/personal).
    let metas = db::get_category_metas(conn)?;
    let cat_id = |slug: &str| -> Option<i64> {
        if slug.is_empty() {
            return None;
        }
        metas
            .iter()
            .find(|(_, s, _, _)| s.as_deref() == Some(slug))
            .map(|(id, _, _, _)| *id)
    };

    // Create the projects once; key -> id.
    let mut project_ids: Vec<(&'static str, i64)> = Vec::with_capacity(PROJECTS.len());
    for (key, display, remote) in PROJECTS {
        let id = db::resolve_or_create_project(conn, key, display, Some(remote), &[])?;
        project_ids.push((key, id));
    }
    let project_id_of = |key: &str| -> Option<i64> {
        project_ids
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, id)| *id)
    };

    let total_weight: u32 = CATALOG.iter().map(|a| a.weight).sum();
    let mut rng = Rng::new(cfg.seed);
    let mut total_events = 0usize;

    for d in 0..cfg.days {
        let day_start = cfg.end_day_start - d * SECS_PER_DAY;
        let spans = if cfg.demo {
            generate_demo_day(&mut rng, day_start, &cat_id, &project_id_of)
        } else {
            generate_day(
                &mut rng,
                day_start,
                cfg.max_spans_per_day,
                total_weight,
                &cat_id,
                &project_id_of,
            )
        };
        if spans.is_empty() {
            continue;
        }
        // One transaction per day keeps memory flat (a day's worth of spans at a time) and
        // makes the millions-of-rows seed finish in seconds (prepared statement, batched commit).
        let tx = conn.unchecked_transaction()?;
        db::bulk_insert_events(&tx, &spans)?;
        tx.commit()?;
        total_events += spans.len();
    }

    Ok(SeedStats {
        days: cfg.days,
        events: total_events,
        projects: project_ids.len(),
    })
}

/// Generate one day's spans: a morning-to-evening active window of short, clustered spans with
/// occasional gaps. Returns `(NewEvent, end_time)` pairs ready for [`db::bulk_insert_events`].
fn generate_day(
    rng: &mut Rng,
    day_start: i64,
    max_spans: usize,
    total_weight: u32,
    cat_id: &impl Fn(&str) -> Option<i64>,
    project_id_of: &impl Fn(&str) -> Option<i64>,
) -> Vec<(NewEvent<'static>, i64)> {
    // Active window: starts mid-morning, ends mid/late-evening. The window (not `max_spans`)
    // is normally the binding constraint, so per-day counts vary naturally with the rng.
    let mut t = day_start + rng.range(7 * 3600, 9 * 3600);
    let active_end = day_start + rng.range(21 * 3600, 23 * 3600);

    let mut spans: Vec<(NewEvent<'static>, i64)> = Vec::new();
    while t < active_end && spans.len() < max_spans {
        let spec = pick_app(rng, total_weight);
        let dur = pick_duration(rng);
        let end = t + dur;
        let title = spec.titles[rng.below(spec.titles.len())];
        let project_id = spec.project_key.and_then(project_id_of);

        spans.push((
            NewEvent {
                process_name: spec.process,
                window_title: title,
                url: spec.url,
                site: spec.site,
                project_id,
                project_abstain_reason: None,
                is_private: false,
                is_idle: false,
                category_id: cat_id(spec.slug),
                timestamp: t,
            },
            end,
        ));

        t = end;
        // ~6% of switches are followed by a real break (untracked idle gap, D38) — this is what
        // ends a category-run and gives the segmentation/absorb pass something to chew on.
        if rng.chance(6) {
            t += rng.range(45, 360);
        }
    }
    spans
}

/// Generate one **demo** day: a believable rhythm of long single-category blocks, so the dial
/// shows a few big clean arcs instead of `generate_day`'s fragmented churn. Each block is one app
/// held for a long stretch, split into several same-category events (rotating titles) so the
/// Timeline still has switch-level detail while the dial/week render one arc per block. Per-day
/// rng variation (block lengths, optional blocks, the odd light day) keeps the week from looking
/// identical. For demos/screenshots only (`SeedConfig::demo`).
fn generate_demo_day(
    rng: &mut Rng,
    day_start: i64,
    cat_id: &impl Fn(&str) -> Option<i64>,
    project_id_of: &impl Fn(&str) -> Option<i64>,
) -> Vec<(NewEvent<'static>, i64)> {
    // (slug, min-minutes, max-minutes, probability%). A normal weekday: focus blocks broken by
    // comms / a midday browse / breaks. The three "deep" blocks become three separate big arcs.
    let weekday: &[(&str, i64, i64, u64)] = &[
        ("deep", 90, 160, 100),
        ("comms", 15, 35, 90),
        ("deep", 60, 130, 100),
        ("research", 30, 55, 100),
        ("deep", 100, 190, 100),
        ("comms", 20, 45, 85),
        ("breaks", 30, 65, 80),
        ("research", 20, 45, 60),
        ("personal", 15, 35, 55),
    ];
    // A light day (weekend-ish): mostly browsing, breaks, a little messaging.
    let light: &[(&str, i64, i64, u64)] = &[
        ("research", 25, 50, 100),
        ("breaks", 30, 70, 100),
        ("comms", 10, 25, 70),
        ("personal", 15, 35, 60),
    ];

    let is_light = rng.chance(22);
    let plan = if is_light { light } else { weekday };
    let mut t = if is_light {
        day_start + rng.range(10 * 3600, 13 * 3600)
    } else {
        day_start + rng.range(8 * 3600, 9 * 3600 + 1800)
    };

    let mut spans: Vec<(NewEvent<'static>, i64)> = Vec::new();
    for (slug, lo, hi, prob) in plan.iter().copied() {
        if !rng.chance(prob) {
            continue;
        }
        let block_end = t + rng.range(lo * 60, hi * 60);
        let category_id = cat_id(slug);
        // Vary the app *within* the block (same category → still one arc/run), so the Timeline
        // shows real switch detail across a session (e.g. Cursor → Terminal in a Work block)
        // instead of one app repeated. Each chunk is 10–25 min.
        while t < block_end {
            let chunk = rng.range(10 * 60, 25 * 60).min(block_end - t);
            if chunk < 60 {
                break;
            }
            let end = t + chunk;
            let spec = pick_in_category(rng, slug);
            spans.push((
                NewEvent {
                    process_name: spec.process,
                    window_title: spec.titles[rng.below(spec.titles.len())],
                    url: spec.url,
                    site: spec.site,
                    project_id: spec.project_key.and_then(project_id_of),
                    project_abstain_reason: None,
                    is_private: false,
                    is_idle: false,
                    category_id,
                    timestamp: t,
                },
                end,
            ));
            t = end;
        }
        // A short untracked break after some blocks (separates the arcs).
        if rng.chance(45) {
            t += rng.range(5 * 60, 20 * 60);
        }
    }
    spans
}

/// Pick one catalog entry of the given category slug (reservoir sample, so no allocation). Our
/// demo plan only uses slugs present in `CATALOG`; falls back to the first entry otherwise.
fn pick_in_category(rng: &mut Rng, slug: &str) -> &'static AppSpec {
    let mut chosen = &CATALOG[0];
    let mut count = 0usize;
    for spec in CATALOG {
        if spec.slug == slug {
            count += 1;
            if rng.below(count) == 0 {
                chosen = spec;
            }
        }
    }
    chosen
}

/// Pick an app by weight. `roll < total_weight` always lands inside the loop; the trailing
/// return is unreachable but satisfies the type (no `unwrap`/`panic`).
fn pick_app(rng: &mut Rng, total_weight: u32) -> &'static AppSpec {
    let mut roll = (rng.next_u64() % total_weight.max(1) as u64) as u32;
    for spec in CATALOG {
        if roll < spec.weight {
            return spec;
        }
        roll -= spec.weight;
    }
    &CATALOG[0]
}

/// A realistic switch duration (seconds): mostly brief context-switches, a minority of focused
/// stretches, rarely a long one. Averages ~40s, so a ~14h active window holds ~1,000 spans.
fn pick_duration(rng: &mut Rng) -> i64 {
    let r = rng.below(100);
    if r < 80 {
        rng.range(6, 35)
    } else if r < 96 {
        rng.range(35, 120)
    } else {
        rng.range(120, 480)
    }
}

// --- in-crate read-path timing harness ---------------------------------------------------
//
// Lives inside the crate (not an external integration test) so it can call the `pub(crate)`
// `rollup::build_recap_facts`. `#[ignore]` keeps it out of normal `cargo test`; run it with:
//   cargo test --release --features perf perf_read -- --ignored --nocapture

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollup::{self, CategoryMeta};
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::time::Instant;

    fn fresh_migrated_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("pragma");
        crate::migrations::run_migrations(&mut conn).expect("migrations");
        conn
    }

    /// Mirror of `lib::load_lookup_maps` (which is private) so the harness feeds the rollup the
    /// same category/project maps the real commands do.
    fn lookup_maps(conn: &Connection) -> (HashMap<i64, CategoryMeta>, HashMap<i64, String>) {
        let categories = db::get_category_metas(conn)
            .expect("category metas")
            .into_iter()
            .map(|(id, slug, name, color)| (id, CategoryMeta { slug, name, color }))
            .collect();
        let projects = db::get_projects(conn)
            .expect("projects")
            .into_iter()
            .map(|p| (p.id, p.display_name))
            .collect();
        (categories, projects)
    }

    fn median(mut xs: Vec<f64>) -> f64 {
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        xs.get(xs.len() / 2).copied().unwrap_or(0.0)
    }

    fn ms_since(t: Instant) -> f64 {
        t.elapsed().as_secs_f64() * 1000.0
    }

    #[test]
    #[ignore = "perf harness — run with: cargo test --release --features perf perf_read -- --ignored --nocapture"]
    fn perf_read_baseline() {
        // (label, days). 30/182/365/730 ≈ 1/6/12/24 months; 1095 ≈ 36 months ("millions" tier).
        let tiers: &[(&str, i64)] = &[
            ("1mo", 30),
            ("6mo", 182),
            ("12mo", 365),
            ("24mo", 730),
            ("heavy-36mo", 1095),
        ];
        const REPS: usize = 9;
        let day_start = DEFAULT_END_DAY_START;
        let day_end = day_start + SECS_PER_DAY;

        println!(
            "\n=== Phase 6 · read-path baseline (release, in-memory SQLite, median of {REPS}) ==="
        );
        println!("recent-day window = [{day_start}, {day_end})  (the most-recent seeded day)");
        println!("all timings in MILLISECONDS. get_day/timeline/recap are measured END-TO-END");
        println!("(each re-runs get_activity_logs, like the real command); day_build isolates the");
        println!("pure rollup over already-fetched events.\n");
        println!(
            "{:<12} {:>9} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>10}",
            "tier", "rows", "day_rows", "seed", "get_day", "timeline", "recap", "week", "day_build"
        );
        println!("{}", "-".repeat(92));

        let mut last_plan = String::new();

        for (label, days) in tiers.iter().copied() {
            let conn = fresh_migrated_db();
            let cfg = SeedConfig {
                days,
                end_day_start: day_start,
                max_spans_per_day: 2000,
                seed: 0xC0FFEE,
                demo: false,
            };
            let t_seed = Instant::now();
            let stats = seed(&conn, &cfg).expect("seed");
            let seed_ms = ms_since(t_seed);

            let rows: i64 = conn
                .query_row("SELECT COUNT(*) FROM activity_logs", [], |r| r.get(0))
                .expect("count");
            let (categories, projects) = lookup_maps(&conn);

            // Day rows actually returned for the recent-day window (post-overlap-clip).
            let day_rows = db::get_activity_logs(&conn, day_start, day_end)
                .expect("get_day events")
                .len();

            let (mut get_day, mut timeline, mut recap, mut week, mut day_build) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for _ in 0..REPS {
                // Each command is timed the way it actually runs: it re-runs get_activity_logs
                // (commands don't share a fetched Vec — they each lock + query independently).
                let t = Instant::now();
                let events = db::get_activity_logs(&conn, day_start, day_end).expect("events");
                let _ = rollup::build_day_view(&events, &categories, &projects, day_start);
                get_day.push(ms_since(t));

                let t = Instant::now();
                let ev = db::get_activity_logs(&conn, day_start, day_end).expect("events");
                let _ = rollup::build_timeline(&ev, &categories, &projects);
                timeline.push(ms_since(t));

                let t = Instant::now();
                let ev = db::get_activity_logs(&conn, day_start, day_end).expect("events");
                let _ = rollup::build_recap_facts(&ev, &categories, &projects, day_start);
                recap.push(ms_since(t));

                // week = 7 day-reads + build_day_slice each + build_week_view.
                let t = Instant::now();
                let mut slices = Vec::with_capacity(7);
                for i in 0..7 {
                    let ds = day_start - (6 - i) * SECS_PER_DAY;
                    let de = ds + SECS_PER_DAY;
                    let wev = db::get_activity_logs(&conn, ds, de).expect("week events");
                    slices.push(rollup::build_day_slice(ds, &wev, &categories, &projects));
                }
                let _ = rollup::build_week_view(slices);
                week.push(ms_since(t));

                // day_build = pure rollup over the SAME pre-fetched events (no query) — isolates
                // build cost so the query-vs-build split is unambiguous in the table.
                let t = Instant::now();
                let _ = rollup::build_day_view(&events, &categories, &projects, day_start);
                day_build.push(ms_since(t));
            }

            println!(
                "{:<12} {:>9} {:>8} {:>7.0}ms {:>7.2}ms {:>7.2}ms {:>7.2}ms {:>7.2}ms {:>8.3}ms",
                label,
                rows,
                day_rows,
                seed_ms,
                median(get_day),
                median(timeline),
                median(recap),
                median(week),
                median(day_build),
            );

            // Capture the query plan for the overlap read at the largest tier.
            if label == "heavy-36mo" {
                // Mirror the bounded overlap query get_activity_logs actually runs (Phase-6 fix):
                // the lower bound is start - 2 days.
                let scan_lo = day_start - 2 * SECS_PER_DAY;
                let mut stmt = conn
                    .prepare(
                        "EXPLAIN QUERY PLAN
                         SELECT id FROM activity_logs
                         WHERE start_time >= ?1 AND start_time < ?3 AND end_time > ?2
                         ORDER BY start_time ASC",
                    )
                    .expect("prepare explain");
                let plan: Vec<String> = stmt
                    .query_map([scan_lo, day_start, day_end], |r| r.get::<_, String>(3))
                    .expect("explain rows")
                    .filter_map(|r| r.ok())
                    .collect();
                last_plan = plan.join("\n  ");
                let _ = stats;
            }
        }

        println!(
            "\nEXPLAIN QUERY PLAN — get_activity_logs overlap read (heavy-36mo tier):\n  {last_plan}\n"
        );
    }

    /// Write-path churn + lock-contention stress (the R57 question). Drives tens of thousands of
    /// rapid app-switch events through the real `on_focus` write path, then runs a writer and a
    /// reader concurrently against one `Arc<Mutex<Connection>>` to see whether the dial's reads
    /// stall behind capture's writes (and vice versa) under the single shared connection.
    #[test]
    #[ignore = "perf harness — cargo test --release --features perf perf_write_churn -- --ignored --nocapture"]
    fn perf_write_churn() {
        use crate::capture::WriteProbe;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex};
        use std::thread;

        const N_WRITES: usize = 50_000;
        // Six months of history so concurrent reads do realistic work (dial reads while writing).
        const SEED_DAYS: i64 = 182;
        let apps = [
            "Cursor",
            "Slack",
            "Google Chrome",
            "iTerm2",
            "Spotify",
            "Notion",
        ];
        let write_base = DEFAULT_END_DAY_START + SECS_PER_DAY + 1; // append after seeded history
        let day_start = DEFAULT_END_DAY_START;
        let day_end = day_start + SECS_PER_DAY;

        let make_db = || {
            let conn = fresh_migrated_db();
            let cfg = SeedConfig {
                days: SEED_DAYS,
                end_day_start: DEFAULT_END_DAY_START,
                max_spans_per_day: 2000,
                seed: 0xC0FFEE,
                demo: false,
            };
            seed(&conn, &cfg).expect("seed");
            Arc::new(Mutex::new(conn))
        };

        // Each event alternates app → always a switch (close prev span + open new = the heaviest
        // write path: set_span_end + find_category + infer_project + insert_event). Locks per
        // event, exactly like `consume`.
        let run_writes = |db: &Arc<Mutex<rusqlite::Connection>>, n: usize| -> f64 {
            let mut probe = WriteProbe::default();
            let t = Instant::now();
            for i in 0..n {
                let app = apps[i % apps.len()];
                let conn = db.lock().expect("lock");
                probe
                    .feed(&conn, app, "window", write_base + i as i64)
                    .expect("write");
            }
            ms_since(t)
        };

        println!("\n=== Phase 6 · write-path churn + lock contention ({N_WRITES} switch events, {SEED_DAYS}d seeded) ===");

        // --- solo writer ---
        let db_w = make_db();
        let w_solo = run_writes(&db_w, N_WRITES);

        // --- solo reader (recent-day get_activity_logs, the dial's read) ---
        let db_r = make_db();
        let reps = 2000usize;
        let t = Instant::now();
        for _ in 0..reps {
            let conn = db_r.lock().expect("lock");
            let _ = db::get_activity_logs(&conn, day_start, day_end).expect("read");
        }
        let r_solo_us = ms_since(t) / reps as f64 * 1000.0;

        // --- concurrent writer + reader on ONE shared connection ---
        let db_c = make_db();
        let done = Arc::new(AtomicBool::new(false));
        let reader = {
            let db = db_c.clone();
            let done = done.clone();
            thread::spawn(move || {
                let (mut count, mut total) = (0u64, 0.0);
                while !done.load(Ordering::Relaxed) {
                    let t = Instant::now();
                    {
                        let conn = db.lock().expect("lock");
                        let _ = db::get_activity_logs(&conn, day_start, day_end).expect("read");
                    }
                    total += ms_since(t);
                    count += 1;
                }
                (count, total)
            })
        };
        let w_conc = run_writes(&db_c, N_WRITES);
        done.store(true, Ordering::Relaxed);
        let (reads_done, reads_ms) = reader.join().expect("join");
        let r_conc_us = if reads_done > 0 {
            reads_ms / reads_done as f64 * 1000.0
        } else {
            0.0
        };

        let w_per_event_us = w_solo / N_WRITES as f64 * 1000.0;
        println!("writes (solo):        {w_solo:.0} ms total  =  {w_per_event_us:.2} µs/event  =  {:.0} events/sec", N_WRITES as f64 / (w_solo / 1000.0));
        println!("read  (solo):         {r_solo_us:.2} µs/read (recent day, ~1030 rows)");
        println!(
            "writes (concurrent):  {w_conc:.0} ms total   ({:.2}× the solo write time)",
            w_conc / w_solo
        );
        println!("read  (concurrent):   {r_conc_us:.2} µs/read   ({:.2}× the solo read), {reads_done} reads completed while writing", r_conc_us / r_solo_us.max(0.0001));
        println!("contention verdict: writer +{:.0}% / reader +{:.0}% under a shared Arc<Mutex<Connection>>.\n",
            (w_conc / w_solo - 1.0) * 100.0,
            (r_conc_us / r_solo_us.max(0.0001) - 1.0) * 100.0);
    }
}
