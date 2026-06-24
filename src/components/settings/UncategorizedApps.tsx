import { useEffect, useState } from "react";

import { AppIcon } from "@/components/ui/AppIcon";
import { loadIconMap, resolveSuggestedSlug } from "@/lib/appIcons";
import { categoryColorVar } from "@/lib/categories";
import { formatDuration } from "@/lib/format";
import { createRule, reprocessLogs, type Category, type UncategorizedApp } from "@/lib/tauri";

export interface UncategorizedAppsProps {
  apps: UncategorizedApp[];
  categories: Category[];
  /** Re-fetch settings data after an assignment lands. */
  onAssigned: () => void | Promise<void>;
  /** Open the editor to make a new category (the app to seed it with). */
  onNewCategory: (appName: string) => void;
  onError: (message: string) => void;
}

/** Apps you've used that aren't in a category yet — sort them in one click. Assigning
 *  writes a process rule then reprocesses, so it re-sorts every past day at once. */
export function UncategorizedApps({
  apps,
  categories,
  onAssigned,
  onNewCategory,
  onError,
}: UncategorizedAppsProps) {
  const [openFor, setOpenFor] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  // The app catalog drives the category suggestion (from each app's Apple-assigned
  // category, D47). Load once, then re-render so suggestions appear.
  const [catalogReady, setCatalogReady] = useState(false);
  useEffect(() => {
    let alive = true;
    void loadIconMap().then(() => alive && setCatalogReady(true));
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (!openFor) return;
    const onDoc = (e: MouseEvent) => {
      if (!(e.target as HTMLElement).closest("[data-assign-menu]")) setOpenFor(null);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [openFor]);

  if (apps.length === 0) {
    return (
      <p className="px-4 py-3.5 text-[13px] text-muted">
        Nothing unsorted — every app you've used is already in a category.
      </p>
    );
  }

  const assign = async (appName: string, categoryId: number) => {
    setOpenFor(null);
    setBusy(appName);
    try {
      await createRule(categoryId, "process", appName);
      await reprocessLogs();
      await onAssigned();
    } catch (e) {
      onError(e instanceof Error ? e.message : "Couldn't sort that app.");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      {apps.map((a) => {
        const suggestedSlug = catalogReady ? resolveSuggestedSlug(a.process_name) : undefined;
        const suggested = suggestedSlug
          ? categories.find((c) => c.slug === suggestedSlug)
          : undefined;
        return (
        <div
          key={a.process_name}
          className="flex items-center gap-3 border-t-2 border-edge px-4 py-3 first:border-t-0"
        >
          <AppIcon name={a.process_name} size={26} />
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">{a.process_name}</div>
            <div className="text-xs text-muted">
              {formatDuration(a.total_secs)} all time · showing as Other
            </div>
          </div>
          <div className="relative ml-auto flex items-center gap-2" data-assign-menu>
            {suggested && (
              <button
                type="button"
                disabled={busy === a.process_name}
                onClick={() => assign(a.process_name, suggested.id)}
                title="Suggested from this app’s category"
                className="flex items-center gap-1.5 border-2 border-edge bg-fg px-2.5 py-[7px] text-[11.5px] font-semibold uppercase tracking-[0.04em] text-bg disabled:opacity-50"
              >
                <span
                  className="h-2.5 w-2.5 shrink-0 border border-bg"
                  style={{
                    background: suggested.slug ? categoryColorVar(suggested.slug) : suggested.color,
                  }}
                />
                {suggested.name}
              </button>
            )}
            <button
              type="button"
              disabled={busy === a.process_name}
              onClick={() => setOpenFor((f) => (f === a.process_name ? null : a.process_name))}
              className="flex items-center gap-1.5 border-2 border-edge bg-bg px-2.5 py-[7px] text-[11.5px] font-semibold uppercase tracking-[0.04em] text-fg disabled:opacity-50"
            >
              {busy === a.process_name ? "Sorting…" : "Assign ▾"}
            </button>
            {openFor === a.process_name && (
              <div className="absolute right-0 top-[calc(100%+4px)] z-10 min-w-[184px] border-2 border-edge bg-bg">
                {categories.map((c) => (
                  <button
                    key={c.id}
                    type="button"
                    onClick={() => assign(a.process_name, c.id)}
                    className="flex w-full items-center gap-2.5 border-t border-rule px-3 py-2.5 text-left text-[12.5px] font-semibold first:border-t-0 hover:bg-surface"
                  >
                    <span
                      className="h-3 w-3 shrink-0 border border-edge"
                      style={{ background: c.slug ? categoryColorVar(c.slug) : c.color }}
                    />
                    {c.name}
                  </button>
                ))}
                <button
                  type="button"
                  onClick={() => {
                    setOpenFor(null);
                    onNewCategory(a.process_name);
                  }}
                  className="flex w-full items-center gap-2.5 border-t border-rule px-3 py-2.5 text-left text-[12.5px] font-semibold hover:bg-surface"
                >
                  <span className="h-3 w-3 shrink-0 border border-dashed border-edge" />
                  New category…
                </button>
              </div>
            )}
          </div>
        </div>
        );
      })}
    </div>
  );
}
