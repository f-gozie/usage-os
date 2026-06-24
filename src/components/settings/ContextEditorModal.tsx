import { useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { Swatches } from "@/components/ui/Swatches";
import { TextInput } from "@/components/ui/TextInput";
import { contextColorVar } from "@/lib/contexts";
import {
  createContext,
  createRule,
  deleteContext,
  deleteRule,
  reprocessLogs,
  updateContext,
  type Context,
  type Rule,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Default palette for user contexts (canonical four take their theme token instead). */
const PALETTE = ["#1B45BE", "#E0241B", "#EAB308", "#7A4FC2", "#1D9E75", "#161616"] as const;

interface PendingRule {
  match_field: string;
  pattern: string;
}

export interface ContextEditorModalProps {
  open: boolean;
  /** `null` = add a new context; otherwise edit this one. */
  context: Context | null;
  rules: Rule[];
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}

/** Parse a free-text rule token: a leading `title:` targets the window title, else the
 *  process name. Returns `null` for an empty token. */
function parseRule(token: string): PendingRule | null {
  const t = token.trim();
  if (!t) return null;
  const m = /^title:\s*(.+)$/i.exec(t);
  return m ? { match_field: "title", pattern: m[1].trim() } : { match_field: "process", pattern: t };
}

export function ContextEditorModal({
  open,
  context,
  rules,
  onClose,
  onSaved,
}: ContextEditorModalProps) {
  const isCanonical = context?.slug != null;
  const [name, setName] = useState("");
  const [color, setColor] = useState<string>(PALETTE[3]);
  const [removedIds, setRemovedIds] = useState<Set<number>>(new Set());
  const [added, setAdded] = useState<PendingRule[]>([]);
  const [ruleInput, setRuleInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // (Re)initialize whenever the target context changes / the modal opens.
  useEffect(() => {
    if (!open) return;
    setName(context?.name ?? "");
    setColor(context?.slug ? contextColorVar(context.slug) : (context?.color ?? PALETTE[3]));
    setRemovedIds(new Set());
    setAdded([]);
    setRuleInput("");
    setError(null);
  }, [open, context]);

  const existing = context ? rules.filter((r) => r.context_id === context.id) : [];
  const visibleExisting = existing.filter((r) => !removedIds.has(r.id));

  const addRules = () => {
    const parsed = ruleInput
      .split(",")
      .map(parseRule)
      .filter((r): r is PendingRule => r !== null);
    if (parsed.length) setAdded((a) => [...a, ...parsed]);
    setRuleInput("");
  };

  const save = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Name can't be empty.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      let id = context?.id;
      if (context) {
        // Canonical colour is theme-driven; keep its stored hex untouched.
        await updateContext(context.id, trimmed, isCanonical ? context.color : color);
      } else {
        id = await createContext(trimmed, color);
      }
      if (id == null) throw new Error("Missing context id");
      for (const r of added) await createRule(id, r.match_field, r.pattern, false);
      for (const rid of removedIds) await deleteRule(rid);
      await reprocessLogs();
      await onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't save this context.");
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!context) return;
    setSaving(true);
    setError(null);
    try {
      await deleteContext(context.id);
      await reprocessLogs();
      await onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't delete this context.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={context ? "Edit context" : "Add context"}
      footer={
        <>
          {context && !isCanonical && (
            <Button variant="danger" className="mr-auto" onClick={remove} disabled={saving}>
              Delete
            </Button>
          )}
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={save} disabled={saving || !name.trim()}>
            {context ? "Save" : "Add context"}
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
        <p className="text-xs text-muted">This is a built-in context — its colour follows the theme.</p>
      ) : (
        <div>
          <span className="mb-2 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
            Colour
          </span>
          <Swatches colors={PALETTE} value={color} onChange={setColor} aria-label="Colour" />
        </div>
      )}

      <div>
        <span className="mb-2 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          Rules — apps or <code className="font-sans font-semibold text-fg">title:</code> patterns
        </span>
        {(visibleExisting.length > 0 || added.length > 0) && (
          <div className="mb-2.5 flex flex-wrap gap-1.5">
            {visibleExisting.map((r) => (
              <RuleChip
                key={`r-${r.id}`}
                label={r.match_field === "title" ? `title: ${r.pattern}` : r.pattern}
                onRemove={() => setRemovedIds((s) => new Set(s).add(r.id))}
              />
            ))}
            {added.map((r, i) => (
              <RuleChip
                key={`a-${i}`}
                label={r.match_field === "title" ? `title: ${r.pattern}` : r.pattern}
                pending
                onRemove={() => setAdded((a) => a.filter((_, j) => j !== i))}
              />
            ))}
          </div>
        )}
        <div className="flex gap-2">
          <TextInput
            className="flex-1"
            value={ruleInput}
            onChange={(e) => setRuleInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addRules();
              }
            }}
            placeholder="Figma, Sketch, title: *.fig"
          />
          <Button variant="secondary" onClick={addRules} disabled={!ruleInput.trim()}>
            Add
          </Button>
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
