import { cn } from "@/lib/utils";

export interface ChipProps {
  label: string;
  /** CSS colour for the swatch (a context token, e.g. `var(--c-deep)`). */
  colorVar: string;
  active?: boolean;
  onClick?: () => void;
}

/** Legend chip: colour swatch + text label (colour is never the only cue). Clicking
 *  isolates the context. */
export function Chip({ label, colorVar, active = false, onClick }: ChipProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "flex items-center gap-2 border-2 border-edge px-[11px] py-1.5",
        "text-[11px] font-semibold uppercase tracking-[0.04em] transition-colors",
        active ? "bg-edge text-bg" : "bg-bg text-fg hover:bg-edge hover:text-bg",
      )}
    >
      <span className="h-3 w-3 border border-edge" style={{ background: colorVar }} />
      {label}
    </button>
  );
}
