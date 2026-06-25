-- UsageOS baseline schema. Immutable once shipped — see migrations/README.md.

-- Contexts. The SQL table name `categories` (and `category_id` elsewhere) is the
-- intentional legacy name; the IPC noun is "category" (see D46). `slug` carries the
-- canonical context identity the UI maps to a colour token (--c-<slug>): deep |
-- research | comms | breaks. A NULL slug is a user-created context (supplies its own `color`).
CREATE TABLE categories (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    slug  TEXT UNIQUE,
    name  TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL
);

-- Canonical projects (D30): keyed on the git remote `owner/repo` (or folder name when a repo
-- has no remote); folder/title/url variants resolve to one project via `project_aliases`.
CREATE TABLE projects (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_key TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    remote_url    TEXT,
    created_at    INTEGER NOT NULL
);

CREATE TABLE project_aliases (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    alias_kind  TEXT NOT NULL,
    alias_value TEXT NOT NULL,
    UNIQUE(alias_kind, alias_value)
);
CREATE INDEX idx_project_aliases_lookup ON project_aliases(alias_kind, alias_value);

-- Browser site registry. `kind` seeds the ambiguous-vs-general distinction (D30):
-- 'general' | 'dashboard' | 'project-host' | 'unknown'.
CREATE TABLE sites (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    host         TEXT NOT NULL UNIQUE,
    display_name TEXT,
    kind         TEXT NOT NULL DEFAULT 'unknown',
    created_at   INTEGER NOT NULL
);

-- Events (legacy table name `activity_logs`): one span of focus on an app/window,
-- enriched with site/project + sensitive-handling flags (D8). project_id NULL =
-- unassigned; project_abstain_reason records the abstain kind ('no-signal' | 'ambiguous').
CREATE TABLE activity_logs (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    process_name           TEXT NOT NULL,
    window_title           TEXT NOT NULL,
    start_time             INTEGER NOT NULL,
    end_time               INTEGER NOT NULL,
    is_idle                INTEGER NOT NULL,
    category_id            INTEGER REFERENCES categories(id),
    url                    TEXT,
    site                   TEXT,
    project_id             INTEGER REFERENCES projects(id),
    project_abstain_reason TEXT,
    is_private             INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_start_time       ON activity_logs(start_time);
CREATE INDEX idx_activity_project ON activity_logs(project_id);

-- Category rules engine: match a process/title substring → context.
CREATE TABLE rules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    match_field  TEXT NOT NULL,             -- 'process' | 'title'
    pattern      TEXT NOT NULL,
    ignore_title INTEGER NOT NULL DEFAULT 0
);

-- Sensitive handling (D8): mode='exclude' drops the event; mode='private' records
-- time + app but omits title/url at write time (never store-then-filter).
CREATE TABLE exclusions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    match_type TEXT NOT NULL,               -- 'app' | 'site' | 'title'
    pattern    TEXT NOT NULL,
    mode       TEXT NOT NULL,               -- 'exclude' | 'private'
    created_at INTEGER NOT NULL,
    UNIQUE(match_type, pattern, mode)
);

-- Key/value app settings.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
