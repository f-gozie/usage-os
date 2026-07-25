import { describe, expect, it } from "vitest";

import { parseAdvancedRule } from "@/components/settings/CategoryEditorModal";

describe("parseAdvancedRule", () => {
  it("reads a bare hostname as a site rule", () => {
    expect(parseAdvancedRule("youtube.com")).toEqual({
      match_field: "site",
      pattern: "youtube.com",
    });
    expect(parseAdvancedRule(" news.ycombinator.com ")).toEqual({
      match_field: "site",
      pattern: "news.ycombinator.com",
    });
  });

  it("reads anything else as a title rule", () => {
    expect(parseAdvancedRule("invoice")).toEqual({ match_field: "title", pattern: "invoice" });
    // Dotted but with a space — a filename fragment, not a host.
    expect(parseAdvancedRule("Q3 report.pdf")).toEqual({
      match_field: "title",
      pattern: "Q3 report.pdf",
    });
    // A leading-dot extension is a title fragment, not a host.
    expect(parseAdvancedRule(".fig")).toEqual({ match_field: "title", pattern: ".fig" });
  });

  it("honours an explicit prefix over the guess", () => {
    expect(parseAdvancedRule("title: youtube.com")).toEqual({
      match_field: "title",
      pattern: "youtube.com",
    });
    expect(parseAdvancedRule("site: invoice")).toEqual({
      match_field: "site",
      pattern: "invoice",
    });
    expect(parseAdvancedRule("SITE:Youtube.com")).toEqual({
      match_field: "site",
      pattern: "Youtube.com",
    });
  });

  it("strips leading dots from a site pattern", () => {
    // `.youtube.com` and `youtube.com` mean the same thing to the matcher.
    expect(parseAdvancedRule("site: .youtube.com")).toEqual({
      match_field: "site",
      pattern: "youtube.com",
    });
  });

  it("returns null for empty or prefix-only input", () => {
    expect(parseAdvancedRule("")).toBeNull();
    expect(parseAdvancedRule("   ")).toBeNull();
    expect(parseAdvancedRule("title:")).toBeNull();
    expect(parseAdvancedRule("site:   ")).toBeNull();
  });
});
