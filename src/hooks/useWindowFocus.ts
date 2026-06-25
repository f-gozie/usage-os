import { useEffect, useRef } from "react";

/**
 * Run `onFocus` each time this Tauri window regains focus — e.g. to re-read state the user may
 * have changed in another app (a permission toggle, a reopened popover). Subscribes once and
 * always calls the latest callback (via a ref), so an inline `onFocus` won't re-subscribe.
 *
 * Guarded for non-Tauri envs (vitest/jsdom): if the window API or listener is unavailable, it's
 * a no-op.
 */
export function useWindowFocus(onFocus: () => void): void {
  const cb = useRef(onFocus);
  cb.current = onFocus;

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) =>
        getCurrentWindow().onFocusChanged(({ payload: focused }) => {
          if (focused) cb.current();
        }),
      )
      .then((stop) => {
        if (active) unlisten = stop;
        else stop();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);
}
