import { useCallback, useEffect, useState } from "react";

import { getRecap, type Recap } from "@/lib/tauri";

export interface RecapState {
  /** The on-device AI recap once it resolves, or `null` while pending / on failure. */
  recap: Recap | null;
  /** Re-run the narration (e.g. on a manual refresh). */
  refetch: () => void;
}

/**
 * Lazily narrate the day's recap with the on-device model, AFTER the day has loaded (D11).
 * The Day view shows the instant template recap from `getDay`; this fetches the richer AI
 * prose in the background and the view upgrades the recap card in place when it resolves.
 *
 * Deliberately NOT polled: the model call is comparatively expensive, so it runs once per
 * day range (and on explicit refetch). On any failure it stays `null` — the template, which
 * `getDay` always provides, remains the floor (the AI recap is an optional upgrade).
 */
export function useRecap(startUnix: number, endUnix: number): RecapState {
  const [recap, setRecap] = useState<Recap | null>(null);

  const fetchRecap = useCallback(async () => {
    try {
      setRecap(await getRecap(startUnix, endUnix));
    } catch {
      // Swallow — the template recap from getDay is always present; the AI prose is optional.
    }
  }, [startUnix, endUnix]);

  useEffect(() => {
    // Clear on range change so we never show the previous day's prose under the new day.
    setRecap(null);
    void fetchRecap();
  }, [fetchRecap]);

  return { recap, refetch: fetchRecap };
}
