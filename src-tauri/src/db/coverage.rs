//! Watch coverage: the stretches during which UsageOS was actually running.
//!
//! The app can only record what it is present for, and until now it kept no record of its own
//! presence — so a day it was dead looked exactly like a day you left the Mac alone. This module
//! owns the one row per app run and the derivation of the gaps between runs.
//!
//! **A session is continuous.** The heartbeat exists to leave a watermark for a process that dies
//! without saying so; it is not a liveness sample. Machine sleep suspends the whole process, so
//! heartbeats stop for hours every night — treating those as blindness would hatch every night on
//! every surface. Within one session we were there, whether or not the Mac was awake. Real gaps
//! are the stretches *between* sessions.

use super::*;

/// A stretch during which the app was running. `end` is the clean exit where there was one, else
/// the last heartbeat — the most that can be honestly claimed for a process that vanished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CoverageSpan {
    pub start: i64,
    pub end: i64,
}

/// A stretch with no coverage at all: the app was not running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CoverageGap {
    pub start: i64,
    pub end: i64,
}

/// Gaps shorter than this are not worth reporting — a reboot, an update relaunch or a crash the
/// supervisor caught would otherwise litter every day with slivers.
pub const GAP_MIN_SECS: i64 = 300;

/// Start a session for this process. Returns its row id.
pub fn open_session(conn: &Connection, now: i64, version: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO watch_coverage (started_at, last_seen, ended_at, version)
         VALUES (?1, ?1, NULL, ?2)",
        (now, version),
    )?;
    Ok(conn.last_insert_rowid())
}

/// Bump this session's watermark. Called on the capture tick, throttled by the caller. A beat
/// also reopens the session if another launch of the app closed it as dead while it was running.
pub fn touch_session(conn: &Connection, id: i64, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE watch_coverage SET last_seen = ?2, ended_at = NULL WHERE id = ?1",
        (id, now),
    )?;
    Ok(())
}

/// Close this session on a clean exit. `last_seen` is moved up too, so the span is consistent
/// however it is later read.
pub fn close_session(conn: &Connection, id: i64, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE watch_coverage SET ended_at = ?2, last_seen = ?2 WHERE id = ?1",
        (id, now),
    )?;
    Ok(())
}

/// Close any session a previous process left open — it died without saying so, so its watermark
/// is the end. Returns how many were reconciled. `except` is this process's own live session.
pub fn reconcile_open_sessions(conn: &Connection, except: i64) -> Result<usize> {
    let n = conn.execute(
        "UPDATE watch_coverage SET ended_at = last_seen WHERE ended_at IS NULL AND id != ?1",
        [except],
    )?;
    Ok(n)
}

/// Coverage spans overlapping `[start, end)`, oldest first. A still-open session belongs to a
/// running app (startup closes any a dead process left open), so it reads as covered up to `now`:
/// a Mac that slept doesn't read as an app that stopped, in the seconds before the next beat.
pub fn get_coverage(
    conn: &Connection,
    start: i64,
    end: i64,
    now: i64,
) -> Result<Vec<CoverageSpan>> {
    let mut stmt = conn.prepare(
        "SELECT started_at, COALESCE(ended_at, MAX(last_seen, ?3)) FROM watch_coverage
         WHERE started_at < ?2 AND COALESCE(ended_at, MAX(last_seen, ?3)) > ?1
         ORDER BY started_at",
    )?;
    let rows = stmt.query_map((start, end, now), |row| {
        Ok(CoverageSpan {
            start: row.get(0)?,
            end: row.get(1)?,
        })
    })?;
    rows.collect()
}

/// When coverage began being recorded at all. Everything before this is unknowable — the feature
/// did not exist — and must never be reported as "not recorded".
pub fn first_coverage_start(conn: &Connection) -> Result<Option<i64>> {
    conn.query_row("SELECT MIN(started_at) FROM watch_coverage", [], |row| {
        row.get::<_, Option<i64>>(0)
    })
}

/// Drop coverage rows whose whole span is past retention, alongside the events they describe.
pub fn prune_coverage(conn: &Connection, cutoff: i64) -> Result<usize> {
    let n = conn.execute(
        "DELETE FROM watch_coverage WHERE COALESCE(ended_at, last_seen) < ?1",
        [cutoff],
    )?;
    Ok(n)
}

