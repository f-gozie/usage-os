import { useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { RadioGroup } from "@/components/ui/RadioGroup";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { TextInput } from "@/components/ui/TextInput";
import { createExclusion } from "@/lib/tauri";

type MatchType = "app" | "site" | "title";
type Mode = "exclude" | "private";

export interface ExclusionModalProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}

export function ExclusionModal({ open, onClose, onSaved }: ExclusionModalProps) {
  const [matchType, setMatchType] = useState<MatchType>("app");
  const [pattern, setPattern] = useState("");
  const [mode, setMode] = useState<Mode>("exclude");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setMatchType("app");
    setPattern("");
    setMode("exclude");
    setError(null);
  }, [open]);

  const submit = async () => {
    if (!pattern.trim()) {
      setError("Pattern can't be empty.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await createExclusion(matchType, pattern.trim(), mode);
      await onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't add this exclusion.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Add exclusion"
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={saving || !pattern.trim()}>
            Add exclusion
          </Button>
        </>
      }
    >
      <div>
        <span className="mb-2 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          Match
        </span>
        <SegmentedControl
          aria-label="Match type"
          value={matchType}
          onChange={(v) => setMatchType(v as MatchType)}
          options={[
            { value: "app", label: "App" },
            { value: "site", label: "Site" },
            { value: "title", label: "Title" },
          ]}
        />
      </div>

      <TextInput
        label="Pattern"
        value={pattern}
        onChange={(e) => setPattern(e.target.value)}
        autoFocus
        placeholder={matchType === "site" ? "chase.com" : "1Password"}
      />

      <div>
        <span className="mb-2.5 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          Mode
        </span>
        <RadioGroup
          aria-label="Mode"
          value={mode}
          onChange={(v) => setMode(v as Mode)}
          options={[
            { value: "exclude", label: "Exclude", description: "Drop the event entirely — no record." },
            { value: "private", label: "Private", description: "Count the time, store no title or URL." },
          ]}
        />
      </div>

      {error && <p className="text-[11px] font-semibold text-c-research">{error}</p>}
    </Modal>
  );
}
