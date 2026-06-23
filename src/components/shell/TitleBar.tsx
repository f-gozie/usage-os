/** The app window titlebar: the three context dots, the wordmark, and the live
 *  "Tracking" indicator. Inverted bar (uses --bar-bg/--bar-fg, theme-safe). */
export function TitleBar() {
  return (
    <div className="flex items-center gap-2.5 bg-bar-bg px-4 py-[11px]">
      <span className="h-3 w-3 rounded-full" style={{ background: "var(--c-research)" }} />
      <span className="h-3 w-3 rounded-full" style={{ background: "var(--c-comms)" }} />
      <span className="h-3 w-3 rounded-full" style={{ background: "var(--c-deep)" }} />
      <span className="-ml-[46px] flex-1 text-center text-xs font-semibold uppercase tracking-[0.34em] text-bar-fg">
        UsageOS
      </span>
      <span className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.12em] text-bar-fg/60">
        <span className="h-2 w-2 animate-pulse rounded-full" style={{ background: "var(--c-comms)" }} />
        Tracking
      </span>
    </div>
  );
}
