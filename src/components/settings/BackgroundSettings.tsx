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
    void setLaunchAtLogin(on).catch(() => undefined);
  };

  return (
    <SettingGroup title="Background">
      <SettingRow
        label="Start at login"
        description="UsageOS starts quietly in the menu bar when you log in, so your day is tracked without you having to remember to open anything. Close the window anytime — tracking keeps going."
      >
        <Toggle checked={enabled} onChange={toggle} aria-label="Start at login" />
      </SettingRow>
    </SettingGroup>
  );
}
