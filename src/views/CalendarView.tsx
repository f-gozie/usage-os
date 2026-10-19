import { memo, useCallback, useEffect, useMemo, useState } from "react";

import { YearRows, YearWall } from "@/components/calendar/YearLayouts";
import { buildYearModel } from "@/components/calendar/yearModel";
import { DateStepper } from "@/components/common/DateStepper";
import { ErrorState } from "@/components/common/ErrorState";
import { MiniDial } from "@/components/dial/MiniDial";
import { Skeleton } from "@/components/ui/Skeleton";
import { StatTile } from "@/components/ui/StatTile";
import { useMonthData, useYearData } from "@/hooks/useCalendarData";
import { useMinuteNow } from "@/hooks/useMinuteNow";
import {
  addMonths,
  addYears,
  dayBounds,
  formatMonthYear,
  isSameDay,
  isSameMonth,
  monthDays,
  yearDays,
} from "@/lib/dates";
import { formatDuration } from "@/lib/format";
import { getSettings, updateSetting, type DaySlice } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Which rung of the calendar you're on. Day is the existing view, reached by clicking through. */
type Rung = "year" | "month";

/** The two Year layouts. Both ship; this is the user's choice, remembered. */
export type YearLayout = "rows" | "wall";
const LAYOUT_KEY = "calendar_year_layout";

/** Sunday-first, matching the shipped `weekDays()` and the Week view. */
const DOW = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** A day is "not recorded" only when essentially none of it was watched (mirrors the Rust
 *  `DAY_UNRECORDED_SECS`, which owns the number). */
const UNRECORDED_SECS = 23 * 3600;

export interface CalendarViewProps {
  date: Date;
  onDateChange: (date: Date) => void;
  /** Click a day → open it in the Day view. */
  onOpenDay: (date: Date) => void;
}

/**
 * The Calendar: Year → Month → Day, one tab, three rungs.
 *
 * The mark changes with the rung because the dial does not survive the shrink — its R77 ink
 * casing is 1.25% of rendered size, so it is a true 1px only at an 80px dial. Month cells are
 * big enough for that; a year cell is ~29px, so Year drops "when" rather than drawing it
 * illegibly, and uses a stacked bar whose *height* carries the hours. Colour still encodes
 * exactly one thing at both rungs.
 */
