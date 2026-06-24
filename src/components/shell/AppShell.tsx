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

/** The app chrome. Fills the native window edge-to-edge (the OS window IS the frame —
 *  no nested card): our titlebar hosts the macOS traffic lights (Overlay style), the
 *  middle scrolls when the window is short, and the privacy footer stays pinned. */
export function AppShell({ view, onViewChange, headerDate, children }: AppShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-bg text-fg">
      <TitleBar />

      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[1040px] px-[22px]">
          <header className="flex items-end justify-between pt-[18px]">
            <div className="font-display text-[36px] uppercase leading-[0.82] tracking-[0.01em]">
              USAGE<span style={{ color: "var(--c-research)" }}>OS</span>
            </div>
            <div className="flex items-center gap-4">
              {/* Reserve the date slot (two lines) so the header height is identical across
                  every view — a missing/short date must never shift the chrome below it. */}
              <div className="flex min-h-[35px] flex-col justify-end text-right text-[11px] font-semibold uppercase leading-[1.6] tracking-[0.12em] text-muted">
                {headerDate}
              </div>
              <ThemeSwitcher />
            </div>
          </header>

          <TabNav view={view} onViewChange={onViewChange} />

          <main className="py-[22px]">{children}</main>
        </div>
      </div>

      <Footer />
    </div>
  );
}
