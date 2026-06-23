/** Local-day helpers for the dial. A "day" is the local calendar day (the day-start
 *  offset for night owls is deferred, D14). */

/** Unix-second bounds [start, end] of the local calendar day containing `date`. */
export function dayBounds(date: Date): { start: number; end: number } {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const end = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59);
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
