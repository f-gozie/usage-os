import { useEffect, useState } from "react";

import { categoryColorVar } from "@/lib/categories";
import { dayBounds } from "@/lib/dates";
import { formatDuration } from "@/lib/format";
import { getDay, quitApp, showMainWindow, type DayView } from "@/lib/tauri";
import { ThemeProvider } from "@/providers/ThemeProvider";

/**
 * The menubar tray's quick-glance popover (a separate webview at `#/glance`). A compact "today
 * so far" — a category-share donut + Active / Top / Focus + the top categories — with Open + Quit.
 * Numbers come straight from `getDay` (computed in Rust, hard rule 6); the donut is share-based
 * (not the time-positioned Day dial) so it stays clean at any granularity or day length.
 */
export function Glance() {
  return (
    <ThemeProvider>
      <GlancePanel />
    </ThemeProvider>
  );
}

function GlancePanel() {
  const [data, setData] = useState<DayView | null>(null);

  useEffect(() => {
    const load = () => {
      const { start, end } = dayBounds(new Date());
      void getDay(start, end)
        .then(setData)
        .catch(() => undefined);
    };
    load();
    // Refresh each time the popover regains focus (i.e. is reopened). Guarded for non-Tauri envs.
    let active = true;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) =>
        getCurrentWindow().onFocusChanged(({ payload: focused }) => {
          if (focused) load();
        }),
      )
      .then((stop) => {
        if (active) unlisten = stop;
        else stop();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  // The glance window is transparent (rounded corners + soft shadow), so drop the opaque
  // html/body background the main app sets. Scoped to this webview; restored on unmount.
  useEffect(() => {
    const { documentElement: html, body } = document;
    const prev = [html.style.background, body.style.background];
    html.style.background = "transparent";
    body.style.background = "transparent";
    return () => {
      html.style.background = prev[0];
      body.style.background = prev[1];
    };
  }, []);

  const now = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const active = data?.active_secs ?? 0;
  const cats = (data?.categories ?? []).filter((c) => c.secs > 0);
  const top = cats[0];
  const deep = cats.find((c) => c.slug === "deep")?.secs ?? 0;
  const research = cats.find((c) => c.slug === "research")?.secs ?? 0;
  const focusPct = active > 0 ? Math.round(((deep + research) / active) * 100) : 0;

  return (
    <div className="flex h-screen flex-col bg-transparent p-2.5 text-fg">
      <div className="flex flex-1 flex-col overflow-hidden rounded-[16px] border border-rule bg-bg shadow-[0_14px_44px_rgba(0,0,0,0.4)]">
        {/* Head */}
        <div className="flex items-center gap-2 bg-bar-bg px-[13px] py-2.5">
        <span className="font-display text-[13px] uppercase tracking-[0.05em] text-bar-fg">
          USAGE<span className="text-c-research">OS</span>
        </span>
        <span className="ml-auto flex items-center gap-1.5 text-[9.5px] font-semibold uppercase tracking-[0.12em] text-bar-fg opacity-60">
          <span className="h-[7px] w-[7px] animate-pulse rounded-full bg-c-comms" />
          Tracking
        </span>
      </div>

      <div className="flex-1 px-[15px] pb-[13px] pt-3.5">
        <div className="mb-3 flex items-baseline justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted">
            Today
          </span>
          <span className="font-display text-[13px]">
            <span className="text-c-research">▸</span> Now {now}
          </span>
        </div>

        {active === 0 ? (
          <div className="flex h-[150px] items-center justify-center px-4 text-center text-[13px] font-medium text-muted">
            No activity tracked today yet.
          </div>
        ) : (
          <>
            <div className="flex justify-center">
              <Donut categories={cats} activeSecs={active} />
            </div>

            <div className="mt-3 flex border-t-2 border-edge">
              <Kpi label="Active" value={formatDuration(active)} />
              {top && (
                <Kpi label="Top" value={top.name} swatch={categoryColorVar(top.slug, top.color)} />
              )}
              <Kpi label="Focus" value={`${focusPct}%`} valueClass="text-c-research" />
            </div>

            <div className="mt-3.5">
              <div className="mb-2 flex items-center gap-2 text-[9px] font-semibold uppercase tracking-[0.16em] text-muted">
                Where it went
                <span className="h-px flex-1 bg-rule" />
              </div>
              {cats.slice(0, 3).map((c) => (
                <div key={c.slug} className="mb-[7px] grid grid-cols-[78px_1fr_60px] items-center gap-[9px]">
                  <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-[0.02em]">
                    <span
                      className="h-[9px] w-[9px] flex-shrink-0 border border-edge"
                      style={{ background: categoryColorVar(c.slug, c.color) }}
                    />
                    {c.name}
                  </span>
                  <span className="h-[7px] bg-track">
                    <span
                      className="block h-full"
                      style={{ width: `${Math.round(c.pct)}%`, background: categoryColorVar(c.slug, c.color) }}
                    />
                  </span>
                  <span className="text-right font-display text-[13px]">
                    {Math.round(c.pct)}%
                  </span>
                </div>
              ))}
            </div>
          </>
        )}
      </div>

        {/* Foot */}
        <div className="flex items-center gap-[9px] border-t border-rule bg-surface px-[15px] py-[11px]">
          <button
            type="button"
            onClick={() => void showMainWindow().catch(() => undefined)}
            className="flex-1 rounded-[7px] bg-edge px-3.5 py-[9px] text-center text-[11px] font-semibold uppercase tracking-[0.06em] text-bg"
          >
            Open UsageOS →
          </button>
          <button
            type="button"
            onClick={() => void quitApp().catch(() => undefined)}
            className="rounded-[7px] border border-rule bg-bg px-[13px] py-[9px] text-[11px] font-semibold uppercase tracking-[0.06em] text-muted"
          >
            Quit
          </button>
        </div>
      </div>
    </div>
  );
}

function Kpi({
  label,
  value,
  swatch,
  valueClass,
}: {
  label: string;
  value: string;
  swatch?: string;
  valueClass?: string;
}) {
  return (
    <div className="flex-1 border-l border-rule px-[11px] py-2.5 first:border-l-0 first:pl-0">
      <div className={`flex items-center gap-1.5 font-display text-[18px] leading-none ${valueClass ?? ""}`}>
        {swatch && (
          <span className="h-[10px] w-[10px] flex-shrink-0 border border-edge" style={{ background: swatch }} />
        )}
        <span className="truncate">{value}</span>
      </div>
      <div className="mt-[7px] text-[8.5px] font-semibold uppercase tracking-[0.12em] text-muted">
        {label}
      </div>
    </div>
  );
}

/** Category-share donut: each arc sized by the category's share of active time. Always ≤5 clean
 *  segments, so it never crowds (unlike the time-positioned Day dial at small size). */
function Donut({
  categories,
  activeSecs,
}: {
  categories: { slug: string; color: string | null; secs: number }[];
  activeSecs: number;
}) {
  const C = 66;
  const R = 44;
  const polar = (deg: number, r: number): [number, number] => {
    const a = ((deg - 90) * Math.PI) / 180;
    return [C + r * Math.cos(a), C + r * Math.sin(a)];
  };
  const arc = (a0: number, a1: number): string => {
    const large = a1 - a0 > 180 ? 1 : 0;
    const [x0, y0] = polar(a0, R);
    const [x1, y1] = polar(a1, R);
    return `M${x0.toFixed(2)} ${y0.toFixed(2)} A${R} ${R} 0 ${large} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  };

  const gap = categories.length > 1 ? 4 : 0;
  let angle = 0;
  const arcs = categories.map((c) => {
    const sweep = (c.secs / activeSecs) * 360;
    const a0 = angle + gap / 2;
    const a1 = angle + sweep - gap / 2;
    angle += sweep;
    return { d: a1 > a0 ? arc(a0, a1) : null, color: categoryColorVar(c.slug, c.color), slug: c.slug };
  });

  return (
    <div className="relative h-[132px] w-[132px]">
      <svg viewBox="0 0 132 132" className="block w-full overflow-visible" role="img" aria-label="Category share">
        <circle cx={C} cy={C} r={R} fill="none" stroke="var(--track)" strokeWidth={11} />
        {arcs.map(
          (a) =>
            a.d && (
              <path key={a.slug} d={a.d} stroke={a.color} strokeWidth={11} fill="none" strokeLinecap="butt" />
            ),
        )}
      </svg>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
        <div className="font-display text-[25px] leading-[0.82]">{formatDuration(activeSecs)}</div>
        <div className="mt-1 text-[8px] font-semibold uppercase tracking-[0.2em] text-muted">Active</div>
      </div>
    </div>
  );
}
