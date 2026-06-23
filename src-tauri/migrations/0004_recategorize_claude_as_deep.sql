-- Re-point the "Claude" starter rule from Research to Deep work.
--
-- The 0003 default filed any process named "Claude" under Research (it was written
-- with the claude.ai chat app in mind). But that pattern also catches the Claude Code
-- desktop app — and supervising an AI coding agent is deep work, not browsing. So the
-- rule moves to Deep work. The claude.ai *web* tab is unaffected: it runs inside a
-- browser (Chrome/Safari/Arc/...) and matches those process rules, never this one.
--
-- Idempotent: a fresh install runs 0003 (Claude->Research) then this (->Deep work),
-- landing on Deep work; an existing install just re-points the row it already has.
UPDATE rules
SET category_id = (SELECT id FROM categories WHERE slug = 'deep')
WHERE match_field = 'process'
  AND pattern = 'Claude';

-- Bring already-recorded Claude spans in line with the new rule, so the change shows up
-- in past days too instead of only newly-captured time. Scoped deliberately: only rows
-- that were auto-filed as Research (the old default) move — never anything the user (or
-- another rule) categorized differently. `category_id` is a derived label, not raw truth,
-- so relabelling history to match the current rule is the expected behaviour.
UPDATE activity_logs
SET category_id = (SELECT id FROM categories WHERE slug = 'deep')
WHERE category_id = (SELECT id FROM categories WHERE slug = 'research')
  AND lower(process_name) LIKE '%claude%';
