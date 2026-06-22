# Decision Log (ADR-style)

Each entry: the decision, why, and rejected alternatives. Append new decisions; don't rewrite. Locked decisions shouldn't be relitigated without a stated reason.

_All decisions below: 2026-06-22, from the end-to-end vision interview._

### D1 — Pure observer, not a coach
The app never interrupts (except an opt-in evening recap ping). **Why:** a calm mirror is maximally trustworthy and a distinct identity from the sibling "Nudge" app. **Rejected:** opt-in nudges (reintroduces productivity-guilt we cut), live menubar HUD (rejected as noise).

### D2 — Two-axis model: context × project
**Why:** "you gave usage_os a 90-min block" needs the project axis; context alone is too coarse. **Rejected:** contexts-only (vague recaps), projects-only (no productive/distracting read).

### D3 — The dial is the soul; ship order dial → categorization → recap
**Why:** the dial is the signature and works with zero AI — lowest-risk path to a real product. **Rejected:** recap-first (depends on the riskiest piece), treating them as inseparable (bigger, riskier v1).

### D4 — Capture: Accessibility + Automation; never Screen Recording
**Why:** titles are empty today because the current crate falls back to CGWindowList (needs Screen Recording). AX (Accessibility) gets titles with a lighter, less-scary permission; AppleScript/Automation gets exact browser URLs. Screen Recording is the worst look for a privacy app. **Evidence:** DB inspection showed ~363/370 Chrome rows and ~100% of Cursor/iTerm rows with empty titles. **Rejected:** Accessibility-only (weaker browser data), app-level-only (guts the product), Screen Recording (toxic optics).

### D5 — Event-driven capture + heartbeat
**Why:** NSWorkspace activation + AX focus-change observers give second-accurate boundaries and near-zero idle CPU; a slow heartbeat handles idle + long-running windows. **Rejected:** 5s polling (blurry boundaries, constant wakeups).

### D6 — Projects auto-inferred (+ correctable); needs a spike
**Why:** low effort, gets smarter. **Risk:** inference accuracy is unproven — **Phase 0 spike required** before committing the pattern. **Rejected:** manual rules (upkeep forever), defer-to-AI (loses the axis early).

