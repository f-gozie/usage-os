import type { ReactNode } from "react";

import { Footer } from "./Footer";
import { TabNav, type View } from "./TabNav";
import { ThemeSwitcher } from "./ThemeSwitcher";
import { TitleBar } from "./TitleBar";

export interface AppShellProps {
  view: View;
  onViewChange: (view: View) => void;
  /** Optional contextual header content (e.g. the viewed day's date). */
  headerDate?: ReactNode;
  children: ReactNode;
}

/** The window frame: titlebar, brand header (+ theme switcher), tab nav, content,
 *  privacy footer. Centred, fixed max width, hard border with the only 5px radius. */
export function AppShell({ view, onViewChange, headerDate, children }: AppShellProps) {
  return (
    <div className="flex justify-center px-[18px] py-7">
      <div className="w-full max-w-[1000px] overflow-hidden rounded-frame border-[3px] border-edge bg-bg">
        <TitleBar />

        <header className="flex items-end justify-between px-[22px] pt-[18px]">
          <div className="font-display text-[36px] uppercase leading-[0.82] tracking-[0.01em]">
            USAGE<span style={{ color: "var(--c-research)" }}>OS</span>
          </div>
          <div className="flex items-center gap-4">
            {headerDate && (
              <div className="text-right text-[11px] font-semibold uppercase leading-[1.6] tracking-[0.12em] text-muted">
                {headerDate}
              </div>
            )}
            <ThemeSwitcher />
          </div>
        </header>

        <TabNav view={view} onViewChange={onViewChange} />

        <main className="p-[22px]">{children}</main>

        <Footer />
      </div>
    </div>
  );
}
