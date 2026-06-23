import { useCallback, useEffect, useState } from "react";

import { getDay, type DayView } from "@/lib/tauri";

export interface DayDataState {
  data: DayView | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Fetches the computed Day view for a range. Shows a loading state only on range
 * change (not on background refresh, so data doesn't flash). When `live` (viewing
 * today), it polls every 30s like the old dashboard did.
 */
export function useDayData(startUnix: number, endUnix: number, live: boolean): DayDataState {
  const [data, setData] = useState<DayView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchDay = useCallback(async () => {
    try {
      setError(null);
      setData(await getDay(startUnix, endUnix));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't load this day.");
    } finally {
      setLoading(false);
    }
  }, [startUnix, endUnix]);

  useEffect(() => {
    setLoading(true);
    void fetchDay();
    if (!live) return;
    const id = setInterval(() => void fetchDay(), 30_000);
    return () => clearInterval(id);
  }, [fetchDay, live]);

  return { data, loading, error, refresh: fetchDay };
}
