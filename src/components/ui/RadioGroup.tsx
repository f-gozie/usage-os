export interface RadioOption<T extends string> {
  value: T;
  label: string;
  description?: string;
}

export interface RadioGroupProps<T extends string> {
  options: ReadonlyArray<RadioOption<T>>;
  value: T;
  onChange: (value: T) => void;
  "aria-label"?: string;
}

/** Vertical radio list with optional descriptions (e.g. exclusion Exclude/Private).
 *  Ported from `design/components.html` (`.radio`/`.dot`). */
export function RadioGroup<T extends string>({
  options,
  value,
  onChange,
  ...rest
}: RadioGroupProps<T>) {
  return (
    <div role="radiogroup" aria-label={rest["aria-label"]} className="flex flex-col gap-3">
      {options.map((o) => {
        const selected = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(o.value)}
            className="flex items-center gap-[9px] text-left"
          >
            <span className="relative h-[18px] w-[18px] flex-shrink-0 border-2 border-edge">
              {selected && <span className="absolute inset-[3px] bg-edge" />}
            </span>
            <span>
              <span className="block text-sm font-semibold">{o.label}</span>
              {o.description && (
                <span className="block text-xs font-medium text-muted">{o.description}</span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}
