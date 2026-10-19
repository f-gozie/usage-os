-- One row per run of UsageOS: the stretches during which the app was actually watching.
--
-- Without this an empty day is ambiguous. "You didn't use the Mac" and "UsageOS wasn't
-- running" are opposite facts that render identically, so the app can lose days and say
-- nothing — which it did, for three days, before anyone noticed.
--
-- `last_seen` is a watermark bumped while the app runs, so a process that dies without
-- saying so still leaves a defensible end. `ended_at` is set only on a clean exit.
CREATE TABLE watch_coverage (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL,
    ended_at   INTEGER,
    version    TEXT NOT NULL
);

CREATE INDEX idx_watch_coverage_started ON watch_coverage(started_at);
