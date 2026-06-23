import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { THEMES, THEME_LABELS, useTheme, type Theme } from "@/providers/ThemeProvider";

const OPTIONS = THEMES.map((t) => ({ value: t, label: THEME_LABELS[t] }));

/** Paper / Warm / Black switcher, bound to the ThemeProvider. */
export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();
  return (
    <SegmentedControl<Theme>
      aria-label="Theme"
      options={OPTIONS}
      value={theme}
      onChange={setTheme}
    />
  );
}
