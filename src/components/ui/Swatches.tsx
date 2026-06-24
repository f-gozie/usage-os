import { cn } from "@/lib/utils";

export interface SwatchesProps {
  colors: readonly string[];
  value: string;
  onChange: (color: string) => void;
  "aria-label"?: string;
}

/** A row of colour swatches; the selected one gets a double-ring.
 *  Ported from `design/components.html` (`.swatches`/`.sw`). */
export function Swatches({ colors, value, onChange, ...rest }: SwatchesProps) {
  return (
    <div role="radiogroup" aria-label={rest["aria-label"]} className="flex gap-2">
      {colors.map((c) => {
        const selected = c.toLowerCase() === value.toLowerCase();
        return (
          <button
            key={c}
            type="button"
            role="radio"
            aria-checked={selected}
            aria-label={c}
            onClick={() => onChange(c)}
            style={{ background: c }}
            className={cn(
              "h-[30px] w-[30px] border-2 border-edge",
              selected && "shadow-[0_0_0_2px_var(--bg),0_0_0_4px_var(--fg)]",
            )}
          />
        );
      })}
    </div>
  );
}
