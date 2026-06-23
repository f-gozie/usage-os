// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render } from "@testing-library/react";

import { MiniDial } from "./MiniDial";
import type { ContextRun } from "@/lib/tauri";

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

describe("MiniDial", () => {
  it("renders a casing + a coloured arc per run", () => {
    const { container } = render(<MiniDial runs={RUNS} dayStartUnix={0} nowMinutes={null} />);
    expect(container.querySelectorAll("path")).toHaveLength(RUNS.length * 2);
    expect(container.querySelector("polygon")).toBeNull(); // no now-triangle on a past day
  });

  it("draws the now-triangle when nowMinutes is set (today)", () => {
    const { container } = render(<MiniDial runs={RUNS} dayStartUnix={0} nowMinutes={540} />);
    expect(container.querySelector("polygon")).toBeInTheDocument();
  });

  it("uses the label for the accessible name", () => {
    const { getByRole } = render(<MiniDial runs={[]} dayStartUnix={0} label="Mon 16" />);
    expect(getByRole("img", { name: "Mon 16 activity" })).toBeInTheDocument();
  });
});
