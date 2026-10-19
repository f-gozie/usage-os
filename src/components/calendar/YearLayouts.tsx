import { memo, useCallback } from "react";

import { dayFromEvent, YearCell } from "@/components/calendar/YearGrid";
import type { DayModel } from "@/components/calendar/yearModel";
import { cn } from "@/lib/utils";

/** Sunday-first, matching the shipped `weekDays()` and the Week view. Two week-starts in one
 *  app would be a bug, so the wall calendar follows the app rather than the other way round. */
const DOW = ["S", "M", "T", "W", "T", "F", "S"];

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const DAY_TICKS = Array.from({ length: 31 }, (_, i) => (i === 0 || (i + 1) % 5 === 0 ? i + 1 : ""));

export interface YearLayoutProps {
  /** The year's day models, grouped by month (from `buildYearModel`). */
  months: DayModel[][];
  /** Index of the current month when this is the current year, else -1. */
  currentMonth: number;
  onOpenMonth: (dayStart: number) => void;
  onOpenDay: (dayStart: number) => void;
}

/** Running count of days before each month, so a cell's stagger index is its day-of-year. */
function monthOffsets(months: DayModel[][]): number[] {
  const offsets: number[] = [];
  let total = 0;
  for (const m of months) {
    offsets.push(total);
    total += m.length;
  }
  return offsets;
}

/** One listener for the whole grid. A click on a bar bubbles here and is resolved from
 *  `data-day`; month names carry `data-month-start` and resolve the same way. */
function useGridClick(onOpenDay: (d: number) => void, onOpenMonth: (d: number) => void) {
  return useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      const monthEl = (event.target as HTMLElement).closest<HTMLElement>("[data-month-start]");
      if (monthEl?.dataset.monthStart) {
        onOpenMonth(Number(monthEl.dataset.monthStart));
        return;
      }
      const day = dayFromEvent(event);
      if (day !== null) onOpenDay(day);
    },
    [onOpenDay, onOpenMonth],
  );
}

/**
 * Layout A — twelve month rows, one slim bar per day.
 *
 * The year as one continuous run: a heavy stretch, a holiday, the month you were ill, all read
 * left to right without the eye resetting. What it gives up is day-of-week alignment.
 */
export const YearRows = memo(function YearRows({
  months,
  currentMonth,
  onOpenMonth,
  onOpenDay,
}: YearLayoutProps) {
  const onClick = useGridClick(onOpenDay, onOpenMonth);
  const offsets = monthOffsets(months);

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions -- delegation target; every
    // actionable descendant is a real <button>, so keyboard and AT reach them directly.
    <div className="cal-view border-2 border-edge bg-surface px-4 pb-2.5 pt-3.5" onClick={onClick}>
      <div className="mb-1.5 grid grid-cols-[42px_1fr] gap-2.5">
        <span />
        <div className="grid grid-cols-31 text-[9px] font-semibold text-muted">
          {DAY_TICKS.map((tick, i) => (
            <span key={i} className="text-center">
              {tick}
            </span>
          ))}
        </div>
      </div>

      {months.map((monthDays, i) => (
        <div
          key={i}
          className="cal-row mb-[3px] grid grid-cols-[42px_1fr] items-end gap-2.5 border-b border-rule pb-[3px] last:mb-0 last:border-b-0"
        >
          <button
            type="button"
            data-month-start={monthDays[0]?.dayStart}
            disabled={monthDays.length === 0}
            className={cn(
              "cal-month pb-[5px] text-left font-display text-[13px] uppercase tracking-[0.05em]",
              i === currentMonth ? "text-fg" : "text-muted",
            )}
          >
            {MONTHS[i]}
          </button>
          <div className="grid h-[34px] grid-cols-31 items-end">
            {monthDays.map((model, d) => (
              <YearCell key={model.dayStart} model={model} index={offsets[i] + d} barWidthClass="w-[46%] min-w-[7px] max-w-[18px]" />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
});

/**
 * Layout B — twelve miniature month calendars.
 *
 * Day-of-week alignment: weekends land in their own columns, so the shape of your week is
 * obvious. What it gives up is continuity — twelve small grids read as twelve things.
 *
 * Four columns, fixed. The app window is ~1000px, so viewport breakpoints like `lg:` (1024px)
 * never fire inside it — the first build used them and silently shipped three columns where
 * the board draws four.
 */
export const YearWall = memo(function YearWall({
  months,
  currentMonth,
  onOpenMonth,
  onOpenDay,
}: YearLayoutProps) {
  const onClick = useGridClick(onOpenDay, onOpenMonth);
  const offsets = monthOffsets(months);

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions -- delegation target, as above.
    <div
      className="cal-view grid grid-cols-4 gap-x-4 gap-y-[18px] border-2 border-edge bg-surface px-4 pb-2.5 pt-3.5"
      onClick={onClick}
    >
      {months.map((monthDays, i) => (
        <div key={i}>
          <button
            type="button"
            data-month-start={monthDays[0]?.dayStart}
            disabled={monthDays.length === 0}
            className={cn(
              "cal-month mb-[5px] w-full border-b pb-[3px] text-left font-display text-[12px] uppercase tracking-[0.06em]",
              i === currentMonth ? "border-b-2 border-edge text-fg" : "border-rule text-muted",
            )}
          >
            {MONTHS[i]}
          </button>
          <div className="mb-0.5 grid grid-cols-7 text-center text-[7.5px] tracking-[0.06em] text-muted">
            {DOW.map((d, j) => (
              <span key={j}>{d}</span>
            ))}
          </div>
          <div className="grid grid-cols-7 gap-0.5">
            {Array.from({ length: monthDays[0]?.dow ?? 0 }, (_, j) => (
              <span key={`pad-${j}`} className="h-[22px]" />
            ))}
            {monthDays.map((model, d) => (
              <span key={model.dayStart} className="cal-wallcell h-[22px] border border-rule">
                <YearCell model={model} index={offsets[i] + d} barWidthClass="w-[52%] min-w-[6px] max-w-[12px]" />
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
});
