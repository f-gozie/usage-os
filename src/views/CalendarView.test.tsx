// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent, waitFor } from "@testing-library/react";

import { CalendarView } from "./CalendarView";

// A year and a month of stub data. The fixtures live inside the factories to dodge hoisting.
vi.mock("@/hooks/useCalendarData", () => {
  // Fixtures live inside the factory: `vi.mock` is hoisted above every top-level const.
  const d = 86_400;
  const yearDay = (i: number) => ({
    day_start: new Date(2026, 0, 1).getTime() / 1000 + i * d,
    active_secs: i % 7 === 0 ? 0 : 3600,
    categories: i % 7 === 0 ? [] : [{ slug: "deep", name: "Work", secs: 3600, pct: 100, color: null }],
    unwatched_secs: i === 40 ? d : 0,
    coverage_known: true,
  });
  const YEAR = { days: Array.from({ length: 365 }, (_, i) => yearDay(i)) };
  const MONTH = {
    days: Array.from({ length: 30 }, (_, i) => ({
      day_start: new Date(2026, 8, 1).getTime() / 1000 + i * d,
      active_secs: i === 15 ? 0 : 3600,
      deep_secs: 0,
      runs: [],
      unwatched_secs: i === 15 ? d : 0,
      coverage_known: true,
    })),
    total_active_secs: 29 * 3600,
    tracked_days: 29,
    empty_days: 1,
  };
  return {
    useYearData: () => ({ data: YEAR, loading: false, error: null, refresh: () => {} }),
    useMonthData: () => ({ data: MONTH, loading: false, error: null, refresh: () => {} }),
  };
});

const updateSetting = vi.fn().mockResolvedValue(undefined);
vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  getSettings: () => Promise.resolve([]),
  updateSetting: (...args: unknown[]) => updateSetting(...args),
}));

vi.mock("@/lib/appIcons", () => ({ loadIconMap: () => Promise.resolve(), iconFor: () => null }));

function setup(overrides: Partial<React.ComponentProps<typeof CalendarView>> = {}) {
  return render(
    <CalendarView
      date={new Date(2026, 8, 16)}
      onDateChange={() => {}}
      onOpenDay={() => {}}
      {...overrides}
    />,
  );
}

beforeEach(() => updateSetting.mockClear());

// This file cold-mounts a 365-cell year in jsdom. On a loaded machine (observed once: a release
// compile running, swap exhausted, load average ~9) the first mount intermittently exceeded
// vitest's 5s default. That failure was never reproduced on demand and its error text was not
// captured, so "timeout under load" is an inference. The budget below is deliberate rather than
// cosmetic: wall-clock is the wrong instrument for render cost — `YearGrid.test.tsx` pins that by
// counting renders, which does not vary with machine load.
vi.setConfig({ testTimeout: 20_000 });

/** The Calendar opens on the Month rung; most tests start by zooming out to the Year. */
function toYear(getByRole: ReturnType<typeof setup>["getByRole"]) {
  fireEvent.click(getByRole("button", { name: "year" }));
}

describe("CalendarView", () => {
  it("opens on the month, and the year offers both layouts", () => {
    const { getByRole } = setup();
    expect(getByRole("button", { name: "month" })).toHaveAttribute("aria-pressed", "true");

    toYear(getByRole);
    expect(getByRole("button", { name: "Month rows" })).toBeInTheDocument();
    expect(getByRole("button", { name: "Wall calendar" })).toBeInTheDocument();
  });

  it("switches layout and remembers the choice", async () => {
    const { getByRole } = setup();
    toYear(getByRole);
    const rows = getByRole("button", { name: "Month rows" });
    const wall = getByRole("button", { name: "Wall calendar" });

    expect(wall).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(rows);

    expect(rows).toHaveAttribute("aria-pressed", "true");
    expect(wall).toHaveAttribute("aria-pressed", "false");
    await waitFor(() =>
      expect(updateSetting).toHaveBeenCalledWith("calendar_year_layout", "rows"),
    );
  });

  it("drills from a month name into that month, and the breadcrumb comes back", () => {
    const onDateChange = vi.fn();
    const { getByRole, getAllByRole } = setup({ onDateChange });
    toYear(getByRole);

    fireEvent.click(getAllByRole("button", { name: "Sep" })[0]);
    expect(onDateChange).toHaveBeenCalled();

    // Now on the month rung: the year is a breadcrumb link back.
    expect(getByRole("button", { name: "2026" })).toBeInTheDocument();
  });

  it("drills one rung at a time: a bar in the Year opens its Month, never the Day", () => {
    const onOpenDay = vi.fn();
    const { getAllByRole, getByRole } = setup({ onOpenDay });
    toYear(getByRole);

    const bar = getAllByRole("button").find((b) => b.hasAttribute("data-day")) as HTMLElement;
    fireEvent.click(bar);

    expect(onOpenDay).not.toHaveBeenCalled();
    expect(getByRole("button", { name: "2026" })).toBeInTheDocument(); // on the Month rung
  });

  it("opens the Day from the Month rung", () => {
    const onOpenDay = vi.fn();
    const { getAllByRole } = setup({ onOpenDay });

    const dayCell = getAllByRole("button").find((b) => b.hasAttribute("data-day")) as HTMLElement;
    fireEvent.click(dayCell);

    expect(onOpenDay).toHaveBeenCalledTimes(1);
  });

  it("switches rungs from the visible Year | Month control", () => {
    const { getByRole } = setup();
    const month = getByRole("button", { name: "month" });

    toYear(getByRole);
    expect(month).toHaveAttribute("aria-pressed", "false");
    expect(getByRole("button", { name: "Wall calendar" })).toBeInTheDocument();

    fireEvent.click(month);
    expect(month).toHaveAttribute("aria-pressed", "true");
    expect(getByRole("button", { name: "2026" })).toBeInTheDocument();
  });

  it("counts empty days, and says which ones UsageOS wasn’t running for", () => {
    const { getByRole, getByText } = setup();

    expect(getByText("Day with nothing tracked")).toBeInTheDocument();
    expect(getByRole("button", { name: /UsageOS wasn’t running/ })).toBeInTheDocument();
  });

  it("steps the range by month or by year depending on the rung", () => {
    const onDateChange = vi.fn();
    const { getByRole } = setup({ onDateChange });

    fireEvent.click(getByRole("button", { name: "Previous month" }));
    expect((onDateChange.mock.calls[0][0] as Date).getMonth()).toBe(7); // August

    onDateChange.mockClear();
    toYear(getByRole);
    fireEvent.click(getByRole("button", { name: "Previous year" }));
    expect((onDateChange.mock.calls[0][0] as Date).getFullYear()).toBe(2025);
  });

  it("offers a jump back to the current range, named for the rung", () => {
    const { getByRole } = setup({ date: new Date(2026, 8, 16) });
    expect(getByRole("button", { name: "Go to this month" })).toBeInTheDocument();
  });
});
