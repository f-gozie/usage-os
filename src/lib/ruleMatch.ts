import type { Rule } from "@/lib/tauri";

// UI-side mirror of the Rust matcher (`db::find_category`) for PROCESS rules, used to
// warn about conflicts in the category editor BEFORE saving. The Rust `reprocess_logs`
// stays the source of truth — this never sorts anything, it only explains what would
// happen. Title rules are ignored here (they match window titles, not app names).

/** The first rule (by id order = priority) whose pattern is a case-insensitive
 *  substring of `appName`, or `null` if none match. Mirrors first-match-wins. */
export function processOwner(appName: string, rules: Rule[]): Rule | null {
  const name = appName.toLowerCase();
  const byPriority = [...rules].sort((a, b) => a.id - b.id);
  for (const r of byPriority) {
    if (r.match_field !== "process") continue;
    if (r.pattern && name.includes(r.pattern.toLowerCase())) return r;
  }
  return null;
}
