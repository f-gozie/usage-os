import { useCallback, useEffect, useState } from "react";

import {
  getContexts,
  getExclusions,
  getRules,
  getSettings,
  type Context,
  type Exclusion,
  type Rule,
} from "@/lib/tauri";

export interface SettingsData {
  contexts: Context[];
  rules: Rule[];
  exclusions: Exclusion[];
  /** The key/value settings table, flattened for lookup. */
  settings: Record<string, string>;
}

export interface SettingsDataState {
  data: SettingsData | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

/**
 * Loads everything the Settings view edits — contexts, rules, exclusions, and the
 * key/value settings — in one shot. Mirrors `useDayData`'s `{ data, loading, error,
 * refresh }` shape; every mutation in the view runs its command then calls `refresh()`.
 */
export function useSettingsData(): SettingsDataState {
  const [data, setData] = useState<SettingsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAll = useCallback(async () => {
    try {
      setError(null);
      const [contexts, rules, exclusions, settings] = await Promise.all([
        getContexts(),
        getRules(),
        getExclusions(),
        getSettings(),
      ]);
      setData({
        contexts,
        rules,
        exclusions,
        settings: Object.fromEntries(settings.map((s) => [s.key, s.value])),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't load settings.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  return { data, loading, error, refresh: fetchAll };
}
