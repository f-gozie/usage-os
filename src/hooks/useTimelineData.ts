import { useCallback } from "react";

import { useViewData, type ViewDataState } from "@/hooks/useViewData";
import { getTimeline, type TimelineView } from "@/lib/tauri";

export type TimelineDataState = ViewDataState<TimelineView>;

/** The computed Timeline view for a range. Polls every 30s while `live` (viewing today). */
export function useTimelineData(
  startUnix: number,
  endUnix: number,
  live: boolean,
): TimelineDataState {
  const fetchTimeline = useCallback(
    () => getTimeline(startUnix, endUnix),
    [startUnix, endUnix],
  );
  return useViewData(fetchTimeline, [startUnix, endUnix], live, "Couldn't load this timeline.");
}
