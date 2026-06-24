import { formatDuration } from "./format";
import type { CategoryRun } from "./tauri";

/** Matches the rollup's sentinel for active time with no resolved project. */
export const NO_PROJECT = "No project";

/**
 * Human summary of a category-run: the project split as a text line (never a bar —
 * D34) and the unique app list. Off-project time is omitted from the project line and
 * shows up via the apps instead.
 */
export function summarizeRun(run: CategoryRun): { projectLabel: string; apps: string } {
  const projectLabel = run.projects
    .filter((p) => p.name !== NO_PROJECT)
    .map((p) => `${p.name} ${formatDuration(p.secs)}`)
    .join(" · ");
  return { projectLabel, apps: run.apps.join(", ") };
}
