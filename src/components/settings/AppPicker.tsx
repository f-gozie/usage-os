import { useState } from "react";

import { AppIcon } from "@/components/ui/AppIcon";
import { categoryColorVar } from "@/lib/categories";
import type { InstalledApp } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export interface AppPickerProps {
  apps: InstalledApp[];
  /** Lowercased names currently selected for this category. */
  selected: Set<string>;
  onToggle: (name: string) => void;
  /** For an app already sorted by ANOTHER category: its name + slug, to dim + mark it. */
  conflictFor?: (name: string) => { name: string; slug: string } | null;
}

/** Pick apps from the user's Mac (with real icons) instead of typing names blind.
 *  Selected apps become process rules; an app owned by another category is dimmed and
 *  marked, and toggling it raises the conflict warning the editor shows. */
export function AppPicker({ apps, selected, onToggle, conflictFor }: AppPickerProps) {
  const [q, setQ] = useState("");
  const query = q.trim().toLowerCase();
  const filtered = query ? apps.filter((a) => a.name.toLowerCase().includes(query)) : apps;
  const chosen = apps.filter((a) => selected.has(a.name.toLowerCase()));

  return (
    <div>
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Search your apps…"
        className="mb-2 w-full border-2 border-edge bg-bg px-3 py-2 text-sm font-semibold text-fg placeholder:font-medium placeholder:text-muted focus:border-c-deep focus:outline-none"
      />
      <div className="grid max-h-52 grid-cols-[repeat(auto-fill,minmax(92px,1fr))] gap-2 overflow-auto border-2 border-edge bg-bg p-2">
        {filtered.map((a) => {
          const isSel = selected.has(a.name.toLowerCase());
          const conflict = isSel ? null : (conflictFor?.(a.name) ?? null);
          return (
            <button
              type="button"
              key={a.name}
              onClick={() => onToggle(a.name)}
              title={conflict ? `Already in ${conflict.name}` : a.name}
              className={cn(
                "relative flex flex-col items-center gap-1.5 border-2 border-transparent p-2 text-fg",
                isSel ? "border-c-deep" : "hover:border-rule",
                conflict && "opacity-60",
              )}
            >
              <AppIcon name={a.name} size={32} />
              <span className="line-clamp-2 text-center text-[11px] font-semibold leading-tight">
                {a.name}
              </span>
              {isSel && (
                <span className="absolute right-1 top-0.5 text-[11px] font-bold text-c-deep">✓</span>
              )}
              {conflict && (
                <span
                  className="absolute right-1 top-1 h-2.5 w-2.5 border border-edge"
                  style={{ background: categoryColorVar(conflict.slug) }}
                />
              )}
            </button>
          );
        })}
        {filtered.length === 0 && (
          <p className="col-span-full p-2 text-xs text-muted">
            {apps.length === 0 ? "Loading your apps…" : `No apps match “${q}”.`}
          </p>
        )}
      </div>

      {chosen.length > 0 && (
        <div className="mt-2.5 flex flex-wrap gap-1.5">
          {chosen.map((a) => (
            <span
              key={a.name}
              className="inline-flex items-center gap-1.5 border-2 border-edge bg-bg px-2 py-0.5 text-[12.5px] font-semibold"
            >
              <AppIcon name={a.name} size={16} />
              {a.name}
              <button
                type="button"
                aria-label={`Remove ${a.name}`}
                onClick={() => onToggle(a.name)}
                className="text-muted hover:text-c-research"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
