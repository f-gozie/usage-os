import type { InputHTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface TextInputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: ReactNode;
  error?: string;
}

/** Bauhaus text input: 2px ink border, blue focus ring, optional label + error message.
 *  Ported from `design/components.html` (`.field`/`.input`/`.err`). */
export function TextInput({ label, error, className, ...props }: TextInputProps) {
  return (
    <label className="block">
      {label != null && (
        <span className="mb-1.5 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          {label}
        </span>
      )}
      <input
        className={cn(
          "w-full border-2 border-edge bg-bg px-3 py-[9px] text-sm font-medium text-fg",
          "placeholder:text-muted",
          "focus:border-c-deep focus:outline-none focus:shadow-[0_0_0_2px_var(--bg),0_0_0_4px_var(--c-deep)]",
          "disabled:opacity-[0.45]",
          error && "border-c-research",
          className,
        )}
        aria-invalid={error ? true : undefined}
        {...props}
      />
      {error && (
        <span className="mt-1.5 block text-[11px] font-semibold text-c-research">{error}</span>
      )}
    </label>
  );
}
