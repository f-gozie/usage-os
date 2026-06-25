import { describe, it, expect } from "vitest";

import { addDays, dayBounds, isSameDay, weekDays } from "./dates";

describe("dayBounds", () => {
  it("returns a half-open [midnight, next-midnight) window", () => {
    const { start, end } = dayBounds(new Date(2026, 5, 17, 13, 45));
    // start is this local midnight; end is the next local midnight (exclusive).
    expect(new Date(start * 1000).getHours()).toBe(0);
    expect(new Date(start * 1000).getDate()).toBe(17);
    expect(new Date(end * 1000).getHours()).toBe(0);
    expect(new Date(end * 1000).getDate()).toBe(18);
    // Exactly one day wide (no dropped final second).
    expect(end - start).toBe(24 * 60 * 60);
  });

  it("ignores the time of day — any instant in a day maps to the same bounds", () => {
    const morning = dayBounds(new Date(2026, 5, 17, 0, 0, 1));
    const night = dayBounds(new Date(2026, 5, 17, 23, 59, 59));
    expect(morning).toEqual(night);
  });

  it("rolls the end into the next month at a month boundary", () => {
    const { start, end } = dayBounds(new Date(2026, 5, 30, 9, 0)); // 30 June
    expect(new Date(start * 1000).getMonth()).toBe(5); // June
    expect(new Date(end * 1000).getMonth()).toBe(6); // July
    expect(new Date(end * 1000).getDate()).toBe(1);
  });

  it("a day's end equals the next day's start (contiguous, no gap or overlap)", () => {
    const day = new Date(2026, 5, 17);
    const next = addDays(day, 1);
    expect(dayBounds(day).end).toBe(dayBounds(next).start);
  });
});

describe("week helpers", () => {
  it("weekDays returns 7 contiguous days starting Sunday", () => {
    const days = weekDays(new Date(2026, 5, 17)); // a Wednesday
    expect(days).toHaveLength(7);
    expect(days[0].getDay()).toBe(0); // Sunday
    expect(isSameDay(days[3], new Date(2026, 5, 17))).toBe(true); // Wed is index 3
  });
});
