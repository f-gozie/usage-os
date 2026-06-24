import { cn } from "@/lib/utils";

export interface RecapCardProps {
  text: string;
  /** "fm" = on-device Foundation Models prose; "template" = the deterministic fallback. */
  generatedBy: string;
}

/** The daily recap card. The badge distinguishes on-device prose from the template. */
export function RecapCard({ text, generatedBy }: RecapCardProps) {
  const isFm = generatedBy === "fm";
  return (
    <div className="border-[3px] border-edge bg-surface px-[18px] py-4">
      <div className="mb-2.5 flex items-center justify-between">
        <span className="text-[10.5px] font-semibold uppercase tracking-[0.16em] text-muted">
          Daily recap
        </span>
        <span
          className={cn(
            "border-2 px-2 py-[3px] text-[10px] font-semibold uppercase tracking-[0.1em]",
            isFm ? "border-c-deep text-c-deep" : "border-rule text-muted",
          )}
        >
          {isFm ? "⌁ Summarized on-device" : "≡ Template"}
        </span>
      </div>
      <p className="text-[17px] font-medium leading-[1.45]">{text}</p>
    </div>
  );
}
