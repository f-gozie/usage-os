import { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

export interface SelectProps<T extends string> {
  options: ReadonlyArray<SelectOption<T>>;
  value: T;
  onChange: (value: T) => void;
  "aria-label"?: string;
  className?: string;
}

/** Custom dropdown (not a native `<select>`, which can't be Bauhaus-styled): a `.val`
 *  trigger + a `.menu` popover with a ✓ on the selected row. Closes on outside-click or
 *  Esc. Ported from `design/components.html` (`.val`/`.menu`/`.mi`). */
export function Select<T extends string>({
  options,
  value,
  onChange,
  className,
  ...rest
}: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const selected = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={ref} className={cn("relative inline-block", className)}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={rest["aria-label"]}
        onClick={() => setOpen((o) => !o)}
        className="flex w-max items-center gap-2.5 border-2 border-edge bg-bg px-[13px] py-[9px] text-[13px] font-semibold text-fg"
      >
        {selected?.label ?? ""}
        <span className="-mt-[3px] text-[15px] leading-none text-muted">⌄</span>
      </button>
      {open && (
        <div
          role="listbox"
          className="absolute right-0 z-20 mt-1 min-w-full whitespace-nowrap border-2 border-edge bg-bg"
        >
          {options.map((o, i) => {
            const sel = o.value === value;
            return (
              <button
                key={o.value}
                type="button"
                role="option"
                aria-selected={sel}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
                className={cn(
                  "flex w-full items-center justify-between gap-6 px-[13px] py-[9px] text-left text-[13px] font-semibold",
                  i > 0 && "border-t-2 border-edge",
                  sel ? "bg-edge text-bg" : "hover:bg-surface",
                )}
              >
                {o.label}
                {sel && <span aria-hidden>✓</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
