// Ergonomic wrappers over the GENERATED IPC client (`src/bindings.ts`, hard rule 2):
// unwrap the generated `Result<T, AppError>` to the throwing `Promise<T>` the UI uses,
// and re-export the generated types from one place.
import { commands } from '../bindings';
import type {
  ActivityLog,
  AppError,
  AutomationRequest,
  Category,
  DayView,
  Exclusion,
  InstalledApp,
  Permissions,
  Recap,
  Result,
  Rule,
  Setting,
  SettingsPane,
  TimelineView,
  UncategorizedApp,
  WatcherStatus,
  WeekView,
} from '../bindings';

export type {
  ActivityLog,
  AppError,
  AutomationRequest,
  Category,
  CategoryRun,
  CategorySlice,
  DaySlice,
  DayView,
  Exclusion,
  InstalledApp,
  Permissions,
  PermissionState,
  ProjectSlice,
  Recap,
  Rule,
  Setting,
  SettingsPane,
  TimelineRun,
  TimelineSegment,
  TimelineView,
  UncategorizedApp,
  WatcherStatus,
  WeekView,
} from '../bindings';

/** Unwrap a generated `Result`, throwing a readable `Error` on the typed failure. */
function unwrap<T>(r: Result<T, AppError>): T {
  if (r.status === 'error') {
    const e = r.error;
    throw new Error('message' in e ? `${e.kind}: ${e.message}` : e.kind);
  }
  return r.data;
}

/** Raw activity logs for a `[startTime, endTime)` Unix-second range. */
export async function getActivityStats(
  startTime: number,
  endTime: number
): Promise<ActivityLog[]> {
  return unwrap(await commands.getActivityStats(startTime, endTime));
}

/** The computed Day view (category aggregates + runs + recap) for a `[start, end)` range. */
export async function getDay(startTime: number, endTime: number): Promise<DayView> {
  return unwrap(await commands.getDay(startTime, endTime));
}

/** Narrate the day's recap with the on-device sidecar, falling back to the template on any
 *  failure. Async + lazy: call AFTER the day loads and upgrade the card in place when it
 *  resolves (`getDay` already returns the instant template recap). */
export async function getRecap(startTime: number, endTime: number): Promise<Recap> {
  return unwrap(await commands.getRecap(startTime, endTime));
}

/** The computed Week view: 7 day-slices + week aggregates. `dayStarts` are the 7 local
 *  midnights; `weekEnd` is the next midnight after the last day (half-open). */
export async function getWeek(dayStarts: number[], weekEnd: number): Promise<WeekView> {
  return unwrap(await commands.getWeek(dayStarts, weekEnd));
}

/** The computed Timeline view: the day's category-runs + their app-switch segments. */
export async function getTimeline(startTime: number, endTime: number): Promise<TimelineView> {
  return unwrap(await commands.getTimeline(startTime, endTime));
}

// --- Categories (the `categories` table; IPC noun is "category", D31) ---

export async function getCategories(): Promise<Category[]> {
  return unwrap(await commands.getCategories());
}

export async function createCategory(name: string, color: string): Promise<number> {
  return unwrap(await commands.createCategory(name, color));
}

export async function updateCategory(id: number, name: string, color: string): Promise<void> {
  unwrap(await commands.updateCategory(id, name, color));
}

export async function deleteCategory(id: number): Promise<void> {
  unwrap(await commands.deleteCategory(id));
}

// --- Rules ---

export async function getRules(): Promise<Rule[]> {
  return unwrap(await commands.getRules());
}

export async function createRule(
  categoryId: number,
  matchField: string,
  pattern: string,
  ignoreTitle: boolean = false,
): Promise<number> {
  return unwrap(await commands.createRule(categoryId, matchField, pattern, ignoreTitle));
}

export async function deleteRule(id: number): Promise<void> {
  unwrap(await commands.deleteRule(id));
}

export async function reprocessLogs(): Promise<void> {
  unwrap(await commands.reprocessLogs());
}

// --- Exclusions (D8) ---

export async function getExclusions(): Promise<Exclusion[]> {
  return unwrap(await commands.getExclusions());
}

export async function createExclusion(
  matchType: string,
  pattern: string,
  mode: string,
): Promise<number> {
  return unwrap(await commands.createExclusion(matchType, pattern, mode));
}

export async function deleteExclusion(id: number): Promise<void> {
  unwrap(await commands.deleteExclusion(id));
}

// --- Watcher Status ---

export async function getWatcherStatus(): Promise<WatcherStatus> {
  return unwrap(await commands.getWatcherStatus());
}

// --- Settings ---

export async function getSettings(): Promise<Setting[]> {
  return unwrap(await commands.getSettings());
}

export async function updateSetting(key: string, value: string): Promise<void> {
  unwrap(await commands.updateSetting(key, value));
}

/** Persist the retention window and prune older rows now. Returns rows deleted. `0` = keep forever. */
export async function setRetentionDays(days: number): Promise<number> {
  return unwrap(await commands.setRetentionDays(days));
}

// --- Data ownership ---

/** Absolute path to the SQLite file (for revealing it in Finder). */
export async function getDatabasePath(): Promise<string> {
  return unwrap(await commands.getDatabasePath());
}

/** Write all events to a CSV next to the DB; returns its absolute path. */
export async function exportEventsCsv(): Promise<string> {
  return unwrap(await commands.exportEventsCsv());
}

/** Erase the captured record (events + derived projects/sites); preserves config. */
export async function deleteAllData(): Promise<void> {
  unwrap(await commands.deleteAllData());
}

// --- Installed-app catalog (for app icons; offline, read-only) ---

/** The user's installed apps + their icons (data-URI PNGs) for the `AppIcon` map. */
export async function listInstalledApps(): Promise<InstalledApp[]> {
  return unwrap(await commands.listInstalledApps());
}

/** Apps with tracked time that match no rule (roll up as "Uncategorized"), all-time,
 *  ranked by total time. For the Settings "Uncategorized" list. */
export async function getUncategorizedApps(): Promise<UncategorizedApp[]> {
  return unwrap(await commands.getUncategorizedApps());
}

// --- Permissions (macOS capture grants: Accessibility + Automation) ---

/** Current state of the Accessibility + Automation permissions (for onboarding + Settings). */
export async function getPermissions(): Promise<Permissions> {
  return unwrap(await commands.getPermissions());
}

/** Prompt for Accessibility and open its System Settings → Privacy pane. */
export async function requestAccessibility(): Promise<void> {
  unwrap(await commands.requestAccessibility());
}

/**
 * Trigger the Automation consent prompt for running browsers and open its Privacy pane.
 * Returns whether a browser was running to prompt — `"no_browser_running"` means the UI should
 * tell the user to open their browser (macOS can't list the app until it scripts a running one).
 */
export async function requestAutomation(): Promise<AutomationRequest> {
  return unwrap(await commands.requestAutomation());
}

/** Open a System Settings → Privacy pane directly (Accessibility or Automation). */
export async function openSettingsPane(pane: SettingsPane): Promise<void> {
  unwrap(await commands.openSettingsPane(pane));
}

// --- Window / app (menubar glance popover actions) ---

/** Show + focus the main window and hide the glance popover. */
export async function showMainWindow(): Promise<void> {
  unwrap(await commands.showMainWindow());
}

/** Quit UsageOS entirely (stops background tracking). */
export async function quitApp(): Promise<void> {
  unwrap(await commands.quitApp());
}

/** Relaunch the app — used after an update installs so the new binary takes over. Never returns. */
export async function restartApp(): Promise<void> {
  await commands.restartApp();
}
