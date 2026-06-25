import { useCallback } from "react";

import { useViewData, type ViewDataState } from "@/hooks/useViewData";
import { getWeek, type WeekView } from "@/lib/tauri";

export type WeekDataState = ViewDataState<WeekView>;

/** The computed Week view for 7 local-midnight day-starts + the week end. Keyed on the
 *  day-starts so navigating weeks refetches. Polls every 30s while `live` (the current
 *  week). */
export function useWeekData(dayStarts: number[], weekEnd: number, live: boolean): WeekDataState {
  const key = dayStarts.join(",");
  const fetchWeek = useCallback(
    () => getWeek(dayStarts, weekEnd),
    // `key` is the content identity of `dayStarts` (a fresh array each render).
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key, weekEnd],
  );
  return useViewData(fetchWeek, [key, weekEnd], live, "Couldn't load this week.");
}
