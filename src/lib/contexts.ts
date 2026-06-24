/**
 * The colour axis is context (D34): the four canonical context slugs each map to a
 * theme-aware token; anything else (user-created or uncategorized) gets a neutral
 * tone. Colour encodes context ONLY — never a project. Returns a CSS `var(...)` so it
 * resolves against the active theme.
 */
/** The four canonical contexts, in the fixed display order used by the legend. */
export const CANONICAL_CONTEXTS: ReadonlyArray<{ slug: string; name: string }> = [
  { slug: "deep", name: "Deep work" },
  { slug: "research", name: "Research" },
  { slug: "comms", name: "Comms" },
  { slug: "breaks", name: "Breaks" },
];

const CANONICAL_SLUGS = CANONICAL_CONTEXTS.map((c) => c.slug);

export function contextColorVar(slug: string): string {
  return CANONICAL_SLUGS.includes(slug) ? `var(--c-${slug})` : "var(--muted)";
}
