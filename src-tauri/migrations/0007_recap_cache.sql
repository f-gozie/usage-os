-- D52: a cache for the on-device AI recap, so a day's prose is narrated once, not on every
-- view. Keyed by (day_start, fingerprint) where the fingerprint is a content hash of the
-- EXACT facts the model narrated (rollup::recap_fingerprint). Because the key is the content,
-- invalidation is free: a rule reprocess that changes a day's facts produces a new fingerprint
-- and the stale row is simply never matched again — there is nothing to find-and-delete.
--
-- Only successful model recaps are stored (never the deterministic template fallback — a
-- cold/unavailable model must retry next open). This is captured-derived data, so it is wiped
-- by delete_all_data and pruned by retention cleanup, exactly like the events it summarizes.
CREATE TABLE recap_cache (
    day_start    INTEGER NOT NULL, -- local midnight (Unix secs); the get_recap range start
    fingerprint  TEXT    NOT NULL, -- FNV-1a of "v<VERSION>\n" + the formatted facts prompt
    text         TEXT    NOT NULL, -- the model-written prose
    generated_by TEXT    NOT NULL, -- always 'foundation-models' (the template is never cached)
    created_at   INTEGER NOT NULL,
    PRIMARY KEY (day_start, fingerprint)
);
