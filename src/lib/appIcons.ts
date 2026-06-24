// App-icon resolution: map a capture `process_name` to an installed app's icon.
//
// The backend (`list_installed_apps`) returns every installed app + its icon as a
// data-URI PNG (offline, cached). We load that ONCE for the app's lifetime and
// resolve names against it — exact, then a short alias table, then a conservative
// prefix match (so "iTerm2" finds "iTerm" but "QR Code" never grabs "Code"). Names
// that don't resolve (the app's own dev binary, an obscure tool) fall back to a
// monogram in `AppIcon`. Shared data-URIs mean the webview decodes one copy per
// unique icon regardless of how many rows show it.
import { listInstalledApps } from './tauri';

/** Normalize for matching: lowercase, drop everything but a–z0–9. */
function norm(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]/g, '');
}

/** Known process-name → app-name mismatches the prefix rule can't bridge. */
const ALIAS: Record<string, string> = {
  code: 'visualstudiocode', // VS Code's process is "Code"
  electron: '', // generic Electron shell — never resolve to a random app
};

let mapPromise: Promise<Map<string, string>> | null = null;
let loadedMap: Map<string, string> | null = null;
const resolveCache = new Map<string, string | null>();

/** Load (once) the installed-app icon map: normalized name → data-URI. Failures
 *  resolve to an empty map so the UI just shows monograms. */
export function loadIconMap(): Promise<Map<string, string>> {
  if (!mapPromise) {
    mapPromise = listInstalledApps()
      .then((apps) => {
        const m = new Map<string, string>();
        for (const a of apps) if (a.icon) m.set(norm(a.name), a.icon);
        loadedMap = m;
        return m;
      })
      .catch(() => {
        loadedMap = new Map();
        return loadedMap;
      });
  }
  return mapPromise;
}

function resolve(map: Map<string, string>, name: string): string | null {
  const key = norm(name);
  if (!key) return null;
  const exact = map.get(key);
  if (exact) return exact;
  const alias = ALIAS[key];
  if (alias) {
    const viaAlias = map.get(alias);
    if (viaAlias) return viaAlias;
  } else if (alias === '') {
    return null; // explicitly non-resolvable (e.g. generic Electron)
  }
  // Conservative fuzzy: one normalized name is a prefix of the other (≥3 chars),
  // catching "iTerm2"↔"iTerm" without matching unrelated substrings.
  for (const [k, v] of map) {
    if (k.length >= 3 && (k.startsWith(key) || key.startsWith(k))) return v;
  }
  return null;
}

/** Resolve a name to an icon data-URI (or null), memoized per name. Returns
 *  `undefined` only while the map hasn't loaded yet. */
export function resolveIcon(name: string): string | null | undefined {
  if (resolveCache.has(name)) return resolveCache.get(name);
  if (!loadedMap) return undefined; // not loaded — caller should await loadIconMap()
  const icon = resolve(loadedMap, name);
  resolveCache.set(name, icon);
  return icon;
}
