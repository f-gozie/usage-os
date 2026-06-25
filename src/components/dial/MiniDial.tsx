import { categoryColorVar } from "@/lib/categories";
import { arcPath, minutesSinceMidnight, polar } from "@/lib/geometry";
import type { CategoryRun } from "@/lib/tauri";

// A compact dial: same 24h polar geometry as the day Dial, sized for the Week grid.
const C = 40; // centre of the 80×80 viewBox
const R = 29; // arc/track radius
const TRACK_W = 6;
const ARC_W = 6;
const CASING_W = 8; // 1px of ink either side (R77); transparent in dark themes
const TRIM_MIN = 3; // breathing room trimmed from each run end

export interface MiniDialProps {
  runs: CategoryRun[];
  /** Local midnight (Unix secs) of this day — the angular origin. */
  dayStartUnix: number;
  /** Minutes past midnight for the now-triangle (today only); omit/null otherwise. */
  nowMinutes?: number | null;
  /** Accessible label, e.g. "Mon 16". */
  label?: string;
}

/** A small 24-hour dial for the Week grid: category-run arcs + idle track + (today) the
 *  now-triangle. No centre figure — the day label and total sit beneath it in the cell. */
export function MiniDial({ runs, dayStartUnix, nowMinutes = null, label }: MiniDialProps) {
  const arcs = runs.map((run) => {
    const startMin = minutesSinceMidnight(run.start, dayStartUnix);
    const endMin = minutesSinceMidnight(run.end, dayStartUnix);
    const trim = endMin - startMin > TRIM_MIN * 2;
    const a = trim ? startMin + TRIM_MIN : startMin;
    const b = trim ? endMin - TRIM_MIN : endMin;
    return {
      key: `${run.start}-${run.end}`,
      d: arcPath(C, C, R, a, b),
      color: categoryColorVar(run.category_slug, run.category_color),
    };
  });
  const triangle = nowMinutes == null ? null : trianglePoints(nowMinutes);

  return (
    <svg
      viewBox="0 0 80 80"
      role="img"
      aria-label={label ? `${label} activity` : "Day activity"}
      className="block w-full"
    >
      <circle cx={C} cy={C} r={R} fill="none" stroke="var(--track)" strokeWidth={TRACK_W} />
      {arcs.map((arc) => (
        <g key={arc.key}>
          <path d={arc.d} stroke="var(--casing)" strokeWidth={CASING_W} fill="none" />
          <path d={arc.d} stroke={arc.color} strokeWidth={ARC_W} fill="none" />
        </g>
      ))}
      {triangle && <polygon points={triangle} fill="var(--now)" />}
    </svg>
  );
}

function trianglePoints(nowMin: number): string {
  const [ax, ay] = polar(C, C, nowMin, 34);
  const [bx, by] = polar(C, C, nowMin - 20, 41);
  const [cx, cy] = polar(C, C, nowMin + 20, 41);
  return `${ax.toFixed(1)},${ay.toFixed(1)} ${bx.toFixed(1)},${by.toFixed(1)} ${cx.toFixed(1)},${cy.toFixed(1)}`;
}
