import { useState } from "react";

import { contextColorVar } from "@/lib/contexts";
import { formatClock, formatDuration } from "@/lib/format";
import { NO_PROJECT } from "@/lib/runs";
import { cn } from "@/lib/utils";
import type { TimelineRun, TimelineSegment } from "@/lib/tauri";

export interface TimelineRowProps {
  run: TimelineRun;
  defaultOpen?: boolean;
}

/** One context-run in the Timeline agenda: a clickable summary (start · colour spine ·
 *  context + project/apps · duration + range) that expands to every app-switch inside it. */
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
            style={{ background: contextColorVar(run.context_slug) }}
          />
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">{run.context_name}</div>
            <div className="mt-[7px] text-[13px] font-semibold leading-[1.5]">
              {projectLine(run)}
            </div>
            {appsLine(run) && (
              <div className="mt-1.5 truncate text-[12px] text-muted">{appsLine(run)}</div>
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
          <div className="py-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted">
            {run.segments.length} app {run.segments.length === 1 ? "switch" : "switches"}
          </div>
          {run.segments.map((seg, i) => (
            <div
              key={`${seg.start}-${i}`}
              className="grid grid-cols-[54px_124px_1fr_auto] items-center gap-3.5 border-t border-rule py-1.5 text-[12.5px] first:border-t-0"
            >
              <span className="tabular-nums text-muted">{formatClock(seg.start)}</span>
              <span className="font-semibold">{seg.app}</span>
              <span className="text-muted">{seg.project ?? "—"}</span>
              <span className="text-right tabular-nums text-muted">{formatDuration(seg.secs)}</span>
            </div>
          ))}
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

/** The apps line (with a switch count) — only when the project line is carrying projects. */
function appsLine(run: TimelineRun): string {
  const hasProjects = run.projects.some((p) => p.name !== NO_PROJECT);
  if (!hasProjects) return "";
  const switches = countSwitches(run.segments);
  return run.apps.join(" · ") + (switches > 1 ? `  ·  ${switches} switches` : "");
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
