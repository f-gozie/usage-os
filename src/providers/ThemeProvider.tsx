import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { getSettings, updateSetting } from "@/lib/tauri";

/** The three first-class themes (design-system.md). */
export type Theme = "paper" | "warm" | "black";

export const THEMES: readonly Theme[] = ["paper", "warm", "black"] as const;
export const THEME_LABELS: Record<Theme, string> = {
  paper: "Paper",
  warm: "Warm",
  black: "Black",
};

const SETTING_KEY = "theme";
const DEFAULT_THEME: Theme = "paper";

function isTheme(value: string): value is Theme {
  return (THEMES as readonly string[]).includes(value);
}

interface ThemeContextValue {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

/**
 * Owns the active theme: reflects it onto `<html data-theme>` (which drives every
 * token), and persists the choice in the Rust settings table so it survives restart.
 * Persistence is best-effort — if the backend isn't there (e.g. Storybook), it falls
 * back to in-memory state and the default.
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(DEFAULT_THEME);
  // True once the user has explicitly picked a theme — so the async initial load can't clobber a
  // choice the user made while it was still in flight.
  const userChosen = useRef(false);

  // Reflect onto the document so the token set swaps.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  // Load the persisted choice once.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const settings = await getSettings();
        const saved = settings.find((s) => s.key === SETTING_KEY)?.value;
        if (!cancelled && !userChosen.current && saved && isTheme(saved)) setThemeState(saved);
      } catch {
        // No backend (Storybook / tests) — keep the default.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const setTheme = useCallback((next: Theme) => {
    userChosen.current = true;
    setThemeState(next);
    void updateSetting(SETTING_KEY, next).catch(() => {
      // Best-effort persistence.
    });
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>{children}</ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within a ThemeProvider");
  return ctx;
}
