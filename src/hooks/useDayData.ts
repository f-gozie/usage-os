import { useCallback } from "react";

import { useViewData, type ViewDataState } from "@/hooks/useViewData";
import { getDay, type DayView } from "@/lib/tauri";

export type DayDataState = ViewDataState<DayView>;

/** The computed Day view for a range. Polls every 30s while `live` (viewing today). */
export function useDayData(startUnix: number, endUnix: number, live: boolean): DayDataState {
  const fetchDay = useCallback(() => getDay(startUnix, endUnix), [startUnix, endUnix]);
  return useViewData(fetchDay, [startUnix, endUnix], live, "Couldn't load this day.");
}
