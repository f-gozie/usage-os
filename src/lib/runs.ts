import { formatDuration } from "./format";
import type { CategoryRun } from "./tauri";

/** Matches the rollup's sentinel for active time with no resolved project. */
export const NO_PROJECT = "No project";

/**
 * Human summary of a category-run: the project split as a text line (never a bar — D34)
 * and the unique app list. When a run mixes real projects with off-project time, the
 * "no project" slice is shown alongside them so the inspector total stays honest; a run
 * with no real project shows just its apps.
 */
export function summarizeRun(run: CategoryRun): { projectLabel: string; apps: string } {
  const real = run.projects.filter((p) => p.name !== NO_PROJECT);
  const noProject = run.projects.find((p) => p.name === NO_PROJECT);
  const parts = real.map((p) => `${p.name} ${formatDuration(p.secs)}`);
  // Only label "no project" when it's mixed with real projects — otherwise the apps line
  // already carries it (an all-off-project run shouldn't read "no project Xm").
  if (noProject && real.length > 0) parts.push(`no project ${formatDuration(noProject.secs)}`);
  return { projectLabel: parts.join(" · "), apps: run.apps.join(", ") };
}