### D7 — Pre-AI sorting: smart default rules + edits
**Why:** real value day one (VS Code→Deep, Slack→Comms…), AI refines later. **Rejected:** blank cold-start, uncategorized-until-AI (kills the dial's colors).

### D8 — Sensitive data: raw-local + exclusion list
**Why:** all local, but a privacy product must let you exclude password managers/banking, mark private apps (time counts, no title), and never record incognito. **Rejected:** raw-no-filter (invasive on screen-share), categories-only (guts detail).

### D9 — Smart runtime: Apple Foundation Models via Swift sidecar; template fallback always
**Why:** on macOS 26 it's free, fully on-device, zero-install, structured output. Cost: a thin Swift bridge + Apple-Silicon/AI-on requirement (fallback covers the rest). **Rejected:** Ollama (install friction), embedded model (app bloat).

### D10 — Categorization via embeddings + corrections
**Why:** fast, stable, can't hallucinate a category; learns from each fix; NaturalLanguage gives on-device embeddings free. **Rejected:** nightly LLM classify (drift), rules+AI-suggestions only (too manual).

### D11 — Recap: lazy + opt-in evening ping; model narrates only
**Why:** computed on open keeps it an "open it to look" ritual; one optional bedtime notification is the sole sanctioned interruption. Numbers are computed in Rust; the model only phrases them. **Rejected:** never-notify (easy to forget), always-live (loses the end-of-day moment).

### D12 — On-device only for v1 (cloud is a future maybe)
**Why:** "nothing leaves this machine" is the moat and must be literally true + auditable. **Status:** BYO-key cloud is a deferred, opt-in-with-warnings possibility, not built.

### D13 — App shape: menubar launcher + main window
**Why:** always-running and one click to look, without a live HUD (rejected in D1). **Rejected:** popover-primary (cramps the dial), dock-only (easy to forget it runs).

### D14 — Dial: fixed 24h, midnight top; idle = faint hollow arcs
**Why:** consistent scale makes the week comparable (the whole point of the week view); the clock metaphor stays pure; empty arc = sleep is honest. A "day starts at 4am" offset is a later setting for night owls. **Rejected:** rolling 24h (breaks the clock + "your day"), auto-fit (every day looks identical — demonstrated in a mockup), per-user configurable scale (complexity).

### D15 — Linear timeline in v1 (secondary), dark mode at launch
**Why:** the timeline is the same event log rendered differently (cheap); dark mode is doable cleanly via token-based theming and many Mac users live in dark. **Note:** dark Bauhaus must be *designed*, not auto-derived — real design time.

### D16 — Architecture: Rust core (objc2) + thin Swift AI sidecar
**Why:** keep ~90% in one language; objc2 reaches NSWorkspace + AX from Rust directly; Swift is needed only for the Swift-only Foundation Models framework, kept to a minimal stdio sidecar. **Rejected:** bigger Swift core (splits codebase), pure-Rust-defer-AI (no smart recap).

### D17 — IPC: tauri-specta generated bindings
**Why:** the Rust↔TS boundary is where drift hides; generating the client+types makes a shape mismatch a compile error. The biggest "trust code you didn't read" lever. **Rejected:** hand-written wrappers (silent desync), ts-rs types-only (half the win).

### D18 — Persistence: rusqlite + typed repository + integration tests
**Why:** tiny schema + analytics-heavy queries where ORMs are weak; simplest, most auditable, no async footguns for a single-writer local DB. **Rejected:** ORM (Diesel/SeaORM — wrong fit, heavy magic, you drop to raw SQL anyway), sqlx (compile-checked SQL is nice but async + a stale-able offline-prepare cache isn't worth it here). _Open to sqlx if compile-time SQL checking becomes a priority._ **Confirmed by reality:** `main` already uses rusqlite + a versioned migration system (`schema_migrations`) + tests — this matches what's built.

### D19 — Design system: Claude Design project + in-repo `design-system.md`, designed end-to-end first
**Why:** a living visual source of truth + a machine-readable spec the coder obeys, fully designed (all components, both themes, all states) **before** any UI code, to eliminate drift. **Dependency:** pushing to Claude Design needs the claude.ai design login / `/design-login`.

### D20 — Open source MIT day one; notarized direct distribution; free + sponsor
**Why:** auditability is the privacy proof; MAS sandbox forbids our permissions; free maximizes trust/adoption. **Rejected:** closed source (undercuts the claim), MAS (impossible), paid-upfront (wrong for OSS v1).

### D21 — Onboarding: primed first-run, run degraded
**Why:** explain *why* each permission, deep-link to Settings, never wall the app; degraded (app-level) if declined. **Rejected:** hard gate (first-launch wall), silent native prompts (scary, unexplained).

### D22 — First milestone: native capture spike (isolated) → then the real dial
**Why:** prove the riskiest bit (AX titles + Automation URLs + NSWorkspace events) in isolation before product code, then wire to DB and render today's real dial. Design system proceeds as a parallel gating track.

### D23 — Discard the uncommitted XP/goals WIP — but build ON existing `main`
**Why:** the XP/goals/old-SettingsView work was uncommitted and off-direction → dropped. **Correction:** an earlier draft said "start clean from scratch" — that was based on a stale local clone ~10 commits behind origin. The real `main` is a healthy, tested, CI'd v0.1.0 + OSS-hygiene foundation; we build on it (see D24).

### D24 — Evolve the existing codebase, don't rebuild
**Why:** `main` already has the Tauri shell, rusqlite + migrations, the rules engine, 24 Rust + 28 TS tests, CI, retention, and OSS docs — all worth keeping. The redesign changes: **capture** (`active_win_pos_rs` → AX/Automation/NSWorkspace via objc2), the **data model** (new migrations for projects/sites/contexts/recaps/embeddings/exclusions), the **UI** (cyberpunk dashboard → Bauhaus dial), and adds the **Swift AI sidecar + tauri-specta**. Keep the migration system, tests, CI, and repo hygiene.

### D25 — Follow the existing agent/PR workflow
**Why:** the repo runs a structured workflow (`context/plans/` with dated, task/workflow-numbered docs; PR-based; a headless-Linux build box that can't launch the desktop GUI). The redesign plan slots into it; capture/AX work is validated on macOS only. _Lesson: always fetch before assuming local state — this session started ~10 commits stale._

---

_D26–D28 added 2026-06-22 from the Phase-0 whole-project feasibility audit (`context/feasibility/2026-06-22-feasibility-audit.md`)._

### D26 — Embeddings run in Rust (NaturalLanguage via `objc2-natural-language`); the Swift sidecar stays Foundation-Models-only
**Refines D10 + D16.** **Why:** `objc2-natural-language` exposes `NLEmbedding`/`NLContextualEmbedding` to Rust, so the categorization layer can live in `enrich/` behind a mockable trait — no second Swift binary, and the Swift surface stays minimal (FM recap only, per D16). The matcher is closed-vocabulary, so it **cannot hallucinate** a category (returns an existing `category_id` or "unclassified") — but accuracy on short / code-heavy / non-English titles is unproven and must be measured on real data (R43; embeddings R39–R44). Default to `NLEmbedding.sentenceEmbedding` (512-d, no download); `NLContextualEmbedding` (better, multilingual) is opt-in because it needs an async asset download. **Rejected:** embeddings inside the Swift sidecar (grows the Swift surface, couples categorization to the AI process); a bundled Rust embedding model (app bloat).

### D27 — Pin the `tauri-specta` RC trio exactly (`=`); accept a pre-1.0 dependency for the IPC contract
**Why:** hard rule 2's generated IPC is load-bearing, but the whole stack is still a release candidate (`tauri-specta` / `specta` / `specta-typescript` ~`2.0.0-rc.x`; no stable `specta` since 2022). Exact-pin the trio with `=`, bump them together, and record the resolved versions in `Cargo.lock`. A known bug (#211: event payloads infer to `never` on rc.24) means **commands-only at launch** — the dial/recap are pull-based; add typed events only once the bug is confirmed fixed. **Rejected:** waiting for a stable release (no timeline; blocks the entire IPC story); hand-written bindings (forbidden by hard rule 2). _Re-evaluate when `specta` v2 ships stable._

### D28 — Phase-0 feasibility verdict: **GO-WITH-CAVEATS**; Phase 1 product code is gated on the native spike
**Why:** the whole-project audit (risk register R1–R83) found the design buildable as specified, with one make-or-break unknown — whether AX returns real titles for Chromium/Electron apps + editors (Chrome, Cursor, VS Code) under Accessibility alone (R4). The dial ships with zero native/AI (the prototype works), so a real v1 survives even if native bets fail; the trait-based architecture degrades gracefully. **Status:** Phase 1 product code does not start until Spikes 1–4 (audit §4) pass on a real Apple-Silicon Mac running macOS 26, with the dev-build signing identity stabilized first (R14). Two repo facts the audit surfaced, to fix before/within Phase 1: hard rule 3 is **currently violated** (production `.expect()` in `lib.rs`/`watcher.rs`/`db.rs` — must be cleaned up before the `clippy -D warnings` gate, R82), and the unused `recharts` dependency must be removed (honor "no chart library", R77). Confirms and extends D22.

_D29 added 2026-06-22 from Spike ② (`spikes/ax-observer/`)._

### D29 — Event-driven capture model: `objc2-application-services` `AXObserver` on the **main run loop**, marshaled to async over a `Send` channel
**Refines D5; proven by Spike ②.** **Why:** the Phase-1 `capture` impl is now pinned to a model that was measured working end-to-end on the real Mac, Accessibility-only. The shape: a `NSWorkspace` `didActivateApplication` observer (a Rust closure in `block2::RcBlock`; **keep the returned token alive**) fires on each app switch; on switch we tear down the old per-PID `AXObserver` and build a fresh one (its run-loop source auto-detaches on release — remove it first to avoid a UAF against the live callback `refcon`); the observer watches `AXFocusedWindowChanged` on the **application** element, and `AXTitleChanged` on the **focused window** (re-pointed whenever the focused window changes, or in-app title changes never fire); chatty duplicate titles are dropped by a dedupe debounce; every event is `send`-marshaled to the existing Tokio runtime over an unbounded `Send` channel — a non-blocking enqueue, so the executor never stalls and the run loop never blocks. **Three findings that closed open questions:** (1) **`AXObserver` is exposed by `objc2-application-services` 0.3.2** (`AXObserver::create` / `add_notification` / `run_loop_source`) — so the whole capture surface stays in one objc2 family and **`accessibility-sys` is dropped** (corrects the architecture doc + the capture standard's "pick one" note); (2) **no `NSApplication` is needed** — a bare `CFRunLoop` delivers activation notifications, and in the app we register sources into **Tauri's main run loop** during `setup` rather than owning the loop; (3) AX queries + the observer source both run on the **main thread** (D5's "AX is main-thread" stands; the constraint is *where the source is serviced*, not that queries are main-only). **Rejected:** a dedicated `std::thread` running its own `CFRunLoopRun()` (unnecessary once the main-loop model was proven, and it would add cross-thread AX-safety questions); `accessibility-sys` for observers (a second FFI style for no gain). Embeddings/idle/etc. unaffected. _Residual: pure in-place `AXTitleChanged` delivery still to be confirmed in a manual browser-navigation pass; the production debounce may add a trailing-edge timer to also coalesce rapid distinct titles._

_D30 added 2026-06-22 from the project-inference spike (`spikes/project-inference/`)._

### D30 — Project identity is canonicalized on the git remote; assignment abstains below a confidence/ambiguity threshold
**Refines D6; informed by the project-inference spike.** **Why:** the spike measured the inference heuristic on real signals and made **zero false assignments**, but surfaced two design requirements. (1) **Canonical project identity = the git remote `owner/repo`.** The same project arrives under different ids depending on the signal — `cwd → git remote` gives `f-gozie/usage-os`, while the window title / folder gives `usage_os`, and a `github.com/owner/repo` URL gives `usenudgeai/nudge` while a local file gives `nudge`. Without reconciliation **one project fragments into several**, so the `projects` table keys on the git remote and stores the folder name, title-derived name, and any GitHub URL as **aliases** that resolve to it (folder name is the fallback key when a repo has no remote). (2) **An explicit abstain threshold** (the calm-mirror principle: a wrong project label costs more trust than a gap): assign a project only on a HIGH (`cwd-git-remote`, `github-url`) or MED (`local-file`, `window-title`) **and unambiguous** signal; otherwise persist **`unassigned`**. Signal-precision order: `cwd → git remote` is the anchor (deterministic, canonical, no false positives), corroborated by `github-url` (R26); window-title is the weakest (folder-name only). (3) **`ambiguous` is a distinct third state** from `no-signal`: `localhost` and dev dashboards (PostHog, Grafana, Cloudflare, App Store Connect) are clearly *work* but project-unknown — abstain now, but **persist the ambiguous reason** so Phase 2 can temporally **correlate** them to the project active in the editor/terminal at the same time (never correlate `no-signal` browsing like YouTube/X/Gmail). **Rejected:** folder name as the canonical key (fragments across renames/forks/worktrees); guessing a project for ambiguous tooling (the main false-positive source); a single binary "has project / no project" flag (loses the correlate-later signal). _Measured on a snapshot — precision + the abstain threshold; multi-day **recall**/volume is re-measured once Phase-1 capture persists data (R23/R26/R27)._
