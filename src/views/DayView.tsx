import { useCallback, useEffect, useMemo, useState } from "react";

import { Chip } from "@/components/ui/Chip";
import { DegradedBanner } from "@/components/ui/DegradedBanner";
import { DetailInspector, type InspectorDetail } from "@/components/ui/DetailInspector";
import { LedgerRow } from "@/components/ui/LedgerRow";
import { RecapCard } from "@/components/ui/RecapCard";
import { Skeleton } from "@/components/ui/Skeleton";
import { StatTile } from "@/components/ui/StatTile";
import { Dial } from "@/components/dial/Dial";
import { useDayData } from "@/hooks/useDayData";
import { useRecap } from "@/hooks/useRecap";
import { CANONICAL_CATEGORIES, categoryColorVar } from "@/lib/categories";
import { addDays, dayBounds, isSameDay } from "@/lib/dates";
import { formatClock, formatDuration } from "@/lib/format";
import { summarizeRun } from "@/lib/runs";
import { getWatcherStatus, type CategoryRun } from "@/lib/tauri";

export interface DayViewProps {
  date: Date;
  onDateChange: (date: Date) => void;
}

export function DayView({ date, onDateChange }: DayViewProps) {
  const { start, end } = useMemo(() => dayBounds(date), [date]);
  const isToday = isSameDay(date, new Date());
  const nowMinutes = isToday ? (Date.now() / 1000 - start) / 60 : null;

  const { data, loading, error, refresh } = useDayData(start, end, isToday);
  // The AI recap is fetched lazily, off the day-load path (D11); the card shows the instant
  // template recap from `getDay` until this resolves, then upgrades in place.
  const { recap: aiRecap, refetch: refetchRecap } = useRecap(start, end);

  // Manual refresh re-runs both the day data and the (un-polled) AI narration.
  const handleRefresh = useCallback(() => {
    refresh();
    refetchRecap();
  }, [refresh, refetchRecap]);

  const [selectedRun, setSelectedRun] = useState<CategoryRun | null>(null);
  const [isolated, setIsolated] = useState<string | null>(null);
  const [captureHealthy, setCaptureHealthy] = useState(true);

  // Reset transient selection when the day changes.
  useEffect(() => {
    setSelectedRun(null);
    setIsolated(null);
  }, [start]);

  // Surface a degraded banner if capture is erroring (real signal from the watcher).
  useEffect(() => {
    let cancelled = false;
    void getWatcherStatus()
      .then((s) => !cancelled && setCaptureHealthy(s.healthy))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [start]);

  const deepSecs = data?.categories.find((c) => c.slug === "deep")?.secs ?? 0;
  const researchSecs = data?.categories.find((c) => c.slug === "research")?.secs ?? 0;
  const activeSecs = data?.active_secs ?? 0;
  const focusPct = activeSecs > 0 ? Math.round(((deepSecs + researchSecs) / activeSecs) * 100) : 0;
  // The day's leading category — `categories` is sorted longest-first by the rollup, so [0]
  // is the headline. Featuring whatever actually led (vs. a fixed "Deep work" that's
  // often ~0) keeps the stat honest instead of looking broken.
  const topCategory = data?.categories[0];

  const inspector: InspectorDetail | null = selectedRun
    ? buildInspector(selectedRun)
    : null;

  const toggleIsolate = (slug: string) => setIsolated((cur) => (cur === slug ? null : slug));

  return (
    <div>
      <DayNav date={date} isToday={isToday} onDateChange={onDateChange} onRefresh={handleRefresh} />

      {!captureHealthy && (
        <div className="mb-5">
          <DegradedBanner
            title="Tracking hit a snag"
            description="UsageOS ran into repeated errors while recording. Your existing data is safe."
            actionLabel="Retry"
            onAction={handleRefresh}
          />
        </div>
      )}

      {error ? (
        <ErrorState message={error} onRetry={handleRefresh} />
      ) : loading && !data ? (
        <LoadingState />
      ) : data ? (
        <>
          <div className="mb-5">
            {/* Prefer the on-device AI prose once it lands; the template (always present) is
                the instant floor until then, and the fallback if narration fails. */}
            <RecapCard
              text={(aiRecap ?? data.recap).text}
              generatedBy={(aiRecap ?? data.recap).generated_by}
            />
          </div>

          <div className="grid grid-cols-1 items-center gap-[30px] md:grid-cols-[330px_1fr]">
            <Dial
              runs={data.runs}
              dayStartUnix={start}
              nowMinutes={nowMinutes}
              activeLabel={formatDuration(activeSecs)}
              isolatedSlug={isolated}
              onSelectRun={setSelectedRun}
            />

            <div>
              <div className="flex border-t-[3px] border-edge">
                <StatTile value={formatDuration(activeSecs)} label="Active" />
                <StatTile
                  value={topCategory ? formatDuration(topCategory.secs) : "—"}
                  label={topCategory ? topCategory.name : "Top category"}
                  colorVar={topCategory ? categoryColorVar(topCategory.slug) : undefined}
                  className="border-l-2 border-edge pl-3.5"
                />
                <StatTile
                  value={`${focusPct}%`}
                  label="Focus"
                  colorVar="var(--c-research)"
                  className="border-l-2 border-edge pl-3.5"
                />
              </div>

              <div className="mt-[18px] flex flex-wrap gap-[9px]">
                {CANONICAL_CATEGORIES.map((c) => (
                  <Chip
                    key={c.slug}
                    label={c.name}
                    colorVar={categoryColorVar(c.slug)}
                    active={isolated === c.slug}
                    onClick={() => toggleIsolate(c.slug)}
                  />
                ))}
              </div>

              <DetailInspector detail={inspector} />
            </div>
          </div>

          <Ledger
            categories={data.categories}
            isolated={isolated}
            onHover={setIsolated}
            onToggle={toggleIsolate}
          />
        </>
      ) : null}
    </div>
  );
}

function DayNav({
  date,
  isToday,
  onDateChange,
  onRefresh,
}: {
  date: Date;
  isToday: boolean;
  onDateChange: (date: Date) => void;
  onRefresh: () => void;
}) {
  const label = isToday
    ? "Today"
    : date.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });
  return (
    <div className="mb-[18px] flex items-center justify-between">
      <div className="font-display text-[22px] uppercase tracking-[0.02em]">{label}</div>
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label="Refresh"
          title="Refresh (updates automatically every 30s)"
          onClick={onRefresh}
          className="mr-1 flex h-[34px] w-9 items-center justify-center border-2 border-edge bg-bg text-sm font-bold text-fg"
        >
          ↻
        </button>
        <button
          type="button"
          aria-label="Previous day"
          onClick={() => onDateChange(addDays(date, -1))}
          className="flex h-[34px] w-9 items-center justify-center border-2 border-edge bg-bg text-base font-bold text-fg"
        >
          ‹
        </button>
        <button
          type="button"
          aria-label="Next day"
          disabled={isToday}
          onClick={() => onDateChange(addDays(date, 1))}
          className="flex h-[34px] w-9 items-center justify-center border-2 border-edge bg-bg text-base font-bold text-fg disabled:opacity-30"
        >
          ›
        </button>
      </div>
    </div>
  );
}

