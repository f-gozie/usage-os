import { useEffect, useState } from "react";

/**
 * "Now" in Unix seconds, stable between minute boundaries.
 *
 * Reading `Date.now()` during render hands every child a new value on every render, which
 * silently defeats memoization for a whole subtree — for the Calendar that meant 365 cells
 * re-rendering on any click. Nothing on a calendar changes faster than a minute, so this ticks
 * once a minute, aligned to the boundary, and is referentially stable in between.
 */
export function useMinuteNow(): number {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | undefined;
    // Align the first tick to the next minute boundary, then tick every 60s.
    const timeout = setTimeout(
      () => {
        setNow(Math.floor(Date.now() / 1000));
        interval = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 60_000);
      },
      60_000 - (Date.now() % 60_000),
    );
    return () => {
      clearTimeout(timeout);
      if (interval) clearInterval(interval);
    };
  }, []);

  return now;
}
