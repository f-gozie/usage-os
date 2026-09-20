import { categoryColorVar } from "@/lib/categories";
import { formatDuration } from "@/lib/format";
import type { CategorySlice, YearDay } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** How many hours fill a bar. Fixed, not scaled to the busiest day: a bar has to mean the same
 *  thing in January as in July, or the year quietly flatters every year equally. */
export const YEAR_CEILING_SECS = 12 * 3600;

/** A day is drawn as "not recorded" only when essentially none of it was watched — the same
 *  threshold the recap uses to decide a day has no story rather than a partial one. */
const UNRECORDED_SECS = 23 * 3600;

export type DayState = "recorded" | "unrecorded" | "unknown" | "future";

/** What a cell is allowed to claim about a day, before anything is drawn. */
export function dayState(day: YearDay, now: number): DayState {
  if (day.day_start > now) return "future";
  if (!day.coverage_known) return "unknown";
  if (day.unwatched_secs >= UNRECORDED_SECS) return "unrecorded";
  return "recorded";
}

/** Plain-words description of a day, for the title and the accessible name. */
export function dayLabel(day: YearDay, state: DayState, dateText: string): string {
  switch (state) {
    case "future":
      return dateText;
    case "unknown":
      return `${dateText} · before UsageOS was recording`;
    case "unrecorded":
      return `${dateText} · UsageOS wasn’t running`;
    default:
      return day.active_secs > 0
        ? `${dateText} · ${formatDuration(day.active_secs)}`
        : `${dateText} · nothing tracked`;
  }
}

/** The stacked bar for one day: height is hours, segments are categories.
 *
 *  Height is geometry, not a second colour channel — which is what keeps this from becoming a
 *  contribution graph. Colour still encodes exactly one thing (design-system rule 1). */
function Stack({ day, state }: { day: YearDay; state: DayState }) {
  if (state === "future") return null;

  if (state === "unrecorded") {
    // A hollow slot at full height — the opposite mark from a short bar, because it is the
    // opposite fact. Never red, never an icon: it is a statement about the app, not about you.
    return (
      <span className="block h-full w-full border border-dashed border-muted opacity-60" />
    );
  }

  if (day.active_secs === 0) {
    // Watched, and you simply didn't touch it. A stub on the baseline, not an empty cell.
    return <span className="block h-px w-full bg-track" />;
  }

  const height = Math.min(day.active_secs / YEAR_CEILING_SECS, 1) * 100;
  return (
    <span
      className="flex w-full flex-col-reverse border border-casing"
      style={{ height: `${height}%` }}
    >
      {day.categories
        .filter((c: CategorySlice) => c.pct >= 2)
        .map((c: CategorySlice) => (
          <span
            key={c.slug || c.name}
            className="block w-full"
            style={{
              height: `${c.pct}%`,
              background: categoryColorVar(c.slug, c.color),
            }}
          />
        ))}
    </span>
  );
}

export interface YearCellProps {
  day: YearDay;
  now: number;
  dateText: string;
  barWidthClass: string;
  onOpen: (dayStart: number) => void;
}

/** One day, in either layout: a bar sitting on a baseline inside a fixed-height slot. */
export function YearCell({ day, now, dateText, barWidthClass, onOpen }: YearCellProps) {
  const state = dayState(day, now);
  const label = dayLabel(day, state, dateText);
  const interactive = state !== "future";

  return (
    <button
      type="button"
      disabled={!interactive}
      title={label}
      aria-label={label}
      onClick={() => onOpen(day.day_start)}
      className={cn(
        "flex h-full items-end justify-center p-px",
        interactive ? "cursor-pointer" : "cursor-default",
        state === "unrecorded" && "bg-[repeating-linear-gradient(45deg,transparent_0_3px,var(--rule)_3px_4px)]",
      )}
    >
      <span className={cn("flex h-full flex-col justify-end", barWidthClass)}>
        <Stack day={day} state={state} />
      </span>
    </button>
  );
}
