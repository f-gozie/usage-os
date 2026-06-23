import { Button } from "./Button";

export interface DegradedBannerProps {
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}

/** Shown when capture is running degraded (e.g. Accessibility not granted). Honest
 *  about what's missing, in plain language (no permission jargon). */
export function DegradedBanner({ title, description, actionLabel, onAction }: DegradedBannerProps) {
  return (
    <div className="flex items-center gap-3.5 border-2 border-c-research bg-surface px-4 py-3.5">
      <span className="h-2.5 w-2.5 flex-shrink-0 bg-c-research" />
      <div className="flex-1">
        <div className="text-sm font-semibold">{title}</div>
        <div className="text-xs text-muted">{description}</div>
      </div>
      {actionLabel && onAction && (
        <Button variant="secondary" onClick={onAction}>
          {actionLabel}
        </Button>
      )}
    </div>
  );
}
