/** Local-day helpers for the dial. A "day" is the local calendar day (the day-start
 *  offset for night owls is deferred, D14). */

/** Unix-second half-open bounds [start, end) of the local calendar day containing `date`:
 *  this midnight up to (not including) the next. Matches the backend's half-open window
 *  and the Week path, so the final second isn't dropped. DST-correct — both ends are real
 *  local midnights (the `Date` constructor normalizes day+1 across month/DST boundaries). */
export function dayBounds(date: Date): { start: number; end: number } {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const end = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);
  return { start: Math.floor(start.getTime() / 1000), end: Math.floor(end.getTime() / 1000) };
}

export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

export function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

/** The Sunday at the start of the local week containing `date` (the Week view is Sun→Sat). */
export function startOfWeek(date: Date): Date {
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  d.setDate(d.getDate() - d.getDay()); // getDay(): 0 = Sunday
  return d;
}

/** The 7 local dates (Sun→Sat) of the week containing `date`. */
export function weekDays(date: Date): Date[] {
  const start = startOfWeek(date);
  return Array.from({ length: 7 }, (_, i) => addDays(start, i));
}

/** Human range for the week containing `date`: "21–27 June 2026", or
 *  "29 June – 5 July 2026" when it spans two months/years. */
export function formatWeekRange(date: Date): string {
  const days = weekDays(date);
  const a = days[0];
  const b = days[6];
  const month = (d: Date) => d.toLocaleDateString(undefined, { month: "long" });
  if (a.getMonth() === b.getMonth() && a.getFullYear() === b.getFullYear()) {
    return `${a.getDate()}–${b.getDate()} ${month(b)} ${b.getFullYear()}`;
  }
  const aYear = a.getFullYear() === b.getFullYear() ? "" : ` ${a.getFullYear()}`;
  return `${a.getDate()} ${month(a)}${aYear} – ${b.getDate()} ${month(b)} ${b.getFullYear()}`;
}

/** "Saturday" / "21 June 2026" parts for the header. */
export function formatDayParts(date: Date): { weekday: string; full: string } {
  return {
    weekday: date.toLocaleDateString(undefined, { weekday: "long" }),
    full: date.toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" }),
  };
}

/** Local midnight on the first of `date`'s month. */
export function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

/** Every local date in `date`'s month, 1st to last. */
export function monthDays(date: Date): Date[] {
  const year = date.getFullYear();
  const month = date.getMonth();
  // Day 0 of the next month is the last day of this one — correct for every month and leap year.
  const count = new Date(year, month + 1, 0).getDate();
  return Array.from({ length: count }, (_, i) => new Date(year, month, i + 1));
}

/** Every local date in `date`'s year — 365 or 366 of them. */
export function yearDays(date: Date): Date[] {
  const year = date.getFullYear();
  const days: Date[] = [];
  for (let month = 0; month < 12; month++) {
    const count = new Date(year, month + 1, 0).getDate();
    for (let d = 1; d <= count; d++) days.push(new Date(year, month, d));
  }
  return days;
}

/** Step whole months, clamping the day so 31 Jan → 28/29 Feb rather than overflowing to March. */
export function addMonths(date: Date, months: number): Date {
  const target = new Date(date.getFullYear(), date.getMonth() + months, 1);
  const lastDay = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate();
  return new Date(target.getFullYear(), target.getMonth(), Math.min(date.getDate(), lastDay));
}

/** Step whole years, clamping 29 Feb to the 28th in a common year. */
export function addYears(date: Date, years: number): Date {
  const year = date.getFullYear() + years;
  const lastDay = new Date(year, date.getMonth() + 1, 0).getDate();
  return new Date(year, date.getMonth(), Math.min(date.getDate(), lastDay));
}

/** True when both dates fall in the same local month. */
export function isSameMonth(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

/** "September 2026". */
export function formatMonthYear(date: Date): string {
  return date.toLocaleDateString(undefined, { month: "long", year: "numeric" });
}
