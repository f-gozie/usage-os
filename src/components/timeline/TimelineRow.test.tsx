// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { TimelineRow } from "./TimelineRow";
import type { TimelineRun } from "@/lib/tauri";

const deep = { category_slug: "deep", category_name: "Deep work", category_color: null };

// A Deep run that absorbed a brief Comms detour (D34a).
const RUN: TimelineRun = {
  category_slug: "deep",
  category_name: "Deep work",
  category_color: null,
  start: 1_000_000_000,
  end: 1_000_006_120,
  secs: 4600, // host (Deep) only
  projects: [{ name: "usageos", secs: 4600 }],
  apps: ["Cursor", "iTerm"],
  segments: [
    { start: 1_000_000_000, end: 1_000_001_800, app: "Cursor", ...deep, project: "usageos", secs: 1800 },
    { start: 1_000_001_800, end: 1_000_003_400, app: "iTerm", ...deep, project: "usageos", secs: 1600 },
    { start: 1_000_003_400, end: 1_000_003_940, app: "Slack", category_slug: "comms", category_name: "Comms", category_color: null, project: null, secs: 540 },
    { start: 1_000_003_940, end: 1_000_006_120, app: "Cursor", ...deep, project: "usageos", secs: 1200 },
  ],
};

describe("TimelineRow", () => {
  it("shows the run summary and keeps the detail collapsed by default", () => {
    const { getByText, queryByText, getByRole } = render(<TimelineRow run={RUN} />);
    expect(getByText("Deep work")).toBeInTheDocument();
    expect(getByRole("button")).toHaveAttribute("aria-expanded", "false");
    expect(queryByText(/app stretch/i)).toBeNull(); // segments hidden
  });

  it("expands to reveal every app stretch, marking the absorbed detour", () => {
    const { getByRole, getByText, getByTitle } = render(<TimelineRow run={RUN} />);
    fireEvent.click(getByRole("button"));
    expect(getByRole("button")).toHaveAttribute("aria-expanded", "true");
    expect(getByText(/4 app stretches/i)).toBeInTheDocument();
    // the absorbed Comms detour carries its own category marker
    expect(getByTitle("Comms")).toBeInTheDocument();
  });
});
