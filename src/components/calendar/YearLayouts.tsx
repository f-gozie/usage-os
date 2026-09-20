import { YearCell } from "@/components/calendar/YearGrid";
import type { YearDay } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Sunday-first, matching the shipped `weekDays()` and the Week view. Two week-starts in one
 *  app would be a bug, so the wall calendar follows the app rather than the other way round. */
const DOW = ["S", "M", "T", "W", "T", "F", "S"];

const MONTHS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

export interface YearLayoutProps {
  days: YearDay[];
  now: number;
  /** Jump to a month (clicking its name). */
  onOpenMonth: (dayStart: number) => void;
  /** Jump to a day (clicking its bar). */
  onOpenDay: (dayStart: number) => void;
}

/** Group the year's days by calendar month, keeping order. */
function byMonth(days: YearDay[]): YearDay[][] {
  const months: YearDay[][] = Array.from({ length: 12 }, () => []);
  for (const day of days) {
    months[new Date(day.day_start * 1000).getMonth()].push(day);
  }
  return months;
}

const dateText = (dayStart: number) =>
  new Date(dayStart * 1000).toLocaleDateString(undefined, {
    weekday: "short",
    day: "numeric",
    month: "long",
  });

/**
 * Layout A — twelve month rows, one slim bar per day.
 *
 * The year as one continuous run: a heavy stretch, a holiday, the month you were ill, all read
 * left to right without the eye resetting. What it gives up is day-of-week alignment.
 */
export function YearRows({ days, now, onOpenMonth, onOpenDay }: YearLayoutProps) {
  const months = byMonth(days);
  const thisMonth = new Date(now * 1000).getMonth();
  const thisYear = new Date(now * 1000).getFullYear();
  const showingThisYear = days.length > 0 && new Date(days[0].day_start * 1000).getFullYear() === thisYear;

  return (
    <div className="border-2 border-edge bg-surface px-4 py-3">
      <div className="mb-1.5 grid grid-cols-[2.6rem_1fr] gap-2.5">
        <span />
        <div className="grid grid-cols-31 text-[9px] font-semibold text-muted">
          {Array.from({ length: 31 }, (_, i) => (
            <span key={i} className="text-center">
              {i === 0 || (i + 1) % 5 === 0 ? i + 1 : ""}
            </span>
          ))}
        </div>
      </div>

      {months.map((monthDays, i) => (
        <div
          key={i}
          className="mb-[3px] grid grid-cols-[2.6rem_1fr] items-end gap-2.5 border-b border-rule pb-[3px] last:mb-0 last:border-b-0"
        >
          <button
            type="button"
            onClick={() => monthDays[0] && onOpenMonth(monthDays[0].day_start)}
            disabled={monthDays.length === 0}
            className={cn(
              "pb-1.5 text-left font-display text-[13px] uppercase tracking-[0.05em]",
              showingThisYear && i === thisMonth ? "text-fg" : "text-muted hover:text-fg",
            )}
          >
            {MONTHS[i]}
          </button>
          <div className="grid h-[34px] grid-cols-31 items-end">
            {monthDays.map((day) => (
              <YearCell
                key={day.day_start}
                day={day}
                now={now}
                dateText={dateText(day.day_start)}
                barWidthClass="w-[13px]"
                onOpen={onOpenDay}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * Layout B — twelve miniature month calendars.
 *
 * Day-of-week alignment: weekends land in their own columns, so the shape of your week is
 * obvious. What it gives up is continuity — twelve small grids read as twelve things.
 */
export function YearWall({ days, now, onOpenMonth, onOpenDay }: YearLayoutProps) {
  const months = byMonth(days);
  const thisMonth = new Date(now * 1000).getMonth();
  const thisYear = new Date(now * 1000).getFullYear();
  const showingThisYear = days.length > 0 && new Date(days[0].day_start * 1000).getFullYear() === thisYear;

  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-5 border-2 border-edge bg-surface p-4 sm:grid-cols-3 lg:grid-cols-4">
      {months.map((monthDays, i) => {
        // Leading blanks so the 1st lands under its weekday.
        const pads = monthDays.length > 0 ? new Date(monthDays[0].day_start * 1000).getDay() : 0;
        return (
          <div key={i}>
            <button
              type="button"
              onClick={() => monthDays[0] && onOpenMonth(monthDays[0].day_start)}
              disabled={monthDays.length === 0}
              className={cn(
                "mb-1.5 w-full border-b pb-1 text-left font-display text-[12px] uppercase tracking-[0.06em]",
                showingThisYear && i === thisMonth
                  ? "border-b-2 border-edge text-fg"
                  : "border-rule text-muted hover:text-fg",
              )}
            >
              {MONTHS[i]}
            </button>
            <div className="mb-0.5 grid grid-cols-7 text-center text-[7.5px] tracking-[0.06em] text-muted">
              {DOW.map((d, j) => (
                <span key={j}>{d}</span>
              ))}
            </div>
            <div className="grid grid-cols-7 gap-px">
              {Array.from({ length: pads }, (_, j) => (
                <span key={`pad-${j}`} className="h-[22px]" />
              ))}
              {monthDays.map((day) => (
                <span key={day.day_start} className="h-[22px] border border-rule">
                  <YearCell
                    day={day}
                    now={now}
                    dateText={dateText(day.day_start)}
                    barWidthClass="w-[9px]"
                    onOpen={onOpenDay}
                  />
                </span>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}
