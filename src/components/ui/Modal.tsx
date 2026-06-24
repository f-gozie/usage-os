import { useEffect, useRef, type ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  /** Red header for destructive confirms (delete-all). */
  danger?: boolean;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}

/** Bauhaus modal: an ink-headed card over a dimmed overlay. Closes on Esc or
 *  overlay-click. Ported from `design/components.html` (`.modal`/`.mb`/`.mf`). */
export function Modal({ open, onClose, title, danger, children, footer, className }: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    // Land focus inside the dialog so Esc works and SR users enter the modal.
    dialogRef.current?.focus();
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        className={cn(
          "w-full max-w-[430px] border-[3px] border-edge bg-bg outline-none",
          className,
        )}
      >
        <div
          className={cn(
            "flex items-center justify-between px-4 py-3 font-display text-base uppercase tracking-[0.03em] text-bar-fg",
            danger ? "bg-c-research" : "bg-bar-bg",
          )}
        >
          <span>{title}</span>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="text-lg leading-none text-bar-fg opacity-70 transition-opacity hover:opacity-100"
          >
            ×
          </button>
        </div>
        <div className="flex flex-col gap-4 px-4 py-[18px]">{children}</div>
        {footer && (
          <div className="flex justify-end gap-2.5 border-t-2 border-edge px-4 py-3.5">{footer}</div>
        )}
      </div>
    </div>
  );
}
