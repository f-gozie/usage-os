import { describe, expect, it } from "vitest";

import { pickProblem } from "@/hooks/useCaptureHealth";

const noop = () => undefined;
const actions = { accessibility: noop, automation: noop, retry: noop };

describe("pickProblem", () => {
  it("is null when everything is fine", () => {
    expect(pickProblem(true, true, true, actions)).toBeNull();
  });

  it("lost Accessibility outranks everything — titles are gone for every app", () => {
    expect(pickProblem(false, false, false, actions)?.kind).toBe("accessibility");
  });

  it("lost Automation shows when titles are fine", () => {
    expect(pickProblem(true, false, true, actions)?.kind).toBe("automation");
  });

  it("repeated capture errors are the fallback", () => {
    expect(pickProblem(true, true, false, actions)?.kind).toBe("errors");
  });

  it("permission copy says to toggle off and on, then relaunch (a live toggle doesn't reach the running process)", () => {
    const p = pickProblem(false, true, true, actions);
    expect(p?.description).toMatch(/off and back on/);
    expect(p?.description).toMatch(/quit and reopen/);
    expect(pickProblem(true, false, true, actions)?.description).toMatch(/quit and reopen/);
  });
});
