import { cn } from "@/lib/utils";

export interface ToggleProps {
  checked: boolean;
  onChange?: (checked: boolean) => void;
  disabled?: boolean;
  "aria-label"?: string;
}

/** Bauhaus switch: a 54×29 ink box with a sliding knob; fills blue when on.
 *  Ported from `design/components.html` (`.toggle`). */
export function Toggle({ checked, onChange, disabled, ...rest }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={rest["aria-label"]}
      disabled={disabled}
      onClick={() => onChange?.(!checked)}
      className={cn(
        "relative h-[29px] w-[54px] flex-shrink-0 border-2 transition-colors disabled:opacity-40",
        checked ? "border-c-deep bg-c-deep" : "border-edge bg-bg",
      )}
    >
      <span
        className={cn(
          "absolute top-[2px] h-[21px] w-[21px] transition-[left]",
          checked ? "left-[29px] bg-bg" : "left-[2px] bg-edge",
        )}
      />
    </button>
  );
}
