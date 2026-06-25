import { cn } from "@/lib/utils";

export interface RecapCardProps {
  text: string;
  /**
   * Who wrote the prose — matches the Rust `Recap.generated_by`: "foundation-models" = the
   * on-device model, "template" = the deterministic fallback (D48). Drives the badge.
   */
  generatedBy: string;
}

/** The daily recap card. The badge distinguishes on-device prose from the template. */
export function RecapCard({ text, generatedBy }: RecapCardProps) {
  const isFm = generatedBy === "foundation-models";
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
      {/* Keyed on the source so remounting (template → on-device) replays the subtle
          fade-up once when the AI prose arrives; the instant template doesn't animate. */}
      <p
        key={isFm ? "fm" : "template"}
        className={cn("text-[17px] font-medium leading-[1.45]", isFm && "recap-in")}
      >
        {text}
      </p>
    </div>
  );
}
