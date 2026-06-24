import { describe, it, expect } from "vitest";

import { processOwner } from "./ruleMatch";
import type { Rule } from "@/lib/tauri";

function rule(id: number, category_id: number, pattern: string, match_field = "process"): Rule {
  return { id, category_id, match_field, pattern, ignore_title: false };
}

describe("processOwner", () => {
  it("returns the first matching process rule by id order (first-match-wins)", () => {
    // "Google Chrome" matches both; the lower id (Google, ctx 20) wins, even out of order.
    const rules = [rule(2, 30, "chrome"), rule(1, 20, "Google")];
    expect(processOwner("Google Chrome", rules)?.category_id).toBe(20);
  });

  it("matches case-insensitive substrings", () => {
    expect(processOwner("Visual Studio Code", [rule(1, 10, "code")])?.category_id).toBe(10);
  });

  it("ignores title rules (they match window titles, not app names)", () => {
    expect(processOwner("Slack", [rule(1, 10, "slack", "title")])).toBeNull();
  });

  it("returns null when nothing matches", () => {
    expect(processOwner("Spotify", [rule(1, 10, "slack")])).toBeNull();
  });
});
