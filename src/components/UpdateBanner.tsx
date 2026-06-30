import { useState } from "react";

import { installUpdate, type Update } from "@/lib/updater";

/**
 * A calm, dismissible "update available" card (bottom-right). Only shown when the launch auto-check
 * (opt-in) found a newer version — never on its own. Installing verifies the ed25519 signature,
 * replaces the app, and relaunches into the new build.
 */
export function UpdateBanner({ update, onDismiss }: { update: Update; onDismiss: () => void }) {
  const [installing, setInstalling] = useState(false);
  const [pct, setPct] = useState<number | null>(null);
  const [error, setError] = useState(false);

  const install = () => {
    setInstalling(true);
    setError(false);
    void installUpdate(update, (downloaded, total) => {
      setPct(total ? Math.round((downloaded / total) * 100) : null);
    }).catch(() => {
      setError(true);
      setInstalling(false);
    });
    // On success the process relaunches, so nothing past this runs.
  };

  return (
    <div className="fixed bottom-5 right-5 z-[100] w-[320px] border-[3px] border-edge bg-bar-bg text-bar-fg">
      <div className="px-4 py-3">
        <div className="text-[10px] font-semibold uppercase tracking-[0.14em] opacity-60">
          Update available
        </div>
        <div className="mt-1 text-sm font-semibold">UsageOS {update.version}</div>
        {update.body ? (
          <div className="mt-1 max-h-24 overflow-auto whitespace-pre-line text-xs leading-normal opacity-75">
            {update.body}
          </div>
        ) : null}
        {error ? (
          <div className="mt-2 text-xs text-c-research">
            Update failed. Try again, or download from the site.
          </div>
        ) : null}
        <div className="mt-3 flex items-center gap-2">
          <button
            type="button"
            onClick={install}
            disabled={installing}
            className="border-2 border-c-deep bg-c-deep px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.06em] text-bg disabled:opacity-60"
          >
            {installing ? (pct != null ? `Installing ${pct}%` : "Installing…") : "Install & restart"}
          </button>
          {installing ? null : (
            <button
              type="button"
              onClick={onDismiss}
              className="px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.06em] opacity-70 hover:opacity-100"
            >
              Later
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
