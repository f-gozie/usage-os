import { useEffect, useMemo, useRef, useState } from "react";

import { contextColorVar } from "@/lib/contexts";
import { formatDuration } from "@/lib/format";
import { arcPath, DIAL_CENTER, DIAL_VIEWBOX, minutesSinceMidnight, polar } from "@/lib/geometry";
import { summarizeRun } from "@/lib/runs";
import type { ContextRun } from "@/lib/tauri";
import { useReducedMotion } from "@/hooks/useReducedMotion";

const TRACK_RADIUS = 99;
const OUTER_RADIUS = 110;
const INNER_RADIUS = 89;
const NUMERAL_RADIUS = 128;
const ARC_GAP_MIN = 4; // visual breathing room trimmed from each run end
const HOUR_MARKS = [0, 6, 12, 18];

interface Arc {
  run: ContextRun;
  d: string;
  color: string;
}

export interface DialProps {
  runs: ContextRun[];
  /** Local midnight (Unix secs) of the viewed day — the angular origin. */
  dayStartUnix: number;
  /** Current time in minutes past midnight, or null when not viewing today. */
  nowMinutes: number | null;
  /** Centre figure (e.g. "4h 15m"). */
  activeLabel: string;
  /** Dim everything except this context (legend/ledger isolate). */
  isolatedSlug?: string | null;
  onSelectRun?: (run: ContextRun) => void;
}

interface Tip {
  x: number;
  y: number;
  title: string;
  sub: string;
}

/** The 24-hour day dial: context-run arcs with a 1px casing (R77), hour ticks, the
 *  now-triangle, and a centre figure. Hover dims the rest + shows a tooltip; clicking
 *  selects the run. One orchestrated draw-in on load (skipped under reduced motion). */
