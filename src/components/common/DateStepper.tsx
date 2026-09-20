export interface DateStepperProps {
  /** Headline for the current range, e.g. "Today" or "This week". */
  title: string;
  /** Disable the "next" and "jump to latest" buttons when already at the latest range. */
  atLatest: boolean;
  onPrev: () => void;
  onNext: () => void;
  onRefresh: () => void;
  /** Jump straight back to the latest range. */
  onLatest: () => void;
  /** Label for the jump-to-latest button ("Today" vs "This week"). */
  latestLabel: string;
  /** Accessible labels for the prev/next buttons ("Previous day" vs "Previous week"). */
  prevLabel: string;
  nextLabel: string;
  /** Optional view-level control rendered at the start of the button row (the Calendar's layout
   *  picker). Separated from the date controls by a hairline so the two never read as one group. */
  leading?: React.ReactNode;
}

const BTN =
  "flex h-[34px] items-center justify-center border-2 border-edge bg-bg font-bold text-fg";
/** The square icon buttons (refresh, prev, next); the latest button sizes to its label. */
const ICON_BTN = `w-9 ${BTN}`;

/** The date navigator shared by the Day, Week and Timeline views: title on the left, a
 *  jump-to-latest button, a refresh button and prev/next steppers on the right.
 *
 *  The app runs for weeks at a time and the viewed date is session state, so wandering
 *  into the past and coming back later used to mean stepping day-by-day home. The latest
 *  button is always rendered and merely disabled at the latest range — same as "next" —
 *  so the control row never changes width. */
export function DateStepper({
  title,
  atLatest,
  onPrev,
  onNext,
  onRefresh,
  onLatest,
  latestLabel,
  prevLabel,
  nextLabel,
  leading,
}: DateStepperProps) {
  return (
    <div className="mb-[18px] flex items-center justify-between">
      <div className="font-display text-[22px] uppercase tracking-[0.02em]">{title}</div>
      <div className="flex items-center gap-2">
        {leading}
        {leading && <span aria-hidden="true" className="mx-1 h-[22px] w-0.5 bg-rule" />}
        <button
          type="button"
          // The accessible name keeps the visible label inside it (WCAG 2.5.3).
          aria-label={`Go to ${latestLabel.toLowerCase()}`}
          disabled={atLatest}
          onClick={onLatest}
          className={`px-3 text-[11px] font-semibold uppercase tracking-[0.14em] ${BTN} disabled:opacity-30`}
        >
          {latestLabel}
        </button>
        <button
          type="button"
          aria-label="Refresh"
          title="Refresh (updates automatically every 30s)"
          onClick={onRefresh}
          className={`mr-1 text-sm ${ICON_BTN}`}
        >
          ↻
        </button>
        <button type="button" aria-label={prevLabel} onClick={onPrev} className={`text-base ${ICON_BTN}`}>
          ‹
        </button>
        <button
          type="button"
          aria-label={nextLabel}
          disabled={atLatest}
          onClick={onNext}
          className={`text-base ${ICON_BTN} disabled:opacity-30`}
        >
          ›
        </button>
      </div>
    </div>
  );
}
