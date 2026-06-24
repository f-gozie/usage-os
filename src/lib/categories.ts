/**
 * The colour axis is category (D34): the four canonical category slugs each map to a
 * theme-aware token; anything else (user-created or uncategorized) gets a neutral
 * tone. Colour encodes category ONLY — never a project. Returns a CSS `var(...)` so it
 * resolves against the active theme.
 */
/** The four canonical categories, in the fixed display order used by the legend. */
export const CANONICAL_CATEGORIES: ReadonlyArray<{ slug: string; name: string }> = [
  { slug: "deep", name: "Deep work" },
  { slug: "research", name: "Research" },
  { slug: "comms", name: "Comms" },
  { slug: "breaks", name: "Breaks" },
];

const CANONICAL_SLUGS = CANONICAL_CATEGORIES.map((c) => c.slug);

export function categoryColorVar(slug: string): string {
  return CANONICAL_SLUGS.includes(slug) ? `var(--c-${slug})` : "var(--muted)";
}
