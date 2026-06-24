import { useEffect, useState } from "react";

import { listInstalledApps, type InstalledApp } from "@/lib/tauri";

// Loaded once and shared (the catalog doesn't change during a session). Failures
// resolve to an empty list so the picker simply shows nothing to pick.
let cache: InstalledApp[] | null = null;
let inflight: Promise<InstalledApp[]> | null = null;

function load(): Promise<InstalledApp[]> {
  if (!inflight) {
    inflight = listInstalledApps()
      .then((apps) => {
        cache = apps;
        return apps;
      })
      .catch(() => {
        cache = [];
        return cache;
      });
  }
  return inflight;
}

/** The user's installed apps (name + icon), loaded once and cached. Empty until the
 *  first load resolves. */
export function useInstalledApps(): InstalledApp[] {
  const [apps, setApps] = useState<InstalledApp[]>(cache ?? []);
  useEffect(() => {
    if (cache) {
      setApps(cache);
      return;
    }
    let alive = true;
    void load().then((a) => {
      if (alive) setApps(a);
    });
    return () => {
      alive = false;
    };
  }, []);
  return apps;
}
