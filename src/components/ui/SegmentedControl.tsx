import { cn } from "@/lib/utils";

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
}

export interface SegmentedControlProps<T extends string> {
  options: ReadonlyArray<SegmentOption<T>>;
  value: T;
  onChange: (value: T) => void;
  "aria-label"?: string;
  className?: string;
}

/** A bordered segmented control (theme switcher, small toggles). Hard edges, no radius. */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
  ...rest
}: SegmentedControlProps<T>) {
  return (
    <div role="group" aria-label={rest["aria-label"]} className={cn("inline-flex border-2 border-edge", className)}>
      {options.map((option, i) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={selected}
            onClick={() => onChange(option.value)}
            className={cn(
              "px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] transition-colors",
              i > 0 && "border-l-2 border-edge",
              selected ? "bg-edge text-bg" : "bg-transparent text-muted hover:text-fg",
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
