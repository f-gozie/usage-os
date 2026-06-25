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

/** Custom dropdown (not a native `<select>`, which can't be Bauhaus-styled): a trigger +
 *  a listbox popover with a ✓ on the selected row. Keyboard: Up/Down/Home/End move the
 *  active option, type-ahead jumps by label, Enter selects, Esc closes. Closes on
 *  outside-click too. Ported from `design/components.html` (`.val`/`.menu`/`.mi`). */
export function Select<T extends string>({
  options,
  value,
  onChange,
  className,
  ...rest
}: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0); // highlighted option while open (roving)
  const ref = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const typeAhead = useRef({ buffer: "", at: 0 });
  const label = rest["aria-label"];
  const selectedIndex = options.findIndex((o) => o.value === value);
  const selected = options[selectedIndex];

  // When opening, start the highlight on the current selection.
  useEffect(() => {
    if (open) setActive(selectedIndex >= 0 ? selectedIndex : 0);
  }, [open, selectedIndex]);

  // Keep the active option in view as it moves.
  useEffect(() => {
    if (!open) return;
    listRef.current?.children[active]?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const commit = (i: number) => {
    onChange(options[i].value);
    setOpen(false);
  };

  // Type-ahead: accumulate keystrokes briefly and jump to the first matching label.
  const typeJump = (key: string) => {
    const now = Date.now();
    const ta = typeAhead.current;
    ta.buffer = now - ta.at > 600 ? key : ta.buffer + key;
    ta.at = now;
    const match = options.findIndex((o) => o.label.toLowerCase().startsWith(ta.buffer.toLowerCase()));
    if (match >= 0) setActive(match);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        setOpen(true);
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActive((i) => Math.min(i + 1, options.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setActive((i) => Math.max(i - 1, 0));
        break;
      case "Home":
        e.preventDefault();
        setActive(0);
        break;
      case "End":
        e.preventDefault();
        setActive(options.length - 1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        commit(active);
        break;
      case "Escape":
        e.preventDefault();
        setOpen(false);
        break;
      case "Tab":
        setOpen(false);
        break;
      default:
        if (e.key.length === 1) typeJump(e.key);
    }
  };

  const optionId = (i: number) => `${label ?? "select"}-opt-${i}`;

  return (
    <div ref={ref} className={cn("relative inline-block", className)}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        aria-activedescendant={open ? optionId(active) : undefined}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={onKeyDown}
        className="flex w-max items-center gap-2.5 border-2 border-edge bg-bg px-[13px] py-[9px] text-[13px] font-semibold text-fg"
      >
        {selected?.label ?? ""}
        <span className="-mt-[3px] text-[15px] leading-none text-muted">⌄</span>
      </button>
      {open && (
        <div
          ref={listRef}
          role="listbox"
          aria-label={label}
          className="absolute right-0 z-20 mt-1 min-w-full whitespace-nowrap border-2 border-edge bg-bg"
        >
          {options.map((o, i) => {
            const sel = o.value === value;
            return (
              <div
                key={o.value}
                id={optionId(i)}
                role="option"
                aria-selected={sel}
                onClick={() => commit(i)}
                onMouseEnter={() => setActive(i)}
                className={cn(
                  "flex w-full cursor-pointer items-center justify-between gap-6 px-[13px] py-[9px] text-left text-[13px] font-semibold",
                  i > 0 && "border-t-2 border-edge",
                  sel ? "bg-edge text-bg" : active === i ? "bg-surface" : "",
                )}
              >
                {o.label}
                {sel && <span aria-hidden>✓</span>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