export function CalendarView({ date, onDateChange, onOpenDay }: CalendarViewProps) {
  const [rung, setRung] = useState<Rung>("month");
  const [layout, setLayout] = useState<YearLayout>("wall");

  // Remembered across launches. A read failure just leaves the default — a missing preference
  // is not worth an error state.
  useEffect(() => {
    void getSettings()
      .then((rows) => {
        const stored = rows.find((r) => r.key === LAYOUT_KEY)?.value;
        if (stored === "rows" || stored === "wall") setLayout(stored);
      })
      .catch(() => undefined);
  }, []);

  const chooseLayout = useCallback((next: YearLayout) => {
    setLayout(next);
    void updateSetting(LAYOUT_KEY, next).catch(() => undefined);
  }, []);

  // Stable between minute boundaries — `Date.now()` in render would hand all 365 cells a new
  // prop on every render and defeat their memoization.
  const now = useMinuteNow();
  const today = useMemo(() => new Date(now * 1000), [now]);

  // ── Year ──────────────────────────────────────────────────────────────
  const yearStarts = useMemo(() => yearDays(date).map((d) => dayBounds(d).start), [date]);
  const yearEnd = useMemo(() => {
    const days = yearDays(date);
    return dayBounds(days[days.length - 1]).end;
  }, [date]);
  const isThisYear = date.getFullYear() === today.getFullYear();
  const year = useYearData(rung === "year" ? yearStarts : [], yearEnd);
  // All per-day derivation happens here, once per data load (or minute tick) — never in a
  // cell's render. Cells receive frozen models and memoize on identity.
  const yearModel = useMemo(
    () => (year.data ? buildYearModel(year.data.days, now) : null),
    [year.data, now],
  );
  const currentMonth = isThisYear ? today.getMonth() : -1;

  // ── Month ─────────────────────────────────────────────────────────────
  const monthStarts = useMemo(() => monthDays(date).map((d) => dayBounds(d).start), [date]);
  const monthEnd = useMemo(() => {
    const days = monthDays(date);
    return dayBounds(days[days.length - 1]).end;
  }, [date]);
  const isThisMonth = isSameMonth(date, today);
  // Live only while the month rung is actually on screen: polling a view nobody is looking at
  // would spend battery re-deriving a grid that is not rendered.
  const month = useMonthData(
    rung === "month" ? monthStarts : [],
    monthEnd,
    rung === "month" && isThisMonth,
  );

  const openMonth = useCallback(
    (dayStart: number) => {
      onDateChange(new Date(dayStart * 1000));
      setRung("month");
    },
    [onDateChange],
  );

  const openDay = useCallback(
    (dayStart: number) => onOpenDay(new Date(dayStart * 1000)),
    [onOpenDay],
  );

  const state = rung === "year" ? year : month;

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <Breadcrumb rung={rung} date={date} onYear={() => setRung("year")} />
        <RungPicker rung={rung} onChange={setRung} />
      </div>

      <DateStepper
        title={rung === "year" ? String(date.getFullYear()) : formatMonthYear(date)}
        atLatest={rung === "year" ? isThisYear : isThisMonth}
        onPrev={() => onDateChange(rung === "year" ? addYears(date, -1) : addMonths(date, -1))}
        onNext={() => onDateChange(rung === "year" ? addYears(date, 1) : addMonths(date, 1))}
        onRefresh={state.refresh}
        onLatest={() => onDateChange(new Date())}
        latestLabel={rung === "year" ? "This year" : "This month"}
        prevLabel={rung === "year" ? "Previous year" : "Previous month"}
        nextLabel={rung === "year" ? "Next year" : "Next month"}
        leading={
          rung === "year" ? <LayoutPicker layout={layout} onChange={chooseLayout} /> : undefined
        }
      />

      {state.error ? (
        <ErrorState message={state.error} onRetry={state.refresh} />
      ) : state.loading && !state.data ? (
        <Skeleton className="h-[420px] w-full" />
      ) : rung === "year" && yearModel ? (
        // Keyed so a layout switch remounts and replays the settle-in, rather than snapping.
        layout === "rows" ? (
          <YearRows
            key="rows"
            months={yearModel}
            currentMonth={currentMonth}
            onOpenMonth={openMonth}
            // One rung at a time: a bar in the Year opens its Month, and the Month opens the Day.
            // Jumping from a 13px bar straight to a Day skipped the rung the ladder is named for.
            onOpenDay={openMonth}
          />
        ) : (
          <YearWall
            key="wall"
            months={yearModel}
            currentMonth={currentMonth}
            onOpenMonth={openMonth}
            onOpenDay={openMonth}
          />
        )
      ) : rung === "month" && month.data ? (
        <MonthGrid
          key={monthStarts[0]}
          days={month.data.days}
          totalActive={month.data.total_active_secs}
          unrecordedDays={month.data.unrecorded_days}
          now={now}
          onOpenDay={openDay}
        />
      ) : null}

      <Legend />
    </div>
  );
}

/** The drill-down's spine: always visible, every rung above you one click away. */
function Breadcrumb({ rung, date, onYear }: { rung: Rung; date: Date; onYear: () => void }) {
  return (
    <div className="flex items-center gap-2.5 text-[11px] font-semibold uppercase tracking-[0.13em] text-muted">
      {rung === "year" ? (
        <span aria-current="page" className="border-b-2 border-edge pb-0.5 text-fg">
          {date.getFullYear()}
        </span>
      ) : (
        <>
          <button
            type="button"
            onClick={onYear}
            className="border-b-2 border-rule pb-0.5 hover:border-edge hover:text-fg"
          >
            {date.getFullYear()}
          </button>
          <span aria-hidden="true" className="opacity-50">
            →
          </span>
          <span aria-current="page" className="border-b-2 border-edge pb-0.5 text-fg">
            {date.toLocaleDateString(undefined, { month: "long" })}
          </span>
        </>
      )}
    </div>
  );
}

/** Year | Month, always visible. The breadcrumb tells you where you are; this lets you go to the
 *  other rung without first discovering that a month name is clickable. */
function RungPicker({ rung, onChange }: { rung: Rung; onChange: (next: Rung) => void }) {
  return (
    <div className="flex border-2 border-edge" role="group" aria-label="Calendar zoom">
      {(["year", "month"] as const).map((value) => (
        <button
          key={value}
          type="button"
          aria-pressed={rung === value}
          onClick={() => onChange(value)}
          className={cn(
            "cal-rung border-r-2 border-edge px-3 py-[5px] text-[10.5px] font-semibold uppercase tracking-[0.13em] last:border-r-0",
            rung === value ? "bg-bar-bg text-bar-fg" : "text-fg",
          )}
        >
          {value}
        </button>
      ))}
    </div>
  );
}

