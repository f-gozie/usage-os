import type { ButtonHTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";

/** A settings group: a 3px-ink card with an Anton ink header bar (optional meta), then
 *  its rows divided by 2px ink rules. Ported from `design/settings.html` (`.setgrp`). */
export function SettingGroup({
  title,
  meta,
  children,
}: {
  title: string;
  meta?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="mb-[18px] border-[3px] border-edge">
      <h4 className="flex items-center justify-between bg-bar-bg px-4 py-2.5 font-display text-[15px] uppercase tracking-[0.04em] text-bar-fg">
        <span>{title}</span>
        {meta != null && (
          <span className="font-sans text-[10px] font-semibold tracking-[0.12em] text-bar-fg opacity-60">
            {meta}
          </span>
        )}
      </h4>
      <div className="divide-y-2 divide-edge">{children}</div>
    </section>
  );
}

/** A label/description row with a control on the right (`.setrow`). */
export function SettingRow({
  label,
  description,
  danger,
  children,
}: {
  label: ReactNode;
  description?: ReactNode;
  danger?: boolean;
  children?: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3.5">
      <div>
        <div className={cn("text-sm font-semibold", danger && "text-c-research")}>{label}</div>
        {description != null && (
          <div className="mt-[3px] max-w-[52ch] text-xs leading-normal text-muted">{description}</div>
        )}
      </div>
      {children}
    </div>
  );
}

/** Small bordered action button (`.pill`); `danger` outlines/red. */
export function Pill({
  danger,
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { danger?: boolean }) {
  return (
    <button
      type="button"
      className={cn(
        "whitespace-nowrap border-2 px-[11px] py-[5px] text-[11px] font-semibold uppercase tracking-[0.06em]",
        danger ? "border-c-research text-c-research" : "border-edge bg-bg text-fg",
        "disabled:cursor-not-allowed disabled:opacity-40",
        className,
      )}
      {...props}
    />
  );
}

/** 32×32 bordered icon button (`.iconbtn`) — edit/remove affordances on rows. */
export function IconButton({ className, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={cn(
        "flex h-8 w-8 flex-shrink-0 items-center justify-center border-2 border-edge bg-bg text-[13px] text-fg",
        className,
      )}
      {...props}
    />
  );
}

/** A 62px match-type tag (`.tag`) used in the exclusions list. */
export function Tag({ children }: { children: ReactNode }) {
  return (
    <span className="w-[62px] flex-shrink-0 border-2 border-edge px-2 py-[3px] text-center text-[10px] font-semibold uppercase tracking-[0.1em] text-muted">
      {children}
    </span>
  );
}

/** The Excluded/Private pill on an exclusion row (`.modepill`). */
export function ModePill({ mode }: { mode: string }) {
  const exclude = mode === "exclude";
  return (
    <span
      className={cn(
        "border-2 border-edge px-2.5 py-1 text-[10.5px] font-semibold uppercase tracking-[0.06em]",
        exclude ? "bg-edge text-bg" : "bg-bg text-fg",
      )}
    >
      {exclude ? "Excluded" : "Private"}
    </span>
  );
}

/** The surface-tinted footer row that holds the "+ Add …" button + a hint. */
export function AddRow({ onAdd, label, hint }: { onAdd: () => void; label: string; hint: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 bg-surface px-4 py-[13px]">
      <button
        type="button"
        onClick={onAdd}
        className="border-2 border-edge bg-bg px-3.5 py-2 text-xs font-semibold uppercase tracking-[0.04em] text-fg"
      >
        {label}
      </button>
      <span className="text-xs text-muted">{hint}</span>
    </div>
  );
}
