/**
 * The colour axis is category (D34): the five canonical category slugs each map to a
 * theme-aware token; anything else (user-created or uncategorized) gets a neutral
 * tone. Colour encodes category ONLY — never a project. Returns a CSS `var(...)` so it
 * resolves against the active theme.
 *
 * Slugs are the stable identity (D46); the display names below are the relatable
 * defaults (D47) — `deep`→Work, `research`→Browsing, `comms`→Messaging,
 * `breaks`→Entertainment, plus `personal`→Personal as the fifth.
 */
/** The five canonical categories, in the fixed display order used by the legend. */
export const CANONICAL_CATEGORIES: ReadonlyArray<{ slug: string; name: string }> = [
  { slug: "deep", name: "Work" },
  { slug: "research", name: "Browsing" },
  { slug: "comms", name: "Messaging" },
  { slug: "breaks", name: "Entertainment" },
  { slug: "personal", name: "Personal" },
];

const CANONICAL_SLUGS = CANONICAL_CATEGORIES.map((c) => c.slug);

export function categoryColorVar(slug: string): string {
  return CANONICAL_SLUGS.includes(slug) ? `var(--c-${slug})` : "var(--muted)";
}
