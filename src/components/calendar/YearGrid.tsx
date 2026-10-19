import { memo } from "react";

import type { DayModel } from "@/components/calendar/yearModel";
import { cn } from "@/lib/utils";

/** The stacked bar for one day: height is hours, segments are categories.
 *
 *  Height is geometry, not a second colour channel — which is what keeps this from becoming a
 *  contribution graph. Colour still encodes exactly one thing (design-system rule 1). */
function Stack({ model }: { model: DayModel }) {
  if (model.state === "future") return null;

  if (model.state === "unrecorded") {
    // A hollow slot at full height — the opposite mark from a short bar, because it is the
    // opposite fact. Never red, never an icon: a statement about the app, not about you.
    return <span className="cal-bar block h-full w-full border border-dashed border-muted opacity-60" />;
  }

  if (model.heightPct === 0) {
    // Watched, and you simply didn't touch it. A stub on the baseline, not an empty cell.
    return <span className="block h-px w-full bg-track" />;
  }

  return (
    <span
      className="cal-bar flex w-full flex-col-reverse border border-casing"
      style={{ height: `${model.heightPct}%` }}
    >
      {model.segments.map((s) => (
        <span key={s.key} className="block w-full" style={{ height: `${s.pct}%`, background: s.color }} />
      ))}
    </span>
  );
}

export interface YearCellProps {
  model: DayModel;
  /** Position in the year, 0-based — drives the entrance stagger via a CSS variable. */
  index: number;
  barWidthClass: string;
}

/**
 * One day. Deliberately has NO click handler and NO callback props: the grid owns a single
 * delegated listener and reads `data-day`. With 365 cells, a closure per cell meant every
 * parent render handed every cell a new function and defeated memoization entirely.
 *
 * Memoized on the frozen model, so a layout switch, a hover, or a minute tick that changes
 * nothing about this day costs this cell nothing.
 */
export const YearCell = memo(function YearCell({ model, index, barWidthClass }: YearCellProps) {
  const interactive = model.state !== "future";
  return (
    <button
      type="button"
      disabled={!interactive}
      data-day={interactive ? model.dayStart : undefined}
      aria-label={model.label}
      title={model.label}
      // The stagger is one CSS variable per cell; the animation itself lives in CSS and runs on
      // the compositor (transform only), so 365 bars rising costs no layout and no JS.
      style={{ "--i": index } as React.CSSProperties}
      className={cn(
        // `w-full` matters: inside the wall layout's wrapper the button would otherwise shrink to
        // its bar and sit hard-left, so `justify-center` had nothing to centre within.
        "cal-cell flex h-full w-full items-end justify-center p-px",
        interactive ? "cursor-pointer" : "cursor-default",
        model.state === "unrecorded" &&
          "bg-[repeating-linear-gradient(45deg,transparent_0_3px,var(--rule)_3px_4px)]",
      )}
    >
      <span className={cn("flex h-full flex-col justify-end", barWidthClass)}>
        <Stack model={model} />
      </span>
    </button>
  );
});

/** Resolve a delegated click to the day it landed on, or null if it hit a gap or a future day. */
export function dayFromEvent(event: React.MouseEvent<HTMLElement>): number | null {
  const cell = (event.target as HTMLElement).closest<HTMLElement>("[data-day]");
  if (!cell?.dataset.day) return null;
  const day = Number(cell.dataset.day);
  return Number.isFinite(day) ? day : null;
}
