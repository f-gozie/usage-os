import { describe, it, expect } from "vitest";

import { addDays, dayBounds, isSameDay, weekDays, monthDays, yearDays, startOfMonth, addMonths, addYears, isSameMonth } from "./dates";

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

describe("month and year ranges", () => {
  it("gives every day of a month, including a leap February", () => {
    expect(monthDays(new Date(2026, 8, 15))).toHaveLength(30); // September
    expect(monthDays(new Date(2026, 1, 15))).toHaveLength(28); // Feb 2026
    expect(monthDays(new Date(2028, 1, 15))).toHaveLength(29); // Feb 2028, a leap year
  });

  it("starts the month on the 1st at local midnight", () => {
    const first = monthDays(new Date(2026, 8, 20))[0];
    expect(first.getDate()).toBe(1);
    expect(first.getHours()).toBe(0);
    expect(startOfMonth(new Date(2026, 8, 20)).getTime()).toBe(first.getTime());
  });

  it("gives every day of a year", () => {
    expect(yearDays(new Date(2026, 5, 1))).toHaveLength(365);
    expect(yearDays(new Date(2028, 5, 1))).toHaveLength(366);
  });

  it("orders the year from 1 January to 31 December", () => {
    const days = yearDays(new Date(2026, 5, 1));
    expect(days[0].getMonth()).toBe(0);
    expect(days[0].getDate()).toBe(1);
    expect(days[days.length - 1].getMonth()).toBe(11);
    expect(days[days.length - 1].getDate()).toBe(31);
  });

  it("clamps the day when stepping into a shorter month", () => {
    // 31 January + 1 month is February, not March 3rd.
    const stepped = addMonths(new Date(2026, 0, 31), 1);
    expect(stepped.getMonth()).toBe(1);
    expect(stepped.getDate()).toBe(28);
  });

  it("steps months backwards across a year boundary", () => {
    const stepped = addMonths(new Date(2026, 0, 15), -1);
    expect(stepped.getFullYear()).toBe(2025);
    expect(stepped.getMonth()).toBe(11);
  });

  it("clamps 29 February when stepping into a common year", () => {
    const stepped = addYears(new Date(2028, 1, 29), 1);
    expect(stepped.getMonth()).toBe(1);
    expect(stepped.getDate()).toBe(28);
  });

  it("knows whether two dates share a month", () => {
    expect(isSameMonth(new Date(2026, 8, 1), new Date(2026, 8, 30))).toBe(true);
    expect(isSameMonth(new Date(2026, 8, 30), new Date(2026, 9, 1))).toBe(false);
    expect(isSameMonth(new Date(2025, 8, 1), new Date(2026, 8, 1))).toBe(false);
  });
});
