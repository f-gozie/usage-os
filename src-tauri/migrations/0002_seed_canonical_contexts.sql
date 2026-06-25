-- Seed the four canonical contexts. `slug` is the stable identity (UI maps it to the
-- --c-<slug> token); `color` is the paper-theme fallback hex. INSERT OR IGNORE → idempotent.
INSERT OR IGNORE INTO categories (slug, name, color) VALUES
    ('deep',     'Deep work', '#1B45BE'),
    ('research', 'Research',  '#E0241B'),
    ('comms',    'Comms',     '#EAB308'),
    ('breaks',   'Breaks',    '#161616');