/// Invert coverage over `[start, end)` — the stretches nobody was watching.
///
/// Pure, so the whole honesty question is testable without a database. `now` clips the window so
/// the rest of today is never "not recorded" (it hasn't happened). `first_known` clips the other
/// end: before coverage existed we cannot say, and claiming otherwise would paint every archived
/// day as a failure the moment this ships.
pub fn coverage_gaps(
    spans: &[CoverageSpan],
    start: i64,
    end: i64,
    now: i64,
    first_known: Option<i64>,
) -> Vec<CoverageGap> {
    let Some(first) = first_known else {
        return Vec::new();
    };
    let from = start.max(first);
    let to = end.min(now);
    if to <= from {
        return Vec::new();
    }

    let mut gaps = Vec::new();
    let mut cursor = from;
    for span in spans {
        if span.end <= cursor {
            continue; // already covered, or entirely before the window
        }
        // Measured after clipping to the window, so a sliver at the day's edge isn't reported.
        if span.start.min(to) - cursor >= GAP_MIN_SECS {
            gaps.push(CoverageGap {
                start: cursor,
                end: span.start.min(to),
            });
        }
        cursor = cursor.max(span.end);
        if cursor >= to {
            break;
        }
    }
    if to - cursor >= GAP_MIN_SECS {
        gaps.push(CoverageGap {
            start: cursor,
            end: to,
        });
    }
    gaps.retain(|g| g.end > g.start);
    gaps
}

