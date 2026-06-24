import { useCallback, useEffect, useState } from "react";

import { getTimeline, type TimelineView } from "@/lib/tauri";

export interface TimelineDataState {
  data: TimelineView | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Fetches the computed Timeline view for a range. Mirrors `useDayData`: a loading state
 * only on range change (not on background refresh), and a 30s poll while `live` (today).
 */
export function useTimelineData(
  startUnix: number,
  endUnix: number,
  live: boolean,
): TimelineDataState {
  const [data, setData] = useState<TimelineView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTimeline = useCallback(async () => {
    try {
      setError(null);
      setData(await getTimeline(startUnix, endUnix));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't load this timeline.");
    } finally {
      setLoading(false);
    }
  }, [startUnix, endUnix]);

  useEffect(() => {
    setLoading(true);
    void fetchTimeline();
    if (!live) return;
    const id = setInterval(() => void fetchTimeline(), 30_000);
    return () => clearInterval(id);
  }, [fetchTimeline, live]);

  return { data, loading, error, refresh: fetchTimeline };
}
