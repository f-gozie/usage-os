import { useCallback, useEffect, useState } from "react";

import { getWatcherStatus } from "@/lib/tauri";

export interface CaptureHealth {
  /** False once the watcher reports repeated capture errors. */
  healthy: boolean;
  /** Re-run the health check (e.g. from a degraded-banner Retry). */
  refetch: () => void;
}

/** Watches capture health so the views can surface a degraded banner. Re-checks when
 *  `deps` change (a new range) and on demand via `refetch`. */
export function useCaptureHealth(deps: React.DependencyList = []): CaptureHealth {
  const [healthy, setHealthy] = useState(true);

  const refetch = useCallback(() => {
    void getWatcherStatus()
      .then((s) => setHealthy(s.healthy))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    refetch();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return { healthy, refetch };
}
