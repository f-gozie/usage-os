import { useEffect, useState } from "react";

import { getWatcherStatus } from "@/lib/tauri";

const BAR_MUTED = "rgba(255,255,255,0.65)"; // muted-on-dark (the bar is dark in every theme)

/** The window titlebar. With the Overlay title-bar style the macOS traffic lights sit
 *  at the left (so we leave room and don't draw fake ones); the bar is the drag region.
 *  Centre wordmark, live capture status on the right. */
export function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="relative flex h-[38px] flex-shrink-0 items-center justify-end bg-bar-bg pl-20 pr-4"
    >
      <span
        data-tauri-drag-region
        className="pointer-events-none absolute inset-x-0 text-center text-xs font-semibold uppercase tracking-[0.34em] text-bar-fg"
      >
        UsageOS
      </span>
      <TrackingStatus />
    </div>
  );
}

/** Polls capture health so the indicator reflects reality (it really is tracking while
 *  the app is open). Pulsing gold = tracking; red = capture is erroring. */
function TrackingStatus() {
  const [healthy, setHealthy] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const poll = () =>
      getWatcherStatus()
        .then((s) => !cancelled && setHealthy(s.healthy))
        .catch(() => undefined);
    void poll();
    const id = setInterval(poll, 15_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <span
      className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.12em]"
      style={{ color: BAR_MUTED }}
    >
      <span
        className={healthy ? "h-2 w-2 animate-pulse rounded-full" : "h-2 w-2 rounded-full"}
        style={{ background: healthy ? "var(--c-comms)" : "var(--c-research)" }}
      />
      {healthy ? "Tracking" : "Capture issue"}
    </span>
  );
}
