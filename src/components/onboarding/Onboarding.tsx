import { useState } from "react";

import { usePermissions } from "@/hooks/usePermissions";
import { requestAccessibility, requestAutomation } from "@/lib/tauri";

/**
 * First-run onboarding: Welcome → Privacy → Accessibility → Automation → Ready. Ported from
 * `design/onboarding.html`. Permissions are optional — every grant step can be skipped, landing
 * on a degraded (app-only) "Ready". Live status comes from `usePermissions` (re-reads on window
 * focus, so a grant flips to "Granted ✓" when the user returns from System Settings).
 */
export function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const { permissions, refetch } = usePermissions();

  const accessibility = permissions?.accessibility ?? false;
  const automation = permissions?.automation === "granted";
  const STEPS = ["Welcome", "Your privacy", "Accessibility", "Automation", "Ready"];

  const next = () => setStep((s) => Math.min(s + 1, STEPS.length - 1));
  const back = () => setStep((s) => Math.max(s - 1, 0));

  const grantAccessibility = () => void requestAccessibility().then(refetch).catch(() => undefined);
  const grantAutomation = () => void requestAutomation().then(refetch).catch(() => undefined);

  return (
    <div className="flex min-h-screen items-center justify-center overflow-hidden bg-bg px-[18px] py-[30px]">
      <div className="relative w-full max-w-[560px]">
        {/* Bauhaus deco behind the card */}
        <span className="pointer-events-none absolute -right-[60px] -top-[70px] z-0 h-[240px] w-[240px] rounded-full bg-c-comms opacity-90" />
        <span className="pointer-events-none absolute -bottom-[40px] -left-[46px] z-0 h-[120px] w-[120px] rounded-bl-[120px] bg-c-research" />

        <div className="relative z-10 border-[3px] border-edge bg-bg">
          {/* Head */}
          <div className="flex items-center justify-between bg-bar-bg px-[18px] py-3">
            <span className="font-display text-base uppercase tracking-[0.04em] text-bar-fg">
              USAGE<span className="text-c-research">OS</span>
            </span>
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-bar-fg opacity-55">
              {STEPS[step]}
            </span>
          </div>

          {/* Progress */}
          <div className="flex gap-[5px] px-[18px] pt-3.5">
            {STEPS.map((label, i) => (
              <span
                key={label}
                className={`h-[5px] flex-1 ${i <= step ? "bg-c-deep" : "bg-track"}`}
              />
            ))}
          </div>

          {/* Body */}
          <div className="min-h-[280px] px-[22px] pb-2 pt-[22px]">
            {step === 0 && (
              <>
                <Motif />
                <Eyebrow>A private time tracker for Mac</Eyebrow>
                <H>
                  Where did your <span className="text-c-research">day</span> go?
                </H>
                <Lead>
                  Your Mac already knows how you spent today. UsageOS quietly keeps track, then
                  tells you the story — where you focused, what pulled you away, and where the
                  hours actually went.
                </Lead>
              </>
            )}

            {step === 1 && (
              <>
                <Eyebrow>Your privacy</Eyebrow>
                <H>
                  Nothing leaves
                  <br />
                  this machine.
                </H>
                <Lead>
                  No cloud, no account, no telemetry. There is no server at all — your data is one
                  file on your Mac, and the code is open for anyone to read.
                </Lead>
                <div className="mt-2 flex border-2 border-edge">
                  <Num value="100%" valueClass="text-c-comms" label="On device" />
                  <Num value="0" label="Servers" />
                  <Num value="MIT" valueClass="text-c-research" label="Open source" />
                </div>
              </>
            )}

            {step === 2 && (
              <>
                <Eyebrow>Permission 1 of 2 · why</Eyebrow>
                <H>
                  Read what
                  <br />
                  you’re working on
                </H>
                <Why
                  items={[
                    ["var(--c-deep)", "It reads the title of the window you’re using — so it can tell what you were working on, not just which app was open."],
                    ["var(--c-comms)", "That’s all it reads. It never sees what’s actually on your screen."],
                    ["var(--c-research)", "Titles stay on this Mac, and you can hide any app you don’t want tracked."],
                  ]}
                />
                <GrantBox
                  label="Accessibility"
                  sub="Opens System Settings → Privacy."
                  granted={accessibility}
                  onGrant={grantAccessibility}
                />
              </>
            )}

            {step === 3 && (
              <>
                <Eyebrow>Permission 2 of 2 · why</Eyebrow>
                <H>
                  See the sites
                  <br />
                  you visit
                </H>
                <Why
                  items={[
                    ["var(--c-deep)", "It reads the address of the page you’re on — so it knows you were reading docs, not scrolling Reddit."],
                    ["var(--c-comms)", "You’re asked once for each browser. Private and incognito windows are never read."],
                    ["var(--c-research)", "Optional — without it, UsageOS just uses the site name from the window title."],
                  ]}
                />
                <GrantBox
                  label="Automation"
                  sub="Prompts your browser the first time."
                  granted={automation}
                  onGrant={grantAutomation}
                />
              </>
            )}

            {step === 4 && (
              <>
                <Motif ready />
                <Eyebrow>You’re all set</Eyebrow>
                <H>That’s it.</H>
                <Lead>
                  UsageOS is tracking quietly from here. Open it whenever you want to look — there’s
                  nothing to keep up with.
                </Lead>
                {!(accessibility && automation) && (
                  <p className="mt-3.5 text-xs font-medium leading-normal text-muted">
                    Running with app-level data for now — you’ll see apps but not titles, projects
                    or sites. Grant the rest anytime in <b>Settings</b>.
                  </p>
                )}
              </>
            )}
          </div>

          {/* Foot */}
          <div className="mt-2 flex items-center justify-between gap-3 border-t-2 border-edge px-[22px] py-4">
            {step === 0 ? (
              <span aria-hidden className="invisible">·</span>
            ) : (
              <FootButton variant="out" onClick={back}>
                ← Back
              </FootButton>
            )}

            {step === 0 && (
              <FootButton onClick={next}>Get started →</FootButton>
            )}
            {step === 1 && <FootButton onClick={next}>Continue →</FootButton>}
            {step === 2 &&
              (accessibility ? (
                <FootButton onClick={next}>Continue →</FootButton>
              ) : (
                <FootButton variant="ghost" onClick={next}>Maybe later</FootButton>
              ))}
            {step === 3 &&
              (automation ? (
                <FootButton onClick={next}>Continue →</FootButton>
              ) : (
                <FootButton variant="ghost" onClick={next}>Skip</FootButton>
              ))}
            {step === 4 && <FootButton onClick={onComplete}>Open my day →</FootButton>}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Small presentational pieces (mirror design/onboarding.html) ───────────────

function Eyebrow({ children }: { children: React.ReactNode }) {
  return (
    <div className="mb-2.5 text-[11px] font-semibold uppercase tracking-[0.18em] text-c-deep">
      {children}
    </div>
  );
}

function H({ children }: { children: React.ReactNode }) {
  return (
    <div className="mb-3.5 font-display text-[38px] uppercase leading-[0.88]">{children}</div>
  );
}

function Lead({ children }: { children: React.ReactNode }) {
  return <div className="max-w-[46ch] text-[15.5px] font-medium leading-normal">{children}</div>;
}

function Num({ value, valueClass, label }: { value: string; valueClass?: string; label: string }) {
  return (
    <div className="flex-1 border-l-2 border-edge px-2 py-3.5 text-center first:border-l-0">
      <div className={`font-display text-[26px] leading-[0.85] ${valueClass ?? ""}`}>{value}</div>
      <div className="mt-2 text-[9.5px] font-semibold uppercase tracking-[0.1em] text-muted">
        {label}
      </div>
    </div>
  );
}

function Why({ items }: { items: [string, string][] }) {
  return (
    <ul className="mt-[18px] flex flex-col gap-3">
      {items.map(([color, text]) => (
        <li key={text} className="relative pl-[23px] text-[13.5px] font-medium leading-normal">
          <span
            className="absolute left-0 top-[5px] h-[11px] w-[11px]"
            style={{ background: color }}
          />
          {text}
        </li>
      ))}
    </ul>
  );
}

function GrantBox({
  label,
  sub,
  granted,
  onGrant,
}: {
  label: string;
  sub: string;
  granted: boolean;
  onGrant: () => void;
}) {
  return (
    <div className="mt-[18px] flex items-center justify-between gap-3.5 border-2 border-edge px-[15px] py-[13px]">
      <div>
        <div className="text-sm font-semibold">{label}</div>
        <div className="text-xs text-muted">{sub}</div>
      </div>
      {granted ? (
        <span className="border-2 border-edge bg-edge px-[11px] py-[5px] text-[11px] font-semibold uppercase tracking-[0.06em] text-bg">
          Granted ✓
        </span>
      ) : (
        <button
          type="button"
          onClick={onGrant}
          className="whitespace-nowrap border-2 border-edge bg-bg px-[11px] py-[5px] text-[11px] font-semibold uppercase tracking-[0.06em] text-fg"
        >
          Grant access
        </button>
      )}
    </div>
  );
}

function FootButton({
  variant = "solid",
  onClick,
  children,
}: {
  variant?: "solid" | "out" | "ghost";
  onClick: () => void;
  children: React.ReactNode;
}) {
  const base =
    "border-2 px-5 py-[11px] text-xs font-semibold uppercase tracking-[0.08em] transition-colors";
  const styles = {
    solid: "border-edge bg-edge text-bg",
    out: "border-edge bg-transparent text-fg",
    ghost: "border-transparent bg-transparent pl-0 text-muted",
  }[variant];
  return (
    <button type="button" onClick={onClick} className={`${base} ${styles}`}>
      {children}
    </button>
  );
}

/** The small three-arc dial motif on the Welcome / Ready steps. */
function Motif({ ready = false }: { ready?: boolean }) {
  return (
    <div className="mb-[18px] h-[60px]">
      <svg viewBox="0 0 60 60" height="60" role="img" aria-label="UsageOS">
        <circle cx="30" cy="30" r="22" fill="none" stroke="var(--track)" strokeWidth="6" />
        <circle
          cx="30"
          cy="30"
          r="22"
          fill="none"
          stroke="var(--c-deep)"
          strokeWidth="6"
          strokeDasharray={ready ? "64 74" : "56 82"}
          transform="rotate(-90 30 30)"
        />
        <circle
          cx="30"
          cy="30"
          r="22"
          fill="none"
          stroke={ready ? "var(--c-research)" : "var(--c-comms)"}
          strokeWidth="6"
          strokeDasharray={ready ? "20 118" : "18 120"}
          strokeDashoffset={ready ? "-64" : "-56"}
          transform="rotate(-90 30 30)"
        />
        {ready && <polygon points="30,5 26,13 34,13" fill="var(--edge)" />}
      </svg>
    </div>
  );
}
