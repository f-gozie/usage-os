// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { dayLabel, dayState, YearCell, YEAR_CEILING_SECS } from "./YearGrid";
import type { YearDay } from "@/lib/tauri";

const DAY = 86_400;
const NOW = 1_800_000_000;

function day(over: Partial<YearDay> = {}): YearDay {
  return {
    day_start: NOW - DAY,
    active_secs: 0,
    categories: [],
    unwatched_secs: 0,
    coverage_known: true,
    ...over,
  };
}

describe("dayState", () => {
  it("tells a day nobody watched from a day you simply didn't use", () => {
    expect(dayState(day({ unwatched_secs: DAY }), NOW)).toBe("unrecorded");
    expect(dayState(day({ active_secs: 0 }), NOW)).toBe("recorded");
  });

  it("refuses to blame a day that predates coverage", () => {
    // No coverage record means we cannot say — never "not recorded".
    expect(dayState(day({ coverage_known: false, unwatched_secs: DAY }), NOW)).toBe("unknown");
  });

  it("does not call a partly-blind day unrecorded", () => {
    expect(dayState(day({ active_secs: 7200, unwatched_secs: 6 * 3600 }), NOW)).toBe("recorded");
  });

  it("knows a day that hasn't happened", () => {
    expect(dayState(day({ day_start: NOW + DAY }), NOW)).toBe("future");
  });
});

describe("dayLabel", () => {
  it("says plainly which of the two empty days it is", () => {
    const d = day({ unwatched_secs: DAY });
    expect(dayLabel(d, "unrecorded", "Wed 16 September")).toBe(
      "Wed 16 September · UsageOS wasn’t running",
    );
    expect(dayLabel(day(), "recorded", "Wed 16 September")).toBe(
      "Wed 16 September · nothing tracked",
    );
  });

  it("names the hours on a day with activity", () => {
    expect(dayLabel(day({ active_secs: 3600 }), "recorded", "Wed 16")).toBe("Wed 16 · 1h 0m");
  });

  it("says a day predating coverage is simply before we were recording", () => {
    expect(dayLabel(day({ coverage_known: false }), "unknown", "Wed 16")).toBe(
      "Wed 16 · before UsageOS was recording",
    );
  });
});

describe("YearCell", () => {
  const props = {
    now: NOW,
    dateText: "Wed 16 September",
    barWidthClass: "w-[13px]",
    onOpen: () => {},
  };

  it("sizes the bar by hours against a fixed ceiling, not the busiest day", () => {
    const half = day({
      active_secs: YEAR_CEILING_SECS / 2,
      categories: [{ slug: "deep", name: "Work", secs: YEAR_CEILING_SECS / 2, pct: 100, color: null }],
    });
    const { container } = render(<YearCell {...props} day={half} />);
    const stack = container.querySelector("span[style*='height']") as HTMLElement;
    expect(stack.style.height).toBe("50%");
  });

  it("caps a very long day at the ceiling rather than overflowing its row", () => {
    const long = day({
      active_secs: YEAR_CEILING_SECS * 3,
      categories: [{ slug: "deep", name: "Work", secs: YEAR_CEILING_SECS * 3, pct: 100, color: null }],
    });
    const { container } = render(<YearCell {...props} day={long} />);
    const stack = container.querySelector("span[style*='height']") as HTMLElement;
    expect(stack.style.height).toBe("100%");
  });

  it("stacks each category as its own share of that day", () => {
    const mixed = day({
      active_secs: 10_800,
      categories: [
        { slug: "deep", name: "Work", secs: 7200, pct: 66.6, color: null },
        { slug: "comms", name: "Messaging", secs: 3600, pct: 33.3, color: null },
      ],
    });
    const { container } = render(<YearCell {...props} day={mixed} />);
    const segments = container.querySelectorAll("span[style*='background']");
    expect(segments).toHaveLength(2);
  });

  it("opens the day when clicked, and a future day cannot be clicked", () => {
    const onOpen = vi.fn();
    const { getByRole, rerender } = render(
      <YearCell {...props} day={day({ active_secs: 60 })} onOpen={onOpen} />,
    );
    fireEvent.click(getByRole("button"));
    expect(onOpen).toHaveBeenCalledWith(NOW - DAY);

    rerender(<YearCell {...props} day={day({ day_start: NOW + DAY })} onOpen={onOpen} />);
    expect(getByRole("button")).toBeDisabled();
  });

  it("gives every cell a name a screen reader can read", () => {
    const { getByRole } = render(<YearCell {...props} day={day({ unwatched_secs: DAY })} />);
    expect(getByRole("button")).toHaveAccessibleName("Wed 16 September · UsageOS wasn’t running");
  });
});
