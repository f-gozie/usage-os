-- D47 — relatable default categories. The shipped defaults leaned on jargon
-- ("Deep work", "Comms", "Breaks") and had no Personal bucket. This gives FRESH installs
-- a plain-spoken five-category model: Work · Browsing · Messaging · Entertainment ·
-- Personal.
--
-- Scope: every statement is gated on an empty activity_logs, so this migration ONLY ever
-- changes a fresh database. An existing user's categories/rules are never modified —
-- those evolve by user action. (The dev DB's relatable names were applied directly, not
-- by this migration.) `slug` stays the stable identity (D46); SQL table names unchanged.

-- (a) Relatable display names (fresh installs only).
UPDATE categories SET name = 'Work'          WHERE slug = 'deep'     AND (SELECT COUNT(*) FROM activity_logs) = 0;
UPDATE categories SET name = 'Browsing'      WHERE slug = 'research' AND (SELECT COUNT(*) FROM activity_logs) = 0;
UPDATE categories SET name = 'Messaging'     WHERE slug = 'comms'    AND (SELECT COUNT(*) FROM activity_logs) = 0;
UPDATE categories SET name = 'Entertainment' WHERE slug = 'breaks'   AND (SELECT COUNT(*) FROM activity_logs) = 0;

-- (b) The fifth canonical category. Theme-aware via the --c-personal token; the hex is the
--     paper-theme fallback. Fresh installs only, and only if a 'personal' slug is absent.
INSERT INTO categories (slug, name, color)
SELECT 'personal', 'Personal', '#2E8B57'
WHERE (SELECT COUNT(*) FROM activity_logs) = 0
  AND NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'personal');

-- (c) Starter rules for the five-category model (fresh installs only). Media (Spotify,
--     Music) moves from Entertainment to Personal; Personal also picks up common life apps.
DELETE FROM rules
WHERE (SELECT COUNT(*) FROM activity_logs) = 0
  AND match_field = 'process'
  AND pattern IN ('Spotify', 'Music')
  AND category_id = (SELECT id FROM categories WHERE slug = 'breaks');

WITH personal_apps(pattern) AS (VALUES
    ('Spotify'), ('Music'), ('Podcasts'), ('Prime Video'), ('VLC'), ('QuickTime Player'),
    ('FaceTime'), ('Photos'), ('Calendar'), ('Reminders'), ('Notes'), ('Maps'),
    ('Books'), ('Weather'), ('Health'), ('Freeform'), ('Contacts')
)
INSERT INTO rules (category_id, match_field, pattern, ignore_title)
SELECT c.id, 'process', personal_apps.pattern, 0
FROM categories c
JOIN personal_apps
WHERE c.slug = 'personal'
  AND (SELECT COUNT(*) FROM activity_logs) = 0;
