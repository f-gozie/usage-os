import { categoryColorVar } from "@/lib/categories";
import { formatDuration } from "@/lib/format";
import type { YearDay } from "@/lib/tauri";

/** How many hours fill a bar. Fixed, not scaled to the busiest day: a bar has to mean the same
 *  thing in January as in July, or the year quietly flatters every year equally. */
export const YEAR_CEILING_SECS = 12 * 3600;

/** A day is drawn as "not recorded" only when essentially none of it was watched — the same
 *  threshold the recap uses to decide a day has no story rather than a partial one. */
const UNRECORDED_SECS = 23 * 3600;

export type DayState = "recorded" | "unrecorded" | "unknown" | "future";

/** Everything a cell needs to paint, computed ONCE per data load rather than per render.
 *
 *  The year is 365 cells. Deriving state, label, colours and a localized date inside render
 *  meant every click anywhere re-did all of it 365 times — and `toLocaleDateString` builds a
 *  fresh formatter on each call, which is the slow part. Cells now receive a frozen model and
 *  can be memoized on identity. */
export interface DayModel {
  dayStart: number;
  month: number;
  /** Day of week, 0 = Sunday — the wall calendar's leading pad for the 1st. */
  dow: number;
  state: DayState;
  label: string;
  /** Bar height as a percentage of the row, already clamped to the ceiling. */
  heightPct: number;
  segments: { key: string; pct: number; color: string }[];
}

// One formatter for the whole app lifetime. Constructing `Intl.DateTimeFormat` is the expensive
// part of `toLocaleDateString`; `format()` on an existing one is cheap.
const DATE_FMT = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  day: "numeric",
  month: "long",
});

export function dayState(day: YearDay, now: number): DayState {
  if (day.day_start > now) return "future";
  if (!day.coverage_known) return "unknown";
  if (day.unwatched_secs >= UNRECORDED_SECS) return "unrecorded";
  return "recorded";
}

export function dayLabel(day: YearDay, state: DayState, dateText: string): string {
  switch (state) {
    case "future":
      return dateText;
    case "unrecorded":
      return `${dateText} · UsageOS wasn’t running`;
    default:
      // "unknown" lands here on purpose. Unknown coverage means only that the coverage log did
      // not exist yet — NOT that nothing was recorded. The first version said "before UsageOS
      // was recording" for every such day and dropped its hours, which on a real install is
      // every day of history: months of plainly-recorded days each claiming the app was absent.
      // We say what we know (the hours, or that nothing was tracked) and claim nothing more.
      return day.active_secs > 0
        ? `${dateText} · ${formatDuration(day.active_secs)}`
        : `${dateText} · nothing tracked`;
  }
}

/** Build the year's models, grouped by month. Pure — `now` is passed in so it is testable and
 *  so the caller controls how often "today" is re-evaluated (once a minute, not per render). */
export function buildYearModel(days: YearDay[], now: number): DayModel[][] {
  const months: DayModel[][] = Array.from({ length: 12 }, () => []);
  for (const day of days) {
    const date = new Date(day.day_start * 1000);
    const state = dayState(day, now);
    months[date.getMonth()].push({
      dayStart: day.day_start,
      month: date.getMonth(),
      dow: date.getDay(),
      state,
      label: dayLabel(day, state, DATE_FMT.format(date)),
      heightPct: Math.min(day.active_secs / YEAR_CEILING_SECS, 1) * 100,
      // Slivers under 2% would render as sub-pixel noise on a 34px row.
      segments: day.categories
        .filter((c) => c.pct >= 2)
        .map((c) => ({
          key: c.slug || c.name,
          pct: c.pct,
          color: categoryColorVar(c.slug, c.color),
        })),
    });
  }
  return months;
}
