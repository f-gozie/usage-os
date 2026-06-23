import { describe, it, expect } from "vitest";
import { polar, arcPath, minutesSinceMidnight, DIAL_CENTER } from "./geometry";

describe("polar", () => {
  it("places midnight at the top of the dial", () => {
    const [x, y] = polar(DIAL_CENTER, DIAL_CENTER, 0, 100);
    expect(x).toBeCloseTo(DIAL_CENTER, 5);
    expect(y).toBeCloseTo(DIAL_CENTER - 100, 5); // straight up
  });

  it("places 06:00 at the right (clockwise)", () => {
    const [x, y] = polar(DIAL_CENTER, DIAL_CENTER, 6 * 60, 100);
    expect(x).toBeCloseTo(DIAL_CENTER + 100, 5);
    expect(y).toBeCloseTo(DIAL_CENTER, 5);
  });

  it("places noon at the bottom", () => {
    const [x, y] = polar(DIAL_CENTER, DIAL_CENTER, 12 * 60, 100);
    expect(x).toBeCloseTo(DIAL_CENTER, 5);
    expect(y).toBeCloseTo(DIAL_CENTER + 100, 5);
  });
});

describe("arcPath", () => {
  it("uses the large-arc flag only past the half-day mark", () => {
    expect(arcPath(150, 150, 99, 0, 600)).toContain(" 0 1 "); // 10h < 12h
    expect(arcPath(150, 150, 99, 0, 800)).toContain(" 1 1 "); // 13h20 > 12h
  });
});

describe("minutesSinceMidnight", () => {
  it("converts a Unix second offset to minutes from the day start", () => {
    const dayStart = 1_700_000_000;
    expect(minutesSinceMidnight(dayStart, dayStart)).toBe(0);
    expect(minutesSinceMidnight(dayStart + 3600, dayStart)).toBe(60);
  });
});
