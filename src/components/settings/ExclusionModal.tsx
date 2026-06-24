import { useEffect, useState } from "react";

import { AppPicker } from "@/components/settings/AppPicker";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { RadioGroup } from "@/components/ui/RadioGroup";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { TextInput } from "@/components/ui/TextInput";
import { useInstalledApps } from "@/hooks/useInstalledApps";
import { createExclusion } from "@/lib/tauri";

type MatchType = "app" | "site" | "title";
type Mode = "exclude" | "private";

export interface ExclusionModalProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
}

export function ExclusionModal({ open, onClose, onSaved }: ExclusionModalProps) {
  const apps = useInstalledApps();
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

  // For the App type, the picker drives `pattern` as a single selection.
  const appSelected = matchType === "app" && pattern ? new Set([pattern.toLowerCase()]) : new Set<string>();
  const toggleApp = (name: string) =>
    setPattern((p) => (p.toLowerCase() === name.toLowerCase() ? "" : name));

  const submit = async () => {
    if (!pattern.trim()) {
      setError(matchType === "app" ? "Pick an app first." : "This can't be empty.");
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
          What to match
        </span>
        <SegmentedControl
          aria-label="Match type"
          value={matchType}
          onChange={(v) => {
            setMatchType(v as MatchType);
            setPattern(""); // switching kind clears the picked app / typed text
          }}
          options={[
            { value: "app", label: "App" },
            { value: "site", label: "Website" },
            { value: "title", label: "Window title" },
          ]}
        />
      </div>

      {matchType === "app" ? (
        <div>
          <span className="mb-2 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
            App
          </span>
          <AppPicker apps={apps} selected={appSelected} onToggle={toggleApp} />
        </div>
      ) : (
        <div>
          <TextInput
            label={matchType === "site" ? "Website contains" : "Window title contains"}
            value={pattern}
            onChange={(e) => setPattern(e.target.value)}
            autoFocus
            placeholder={matchType === "site" ? "chase.com" : "Passwords"}
          />
          <p className="mt-2 text-xs text-muted">
            Matches when the {matchType === "site" ? "web address" : "window title"}{" "}
            <span className="font-semibold text-fg">contains</span> this text — plain text, not a
            wildcard.
          </p>
        </div>
      )}

      <div>
        <span className="mb-2.5 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">
          How
        </span>
        <RadioGroup
          aria-label="Mode"
          value={mode}
          onChange={(v) => setMode(v as Mode)}
          options={[
            { value: "exclude", label: "Exclude", description: "Drop it entirely — no record that it ran." },
            { value: "private", label: "Private", description: "Count the time, but save no title or web address." },
          ]}
        />
      </div>

      {error && <p className="text-[11px] font-semibold text-c-research">{error}</p>}
    </Modal>
  );
}
