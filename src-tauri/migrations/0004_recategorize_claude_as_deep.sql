-- Re-point the "Claude" process rule from Research to Deep work (supervising the Claude Code
-- desktop app is deep work, not browsing; the claude.ai web tab matches browser rules, not this).
-- Idempotent — re-running just lands the row on Deep work.
UPDATE rules
SET category_id = (SELECT id FROM categories WHERE slug = 'deep')
WHERE match_field = 'process'
  AND pattern = 'Claude';

-- Relabel past Claude spans to match (category_id is a derived label). Scoped to rows still
-- on the old Research default — never anything the user or another rule already moved.
UPDATE activity_logs
SET category_id = (SELECT id FROM categories WHERE slug = 'deep')
WHERE category_id = (SELECT id FROM categories WHERE slug = 'research')
  AND lower(process_name) LIKE '%claude%';
