// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { Dial } from "./Dial";
import type { ContextRun } from "@/lib/tauri";

// Skip the imperative draw-in (jsdom has no SVGPathElement.getTotalLength).
vi.mock("@/hooks/useReducedMotion", () => ({ useReducedMotion: () => true }));

const RUNS: ContextRun[] = [
  {
    context_slug: "deep",
    context_name: "Deep work",
    start: 0,
    end: 3600,
    secs: 3600,
    projects: [{ name: "usageos", secs: 3600 }],
    apps: ["Cursor"],
  },
  {
    context_slug: "comms",
    context_name: "Comms",
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
