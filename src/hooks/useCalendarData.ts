import { useCallback } from "react";

import { useViewData, type ViewDataState } from "@/hooks/useViewData";
import { getMonth, getYear, type MonthView, type YearView } from "@/lib/tauri";

export type MonthDataState = ViewDataState<MonthView>;
export type YearDataState = ViewDataState<YearView>;

/** The computed Month view for a month's local-midnight day-starts + the month end. Keyed on
 *  the day-starts so stepping months refetches. Polls every 30s while `live` (this month);
 *  fetches nothing while not `enabled` (the Year rung is showing). */
export function useMonthData(
  dayStarts: number[],
  monthEnd: number,
  live: boolean,
  enabled: boolean,
): MonthDataState {
  const key = dayStarts.join(",");
  const fetchMonth = useCallback(
    () => getMonth(dayStarts, monthEnd),
    // `key` is the content identity of `dayStarts` (a fresh array each render).
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key, monthEnd],
  );
  return useViewData(fetchMonth, [key, monthEnd], live, "Couldn't load this month.", enabled);
}

/** The computed Year view. Never polls: a year is 365 days of arithmetic and all but one of
 *  them is frozen, so re-running it every 30s would burn battery to redraw the same field.
 *  The ↻ button and a range change are the only refreshes; nothing is fetched while not
 *  `enabled` (the Month rung is showing). */
export function useYearData(dayStarts: number[], yearEnd: number, enabled: boolean): YearDataState {
  // A year's day-starts make a long string; its first day and length identify it exactly.
  const key = `${dayStarts[0] ?? 0}:${dayStarts.length}`;
  const fetchYear = useCallback(
    () => getYear(dayStarts, yearEnd),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key, yearEnd],
  );
  return useViewData(fetchYear, [key, yearEnd], false, "Couldn't load this year.", enabled);
}
