import { Fragment, useMemo } from "react";

import { DateStepper } from "@/components/common/DateStepper";
import { ErrorState } from "@/components/common/ErrorState";
import { TimelineRow } from "@/components/timeline/TimelineRow";
import { DegradedBanner } from "@/components/ui/DegradedBanner";
import { Skeleton } from "@/components/ui/Skeleton";
import { useCaptureHealth } from "@/hooks/useCaptureHealth";
import { useTimelineData } from "@/hooks/useTimelineData";
import { CANONICAL_CATEGORIES, categoryColorVar, categoryDisplayName } from "@/lib/categories";
import { addDays, dayBounds, isSameDay } from "@/lib/dates";
import { formatClock, formatDuration } from "@/lib/format";

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
  const { healthy: captureHealthy, refetch: recheckHealth } = useCaptureHealth([start]);

  const retry = () => {
    refresh();
    recheckHealth();
  };

  // Slug → current DB display name, so the legend reflects a renamed canonical category.
  const dbNames = useMemo(
    () => new Map((data?.runs ?? []).map((r) => [r.category_slug, r.category_name])),
    [data?.runs],
  );

  return (
    <div>
      <DateStepper
        title={
          isToday
            ? "Today"
            : date.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" })
        }
        atLatest={isToday}
        onPrev={() => onDateChange(addDays(date, -1))}
        onNext={() => onDateChange(addDays(date, 1))}
        onRefresh={refresh}
        prevLabel="Previous day"
        nextLabel="Next day"
      />

      {!captureHealthy && (
        <div className="mb-5">
          <DegradedBanner
            title="Tracking hit a snag"
            description="UsageOS ran into repeated errors while recording. Your existing data is safe."
            actionLabel="Retry"
            onAction={retry}
          />
        </div>
      )}

      {error ? (
        <ErrorState message={error} onRetry={refresh} />
      ) : loading && !data ? (
        <LoadingState />
      ) : data ? (
        <>
          <Legend dbNames={dbNames} />
          {data.runs.length === 0 ? (
            <EmptyState />
          ) : (
            <>
              <div>
                {/* Latest first: "Now" on top, then runs newest→oldest, so recent
                    activity sits at the top of the page (no scrolling to the bottom). */}
                {isToday && <NowRow />}
                {data.runs
                  .map((run, i) => ({ run, i }))
                  .reverse()
                  .map(({ run, i }) => {
                    // Gap to the chronologically-earlier run — which renders BELOW this one.
                    const gap = i > 0 ? run.start - data.runs[i - 1].end : 0;
                    return (
                      <Fragment key={run.start}>
                        <TimelineRow run={run} />
                        {gap >= AWAY_MIN_SECS && <AwayRow secs={gap} />}
                      </Fragment>
                    );
                  })}
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

function Legend({ dbNames }: { dbNames: ReadonlyMap<string, string> }) {
  return (
    <div className="mb-3.5 flex flex-wrap gap-2">
      {CANONICAL_CATEGORIES.map((c) => (
        <span
          key={c.slug}
          className="flex items-center gap-[7px] border-2 border-edge px-[9px] py-[5px] text-[10.5px] font-semibold uppercase tracking-[0.03em]"
        >
          <span
            className="h-[11px] w-[11px] border border-edge"
            style={{ background: categoryColorVar(c.slug) }}
          />
          {categoryDisplayName(c.slug, dbNames)}
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
