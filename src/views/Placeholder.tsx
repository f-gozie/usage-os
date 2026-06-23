/** Placeholder for views not yet built (Week, Timeline, Settings — M2–M4). Honest
 *  about being unfinished rather than shipping a half-styled old screen. */
export function Placeholder({ title }: { title: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 border-2 border-dashed border-edge px-6 py-20 text-center">
      <div className="font-display text-[22px] uppercase tracking-[0.04em]">{title}</div>
      <p className="text-sm font-medium text-muted">Coming soon.</p>
    </div>
  );
}
