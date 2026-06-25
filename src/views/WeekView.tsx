import { useMemo } from "react";

import { DateStepper } from "@/components/common/DateStepper";
import { ErrorState } from "@/components/common/ErrorState";
import { MiniDial } from "@/components/dial/MiniDial";
import { DegradedBanner } from "@/components/ui/DegradedBanner";
import { Skeleton } from "@/components/ui/Skeleton";
import { StatTile } from "@/components/ui/StatTile";
import { useCaptureHealth } from "@/hooks/useCaptureHealth";
import { useWeekData } from "@/hooks/useWeekData";
import { addDays, dayBounds, formatWeekRange, isSameDay, weekDays } from "@/lib/dates";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

export interface WeekViewProps {
  /** Any date within the displayed week. */
  date: Date;
  /** Move to another week (prev/next). */
  onDateChange: (date: Date) => void;
  /** Open one day in the Day view. */
  onOpenDay: (date: Date) => void;
}

export function WeekView({ date, onDateChange, onOpenDay }: WeekViewProps) {
  const days = useMemo(() => weekDays(date), [date]);
  const dayStarts = useMemo(() => days.map((d) => dayBounds(d).start), [days]);
  const weekEnd = useMemo(() => dayBounds(days[6]).end, [days]);
  const today = new Date();
  const isCurrentWeek = days.some((d) => isSameDay(d, today));

  const { data, loading, error, refresh } = useWeekData(dayStarts, weekEnd, isCurrentWeek);
  const { healthy: captureHealthy, refetch: recheckHealth } = useCaptureHealth([dayStarts]);

  const retry = () => {
    refresh();
    recheckHealth();
  };

  const deepestLabel =
    data?.deepest_day != null
      ? days[data.deepest_day].toLocaleDateString(undefined, { weekday: "short" })
      : "—";

  return (
    <div>
      <DateStepper
        title={isCurrentWeek ? "This week" : formatWeekRange(date)}
        atLatest={isCurrentWeek}
        onPrev={() => onDateChange(addDays(date, -7))}
        onNext={() => onDateChange(addDays(date, 7))}
        onRefresh={refresh}
        prevLabel="Previous week"
        nextLabel="Next week"
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
          <div className="mb-6 flex border-[3px] border-edge">
            <StatTile value={formatDuration(data.total_active_secs)} label="Active this week" className="px-[18px]" />
            <StatTile
              value={formatDuration(data.avg_active_secs)}
              label="Avg / day"
              className="border-l-2 border-edge px-[18px]"
            />
            <StatTile
              value={deepestLabel}
              label="Deepest day"
              colorVar="var(--c-deep)"
              className="border-l-2 border-edge px-[18px]"
            />
          </div>

          <div className="mb-3 flex items-center gap-2.5 font-display text-base uppercase tracking-[0.04em]">
            This week
            <span className="h-0.5 flex-1 bg-edge" />
          </div>

          <div className="grid grid-cols-7 gap-2.5">
            {days.map((d, i) => {
              const slice = data.days[i];
              const isToday = isSameDay(d, today);
              const nowMinutes = isToday ? (Date.now() / 1000 - dayStarts[i]) / 60 : null;
              const label = `${d.toLocaleDateString(undefined, { weekday: "short" })} ${d.getDate()}`;
              return (
                <button
                  key={dayStarts[i]}
                  type="button"
                  onClick={() => onOpenDay(d)}
                  className={cn(
                    "group border-2 border-transparent px-[5px] py-2.5 text-center transition-colors hover:border-edge hover:bg-surface",
                    isToday && "border-edge bg-surface",
                  )}
                >
                  <div className="transition-transform duration-200 group-hover:scale-105">
                    <MiniDial
                      runs={slice?.runs ?? []}
                      dayStartUnix={dayStarts[i]}
                      nowMinutes={nowMinutes}
                      label={label}
                    />
                  </div>
                  <div
                    className={cn(
                      "mt-2.5 text-[10px] font-semibold uppercase tracking-[0.06em] text-muted",
                      isToday && "text-c-research",
                    )}
                  >
                    {label}
                  </div>
                  <div className="mt-0.5 font-display text-base">
                    {formatDuration(slice?.active_secs ?? 0)}
                  </div>
                </button>
              );
            })}
          </div>

          <p className="mt-[18px] text-center text-[11px] font-semibold uppercase tracking-[0.06em] text-muted">
            Click a day to open it ↑
          </p>
        </>
      ) : null}
    </div>
  );
}

function LoadingState() {
  return (
    <div>
      <Skeleton className="mb-6 h-[78px] w-full" />
      <Skeleton className="mb-3 h-5 w-40" />
      <div className="grid grid-cols-7 gap-2.5">
        {Array.from({ length: 7 }, (_, i) => (
          <Skeleton key={i} className="aspect-[3/4] w-full" />
        ))}
      </div>
    </div>
  );
}
