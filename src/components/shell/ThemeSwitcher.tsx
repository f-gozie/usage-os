import { cn } from "@/lib/utils";
import { THEMES, THEME_LABELS, useTheme, type Theme } from "@/providers/ThemeProvider";

// Each theme's page colour, shown as a small swatch. Fixed brand identities (mirror
// tokens.css) — they must show theme X regardless of the currently active theme.
const SWATCH: Record<Theme, string> = {
  paper: "#EEEBE1",
  warm: "#1A1916",
  black: "#0E0E0E",
};

/** Subtle Paper/Warm/Black switcher: three small swatches, the active one outlined. */
export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();
  return (
    <div role="group" aria-label="Theme" className="flex items-center gap-1.5">
      {THEMES.map((t) => {
        const active = t === theme;
        return (
          <button
            key={t}
            type="button"
            title={THEME_LABELS[t]}
            aria-label={THEME_LABELS[t]}
            aria-pressed={active}
            onClick={() => setTheme(t)}
            className={cn(
              "h-[18px] w-[18px] border border-edge transition-transform",
              active
                ? "outline outline-2 outline-offset-1 outline-edge"
                : "opacity-70 hover:opacity-100",
            )}
            style={{ background: SWATCH[t] }}
          />
        );
      })}
    </div>
  );
}
