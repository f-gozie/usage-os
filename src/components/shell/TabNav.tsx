import { cn } from "@/lib/utils";

export type View = "day" | "week" | "timeline" | "settings";

export const VIEWS: { value: View; label: string }[] = [
  { value: "day", label: "Day" },
  { value: "week", label: "Week" },
  { value: "timeline", label: "Timeline" },
  { value: "settings", label: "Settings" },
];

export interface TabNavProps {
  view: View;
  onViewChange: (view: View) => void;
}

/** Top navigation tabs (Day / Week / Timeline / Settings). */
export function TabNav({ view, onViewChange }: TabNavProps) {
  return (
    <nav className="mx-[22px] mt-4 flex items-center border-b-[3px] border-edge">
      {VIEWS.map((tab) => {
        const active = tab.value === view;
        return (
          <button
            key={tab.value}
            type="button"
            onClick={() => onViewChange(tab.value)}
            className={cn(
              "relative top-[3px] border-b-[3px] px-4 py-[11px]",
              "text-xs font-semibold uppercase tracking-[0.1em] transition-colors",
              active ? "border-fg text-fg" : "border-transparent text-muted hover:text-fg",
            )}
          >
            {tab.label}
          </button>
        );
      })}
    </nav>
  );
}
