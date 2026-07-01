import { useEffect, useState } from "react";

import { GrantedPill } from "@/components/ui/GrantedPill";
import { usePermissions } from "@/hooks/usePermissions";
import {
  getLaunchAtLogin,
  requestAccessibility,
  requestAutomation,
  setLaunchAtLogin,
} from "@/lib/tauri";
import { autoUpdateEnabled, setAutoUpdateEnabled } from "@/lib/updater";

const STEPS = [
  "Welcome",
  "Your privacy",
  "Accessibility",
  "Automation",
  "Background",
  "Updates",
  "Ready",
];

/**
 * First-run onboarding: Welcome → Privacy → Accessibility → Automation → Background → Updates →
 * Ready. Ported from `design/onboarding.html`. Permissions are optional — every grant step can be
 * skipped, landing on a degraded (app-only) "Ready". Start-at-login (D68) and auto-update (D67)
 * are opt-in, recommended here. Live permission status comes from `usePermissions` (re-reads on
 * window focus).
 */
export function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const { permissions, refetch } = usePermissions();

  const accessibility = permissions?.accessibility ?? false;
  const automation = permissions?.automation === "granted";

  const next = () => setStep((s) => Math.min(s + 1, STEPS.length - 1));
  const back = () => setStep((s) => Math.max(s - 1, 0));

  const [noBrowser, setNoBrowser] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(false);
  const [launchAtLogin, setLaunchAtLoginState] = useState(false);
  useEffect(() => {
    void autoUpdateEnabled().then(setAutoUpdate).catch(() => undefined);
    void getLaunchAtLogin().then(setLaunchAtLoginState).catch(() => undefined);
  }, []);
  const enableAutoUpdate = () => {
    setAutoUpdate(true);
    void setAutoUpdateEnabled(true).catch(() => undefined);
  };
  const enableLaunchAtLogin = () => {
    setLaunchAtLoginState(true);
    void setLaunchAtLogin(true).catch(() => undefined);
  };

  const grantAccessibility = () => void requestAccessibility().then(refetch).catch(() => undefined);
  const grantAutomation = () =>
    void requestAutomation()
      .then((outcome) => {
        setNoBrowser(outcome === "no_browser_running");
        refetch();
      })
      .catch(() => undefined);

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
                <BuildingWordmark />
                <div className="ob-copy">
                  <Eyebrow>A private, on-device time tracker</Eyebrow>
                  <H>
                    Where did your <span className="text-c-research">day</span> go?
                  </H>
                  <Lead>
                    Your computer already knows how you spent today. UsageOS quietly keeps track, then
                    tells you the story — where you focused, what pulled you away, and where the
                    hours actually went.
                  </Lead>
                </div>
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
                  file on your machine, and the code is open for anyone to read.
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
                    ["var(--c-research)", "Titles stay on this machine, and you can hide any app you don’t want tracked."],
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
                {noBrowser && !automation && (
                  <p className="mt-3 text-xs font-medium leading-normal text-muted">
                    No browser is open, so there’s nothing to ask yet. Open your browser — UsageOS
                    will request access the first time you use it, or grant it here once it’s
                    running.
                  </p>
                )}
              </>
            )}

            {step === 4 && (
              <>
                <Eyebrow>Always on</Eyebrow>
                <H>
                  Runs quietly in
                  <br />
                  the background
                </H>
                <Why
                  items={[
                    ["var(--c-deep)", "UsageOS lives in your menu bar. Close the window and tracking keeps going — no Dock icon hanging around."],
                    ["var(--c-comms)", "Start at login and your day is tracked from the moment you sit down. Nothing to remember, nothing to open."],
                    ["var(--c-research)", "Off by default. Turn it on here (recommended), or anytime in Settings."],
                  ]}
                />
                <GrantBox
                  label="Start at login"
                  sub="Recommended. Change anytime in Settings."
                  granted={launchAtLogin}
                  onGrant={enableLaunchAtLogin}
                  grantLabel="Enable"
                  grantedLabel="Enabled ✓"
                />
              </>
            )}

            {step === 5 && (
              <>
                <Eyebrow>One last thing</Eyebrow>
                <H>
                  Keep it
                  <br />
                  up to date
                </H>
                <Why
                  items={[
                    ["var(--c-deep)", "UsageOS can ask GitHub once a day whether a newer version exists. It sends only the version number — never your activity or any tracked data."],
                    ["var(--c-comms)", "It’s how fixes reach you, and every update is signed so a tampered one can’t install."],
                    ["var(--c-research)", "Off by default. Turn it on here (recommended), or anytime in Settings."],
                  ]}
                />
                <GrantBox
                  label="Automatic updates"
                  sub="Recommended. Change anytime in Settings."
                  granted={autoUpdate}
                  onGrant={enableAutoUpdate}
                  grantLabel="Enable"
                  grantedLabel="Enabled ✓"
                />
              </>
            )}

            {step === 6 && (
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
            {step === 4 &&
              (launchAtLogin ? (
                <FootButton onClick={next}>Continue →</FootButton>
              ) : (
                <FootButton variant="ghost" onClick={next}>Not now</FootButton>
              ))}
            {step === 5 &&
              (autoUpdate ? (
                <FootButton onClick={next}>Continue →</FootButton>
              ) : (
                <FootButton variant="ghost" onClick={next}>Not now</FootButton>
              ))}
            {step === 6 && <FootButton onClick={onComplete}>Open my day →</FootButton>}
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
  grantLabel = "Grant access",
  grantedLabel,
}: {
  label: string;
  sub: string;
  granted: boolean;
  onGrant: () => void;
  grantLabel?: string;
  grantedLabel?: string;
}) {
  return (
    <div className="mt-[18px] flex items-center justify-between gap-3.5 border-2 border-edge px-[15px] py-[13px]">
      <div>
        <div className="text-sm font-semibold">{label}</div>
        <div className="text-xs text-muted">{sub}</div>
      </div>
      {granted ? (
        <GrantedPill label={grantedLabel} />
      ) : (
        <button
          type="button"
          onClick={onGrant}
          className="whitespace-nowrap border-2 border-edge bg-bg px-[11px] py-[5px] text-[11px] font-semibold uppercase tracking-[0.06em] text-fg"
        >
          {grantLabel}
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
    // Always-inverted pair (legible in every theme — `bg-edge`/`text-bg` is dark-on-dark on the
    // dark themes); mirrors the titlebar treatment.
    solid: "border-edge bg-bar-bg text-bar-fg",
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

/** The Welcome opening beat (first run only): USAGE_S settles in, then the dial draws into the gap
 *  as the O — the logo assembling itself. CSS-driven (index.css `.ob-*`), one-shot, reduced-motion
 *  safe. The dial-O carries a faint track ring from the start so it reads as a letter while the
 *  coloured runs draw in. */
function BuildingWordmark() {
  const pre = ["U", "S", "A", "G", "E"];
  return (
    <div className="mb-[18px]">
      <span
        role="img"
        aria-label="UsageOS"
        className="inline-flex items-baseline font-display text-[44px] uppercase leading-none tracking-[0.02em] text-fg"
      >
        {pre.map((c, i) => (
          <span key={i} className="ob-rise inline-block" style={{ animationDelay: `${i * 45}ms` }}>
            {c}
          </span>
        ))}
        <span
          aria-hidden
          className="ob-fade inline-block"
          style={{
            width: "1.02em",
            height: "1.02em",
            margin: "0 -0.05em",
            transform: "translateY(0.08em)",
            animationDelay: "225ms",
          }}
        >
          <svg viewBox="0 0 100 100" className="block h-full w-full overflow-visible">
            <circle cx="50" cy="50" r="34" fill="none" stroke="var(--track)" strokeWidth="17" />
            <path className="ob-arc" pathLength={100} d="M55.90 16.52 A34 34 0 0 1 80.02 65.96" fill="none" stroke="var(--c-deep)" strokeWidth="17" />
            <path className="ob-arc" pathLength={100} d="M71.85 76.04 A34 34 0 0 1 27.25 75.27" fill="none" stroke="var(--c-research)" strokeWidth="17" />
            <path className="ob-arc" pathLength={100} d="M19.44 64.91 A34 34 0 0 1 44.10 16.52" fill="none" stroke="var(--c-comms)" strokeWidth="17" />
          </svg>
        </span>
        <span className="ob-rise inline-block" style={{ animationDelay: "270ms" }}>
          S
        </span>
      </span>
    </div>
  );
}
