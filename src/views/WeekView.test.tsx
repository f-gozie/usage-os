// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { WeekView } from "./WeekView";

// Stub the data hook with a fixed week (fixture lives inside the factory to dodge hoisting).
vi.mock("@/hooks/useWeekData", () => {
  const slice = (deep: number) => ({ day_start: 0, active_secs: deep, deep_secs: deep, runs: [] });
  const WEEK = {
    days: [slice(0), slice(600), slice(1200), slice(3600), slice(1200), slice(0), slice(1800)],
    total_active_secs: 8400, // 2h 20m
    avg_active_secs: 1200, // 20m
    deepest_day: 3,
  };
  return { useWeekData: () => ({ data: WEEK, loading: false, error: null, refresh: () => {} }) };
});

vi.mock("@/lib/tauri", () => ({
  getWatcherStatus: vi.fn().mockResolvedValue({ healthy: true, consecutive_errors: 0 }),
}));

describe("WeekView", () => {
  const date = new Date(2026, 5, 17); // a Wednesday in June 2026

  it("renders the week summary and seven day cells", () => {
    const { getByText, getAllByRole } = render(
      <WeekView date={date} onDateChange={() => {}} onOpenDay={() => {}} />,
    );
    expect(getByText("Active this week")).toBeInTheDocument();
    expect(getByText("2h 20m")).toBeInTheDocument(); // total
    expect(getByText("Avg / day")).toBeInTheDocument();
    // One mini-dial per day.
    expect(getAllByRole("img")).toHaveLength(7);
  });

  it("opens a day when its cell is clicked", () => {
    const onOpenDay = vi.fn();
    const { getAllByRole } = render(
      <WeekView date={date} onDateChange={() => {}} onOpenDay={onOpenDay} />,
    );
    // Day cells are the buttons without an aria-label (nav buttons have one).
    const cells = getAllByRole("button").filter((b) => !b.getAttribute("aria-label"));
    expect(cells).toHaveLength(7);
    fireEvent.click(cells[0]);
    expect(onOpenDay).toHaveBeenCalledTimes(1);
    expect(onOpenDay.mock.calls[0][0]).toBeInstanceOf(Date);
  });
});
