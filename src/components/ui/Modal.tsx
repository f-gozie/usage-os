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

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Bauhaus modal: an ink-headed card over a dimmed overlay. Closes on Esc or
 *  overlay-click. Traps focus while open and restores it to the trigger on close. */
export function Modal({ open, onClose, title, danger, children, footer, className }: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const restoreRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    // Remember what had focus so we can hand it back on close.
    restoreRef.current = document.activeElement as HTMLElement | null;

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      // Keep Tab inside the dialog so it can't reach the page behind it.
      const dialog = dialogRef.current;
      if (!dialog) return;
      const items = Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => el.offsetParent !== null,
      );
      if (items.length === 0) {
        e.preventDefault();
        dialog.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && (active === first || active === dialog)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", onKey);
    // Land focus inside the dialog so Esc/Tab work and SR users enter the modal.
    dialogRef.current?.focus();

    return () => {
      document.removeEventListener("keydown", onKey);
      restoreRef.current?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
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
          "flex max-h-[calc(100vh-2rem)] w-full max-w-[430px] flex-col border-[3px] border-edge bg-bg outline-none",
          className,
        )}
      >
        <div
          className={cn(
            "flex flex-shrink-0 items-center justify-between px-4 py-3 font-display text-base uppercase tracking-[0.03em] text-bar-fg",
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
        <div className="flex flex-col gap-4 overflow-y-auto px-4 py-[18px]">{children}</div>
        {footer && (
          <div className="flex flex-shrink-0 justify-end gap-2.5 border-t-2 border-edge px-4 py-3.5">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
