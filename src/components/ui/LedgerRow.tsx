import { cn } from "@/lib/utils";

export interface LedgerRowProps {
  name: string;
  colorVar: string;
  durationLabel: string;
  pct: number;
  dimmed?: boolean;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  onClick?: () => void;
}

/** A "where it went" row: swatch + name, a proportional bar, and the figure + percent.
 *  Hovering dims the others; clicking isolates the category. */
export function LedgerRow({
  name,
  colorVar,
  durationLabel,
  pct,
  dimmed = false,
  onMouseEnter,
  onMouseLeave,
  onClick,
}: LedgerRowProps) {
  return (
    <div
      className={cn(
        "grid cursor-pointer items-center gap-3.5 border-t-2 border-edge py-2.5 transition-opacity last:border-b-2",
        "grid-cols-[130px_1fr_118px]",
        dimmed && "opacity-30",
      )}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onClick={onClick}
    >
      <div className="flex items-center gap-2.5 text-[13px] font-semibold uppercase tracking-[0.03em]">
        <span className="h-4 w-4 flex-shrink-0 border border-edge" style={{ background: colorVar }} />
        {name}
      </div>
      <div className="h-[9px] bg-track">
        <div className="h-full" style={{ width: `${pct}%`, background: colorVar }} />
      </div>
      <div className="text-right font-display text-[19px]">
        {durationLabel}
        <span className="ml-[5px] font-sans text-[11px] font-semibold text-muted">{pct}%</span>
      </div>
    </div>
  );
}