const LAYOUT_LABELS: Record<YearLayout, string> = {
  rows: "Month rows",
  wall: "Wall calendar",
};

/** Two icon buttons in the stepper's control row. Icons only, so each carries its name as an
 *  accessible label and a hover tooltip — never the tooltip alone. */
function LayoutPicker({
  layout,
  onChange,
}: {
  layout: YearLayout;
  onChange: (next: YearLayout) => void;
}) {
  return (
    <div className="flex h-[34px] border-2 border-edge" role="group" aria-label="Year layout">
      {(["rows", "wall"] as const).map((value) => (
        <button
          key={value}
          type="button"
          aria-pressed={layout === value}
          aria-label={LAYOUT_LABELS[value]}
          data-name={LAYOUT_LABELS[value]}
          onClick={() => onChange(value)}
          className={cn(
            "cal-pick flex w-9 items-center justify-center border-r-2 border-edge last:border-r-0",
            layout === value ? "bg-bar-bg text-bar-fg" : "text-fg",
          )}
        >
          {value === "rows" ? <RowsIcon /> : <WallIcon />}
        </button>
      ))}
    </div>
  );
}

function RowsIcon() {
  // Three rows of bars — the strip layout, at a size where four marks per row still read.
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
      <rect x="1" y="1.5" width="2.5" height="3.5" />
      <rect x="5" y="2.5" width="2.5" height="2.5" />
      <rect x="9" y="1" width="2.5" height="4" />
      <rect x="13" y="2.5" width="2.5" height="2.5" />
      <rect x="1" y="7" width="2.5" height="2.5" />
      <rect x="5" y="6" width="2.5" height="3.5" />
      <rect x="9" y="7.5" width="2.5" height="2" />
      <rect x="13" y="6" width="2.5" height="3.5" />
      <rect x="1" y="11" width="2.5" height="3" />
      <rect x="5" y="12" width="2.5" height="2" />
      <rect x="9" y="10.5" width="2.5" height="3.5" />
      <rect x="13" y="11.5" width="2.5" height="2.5" />
    </svg>
  );
}

function WallIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <rect x="1" y="2.5" width="14" height="12.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <line x1="1" y1="6" x2="15" y2="6" stroke="currentColor" strokeWidth="1.6" />
      <line x1="4.5" y1="1" x2="4.5" y2="3.5" stroke="currentColor" strokeWidth="1.6" />
      <line x1="11.5" y1="1" x2="11.5" y2="3.5" stroke="currentColor" strokeWidth="1.6" />
      <g fill="currentColor">
        <rect x="3.2" y="7.8" width="2" height="2" />
        <rect x="7" y="7.8" width="2" height="2" />
        <rect x="10.8" y="7.8" width="2" height="2" />
        <rect x="3.2" y="11.3" width="2" height="2" />
        <rect x="7" y="11.3" width="2" height="2" />
        <rect x="10.8" y="11.3" width="2" height="2" />
      </g>
    </svg>
  );
}

// One formatter for every month cell, for the app's lifetime — constructing one per cell per
// render is what `toLocaleDateString` does, and it is the slow part.
const MONTH_DATE_FMT = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  day: "numeric",
  month: "long",
});