/// Seconds of `[start, end)` that nobody was watching.
pub fn unwatched_secs(gaps: &[CoverageGap]) -> i64 {
    gaps.iter().map(|g| g.end - g.start).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;
    const T0: i64 = 1_700_000_000; // an arbitrary local midnight for the tests
    fn span(a: i64, b: i64) -> CoverageSpan {
        CoverageSpan { start: a, end: b }
    }

    #[test]
    fn a_fully_covered_day_has_no_gaps() {
        let spans = [span(T0 - 100, T0 + DAY + 100)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0 - DAY));
        assert!(gaps.is_empty(), "got {:?}", gaps);
    }

    #[test]
    fn a_day_with_no_session_at_all_is_one_whole_gap() {
        // The Sep 16 case: the app died the day before and stayed dead.
        let spans = [];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0 - DAY * 5));
        assert_eq!(
            gaps,
            vec![CoverageGap {
                start: T0,
                end: T0 + DAY
            }]
        );
        assert_eq!(unwatched_secs(&gaps), DAY);
    }

    #[test]
    fn the_gap_between_two_sessions_is_reported() {
        let spans = [span(T0, T0 + 3600), span(T0 + 7200, T0 + DAY)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0));
        assert_eq!(
            gaps,
            vec![CoverageGap {
                start: T0 + 3600,
                end: T0 + 7200
            }]
        );
    }

    #[test]
    fn a_relaunch_sliver_is_not_worth_reporting() {
        // An update restart or a reboot: seconds, not a blind spot.
        let spans = [span(T0, T0 + 3600), span(T0 + 3630, T0 + DAY)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0));
        assert!(gaps.is_empty(), "got {:?}", gaps);
    }

    #[test]
    fn the_rest_of_today_is_not_a_gap() {
        // It's noon and the app is running. The afternoon hasn't happened yet.
        let noon = T0 + DAY / 2;
        let spans = [span(T0, noon)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, noon, Some(T0));
        assert!(gaps.is_empty(), "got {:?}", gaps);
    }

    #[test]
    fn history_from_before_coverage_existed_is_never_blamed() {
        // The migration landed today; yesterday is unknowable, not "not recorded".
        let gaps = coverage_gaps(&[], T0, T0 + DAY, T0 + DAY * 9, Some(T0 + DAY * 8));
        assert!(gaps.is_empty(), "got {:?}", gaps);
    }

    #[test]
    fn with_no_coverage_recorded_at_all_nothing_is_claimed() {
        let gaps = coverage_gaps(&[], T0, T0 + DAY, T0 + DAY * 2, None);
        assert!(gaps.is_empty());
    }

    #[test]
    fn coverage_partway_through_the_day_clips_to_when_it_began() {
        // Coverage started at 06:00; the night before it is unknowable, the evening gap is real.
        let six = T0 + 6 * 3600;
        let spans = [span(six, six + 3600)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(six));
        assert_eq!(
            gaps,
            vec![CoverageGap {
                start: six + 3600,
                end: T0 + DAY
            }]
        );
    }

    #[test]
    fn overlapping_sessions_do_not_invent_a_gap() {
        // Two processes briefly overlapping across a restart.
        let spans = [span(T0, T0 + 7200), span(T0 + 3600, T0 + DAY)];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0));
        assert!(gaps.is_empty(), "got {:?}", gaps);
    }

    // ── the repository, against a real in-memory database ──────────────────

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        crate::migrations::run_migrations(&mut conn).expect("migrations");
        conn
    }

    #[test]
    fn a_session_round_trips_and_reads_back_as_a_span() {
        let conn = db();
        let id = open_session(&conn, T0, "0.1.3").unwrap();
        touch_session(&conn, id, T0 + 600).unwrap();

        let spans = get_coverage(&conn, T0 - 10, T0 + DAY, T0 + 700).unwrap();
        assert_eq!(
            spans,
            vec![span(T0, T0 + 700)],
            "an open session is a running app: covered up to now"
        );

        close_session(&conn, id, T0 + 900).unwrap();
        let spans = get_coverage(&conn, T0 - 10, T0 + DAY, T0 + 2000).unwrap();
        assert_eq!(spans, vec![span(T0, T0 + 900)]);
    }

    #[test]
    fn a_session_left_open_by_a_dead_process_is_closed_at_its_watermark() {
        let conn = db();
        let dead = open_session(&conn, T0, "0.1.3").unwrap();
        touch_session(&conn, dead, T0 + 1800).unwrap();

        // A new process starts three days later and reconciles the corpse.
        let live = open_session(&conn, T0 + DAY * 3, "0.1.3").unwrap();
        assert_eq!(reconcile_open_sessions(&conn, live).unwrap(), 1);

        let spans = get_coverage(&conn, T0, T0 + DAY * 4, T0 + DAY * 3).unwrap();
        assert_eq!(
            spans,
            vec![span(T0, T0 + 1800), span(T0 + DAY * 3, T0 + DAY * 3)]
        );

        // The days in between are wholly unrecorded — the bug this table exists for.
        let day2 = T0 + DAY;
        let gaps = coverage_gaps(
            &get_coverage(&conn, day2, day2 + DAY, T0 + DAY * 4).unwrap(),
            day2,
            day2 + DAY,
            T0 + DAY * 4,
            first_coverage_start(&conn).unwrap(),
        );
        assert_eq!(
            unwatched_secs(&gaps),
            DAY,
            "a whole day with nobody watching"
        );
    }

    #[test]
    fn reconcile_leaves_the_live_session_open() {
        let conn = db();
        let live = open_session(&conn, T0, "0.1.3").unwrap();
        assert_eq!(reconcile_open_sessions(&conn, live).unwrap(), 0);
        touch_session(&conn, live, T0 + 60).unwrap();
        assert_eq!(
            get_coverage(&conn, T0, T0 + DAY, T0 + 60).unwrap(),
            vec![span(T0, T0 + 60)]
        );
    }

    #[test]
    fn a_running_app_closed_by_a_second_launch_reopens_on_its_next_beat() {
        let conn = db();
        let first = open_session(&conn, T0, "0.1.4").unwrap();
        touch_session(&conn, first, T0 + 60).unwrap();
        // A second launch reconciles the first as if it had died.
        let second = open_session(&conn, T0 + 120, "0.1.4").unwrap();
        reconcile_open_sessions(&conn, second).unwrap();
        close_session(&conn, second, T0 + 130).unwrap();

        // The first is still running; its next beat undoes that.
        touch_session(&conn, first, T0 + 600).unwrap();
        assert_eq!(
            get_coverage(&conn, T0, T0 + DAY, T0 + 900).unwrap(),
            vec![span(T0, T0 + 900), span(T0 + 120, T0 + 130)]
        );
    }

    #[test]
    fn a_sliver_at_the_edge_of_the_day_is_not_a_gap() {
        // Covered until 100s before midnight; the next session starts the following day.
        let spans = [
            span(T0, T0 + DAY - 100),
            span(T0 + DAY + 1000, T0 + DAY + 2000),
        ];
        let gaps = coverage_gaps(&spans, T0, T0 + DAY, T0 + DAY * 2, Some(T0));
        assert!(gaps.is_empty(), "got {gaps:?}");
    }

    #[test]
    fn first_coverage_start_is_none_before_anything_is_recorded() {
        let conn = db();
        assert_eq!(first_coverage_start(&conn).unwrap(), None);
        open_session(&conn, T0, "0.1.3").unwrap();
        assert_eq!(first_coverage_start(&conn).unwrap(), Some(T0));
    }

    #[test]
    fn pruning_drops_only_spans_wholly_past_the_cutoff() {
        let conn = db();
        let old = open_session(&conn, T0, "0.1.3").unwrap();
        close_session(&conn, old, T0 + 60).unwrap();
        let recent = open_session(&conn, T0 + DAY * 10, "0.1.3").unwrap();
        close_session(&conn, recent, T0 + DAY * 10 + 60).unwrap();

        assert_eq!(prune_coverage(&conn, T0 + DAY * 5).unwrap(), 1);
        assert_eq!(first_coverage_start(&conn).unwrap(), Some(T0 + DAY * 10));
    }
}
