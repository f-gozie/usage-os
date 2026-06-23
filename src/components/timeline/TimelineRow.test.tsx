// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { TimelineRow } from "./TimelineRow";
import type { TimelineRun } from "@/lib/tauri";

const RUN: TimelineRun = {
  context_slug: "deep",
  context_name: "Deep work",
  start: 1_000_000_000,
  end: 1_000_006_120,
  secs: 6120,
  projects: [
    { name: "usageos", secs: 4000 },
    { name: "nudge", secs: 2120 },
  ],
  apps: ["Cursor", "iTerm", "Claude"],
  segments: [
    { start: 1_000_000_000, end: 1_000_001_800, app: "Cursor", project: "usageos", secs: 1800 },
    { start: 1_000_001_800, end: 1_000_003_400, app: "iTerm", project: "usageos", secs: 1600 },
    { start: 1_000_003_400, end: 1_000_004_600, app: "Cursor", project: "nudge", secs: 1200 },
    { start: 1_000_004_600, end: 1_000_006_120, app: "Claude", project: null, secs: 1520 },
  ],
};

describe("TimelineRow", () => {
  it("shows the run summary and keeps the detail collapsed by default", () => {
    const { getByText, queryByText, getByRole } = render(<TimelineRow run={RUN} />);
    expect(getByText("Deep work")).toBeInTheDocument();
    expect(getByRole("button")).toHaveAttribute("aria-expanded", "false");
    expect(queryByText(/app switch/i)).toBeNull(); // segments hidden
  });

  it("expands to reveal every app-switch segment on click", () => {
    const { getByRole, getByText } = render(<TimelineRow run={RUN} />);
    fireEvent.click(getByRole("button"));
    expect(getByRole("button")).toHaveAttribute("aria-expanded", "true");
    expect(getByText(/4 app switches/i)).toBeInTheDocument();
  });
});
