// Opt-in software updates (D67). The update check is the one sanctioned network call (hard rule
// 1) and it's OFF by default: nothing contacts GitHub until the user enables it. The request is
// made from Rust by the updater plugin and carries only the current version — never any tracked
// data. Updates are ed25519-signed (pubkey in tauri.conf.json); a tampered build can't install.

import { check, type Update } from "@tauri-apps/plugin-updater";

import { getSettings, restartApp, updateSetting } from "./tauri";

export type { Update };

/** Settings keys (stored via the generic key-value `update_setting`). */
const ENABLED_KEY = "auto_update_enabled";
const LAST_CHECK_KEY = "last_update_check";
const DAY_MS = 24 * 60 * 60 * 1000;

/** Whether the user opted in to automatic checks. Default OFF — opt-in (D67). */
export async function autoUpdateEnabled(): Promise<boolean> {
  const settings = await getSettings();
  return settings.some((s) => s.key === ENABLED_KEY && s.value === "true");
}

export async function setAutoUpdateEnabled(on: boolean): Promise<void> {
  await updateSetting(ENABLED_KEY, on ? "true" : "false");
}

/**
 * Ask GitHub whether a newer signed version exists. Returns the `Update` (with `.version` and
 * `.body` release notes) or `null` when up to date. Always available — the manual "Check for
 * updates" button calls this regardless of the auto setting.
 */
export async function checkForUpdate(): Promise<Update | null> {
  return check();
}

/** Download + verify (ed25519) + install, then relaunch into the new binary. */
export async function installUpdate(
  update: Update,
  onProgress?: (downloaded: number, total: number | null) => void,
): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? null;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.(downloaded, total);
    }
  });
  await restartApp();
}

/**
 * Launch-time check: only when the user opted in AND the last check was over 24h ago (debounced
 * via `last_update_check`, so it's one request per day at most). Returns the `Update` if found.
 */
export async function maybeAutoCheck(): Promise<Update | null> {
  if (!(await autoUpdateEnabled())) return null;
  const settings = await getSettings();
  const last = Number(settings.find((s) => s.key === LAST_CHECK_KEY)?.value ?? "0");
  if (Date.now() - last < DAY_MS) return null;
  await updateSetting(LAST_CHECK_KEY, String(Date.now()));
  return check();
}
