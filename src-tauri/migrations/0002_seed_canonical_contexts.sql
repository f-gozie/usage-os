-- The four canonical contexts. `slug` is the stable identity the UI maps to a colour
-- token (--c-<slug>, theme-aware); `color` is a paper-theme fallback hex for any
-- consumer that isn't slug-aware. INSERT OR IGNORE keeps a re-run (or a user who
-- already has them) a no-op.
INSERT OR IGNORE INTO categories (slug, name, color) VALUES
    ('deep',     'Deep work', '#1B45BE'),
    ('research', 'Research',  '#E0241B'),
    ('comms',    'Comms',     '#EAB308'),
    ('breaks',   'Breaks',    '#161616');
