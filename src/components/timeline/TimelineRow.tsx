import { useState } from "react";

import { AppIcon } from "@/components/ui/AppIcon";
import { categoryColorVar } from "@/lib/categories";
import { formatClock, formatDuration } from "@/lib/format";
import { NO_PROJECT } from "@/lib/runs";
import { cn } from "@/lib/utils";
import type { TimelineRun, TimelineSegment } from "@/lib/tauri";

export interface TimelineRowProps {
  run: TimelineRun;
  defaultOpen?: boolean;
}

/** One category-run in the Timeline agenda: a clickable summary (start · colour spine ·
 *  category + project/apps · duration + range) that expands to every app-switch inside it. */
export function TimelineRow({ run, defaultOpen = false }: TimelineRowProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="border-t border-rule">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="-mx-2.5 grid w-full grid-cols-[50px_1fr] items-start gap-4 px-2.5 py-3.5 text-left transition-colors hover:bg-surface"
      >
        <div className="pt-0.5 text-right text-[13px] font-semibold tabular-nums text-muted">
          {formatClock(run.start)}
        </div>
        <div className="flex min-w-0 gap-3.5">
          <span
            className="min-h-[40px] w-[5px] shrink-0 self-stretch"
            style={{ background: categoryColorVar(run.category_slug, run.category_color) }}
          />
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">{run.category_name}</div>
            <div className="mt-[7px] text-[13px] font-semibold leading-[1.5]">
              {projectLine(run)}
            </div>
            {hasProjects(run) && (
              <div className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[12px] text-muted">
                {run.apps.map((app) => (
                  <span
                    key={app}
                    className="inline-flex items-center gap-1.5 font-semibold"
                    style={{ color: "var(--fg)" }}
                  >
                    <AppIcon name={app} size={14} />
                    {app}
                  </span>
                ))}
                {countSwitches(run.segments) > 1 && (
                  <span>· {countSwitches(run.segments)} switches</span>
                )}
              </div>
            )}
          </div>
          <div className="flex w-32 shrink-0 items-start justify-end gap-[11px]">
            <div className="text-right">
              <div className="font-display text-[18px] leading-none">
                {formatDuration(run.secs)}
              </div>
              <div className="mt-[3px] text-[11px] font-semibold tabular-nums text-muted">
                {formatClock(run.start)}–{formatClock(run.end)}
              </div>
            </div>
            <span
              className={cn(
                "w-3 pt-px text-[15px] text-muted transition-transform",
                open && "rotate-90",
              )}
              aria-hidden
            >
              ›
            </span>
          </div>
        </div>
      </button>

      {open && (
        <div className="pb-3.5 pl-[66px] pr-2.5">
          {/* One row per segment (a continuous stretch in one app) — so the count is
              segments, not the switch count the collapsed summary shows. */}
          <div className="py-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted">
            {run.segments.length} app {run.segments.length === 1 ? "stretch" : "stretches"}
          </div>
          {/* Newest-first, matching the agenda: the run list shows the most recent run on top,
              so the most recent app-stretch sits on top inside an expanded run too. */}
          {[...run.segments].reverse().map((seg, i) => {
            // An absorbed detour (D34a) carries a different category than the run — mark it with
            // its own colour dot + label so the expand stays honest about what happened.
            const isDetour = seg.category_slug !== run.category_slug;
            return (
              <div
                key={`${seg.start}-${i}`}
                className="grid grid-cols-[54px_150px_minmax(0,1fr)_auto] items-center gap-3.5 border-t border-rule py-1.5 text-[12.5px] first:border-t-0"
              >
                <span className="tabular-nums text-muted">{formatClock(seg.start)}</span>
                <span className="flex items-center gap-2 truncate font-semibold">
                  <AppIcon name={seg.app} size={16} />
                  <span className="truncate">{seg.app}</span>
                  {isDetour && (
                    <span
                      className="inline-block h-2 w-2 shrink-0 rounded-[2px]"
                      style={{ background: categoryColorVar(seg.category_slug, seg.category_color) }}
                      title={seg.category_name}
                    />
                  )}
                </span>
                <span className="truncate text-muted" title={seg.title ?? undefined}>
                  {seg.title ?? "—"}
                </span>
                <span className="text-right tabular-nums text-muted">{formatDuration(seg.secs)}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** The project line: every project with its time when ≥2 buckets, the lone project's name
 *  when one, else the apps (off-project work shows via the apps, not the project line). */
function projectLine(run: TimelineRun): string {
  const real = run.projects.filter((p) => p.name !== NO_PROJECT);
  const noProject = run.projects.find((p) => p.name === NO_PROJECT);
  const bucketCount = real.length + (noProject ? 1 : 0);

  if (bucketCount >= 2) {
    const parts = real.map((p) => `${p.name} ${formatDuration(p.secs)}`);
    if (noProject) parts.push(`no project ${formatDuration(noProject.secs)}`);
    return parts.join("  ·  ");
  }
  if (real.length === 1) return real[0].name;
  return run.apps.join(", ");
}

/** Whether the run carries real projects — when it does, the project line shows the
 *  projects and the apps line (icons + names + a switch count) is shown beneath it. */
function hasProjects(run: TimelineRun): boolean {
  return run.projects.some((p) => p.name !== NO_PROJECT);
}

function countSwitches(segments: TimelineSegment[]): number {
  let switches = 0;
  for (let i = 1; i < segments.length; i++) {
    if (segments[i].app !== segments[i - 1].app || segments[i].project !== segments[i - 1].project) {
      switches++;
    }
  }
  return switches;
}
