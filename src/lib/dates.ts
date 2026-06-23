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

/** "Saturday" / "21 June 2026" parts for the header. */
export function formatDayParts(date: Date): { weekday: string; full: string } {
  return {
    weekday: date.toLocaleDateString(undefined, { weekday: "long" }),
    full: date.toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" }),
  };
}
