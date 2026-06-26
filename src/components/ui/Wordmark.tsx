import { cn } from "@/lib/utils";

type WordmarkProps = {
  className?: string;
  /** Render the dial-O in one ink (currentColor) instead of the three category colours.
   *  Use below ~16px or on busy backgrounds where the colours would muddy. */
  mono?: boolean;
};

/**
 * The UsageOS wordmark: USAGE · the Contexts dial as the letter O · S. The dial-O *is* the logo,
 * so it stands alone — never paired with a separate mark. Inherits the caller's font-size and
 * colour (the letters use `currentColor`; the dial uses the category tokens, or `currentColor` when
 * `mono`). Frozen geometry (2026-06-26): O = 1.02em, baseline nudge 0.08em, ring weight 17,
 * side space -0.05em.
 */
export function Wordmark({ className, mono = false }: WordmarkProps) {
  return (
    <span
      role="img"
      aria-label="UsageOS"
      className={cn(
        "inline-flex items-baseline font-display uppercase leading-none tracking-[0.02em]",
        className,
      )}
    >
      USAGE
      <span
        aria-hidden
        className="inline-block"
        style={{ width: "1.02em", height: "1.02em", margin: "0 -0.05em", transform: "translateY(0.08em)" }}
      >
        <DialO mono={mono} />
      </span>
      S
    </span>
  );
}

/** The Contexts dial sized as the letter O. Faint track keeps it reading as a closed ring (a letter)
 *  rather than three loose arcs; the three category runs sit over it. */
function DialO({ mono }: { mono: boolean }) {
  const work = mono ? "currentColor" : "var(--c-deep)";
  const browsing = mono ? "currentColor" : "var(--c-research)";
  const messaging = mono ? "currentColor" : "var(--c-comms)";
  return (
    <svg viewBox="0 0 100 100" className="block h-full w-full overflow-visible">
      <circle cx="50" cy="50" r="34" fill="none" stroke="var(--track)" strokeWidth="17" />
      <path d="M55.90 16.52 A34 34 0 0 1 80.02 65.96" fill="none" stroke={work} strokeWidth="17" />
      <path d="M71.85 76.04 A34 34 0 0 1 27.25 75.27" fill="none" stroke={browsing} strokeWidth="17" />
      <path d="M19.44 64.91 A34 34 0 0 1 44.10 16.52" fill="none" stroke={messaging} strokeWidth="17" />
    </svg>
  );
}
