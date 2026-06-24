import { cn } from "@/lib/utils";

export interface StatTileProps {
  value: string;
  label: string;
  /** Optional accent colour for the figure (e.g. a category token). */
  colorVar?: string;
  className?: string;
}

/** A single headline figure + caption. Composed into a row by the Day view. */
export function StatTile({ value, label, colorVar, className }: StatTileProps) {
  return (
    <div className={cn("flex-1 pb-1 pt-[13px]", className)}>
      <div
        className="font-display text-[28px] leading-[0.9]"
        style={colorVar ? { color: colorVar } : undefined}
      >
        {value}
      </div>
      <div className="mt-[7px] text-[9.5px] font-semibold uppercase tracking-[0.13em] text-muted">
        {label}
      </div>
    </div>
  );
}
