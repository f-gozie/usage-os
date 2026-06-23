import { useCallback, useEffect, useState } from "react";

import { getWeek, type WeekView } from "@/lib/tauri";

export interface WeekDataState {
  data: WeekView | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Fetches the computed Week view for 7 local-midnight day-starts + the week end. Mirrors
 * `useDayData`: a loading state only on week change (not on background refresh), and a 30s
 * poll while `live` (the current week). Keyed on the day-starts so navigating weeks refetches.
 */
export function useWeekData(dayStarts: number[], weekEnd: number, live: boolean): WeekDataState {
  const [data, setData] = useState<WeekView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const key = dayStarts.join(",");

  const fetchWeek = useCallback(async () => {
    try {
      setError(null);
      const starts = key.split(",").map(Number);
      setData(await getWeek(starts, weekEnd));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't load this week.");
    } finally {
      setLoading(false);
    }
  }, [key, weekEnd]);

  useEffect(() => {
    setLoading(true);
    void fetchWeek();
    if (!live) return;
    const id = setInterval(() => void fetchWeek(), 30_000);
    return () => clearInterval(id);
  }, [fetchWeek, live]);

  return { data, loading, error, refresh: fetchWeek };
}
