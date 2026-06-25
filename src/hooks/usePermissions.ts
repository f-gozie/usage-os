import { useCallback, useEffect, useState } from "react";

import { useWindowFocus } from "@/hooks/useWindowFocus";
import { getPermissions, type Permissions } from "@/lib/tauri";

export interface PermissionsState {
  /** The two capture grants, or `null` until the first read resolves. */
  permissions: Permissions | null;
  /** Re-read the grants (called after a request, or manually). */
  refetch: () => void;
}

/**
 * Reads the macOS capture permissions (Accessibility + Automation) for onboarding + Settings,
 * and re-reads them whenever our window regains focus — so a grant the user just toggled in
 * System Settings flips to "Granted" the moment they switch back, with no manual refresh.
 */
export function usePermissions(): PermissionsState {
  const [permissions, setPermissions] = useState<Permissions | null>(null);

  const refetch = useCallback(() => {
    void getPermissions()
      .then(setPermissions)
      .catch(() => undefined);
  }, []);

  useEffect(() => refetch(), [refetch]);
  // Re-read whenever we regain focus — so a grant the user just toggled in System Settings flips
  // to "Granted" the moment they switch back.
  useWindowFocus(refetch);

  return { permissions, refetch };
}
