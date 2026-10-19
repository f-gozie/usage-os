// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { Dial } from "./Dial";
import type { CategoryRun } from "@/lib/tauri";
import { formatClock } from "@/lib/format";

// Skip the imperative draw-in (jsdom has no SVGPathElement.getTotalLength).
vi.mock("@/hooks/useReducedMotion", () => ({ useReducedMotion: () => true }));

const RUNS: CategoryRun[] = [
  {
    category_slug: "deep",
    category_name: "Deep work",
    category_color: null,
    start: 0,
    end: 3600,
    secs: 3600,
    projects: [{ name: "usageos", secs: 3600 }],
    apps: ["Cursor"],
  },
  {
    category_slug: "comms",
    category_name: "Comms",
    category_color: null,
    start: 3600,
    end: 4200,
    secs: 600,
    projects: [{ name: "No project", secs: 600 }],
    apps: ["Slack"],
  },
];

describe("Dial", () => {
  it("renders a casing + a coloured arc per run, and the centre figure", () => {
    const { container, getByText } = render(
      <Dial runs={RUNS} dayStartUnix={0} nowMinutes={null} activeLabel="1h 10m" />,
    );
    expect(container.querySelectorAll("path")).toHaveLength(RUNS.length * 2);
    expect(getByText("1h 10m")).toBeInTheDocument();
  });

  it("selects the run when its arc is clicked", () => {
    const onSelect = vi.fn();
    const { container } = render(
      <Dial runs={RUNS} dayStartUnix={0} nowMinutes={null} activeLabel="1h 10m" onSelectRun={onSelect} />,
    );
    const paths = container.querySelectorAll("path");
    // Each run is [casing, coloured]; the coloured arc (index 1) carries the handler.
    fireEvent.click(paths[1]);
    expect(onSelect).toHaveBeenCalledWith(RUNS[0]);
  });
});

// A local midnight; the dial's angular origin and the base for every offset below.
const MIDNIGHT = new Date(2026, 8, 16, 0, 0, 0).getTime() / 1000;
const at = (hours: number) => MIDNIGHT + hours * 3600;

const base = {
  runs: [] as CategoryRun[],
  dayStartUnix: MIDNIGHT,
  nowMinutes: null,
  activeLabel: "0m",
};

/** The blind-stretch arcs, found by the title they carry (their only accessible name). */
function gapTitles(container: HTMLElement): string[] {
  return [...container.querySelectorAll("path > title")]
    .map((t) => t.textContent ?? "")
    .filter((t) => t.includes("wasn’t running"));
}

describe("Dial coverage gaps", () => {
  it("draws nothing extra on a day that was fully watched", () => {
    const { container } = render(<Dial {...base} gaps={[]} />);
    expect(gapTitles(container)).toHaveLength(0);
  });

  it("draws a blind stretch and names the hours it covers", () => {
    const { container } = render(<Dial {...base} gaps={[{ start: at(17.5), end: at(23) }]} />);

    const titles = gapTitles(container);
    expect(titles).toHaveLength(1);
    expect(titles[0]).toBe(
      `UsageOS wasn’t running · ${formatClock(at(17.5))}–${formatClock(at(23))}`,
    );
  });

  it("uses a broken track, not a coloured arc — an absence, never an alarm", () => {
    const { container } = render(<Dial {...base} gaps={[{ start: at(2), end: at(9) }]} />);

    const gap = [...container.querySelectorAll("path")].find((p) =>
      p.querySelector("title")?.textContent?.includes("wasn’t running"),
    );
    expect(gap).toBeDefined();
    expect(gap).toHaveAttribute("stroke", "var(--bg)");
    expect(gap).toHaveAttribute("stroke-dasharray", "3 7");
  });

  it("draws a whole day nobody watched as a full broken ring, not an empty arc", () => {
    const { container } = render(<Dial {...base} gaps={[{ start: at(0), end: at(24) }]} />);

    const gap = [...container.querySelectorAll("path")].find((p) =>
      p.querySelector("title")?.textContent?.includes("wasn’t running"),
    );
    // Two half-circle arcs; a single arc from midnight to midnight would draw nothing.
    expect(gap?.getAttribute("d")?.match(/A/g)).toHaveLength(2);
  });

  it("skips a stretch too short to read, so a reboot leaves no speck", () => {
    const { container } = render(
      <Dial {...base} gaps={[{ start: at(12), end: at(12) + 120 }]} />,
    );
    expect(gapTitles(container)).toHaveLength(0);
  });

  it("draws every blind stretch of a day that lost the app more than once", () => {
    const { container } = render(
      <Dial
        {...base}
        gaps={[
          { start: at(1), end: at(6) },
          { start: at(14), end: at(18) },
        ]}
      />,
    );
    expect(gapTitles(container)).toHaveLength(2);
  });
});
