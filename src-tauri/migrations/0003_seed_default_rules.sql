-- Starter categorization rules so the dial is meaningful out-of-the-box. Conservative
-- process-name matches (the rules engine does case-insensitive substring matching);
-- the user refines them in Settings (M4). Browsers lean to Research as a safe default.
-- Joined by context slug so it doesn't depend on category id ordering.
WITH starter(slug, pattern) AS (VALUES
    ('deep', 'Cursor'),
    ('deep', 'Code'),
    ('deep', 'Xcode'),
    ('deep', 'iTerm'),
    ('deep', 'Terminal'),
    ('deep', 'Warp'),
    ('deep', 'Ghostty'),
    ('deep', 'Zed'),
    ('deep', 'Neovim'),
    ('deep', 'Vim'),
    ('research', 'Chrome'),
    ('research', 'Safari'),
    ('research', 'Arc'),
    ('research', 'Firefox'),
    ('research', 'Brave'),
    ('research', 'Claude'),
    ('research', 'Preview'),
    ('comms', 'Slack'),
    ('comms', 'Discord'),
    ('comms', 'Mail'),
    ('comms', 'Messages'),
    ('comms', 'Zoom'),
    ('comms', 'Telegram'),
    ('comms', 'WhatsApp'),
    ('comms', 'Teams'),
    ('breaks', 'Spotify'),
    ('breaks', 'Music'),
    ('breaks', 'Reddit'),
    ('breaks', 'YouTube'),
    ('breaks', 'Netflix'),
    ('breaks', 'Steam')
)
INSERT INTO rules (category_id, match_field, pattern, ignore_title)
SELECT c.id, 'process', starter.pattern, 0
FROM categories c
JOIN starter ON starter.slug = c.slug;