export function Dial({
  runs,
  dayStartUnix,
  nowMinutes,
  activeLabel,
  isolatedSlug = null,
  onSelectRun,
}: DialProps) {
  const reducedMotion = useReducedMotion();
  const containerRef = useRef<HTMLDivElement>(null);
  const pathRefs = useRef<Array<SVGPathElement | null>>([]);
  const [hovered, setHovered] = useState<number | null>(null);
  const [tip, setTip] = useState<Tip | null>(null);

  const arcs = useMemo<Arc[]>(() => {
    return runs.map((run) => {
      const startMin = minutesSinceMidnight(run.start, dayStartUnix);
      const endMin = minutesSinceMidnight(run.end, dayStartUnix);
      const trim = endMin - startMin > ARC_GAP_MIN * 2;
      const a = trim ? startMin + ARC_GAP_MIN : startMin;
      const b = trim ? endMin - ARC_GAP_MIN : endMin;
      return {
        run,
        d: arcPath(DIAL_CENTER, DIAL_CENTER, TRACK_RADIUS, a, b),
        color: contextColorVar(run.context_slug),
      };
    });
  }, [runs, dayStartUnix]);

  // One orchestrated draw-in: each arc strokes itself on, staggered.
  useEffect(() => {
    if (reducedMotion) return;
    const paths = pathRefs.current;
    paths.forEach((p) => {
      if (!p) return;
      const len = p.getTotalLength();
      p.style.transition = "none";
      p.style.strokeDasharray = String(len);
      p.style.strokeDashoffset = String(len);
    });
    const raf = requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        paths.forEach((p, i) => {
          if (!p) return;
          p.style.transition = `stroke-dashoffset .5s cubic-bezier(.3,.9,.3,1) ${(0.06 * i).toFixed(2)}s, opacity .18s, stroke-width .15s`;
          p.style.strokeDashoffset = "0";
        });
      }),
    );
    return () => cancelAnimationFrame(raf);
  }, [arcs, reducedMotion]);

  function opacityFor(index: number, slug: string): number {
    if (isolatedSlug && slug !== isolatedSlug) return 0.16;
    if (hovered !== null && hovered !== index) return 0.2;
    return 1;
  }

  function onArcMove(e: React.MouseEvent, arc: Arc) {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const sm = summarizeRun(arc.run);
    setTip({
      x: Math.min(rect.width - 150, Math.max(0, e.clientX - rect.left + 12)),
      y: e.clientY - rect.top - 8,
      title: `${arc.run.context_name} · ${formatDuration(arc.run.secs)}`,
      sub: sm.projectLabel || sm.apps,
    });
  }

  const nowTriangle = nowMinutes === null ? null : trianglePoints(nowMinutes);

  return (
    <div ref={containerRef} className="relative mx-auto w-full max-w-[330px]">
      <svg
        viewBox={`0 0 ${DIAL_VIEWBOX} ${DIAL_VIEWBOX}`}
        role="img"
        aria-label="24-hour activity dial"
        className="block w-full"
        style={{ overflow: "visible" }}
      >
        {/* idle track + edge rings */}
        <circle cx={DIAL_CENTER} cy={DIAL_CENTER} r={TRACK_RADIUS} fill="none" stroke="var(--track)" strokeWidth={20} />
        <circle cx={DIAL_CENTER} cy={DIAL_CENTER} r={OUTER_RADIUS} fill="none" stroke="var(--edge)" strokeWidth={2} />
        <circle cx={DIAL_CENTER} cy={DIAL_CENTER} r={INNER_RADIUS} fill="none" stroke="var(--edge)" strokeWidth={2} />

        {/* hour spokes + numerals at 0/6/12/18 */}
        {HOUR_MARKS.map((h) => {
          const t = h * 60;
          const [xi, yi] = polar(DIAL_CENTER, DIAL_CENTER, t, INNER_RADIUS);
          const [xo, yo] = polar(DIAL_CENTER, DIAL_CENTER, t, OUTER_RADIUS);
          const [xn, yn] = polar(DIAL_CENTER, DIAL_CENTER, t, NUMERAL_RADIUS);
          return (
            <g key={h}>
              <line x1={xi} y1={yi} x2={xo} y2={yo} stroke="var(--edge)" strokeWidth={2} />
              <text
                x={xn}
                y={yn + 5}
                textAnchor="middle"
                fontSize={14}
                fill="var(--muted)"
                style={{ fontFamily: "Anton, sans-serif" }}
              >
                {h === 0 ? "24" : h}
              </text>
            </g>
          );
        })}

        {/* context-run arcs: a 1px ink casing (R77) behind the coloured stroke */}
        {arcs.map((arc, i) => (
          <g key={`${arc.run.start}-${i}`}>
            <path d={arc.d} stroke="var(--casing)" strokeWidth={21} fill="none" />
            <path
              ref={(el) => {
                pathRefs.current[i] = el;
              }}
              d={arc.d}
              stroke={arc.color}
              strokeWidth={hovered === i ? 24 : 18}
              fill="none"
              style={{ opacity: opacityFor(i, arc.run.context_slug), cursor: "pointer" }}
              onMouseEnter={(e) => {
                setHovered(i);
                onArcMove(e, arc);
              }}
              onMouseMove={(e) => onArcMove(e, arc)}
              onMouseLeave={() => {
                setHovered(null);
                setTip(null);
              }}
              onClick={() => onSelectRun?.(arc.run)}
            />
          </g>
        ))}

        {/* now marker */}
        {nowTriangle && <polygon points={nowTriangle} fill="var(--now)" />}
      </svg>

      {/* centre figure */}
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
        <div className="font-display text-[42px] leading-[0.82]">{activeLabel}</div>
        <div className="mt-[5px] text-[10px] font-semibold uppercase tracking-[0.2em] text-muted">Active</div>
      </div>

      {/* hover tooltip */}
      {tip && (
        <div
          className="pointer-events-none absolute z-10 whitespace-nowrap bg-edge px-[11px] py-2 text-[11.5px] font-medium leading-[1.45] text-bg"
          style={{ left: tip.x, top: tip.y }}
        >
          <span className="block font-semibold">{tip.title}</span>
          {tip.sub && <span className="mt-0.5 block text-[10.5px] text-c-comms">{tip.sub}</span>}
        </div>
      )}
    </div>
  );
}

function trianglePoints(nowMin: number): string {
  const [ax, ay] = polar(DIAL_CENTER, DIAL_CENTER, nowMin, 112);
  const [bx, by] = polar(DIAL_CENTER, DIAL_CENTER, nowMin - 9, 126);
  const [cx, cy] = polar(DIAL_CENTER, DIAL_CENTER, nowMin + 9, 126);
  return `${ax.toFixed(1)},${ay.toFixed(1)} ${bx.toFixed(1)},${by.toFixed(1)} ${cx.toFixed(1)},${cy.toFixed(1)}`;
}
