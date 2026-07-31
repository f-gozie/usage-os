-- D70 — site rules + precedence. Until now every rule matched on process name, so one
-- broad `Chrome → Browsing` rule swallowed every tab: YouTube, football streams and a
-- code review all landed in the same arc, and Entertainment read ~0 for real users.
--
-- The matcher now consults `site` rules first (see `db::categories::match_field_tier`), so
-- these seeds override the browser's own process rule without reordering anything.
--
-- INVARIANT (differs from 0006 deliberately): this is NOT gated on an empty activity_logs.
-- Existing installs are exactly who needs the fix. It is still strictly additive — every
-- INSERT is guarded by NOT EXISTS on the same (category, field, pattern), and nothing a
-- user wrote is ever edited or deleted. Re-running is a no-op.
--
-- Hosts are matched on a `.` boundary (the host itself or a subdomain), so `x.com` does not
-- match `netflix.com`. Only unambiguous hosts are seeded — social/news stay in the browser's
-- own category rather than being mis-sorted by us.

-- Entertainment: video, streaming, endless-scroll.
WITH site_rule(slug, pattern) AS (VALUES
    ('breaks', 'youtube.com'),
    ('breaks', 'netflix.com'),
    ('breaks', 'primevideo.com'),
    ('breaks', 'hulu.com'),
    ('breaks', 'disneyplus.com'),
    ('breaks', 'max.com'),
    ('breaks', 'twitch.tv'),
    ('breaks', 'tiktok.com'),
    ('breaks', 'reddit.com'),
    ('breaks', 'instagram.com'),
    ('breaks', 'espn.com'),
    ('breaks', 'dstv.stream'),
    ('breaks', 'crunchyroll.com'),

    -- Personal: music in a tab follows the desktop apps into Personal (D47).
    ('personal', 'open.spotify.com'),
    ('personal', 'soundcloud.com'),
    ('personal', 'music.apple.com'),

    -- Messaging: calls and inboxes in a tab.
    ('comms', 'meet.google.com'),
    ('comms', 'zoom.us'),
    ('comms', 'teams.microsoft.com'),
    ('comms', 'discord.com'),
    ('comms', 'web.whatsapp.com'),
    ('comms', 'mail.google.com'),
    ('comms', 'outlook.office.com'),
    ('comms', 'slack.com'),

    -- Work: code, docs, dashboards, the things you have open because of a job.
    ('deep', 'github.com'),
    ('deep', 'gitlab.com'),
    ('deep', 'bitbucket.org'),
    ('deep', 'stackoverflow.com'),
    ('deep', 'developer.mozilla.org'),
    ('deep', 'localhost'),
    ('deep', '127.0.0.1'),
    ('deep', 'docs.google.com'),
    ('deep', 'notion.so'),
    ('deep', 'linear.app'),
    ('deep', 'atlassian.net'),
    ('deep', 'figma.com'),
    ('deep', 'vercel.com'),
    ('deep', 'dash.cloudflare.com'),
    ('deep', 'console.cloud.google.com'),
    ('deep', 'console.aws.amazon.com'),
    ('deep', 'appstoreconnect.apple.com'),
    ('deep', 'developer.apple.com'),
    ('deep', 'posthog.com'),
    ('deep', 'npmjs.com'),
    ('deep', 'crates.io'),
    ('deep', 'platform.openai.com'),
    ('deep', 'console.anthropic.com')
)
INSERT INTO rules (category_id, match_field, pattern, ignore_title)
SELECT c.id, 'site', site_rule.pattern, 0
FROM categories c
JOIN site_rule ON site_rule.slug = c.slug
WHERE NOT EXISTS (
    SELECT 1 FROM rules r
    WHERE r.category_id = c.id
      AND r.match_field = 'site'
      AND lower(r.pattern) = lower(site_rule.pattern)
);
