export interface DateStepperProps {
  /** Headline for the current range, e.g. "Today" or "This week". */
  title: string;
  /** Disable the "next" button when already at the latest range. */
  atLatest: boolean;
  onPrev: () => void;
  onNext: () => void;
  onRefresh: () => void;
  /** Accessible labels for the prev/next buttons ("Previous day" vs "Previous week"). */
  prevLabel: string;
  nextLabel: string;
}

const BTN =
  "flex h-[34px] w-9 items-center justify-center border-2 border-edge bg-bg font-bold text-fg";

/** The date navigator shared by the Day, Week and Timeline views: title on the left, a
 *  refresh button and prev/next steppers on the right. */
export function DateStepper({
  title,
  atLatest,
  onPrev,
  onNext,
  onRefresh,
  prevLabel,
  nextLabel,
}: DateStepperProps) {
  return (
    <div className="mb-[18px] flex items-center justify-between">
      <div className="font-display text-[22px] uppercase tracking-[0.02em]">{title}</div>
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label="Refresh"
          title="Refresh (updates automatically every 30s)"
          onClick={onRefresh}
          className={`mr-1 text-sm ${BTN}`}
        >
          ↻
        </button>
        <button type="button" aria-label={prevLabel} onClick={onPrev} className={`text-base ${BTN}`}>
          ‹
        </button>
        <button
          type="button"
          aria-label={nextLabel}
          disabled={atLatest}
          onClick={onNext}
          className={`text-base ${BTN} disabled:opacity-30`}
        >
          ›
        </button>
      </div>
    </div>
  );
}
