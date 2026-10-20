-- Clear project labels that came from a window title, then delete the projects left with no
-- spans. A title now names a project only in a code editor (D77); before that, any app's title
-- could, and on real data those projects were browser profile names, track titles and calls.
-- What stays: projects with a git remote (cwd or a github.com URL) and spans whose file:// URL
-- names the project. A git folder with no remote also loses its old labels; new spans get
-- it back from the terminal's cwd.
UPDATE activity_logs
SET project_id = NULL, project_abstain_reason = 'no-signal'
WHERE (url IS NULL OR url NOT LIKE 'file://%')
  AND project_id IN (SELECT id FROM projects WHERE remote_url IS NULL);

DELETE FROM project_aliases
WHERE project_id IN (
    SELECT id FROM projects
    WHERE remote_url IS NULL
      AND id NOT IN (SELECT project_id FROM activity_logs WHERE project_id IS NOT NULL)
);

DELETE FROM projects
WHERE remote_url IS NULL
  AND id NOT IN (SELECT project_id FROM activity_logs WHERE project_id IS NOT NULL);
