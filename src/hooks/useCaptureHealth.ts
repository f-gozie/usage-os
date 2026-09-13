import { useCallback, useEffect, useState } from "react";

import { getWatcherStatus, requestAccessibility, requestAutomation } from "@/lib/tauri";

/** What (if anything) the degraded banner should say. One banner at a time, worst first:
 *  lost window titles hits every app; lost URLs only browsers; repeated errors is the
 *  legacy catch-all. `null` = capture is fine, show nothing. */
export interface CaptureProblem {
  kind: "accessibility" | "automation" | "errors";
  title: string;
  description: string;
  actionLabel: string;
  /** Opens the matching System Settings pane (or retries, for plain errors). */
  action: () => void;
}

export interface CaptureHealth {
  /** False once the watcher reports repeated capture errors. */
  healthy: boolean;
  /** The banner to show, or `null` when capture is fine. */
  problem: CaptureProblem | null;
  /** Re-run the health check (e.g. from a degraded-banner Retry). */
  refetch: () => void;
}

/** Pure banner pick from the watcher facts — split out so it's testable without Tauri. */
export function pickProblem(
  accessibility: boolean,
  automationOk: boolean,
  healthy: boolean,
  actions: { accessibility: () => void; automation: () => void; retry: () => void },
): CaptureProblem | null {
  if (!accessibility) {
    return {
      kind: "accessibility",
      title: "Window titles aren't being recorded",
      description:
        "macOS switched off UsageOS's access to window titles — this can happen after an update, even while the checkbox still looks on. In System Settings, switch Accessibility for UsageOS off and back on.",
      actionLabel: "Open System Settings",
      action: actions.accessibility,
    };
  }
  if (!automationOk) {
    return {
      kind: "automation",
      title: "Sites aren't being recorded",
      description:
        "macOS stopped UsageOS from asking your browser which site is open — this can happen after a browser update. In System Settings, switch UsageOS back on under Automation.",
      actionLabel: "Open System Settings",
      action: actions.automation,
    };
  }
  if (!healthy) {
    return {
      kind: "errors",
      title: "Tracking hit a snag",
      description: "UsageOS ran into repeated errors while recording. Your existing data is safe.",
      actionLabel: "Retry",
      action: actions.retry,
    };
  }
  return null;
}

/** Watches capture health so the views can surface a degraded banner. Re-checks when
 *  `deps` change (a new range) and on demand via `refetch`. The permission problems are
 *  the ones macOS causes behind the app's back (D71): Accessibility dies when an update
 *  replaces the app bundle, Automation when the browser updates — both while the System
 *  Settings checkbox still looks on, which is why the copy says to switch it off and on. */
export function useCaptureHealth(deps: React.DependencyList = []): CaptureHealth {
  const [healthy, setHealthy] = useState(true);
  const [accessibility, setAccessibility] = useState(true);
  const [automationOk, setAutomationOk] = useState(true);

  const refetch = useCallback(() => {
    void getWatcherStatus()
      .then((s) => {
        setHealthy(s.healthy);
        setAccessibility(s.accessibility);
        setAutomationOk(s.automation_ok);
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    refetch();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const problem = pickProblem(accessibility, automationOk, healthy, {
    accessibility: () => void requestAccessibility().catch(() => undefined),
    automation: () => void requestAutomation().catch(() => undefined),
    retry: refetch,
  });

  return { healthy, problem, refetch };
}
