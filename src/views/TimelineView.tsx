import { Fragment, useEffect, useMemo, useState } from "react";

import { TimelineRow } from "@/components/timeline/TimelineRow";
import { DegradedBanner } from "@/components/ui/DegradedBanner";
import { Skeleton } from "@/components/ui/Skeleton";
import { useTimelineData } from "@/hooks/useTimelineData";
import { CANONICAL_CONTEXTS, contextColorVar } from "@/lib/contexts";
import { addDays, dayBounds, isSameDay } from "@/lib/dates";
import { formatClock, formatDuration } from "@/lib/format";
import { getWatcherStatus } from "@/lib/tauri";

// A gap between runs at least this long shows an "Away" marker. Matches the backend's
// run-split threshold (rollup IDLE_GAP_ENDS_RUN_SECS) — D34a dogfood-tunable.
const AWAY_MIN_SECS = 5 * 60;

export interface TimelineViewProps {
  date: Date;
  onDateChange: (date: Date) => void;
}

export function TimelineView({ date, onDateChange }: TimelineViewProps) {
  const { start, end } = useMemo(() => dayBounds(date), [date]);
  const isToday = isSameDay(date, new Date());
  const { data, loading, error, refresh } = useTimelineData(start, end, isToday);
  const [captureHealthy, setCaptureHealthy] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void getWatcherStatus()
      .then((s) => !cancelled && setCaptureHealthy(s.healthy))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [start]);

  return (
    <div>
      <TimelineNav date={date} isToday={isToday} onDateChange={onDateChange} onRefresh={refresh} />

      {!captureHealthy && (
        <div className="mb-5">
          <DegradedBanner
            title="Tracking hit a snag"
            description="UsageOS ran into repeated errors while recording. Your existing data is safe."
            actionLabel="Retry"
            onAction={refresh}
          />
        </div>
      )}

      {error ? (
        <ErrorState message={error} onRetry={refresh} />
      ) : loading && !data ? (
        <LoadingState />
      ) : data ? (
        <>
          <Legend />
          {data.runs.length === 0 ? (
            <EmptyState />
          ) : (
            <>
              <div>
                {data.runs.map((run, i) => {
                  const gap = i > 0 ? run.start - data.runs[i - 1].end : 0;
                  return (
                    <Fragment key={run.start}>
                      {gap >= AWAY_MIN_SECS && <AwayRow secs={gap} />}
                      <TimelineRow run={run} />
                    </Fragment>
                  );
                })}
                {isToday && <NowRow />}
              </div>
              <p className="mt-4 text-center text-[11px] font-medium text-muted">
                Each block is a stretch of one kind of work. Click one to see every app switch inside it.
              </p>
            </>
          )}
        </>
      ) : null}
    </div>
  );
}

function Legend() {
  return (
    <div className="mb-3.5 flex flex-wrap gap-2">
      {CANONICAL_CONTEXTS.map((c) => (
        <span
          key={c.slug}
          className="flex items-center gap-[7px] border-2 border-edge px-[9px] py-[5px] text-[10.5px] font-semibold uppercase tracking-[0.03em]"
        >
          <span
            className="h-[11px] w-[11px] border border-edge"
            style={{ background: contextColorVar(c.slug) }}
          />
          {c.name}
        </span>
      ))}
    </div>
  );
}

function AwayRow({ secs }: { secs: number }) {
  return (
    <div className="-mx-2.5 grid grid-cols-[50px_1fr] items-center gap-4 border-t border-rule px-2.5 py-[9px]">
      <div />
      <div className="flex items-center gap-3 text-[10.5px] font-semibold uppercase tracking-[0.16em] text-muted">
        <span>Away · {formatDuration(secs)}</span>
        <span className="flex-1 border-t-2 border-dotted border-rule" />
      </div>
    </div>
  );
}

function NowRow() {
  return (
    <div className="-mx-2.5 grid grid-cols-[50px_1fr] items-center gap-4 border-t border-rule px-2.5 pb-0.5 pt-3">
      <div className="text-right text-[13px] font-semibold tabular-nums text-c-research">
        {formatClock(Date.now() / 1000)}
      </div>
      <div className="flex items-center gap-3 text-[11px] font-semibold uppercase tracking-[0.14em]">
        <span
          className="h-0 w-0 border-y-[5px] border-l-[8px] border-y-transparent"
          style={{ borderLeftColor: "var(--now)" }}
        />
        Now
        <span className="h-0.5 flex-1" style={{ background: "var(--now)" }} />
      </div>
    </div>
  );
}

function TimelineNav({
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

function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-2 border-2 border-dashed border-edge px-6 py-16 text-center">
      <p className="text-sm font-medium text-muted">No activity tracked for this day.</p>
    </div>
  );
}

function LoadingState() {
  return (
    <div>
      <Skeleton className="mb-3.5 h-8 w-72" />
      <div className="space-y-2">
        {Array.from({ length: 5 }, (_, i) => (
          <Skeleton key={i} className="h-[72px] w-full" />
        ))}
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
