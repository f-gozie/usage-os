import { useEffect, useState } from "react";

import { Pill, SettingGroup, SettingRow } from "@/components/settings/primitives";
import { Toggle } from "@/components/ui/Toggle";
import {
  autoUpdateEnabled,
  checkForUpdate,
  installUpdate,
  setAutoUpdateEnabled,
  type Update,
} from "@/lib/updater";

/**
 * The "Software update" settings section: the opt-in toggle (default OFF — D67), a one-line
 * honest disclosure of exactly what the check sends, and a manual "Check now" that works
 * regardless of the toggle. When a newer version is found, the button becomes "Install".
 */
export function UpdateSettings() {
  const [enabled, setEnabled] = useState(false);
  const [status, setStatus] = useState<"idle" | "checking" | "uptodate" | "error">("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    void autoUpdateEnabled().then(setEnabled).catch(() => undefined);
  }, []);

  const toggle = (on: boolean) => {
    setEnabled(on);
    void setAutoUpdateEnabled(on).catch(() => undefined);
  };

  const check = () => {
    setStatus("checking");
    setUpdate(null);
    void checkForUpdate()
      .then((u) => {
        if (u) {
          setUpdate(u);
          setStatus("idle");
        } else {
          setStatus("uptodate");
        }
      })
      .catch(() => setStatus("error"));
  };

  const install = () => {
    if (!update) return;
    setInstalling(true);
    void installUpdate(update).catch(() => setInstalling(false));
  };

  const checkDescription = update
    ? `Version ${update.version} is available.`
    : status === "checking"
      ? "Checking…"
      : status === "uptodate"
        ? "You’re on the latest version."
        : status === "error"
          ? "Couldn’t check just now. Try again later."
          : "Look now for a newer version.";

  return (
    <SettingGroup title="Software update">
      <SettingRow
        label="Automatic updates"
        description="Off by default. When on, UsageOS asks GitHub once a day whether a newer version exists — it sends only the version number, never your activity or any tracked data. Updates are signed."
      >
        <Toggle checked={enabled} onChange={toggle} aria-label="Automatic updates" />
      </SettingRow>
      <SettingRow label="Check for updates" description={checkDescription}>
        {update ? (
          <Pill onClick={install} disabled={installing}>
            {installing ? "Installing…" : `Install ${update.version}`}
          </Pill>
        ) : (
          <Pill onClick={check} disabled={status === "checking"}>
            Check now
          </Pill>
        )}
      </SettingRow>
    </SettingGroup>
  );
}
