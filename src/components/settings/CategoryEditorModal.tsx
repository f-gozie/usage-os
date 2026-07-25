import { useEffect, useState } from "react";

import { AppPicker } from "@/components/settings/AppPicker";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { Swatches } from "@/components/ui/Swatches";
import { TextInput } from "@/components/ui/TextInput";
import { useInstalledApps } from "@/hooks/useInstalledApps";
import { categoryColorVar } from "@/lib/categories";
import { processOwner } from "@/lib/ruleMatch";
import {
  createCategory,
  createRule,
  deleteCategory,
  deleteRule,
  reprocessLogs,
  updateCategory,
  type Category,
  type Rule,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

// The colour-picker choices for a user category — intentional literals (the user picks an
// actual stored hex), not theme tokens. Canonical categories take their theme token instead.
const PALETTE = ["#1B45BE", "#E0241B", "#EAB308", "#7A4FC2", "#1D9E75", "#161616"] as const;

interface PendingRule {
  match_field: string;
  pattern: string;
}

interface Conflict {
  app: string;
  rule: Rule;
  ownerName: string;
}

export interface CategoryEditorModalProps {
  open: boolean;
  /** `null` = add a new category; otherwise edit this one. */
  category: Category | null;
  /** A process name to pre-add as a rule (from "New category…" in the Uncategorized list). */
  seedApp?: string | null;
  /** All rules (to detect cross-category conflicts) and all categories (to name them). */
  rules: Rule[];
  categories: Category[];
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}

/** A bare token that is unmistakably a hostname: dotted, no whitespace, no path or scheme.
 *  Lets `youtube.com` be typed without the `site:` prefix while `invoice` stays a title. */
const HOSTNAME = /^[a-z0-9-]+(\.[a-z0-9-]+)+$/i;

/** Parse a token from the Advanced drawer into a site or title rule. An explicit
 *  `site:` / `title:` prefix always wins; a bare hostname is read as a site rule and
 *  anything else as a title rule. Returns `null` when empty. */
export function parseAdvancedRule(token: string): PendingRule | null {
  const raw = token.trim();
  if (!raw) return null;

  const site = raw.match(/^site:\s*(.*)$/i);
  if (site) {
    const pattern = site[1].trim().replace(/^\.+/, "");
    return pattern ? { match_field: "site", pattern } : null;
  }

  const t = raw.replace(/^title:\s*/i, "").trim();
  if (!t) return null;
  const bare = !/^title:/i.test(raw);
  return { match_field: bare && HOSTNAME.test(t) ? "site" : "title", pattern: t };
}

/** How a non-app rule reads on its chip. */
function ruleChipLabel(r: { match_field: string; pattern: string }): string {
  if (r.match_field === "title") return `title: ${r.pattern}`;
  if (r.match_field === "site") return `site: ${r.pattern}`;
  return r.pattern;
}

export function CategoryEditorModal({
  open,
  category,
  seedApp,
  rules,
  categories,
  onClose,
  onSaved,
}: CategoryEditorModalProps) {
  const isCanonical = category?.slug != null;
  const apps = useInstalledApps();

  const [name, setName] = useState("");
  const [color, setColor] = useState<string>(PALETTE[3]);
  const [removedIds, setRemovedIds] = useState<Set<number>>(new Set());
  const [movedIds, setMovedIds] = useState<Set<number>>(new Set()); // other-category rules to delete (a "move")
  const [added, setAdded] = useState<PendingRule[]>([]);
  const [conflict, setConflict] = useState<Conflict | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [ruleInput, setRuleInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // (Re)initialize whenever the target category changes / the modal opens. A `seedApp`
  // (from "New category…" in the Uncategorized list) pre-adds that app so the one you
  // were sorting lands in the new category.
  useEffect(() => {
    if (!open) return;
    setName(category?.name ?? "");
    setColor(category?.slug ? categoryColorVar(category.slug) : (category?.color ?? PALETTE[3]));
    setRemovedIds(new Set());
    setMovedIds(new Set());
    setAdded(seedApp ? [{ match_field: "process", pattern: seedApp }] : []);
    setConflict(null);
    setShowAdvanced(false);
    setRuleInput("");
    setError(null);
  }, [open, category, seedApp]);

  const appNames = new Set(apps.map((a) => a.name.toLowerCase()));
  const existingThis = category ? rules.filter((r) => r.category_id === category.id) : [];
  const workingExisting = existingThis.filter((r) => !removedIds.has(r.id));

  // Process rules currently sorting into this category (existing kept + newly added).
  const selected = new Set<string>([
    ...workingExisting.filter((r) => r.match_field === "process").map((r) => r.pattern.toLowerCase()),
    ...added.filter((r) => r.match_field === "process").map((r) => r.pattern.toLowerCase()),
  ]);

  // Site + title rules, plus process rules that aren't a plain installed-app name (e.g.
  // seeded substrings like "Code") — shown as editable chips, not in the picker grid.
  const isChipRule = (r: { match_field: string; pattern: string }) =>
    r.match_field !== "process" || !appNames.has(r.pattern.toLowerCase());
  const otherExisting = workingExisting.filter(isChipRule);
  const otherAdded = added.map((r, i) => ({ r, i })).filter(({ r }) => isChipRule(r));

  // The owner (in ANOTHER category) of an app, if any — drives the conflict marker/warning.
  const ownerElsewhere = (appName: string): { rule: Rule; ctx: Category | undefined } | null => {
    const others = rules.filter(
      (r) => (!category || r.category_id !== category.id) && !movedIds.has(r.id),
    );
    const owner = processOwner(appName, others);
    if (!owner) return null;
    return { rule: owner, ctx: categories.find((c) => c.id === owner.category_id) };
  };

  const conflictFor = (appName: string): { name: string; slug: string } | null => {
    if (selected.has(appName.toLowerCase())) return null;
    const owner = ownerElsewhere(appName);
    if (!owner) return null;
    return { name: owner.ctx?.name ?? "another category", slug: owner.ctx?.slug ?? "other" };
  };

  const toggleApp = (appName: string) => {
    const key = appName.toLowerCase();
    const ex = workingExisting.find(
      (r) => r.match_field === "process" && r.pattern.toLowerCase() === key,
    );
    if (ex) {
      setRemovedIds((s) => new Set(s).add(ex.id));
      return;
    }
    const idx = added.findIndex(
      (r) => r.match_field === "process" && r.pattern.toLowerCase() === key,
    );
    if (idx >= 0) {
      setAdded((a) => a.filter((_, j) => j !== idx));
      // If this app was a "move" (we queued the other category's rule for deletion),
      // un-queue that delete too — undoing the toggle must fully undo the move. Match the owner
      // rule by the same substring relationship `processOwner` used to find it (its pattern may
      // be a seeded substring like "Code" for "Visual Studio Code", not the full app name).
      const moved = rules.find(
        (r) =>
          movedIds.has(r.id) &&
          r.match_field === "process" &&
          r.pattern !== "" &&
          key.includes(r.pattern.toLowerCase()),
      );
      if (moved) {
        setMovedIds((s) => {
          const next = new Set(s);
          next.delete(moved.id);
          return next;
        });
      }
      return;
    }
    const owner = ownerElsewhere(appName);
    if (owner) {
      setConflict({ app: appName, rule: owner.rule, ownerName: owner.ctx?.name ?? "another category" });
      return;
    }
    setAdded((a) => [...a, { match_field: "process", pattern: appName }]);
  };

  const resolveConflict = (move: boolean) => {
    if (move && conflict) {
      setMovedIds((s) => new Set(s).add(conflict.rule.id));
      setAdded((a) => [...a, { match_field: "process", pattern: conflict.app }]);
    }
    setConflict(null);
  };

  const addAdvanced = () => {
    const parsed = ruleInput
      .split(",")
      .map(parseAdvancedRule)
      .filter((r): r is PendingRule => r !== null);
    if (parsed.length) setAdded((a) => [...a, ...parsed]);
    setRuleInput("");
  };

  const removeExisting = (id: number) => setRemovedIds((s) => new Set(s).add(id));
  const removeAdded = (i: number) => setAdded((a) => a.filter((_, j) => j !== i));

  const save = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Name can't be empty.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      let id = category?.id;
      if (category) {
        await updateCategory(category.id, trimmed, isCanonical ? category.color : color);
      } else {
        id = await createCategory(trimmed, color);
      }
      if (id == null) throw new Error("Missing category id");
      for (const r of added) await createRule(id, r.match_field, r.pattern, false);
      for (const rid of removedIds) await deleteRule(rid);
      for (const rid of movedIds) await deleteRule(rid); // a "move" deletes the other category's rule
      await reprocessLogs();
      await onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't save this category.");
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!category) return;
    setSaving(true);
    setError(null);
    try {
      await deleteCategory(category.id);
      await reprocessLogs();
      await onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't delete this category.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={category ? "Edit category" : "Add category"}
      footer={
        <>
          {category && !isCanonical && (
            <Button variant="danger" className="mr-auto" onClick={remove} disabled={saving}>
              Delete
            </Button>
          )}
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={save} disabled={saving || !name.trim()}>
            {category ? "Save" : "Add category"}
          </Button>
        </>
      }
    >
      <TextInput
        label="Name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        autoFocus
        placeholder="e.g. Design"
      />

      {isCanonical ? (
        <p className="text-xs text-muted">This is a built-in category — its colour follows the theme.</p>
      ) : (
        <div>
          <span className="mb-2 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
            Colour
          </span>
          <Swatches colors={PALETTE} value={color} onChange={setColor} aria-label="Colour" />
        </div>
      )}

      <div>
        <span className="mb-1 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          Apps
        </span>
        <p className="mb-2 text-xs text-muted">
          Pick from the apps on your machine — each one sorts into this category automatically.
        </p>
        <AppPicker apps={apps} selected={selected} onToggle={toggleApp} conflictFor={conflictFor} />

        {conflict && (
          <div
            className="mt-2.5 flex gap-2.5 border-2 border-c-research p-2.5 text-[12.5px] leading-snug"
            style={{ background: "color-mix(in srgb, var(--c-research) 8%, transparent)" }}
          >
            <span className="font-bold text-c-research">!</span>
            <div>
              <span className="font-semibold text-fg">{conflict.app}</span> is already in{" "}
              <span className="font-semibold text-fg">{conflict.ownerName}</span>. An app lives in
              one category, and the first rule wins — so it would stay in {conflict.ownerName}.
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={() => resolveConflict(true)}
                  className="border-2 border-c-research bg-c-research px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.03em] text-bg"
                >
                  Move it here
                </button>
                <button
                  type="button"
                  onClick={() => resolveConflict(false)}
                  className="border-2 border-edge bg-bg px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.03em] text-fg"
                >
                  Leave in {conflict.ownerName}
                </button>
              </div>
            </div>
          </div>
        )}

        {selected.size > 0 && (
          <p className="mt-2.5 text-[11px] font-semibold text-muted">
            Sorts <span className="text-fg">{selected.size}</span>{" "}
            {selected.size === 1 ? "app" : "apps"} into this category.
          </p>
        )}

        {(otherExisting.length > 0 || otherAdded.length > 0) && (
          <div className="mt-3">
            <span className="mb-1.5 block text-[10px] font-semibold uppercase tracking-[0.1em] text-muted">
              Site, title &amp; pattern rules
            </span>
            <div className="flex flex-wrap gap-1.5">
              {otherExisting.map((r) => (
                <RuleChip
                  key={`r-${r.id}`}
                  label={ruleChipLabel(r)}
                  onRemove={() => removeExisting(r.id)}
                />
              ))}
              {otherAdded.map(({ r, i }) => (
                <RuleChip
                  key={`a-${i}`}
                  label={ruleChipLabel(r)}
                  pending
                  onRemove={() => removeAdded(i)}
                />
              ))}
            </div>
          </div>
        )}

        <div className="mt-3 border-t border-rule pt-3">
          <button
            type="button"
            onClick={() => setShowAdvanced((s) => !s)}
            className="text-[10px] font-semibold uppercase tracking-[0.1em] text-muted"
          >
            {showAdvanced ? "▾" : "▸"} Advanced — match by site or window title
          </button>
          {showAdvanced && (
            <div className="mt-2.5">
              <div className="flex gap-2">
                <TextInput
                  className="flex-1"
                  value={ruleInput}
                  onChange={(e) => setRuleInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addAdvanced();
                    }
                  }}
                  placeholder="e.g. youtube.com, or invoice"
                />
                <Button variant="secondary" onClick={addAdvanced} disabled={!ruleInput.trim()}>
                  Add
                </Button>
              </div>
              <p className="mt-2 text-xs text-muted">
                Type a site (<code className="font-sans font-semibold text-fg">youtube.com</code>) to sort that
                host and its subdomains; anything else becomes a title rule, sorting a window when its title{" "}
                <span className="font-semibold text-fg">contains</span> that text. Both are plain text, not
                wildcards (<code className="font-sans font-semibold text-fg">.fig</code> works,{" "}
                <code className="font-sans font-semibold text-fg">*.fig</code> doesn't). Force either with{" "}
                <code className="font-sans font-semibold text-fg">site:</code> or{" "}
                <code className="font-sans font-semibold text-fg">title:</code>.
              </p>
              <p className="mt-1.5 text-xs text-muted">
                A site rule beats an app rule, so{" "}
                <code className="font-sans font-semibold text-fg">youtube.com</code> here outranks Chrome
                wherever Chrome sits.
              </p>
            </div>
          )}
        </div>
      </div>

      {error && <p className="text-[11px] font-semibold text-c-research">{error}</p>}
    </Modal>
  );
}

function RuleChip({
  label,
  onRemove,
  pending,
}: {
  label: string;
  onRemove: () => void;
  pending?: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 border bg-surface px-[7px] py-px font-sans text-[12.5px] font-semibold text-fg",
        pending ? "border-c-deep" : "border-rule",
      )}
    >
      {label}
      <button
        type="button"
        aria-label={`Remove ${label}`}
        onClick={onRemove}
        className="text-muted hover:text-c-research"
      >
        ×
      </button>
    </span>
  );
}
