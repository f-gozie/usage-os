export interface InspectorDetail {
  colorVar: string;
  /** Category name, e.g. "Deep work". */
  title: string;
  /** Project split + apps, e.g. "usageos 1h 3m · nudge 39m · Cursor, iTerm". */
  subtitle: string;
  durationLabel: string;
  rangeLabel: string;
}

export interface DetailInspectorProps {
  detail: InspectorDetail | null;
  emptyHint?: string;
}

/** Shows the selected category-run's detail, or an empty hint. Dashed border marks it
 *  as the inspect surface. */
export function DetailInspector({
  detail,
  emptyHint = "Click any block on the dial to see the session.",
}: DetailInspectorProps) {
  return (
    <div className="mt-[18px] flex min-h-[64px] items-center gap-[15px] border-2 border-dashed border-edge px-4 py-3.5">
      {detail ? (
        <>
          <span
            className="h-[34px] w-[34px] flex-shrink-0 border border-edge"
            style={{ background: detail.colorVar }}
          />
          <div className="min-w-0">
            <div className="text-[15px] font-semibold">{detail.title}</div>
            <div className="mt-0.5 text-[12.5px] text-muted">{detail.subtitle}</div>
          </div>
          <div className="ml-auto flex-shrink-0 text-right font-display text-[18px]">
            {detail.durationLabel}
            <div className="font-sans text-[12px] font-medium text-muted">{detail.rangeLabel}</div>
          </div>
        </>
      ) : (
        <span className="text-[13px] font-medium text-muted">{emptyHint}</span>
      )}
    </div>
  );
}
