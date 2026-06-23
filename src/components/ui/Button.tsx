import type { ButtonHTMLAttributes } from "react";

import { cn } from "@/lib/utils";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

const VARIANTS: Record<ButtonVariant, string> = {
  primary: "border-edge bg-edge text-bg hover:bg-c-deep hover:border-c-deep",
  secondary: "border-edge bg-transparent text-fg hover:bg-edge hover:text-bg",
  ghost: "border-transparent bg-transparent text-c-deep hover:underline",
  danger: "border-c-research bg-transparent text-c-research hover:bg-c-research hover:text-bg",
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
}

/** Bauhaus button: hard-edged, uppercase, flat fill. */
export function Button({ variant = "primary", className, type = "button", ...props }: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center gap-2 border-2 px-4 py-2",
        "text-xs font-semibold uppercase tracking-[0.08em] transition-colors",
        "disabled:cursor-not-allowed disabled:opacity-40",
        VARIANTS[variant],
        className,
      )}
      {...props}
    />
  );
}
