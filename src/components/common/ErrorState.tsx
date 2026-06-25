export interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

/** The shared "couldn't load this" panel used by the Day/Week/Timeline views. */
export function ErrorState({ message, onRetry }: ErrorStateProps) {
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