/** The middle rung: a dial per day, at the size that keeps R77's casing a real pixel. */
const MonthGrid = memo(function MonthGrid({
  days,
  totalActive,
  unrecordedDays,
  now,
  onOpenDay,
}: {
  days: DaySlice[];
  totalActive: number;
  unrecordedDays: number;
  now: number;
  onOpenDay: (dayStart: number) => void;
}) {
  const pads = days.length > 0 ? new Date(days[0].day_start * 1000).getDay() : 0;

  // Everything a cell needs, derived once per data load — not inside the cell's render.
  const cells = useMemo(() => {
    const today = new Date(now * 1000);
    return days.map((day) => {
      const date = new Date(day.day_start * 1000);
      const future = day.day_start > now;
      const unrecorded = day.coverage_known && day.unwatched_secs >= UNRECORDED_SECS;
      const dateText = MONTH_DATE_FMT.format(date);
      return {
        day,
        date,
        future,
        unrecorded,
        isToday: isSameDay(date, today),
        dateText,
        label: future
          ? dateText
          : unrecorded
            ? `${dateText} · UsageOS wasn’t running`
            : `${dateText} · ${day.active_secs > 0 ? formatDuration(day.active_secs) : "nothing tracked"}`,
      };
    });
  }, [days, now]);

  // One listener for the grid; cells carry `data-day`.
  const onClick = useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      const cell = (event.target as HTMLElement).closest<HTMLElement>("[data-day]");
      if (cell?.dataset.day) onOpenDay(Number(cell.dataset.day));
    },
    [onOpenDay],
  );

  return (
    <div className="cal-view">
      <div className="mb-5 flex border-[3px] border-edge">
        <StatTile value={formatDuration(totalActive)} label="Tracked this month" className="px-[18px]" />
        <StatTile
          value={String(unrecordedDays)}
          label={unrecordedDays === 1 ? "Day not recorded" : "Days not recorded"}
          className="border-l-2 border-edge px-[18px]"
        />
      </div>

      <div className="mb-[7px] grid grid-cols-7 gap-[7px]">
        {DOW.map((d) => (
          <span
            key={d}
            className="text-center text-[10.5px] font-semibold uppercase tracking-[0.18em] text-muted"
          >
            {d}
          </span>
        ))}
      </div>

      {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- delegation target;
          every actionable descendant is a real <button>. */}
      <div className="grid grid-cols-7 gap-[7px]" onClick={onClick}>
        {Array.from({ length: pads }, (_, i) => (
          <span key={`pad-${i}`} className="min-h-[130px] border-2 border-rule" />
        ))}
        {cells.map((c, i) => (
          <button
            key={c.day.day_start}
            type="button"
            disabled={c.future}
            data-day={c.future ? undefined : c.day.day_start}
            aria-label={c.label}
            title={c.label}
            style={{ "--i": i } as React.CSSProperties}
            className={cn(
              "cal-daycell relative flex min-h-[130px] flex-col items-center gap-1 border-2 px-1.5 pb-2 pt-[7px]",
              c.future
                ? "cursor-default border-dashed border-rule bg-transparent"
                : "cursor-pointer border-edge bg-surface",
              c.isToday && "border-[3px]",
              c.unrecorded &&
                "bg-[repeating-linear-gradient(45deg,transparent_0_5px,var(--rule)_5px_6px)]",
            )}
          >
            <span
              className={cn(
                "absolute left-[7px] top-[5px] text-[10.5px] font-semibold leading-none",
                c.isToday ? "left-[5px] top-1 bg-fg px-1 py-0.5 text-on-ink" : "text-muted",
              )}
            >
              {c.date.getDate()}
            </span>

            {!c.future &&
              (c.unrecorded ? (
                <>
                  <span className="mt-[10px] flex h-[80px] w-[80px] items-center justify-center rounded-full border-2 border-dashed border-rule text-[15px] font-semibold text-muted">
                    —
                  </span>
                  <span className="text-center text-[9.5px] font-semibold uppercase leading-tight tracking-[0.11em] text-muted">
                    Not recorded
                  </span>
                </>
              ) : (
                <>
                  <span className="mt-[10px] w-[80px]">
                    <MiniDial
                      runs={c.day.runs}
                      dayStartUnix={c.day.day_start}
                      nowMinutes={c.isToday ? (now - c.day.day_start) / 60 : null}
                      label={c.dateText}
                    />
                  </span>
                  <span className="font-display text-[15px] leading-none">
                    {c.day.active_secs > 0 ? formatDuration(c.day.active_secs) : "—"}
                  </span>
                </>
              ))}
          </button>
        ))}
      </div>
    </div>
  );
});

/** Colour is category and nothing else, and every swatch carries its word (WCAG 1.4.1). */
function Legend() {
  const items = [
    { label: "Work", var: "var(--c-deep)" },
    { label: "Browsing", var: "var(--c-research)" },
    { label: "Messaging", var: "var(--c-comms)" },
    { label: "Entertainment", var: "var(--c-breaks)" },
    { label: "Personal", var: "var(--c-personal)" },
  ];
  return (
    <div className="mt-[18px] flex flex-wrap gap-2">
      {items.map((item) => (
        <span
          key={item.label}
          className="flex items-center gap-[7px] border-2 border-edge bg-bg px-2.5 py-[5px] text-[11.5px] font-semibold uppercase tracking-[0.1em]"
        >
          <span className="block h-2.5 w-2.5 border border-casing" style={{ background: item.var }} />
          {item.label}
        </span>
      ))}
      <span className="flex items-center gap-[7px] border-2 border-dashed border-edge bg-bg px-2.5 py-[5px] text-[11.5px] font-semibold uppercase tracking-[0.1em]">
        <span className="block h-2.5 w-2.5 border border-dashed border-muted" />
        Not recorded
      </span>
    </div>
  );
}
