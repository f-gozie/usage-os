import { useEffect, useState } from "react";

import { SettingGroup, SettingRow } from "@/components/settings/primitives";
import { Toggle } from "@/components/ui/Toggle";
import { getLaunchAtLogin, setLaunchAtLogin } from "@/lib/tauri";

/**
 * The "Background" settings section: the opt-in start-at-login toggle (default OFF — D68).
 * No stored setting — the system's LaunchAgent state is read and written directly.
 */
export function BackgroundSettings() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    void getLaunchAtLogin().then(setEnabled).catch(() => undefined);
  }, []);

  const toggle = (on: boolean) => {
    setEnabled(on);
    // The LaunchAgent is the source of truth (D68) — if the write fails, show the real state.
    void setLaunchAtLogin(on).catch(() => setEnabled(!on));
  };

  return (
    <SettingGroup title="Background">
      <SettingRow
        label="Start at login"
        description="UsageOS starts quietly in the menu bar when you log in — your day is tracked from the moment you sit down."
      >
        <Toggle checked={enabled} onChange={toggle} aria-label="Start at login" />
      </SettingRow>
    </SettingGroup>
  );
}
