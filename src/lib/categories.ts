/**
 * The colour axis is category (D34); colour encodes category only, never a project. The
 * canonical list below is the source of truth for slug + legend order; the DB row's `name`
 * is the source of truth for what's shown (a user can rename a canonical category — D47).
 */
/** The five canonical categories, in the fixed legend order. `name` is the default label,
 *  overridden by the DB name when one exists (see `categoryDisplayName`). */
export const CANONICAL_CATEGORIES: ReadonlyArray<{ slug: string; name: string }> = [
  { slug: "deep", name: "Work" },
  { slug: "research", name: "Browsing" },
  { slug: "comms", name: "Messaging" },
  { slug: "breaks", name: "Entertainment" },
  { slug: "personal", name: "Personal" },
];

const CANONICAL_SLUGS = CANONICAL_CATEGORIES.map((c) => c.slug);

/** The label to show for a canonical slug: the user's DB name if present, else the default.
 *  `dbNames` maps slug → the current DB display name. */
export function categoryDisplayName(
  slug: string,
  dbNames?: ReadonlyMap<string, string>,
): string {
  return (
    dbNames?.get(slug) ?? CANONICAL_CATEGORIES.find((c) => c.slug === slug)?.name ?? slug
  );
}

/**
 * Resolve a category's render colour. Canonical slug → theme token; otherwise the category's
 * own hex (`color`, set for user-created categories); else the neutral token for uncategorized.
 */
export function categoryColorVar(slug: string, color?: string | null): string {
  if (CANONICAL_SLUGS.includes(slug)) return `var(--c-${slug})`;
  if (color) return color;
  return "var(--muted)";
}