function Ledger({
  categories,
  isolated,
  onHover,
  onToggle,
}: {
  categories: { slug: string; name: string; secs: number; pct: number }[];
  isolated: string | null;
  onHover: (slug: string | null) => void;
  onToggle: (slug: string) => void;
}) {
  const rows = categories.filter((c) => c.secs > 0);
  if (rows.length === 0) return null;
  return (
    <>
      <div className="mb-3 mt-[26px] flex items-center gap-2.5 font-display text-base uppercase tracking-[0.04em]">
        Where it went
        <span className="h-0.5 flex-1 bg-edge" />
      </div>
      <div>
        {rows.map((row) => (
          <LedgerRow
            key={row.slug}
            name={row.name}
            colorVar={categoryColorVar(row.slug)}
            durationLabel={formatDuration(row.secs)}
            pct={Math.round(row.pct)}
            dimmed={isolated !== null && isolated !== row.slug}
            onMouseEnter={() => onHover(row.slug)}
            onMouseLeave={() => onHover(isolated)}
            onClick={() => onToggle(row.slug)}
          />
        ))}
      </div>
    </>
  );
}

function buildInspector(run: CategoryRun): InspectorDetail {
  const sm = summarizeRun(run);
  const subtitle = sm.projectLabel ? `${sm.projectLabel} · ${sm.apps}` : sm.apps;
  return {
    colorVar: categoryColorVar(run.category_slug),
    title: run.category_name,
    subtitle,
    durationLabel: formatDuration(run.secs),
    rangeLabel: `${formatClock(run.start)}–${formatClock(run.end)}`,
  };
}

function LoadingState() {
  return (
    <div>
      <Skeleton className="mb-5 h-[88px] w-full" />
      <div className="grid grid-cols-1 items-center gap-[30px] md:grid-cols-[330px_1fr]">
        <Skeleton circle className="mx-auto aspect-square w-full max-w-[280px]" />
        <div className="space-y-3">
          <Skeleton className="h-[60px] w-full" />
          <Skeleton className="h-9 w-2/3" />
          <Skeleton className="h-16 w-full" />
        </div>
      </div>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="flex flex-col items-center gap-4 border-2 border-dashed border-edge px-6 py-16 text-center">
      <p className="text-sm font-medium text-muted">{message}</p>
      <button
        type="button"
        onClick={onRetry}
        className="border-2 border-edge bg-edge px-4 py-2 text-xs font-semibold uppercase tracking-[0.08em] text-bg"
      >
        Try again
      </button>
    </div>
  );
}
