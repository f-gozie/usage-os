/**
 * Dial geometry — a fixed 24-hour polar clock, midnight at the top, clockwise (D14).
 * Ported from design/day.html so the React dial matches the frozen mockup exactly.
 * All angles are derived from minutes-since-local-midnight (0–1440).
 */

export const DIAL_VIEWBOX = 300;
export const DIAL_CENTER = 150;
export const MINUTES_PER_DAY = 1440;

/** A point on a circle of radius `r` at `minutes` past midnight (0 = top, clockwise). */
export function polar(cx: number, cy: number, minutes: number, r: number): [number, number] {
  const angle = (minutes / MINUTES_PER_DAY) * 2 * Math.PI;
  return [cx + r * Math.sin(angle), cy - r * Math.cos(angle)];
}

/** SVG path for the arc from `aMin` to `bMin` on radius `r` (clockwise sweep). */
export function arcPath(cx: number, cy: number, r: number, aMin: number, bMin: number): string {
  const largeArc = bMin - aMin > MINUTES_PER_DAY / 2 ? 1 : 0;
  const [x0, y0] = polar(cx, cy, aMin, r);
  const [x1, y1] = polar(cx, cy, bMin, r);
  return `M${x0.toFixed(2)} ${y0.toFixed(2)} A${r} ${r} 0 ${largeArc} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}

/** Minutes past local midnight for a Unix-second timestamp within the given day. */
export function minutesSinceMidnight(unixSec: number, dayStartUnix: number): number {
  return (unixSec - dayStartUnix) / 60;
}
