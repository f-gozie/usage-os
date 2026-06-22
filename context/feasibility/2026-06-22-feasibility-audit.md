# UsageOS — Whole-Project Feasibility Audit

_Date: 2026-06-22 · Scope: the entire redesign as specified in `CLAUDE.md`, `vision.md`, `decisions.md` (D1–D25), `architecture.md`. Inputs: 15 area desk-research dossiers under `/tmp/usageos_research/`._

> **CRITICAL CAVEAT — read first.** This audit is **desk research only**. There was **no independent verification pass** and **no execution on a real Apple-Silicon Mac running macOS 26**. Every native API name, crate version, OS-gating, permission behavior, and latency figure below is **provisional**. Items that secondary sources could not confirm are tagged **⚠️ unverified — spike required**. Treat this document as a map of where to point the spikes, not a guarantee that any native call compiles or returns what we expect. The product's riskiest claims are precisely the ones that cannot be settled from documentation.

---

## 1. Executive read

**Verdict: GO-WITH-CAVEATS.**

The project is buildable as designed, and the strategic shape is sound. The foundation already exists and is healthy (rusqlite + versioned migrations, CI, tests, the rules engine — keep per D24). The redesign's three pillars decompose into one genuinely de-risked pillar, one mechanically-feasible pillar, and one cluster of native unknowns:

- **The dial (D3, D14) is the lowest-risk pillar and the right thing to ship first.** A complete working SVG implementation already exists in `prototype/index.html`; porting it to React is low-risk. It needs **zero AI and zero native capture** to be a real product. This single fact is what makes the overall verdict GO rather than NO-GO: there is a valuable v1 that does not depend on any of the unproven native work.
- **The "plumbing" pillar (persistence D18, IPC D17, app shell D13, distribution D20) is feasible-with-caveats** — all mechanical, all on current/maintained crates, no fundamental blockers, but with real pre-1.0 churn risk (tauri-specta is still RC) and a notarization footgun for the embedded sidecar.
- **The native + AI pillar (capture D4/D5, project inference D6, Foundation Models D9/D16, embeddings D10, TCC) is the whole-project risk.** None of it can be proven from desk research; all of it is gated on a real Mac. The architecture's saving grace is that it is correctly designed for graceful degradation — capture behind a `capture` trait, AI behind an `ai` trait with a deterministic template fallback, embeddings rankable-or-defer-to-rules — so even if individual native bets fail, the product still functions in a degraded mode.

### The 3–5 things that most threaten the project

1. **AX window titles for Chromium/Electron apps and editors WITHOUT Screen Recording (D4).** This is the make-or-break bet. The whole capture redesign exists because the current `active-win-pos-rs`/`CGWindowList` path returns empty titles (~363/370 Chrome rows empty). The fix — read titles via the AX API under the Accessibility permission only — is well-founded for native apps, but whether AX returns a *real, non-empty* focused-window title for **Chrome, Cursor, VS Code** (the user's most important apps, which lazily build their a11y tree and gate it behind `AXEnhancedUserInterface`) is the single most-likely-to-fail assumption in the entire project, and is unconfirmed by any authoritative source. If it fails for editors+browsers, project inference (D6) and the recap's richness collapse with it. **⚠️ unverified — spike required.**
2. **The native threading/run-loop model (D5).** Both NSWorkspace notification delivery and the AXObserver run-loop source require a *running* CFRunLoop, while the watcher lives on a Tokio task. Picking the wrong model (main run loop vs dedicated CFRunLoop thread) silently produces callbacks that never fire and wastes the entire native spike. AX is also documented main-thread-only. **⚠️ unverified — spike required.**
3. **Foundation Models hard requirements + sidecar notarization (D9, D16, D20).** FM needs macOS 26 + Apple Silicon + Apple Intelligence ON — a large fraction of users will never get it, so the template fallback is the *primary* path, not a nicety. Separately, an **open, untriaged Tauri bug (#11992)** reports that `externalBin` sidecars break notarization ("signature of the binary is invalid"). This threatens the distribution story for the one piece of Swift in the project. **⚠️ unverified — spike required.**
4. **Project-inference accuracy (D6) and cross-process cwd reads.** D6 itself flags accuracy as unproven and Phase-0-gated. Title parsing is feasible; reading another process's cwd via `proc_pidinfo` from a non-root, unsandboxed binary is **unconfirmed** (may return EPERM), which would kill the terminal-cwd branch. Accuracy is a *product* risk, not just an API one — false positives erode trust in a "calm mirror." **⚠️ unverified — spike required.**
5. **Pre-1.0 / RC dependency churn on a load-bearing contract (D17).** `tauri-specta` v2 is still a release candidate (latest `2.0.0-rc.25`); the typed IPC boundary — hard rule 2 — is built on it. A known bug (#211) makes generated event payloads infer to `never` on rc.24. Must exact-pin the trio and decide commands-only-first.

A sixth, non-native risk worth surfacing now: the **Bauhaus "Comms" yellow (#F2BC0C) fails WCAG non-text contrast** (1.47:1 vs paper; needs ≥3:1) — a design blocker on the locked palette, not a code one, but it must be resolved before `design-system.md` is frozen.

---

## 2. Risk register

Verdict legend: **feasible** / **feasible-with-caveats (FWC)** / **uncertain** / **blocked**. Confidence: H/M/L. Every native/version claim is provisional (see §1 caveat).

| # | Material assumption | Verdict | Conf | Evidence (citation) | What the spike must prove | Backs |
|---|---|---|---|---|---|---|
| R1 | A maintained Rust crate exposes AXUIElement (`objc2-application-services` 0.3.2) | feasible | H | docs.rs objc2-application-services 0.3.2 struct AXUIElement [C1] | Add crate; call `new_application(pid).copy_attribute_value(...)` end-to-end; confirm the *resolved* version exposes it (one stale 2024 source claimed objc2 lacked it) | D4, D16 |
| R2 | AX titles need ONLY Accessibility, NOT Screen Recording | feasible | H | active-win-pos uses CGWindowList/`kCGWindowName` (Screen-Recording-gated); AX is a different mechanism [C2] | Grant only Accessibility (SR off), read real titles; toggle SR, confirm no effect | D4 |
| R3 | AX returns real titles for native apps (Terminal, Safari, Finder) | feasible | H | Standard documented AX pattern [C3] | Read non-empty titles for Terminal/Safari/Finder while frontmost | D4 |
| R4 | **AX returns usable titles for Chromium/Electron (Chrome, Cursor, VS Code) + iTerm2** ⚠️ unverified | FWC | M | Chromium lazily builds a11y tree, `AXManualAccessibility` unsettable externally (electron #37465); top-level NSWindow AXTitle *usually* present but unconfirmed [C4] | With only Accessibility, read focused-window AXTitle for Cursor/VS Code/Chrome/Brave/iTerm2; record real/empty/nil per app; this is the #1 make-or-break | D4, D6 |
| R5 | AX permission detectable/promptable via `AXIsProcessTrusted(WithOptions)` + `kAXTrustedCheckOptionPrompt` | feasible | H | docs.rs all-items lists the fns + static [C5] | Call with prompt option; confirm system dialog appears and bool reflects grant | D4, D21 |
| R6 | AX calls are main-thread-only ⚠️ unverified threading model | FWC | M | Apple DTS "AX functions safe only on main thread" [C6]; research notes queries may be off-main but observer source needs a live run loop | Prove a working model: drive AX on main run loop, deliver to Tokio without blocking executor; stable over many switches | D5 |
| R7 | Attribute constants (kAXTitle/kAXFocusedWindow) absent from crate; build CFStrings manually | FWC | H | crate all-items page does not list the constants [C5] | `CFString::from_static_str("AXFocusedWindow"/"AXTitle")` → `copy_attribute_value` returns Success + non-null | D4 |
| R8 | NSWorkspace activation events from Rust (`objc2-app-kit` 0.3.2) | feasible | H | docs.rs NSWorkspace sharedWorkspace/notificationCenter [C7] | Observe `didActivateApplicationNotification`; callback fires per switch; read NSRunningApplication name/bundleId | D5 |
| R9 | Block-based notification observer via `block2::RcBlock` | FWC | M | objc2-foundation gates `addObserverForName...usingBlock` on NSOperation+NSString+block2; token must be retained [C8] | RcBlock fires, token kept alive, clean removeObserver, no UAF across switches | D5 |
| R10 | AXObserver focus/title-change API (`accessibility-sys` 0.2.0) | feasible | H | accessibility-sys 0.2.0 exposes AXObserverCreate/AddNotification/GetRunLoopSource + constants [C9] | Per-PID observer fires on window/title change; rebuilt on app (PID) change | D5 |
| R11 | Event-driven + heartbeat → near-zero idle CPU | FWC | M | Push-based design; but `kAXTitleChangedNotification` can be chatty (browser load, progress) [C10] | Measure idle wakeups/CPU; confirm chatty-title apps don't storm; debounce works | D5 |
| R12 | Idle detection needs no extra permission (`user-idle` 0.6) | feasible | H | Already shipped in v0.1.0; reads CG aggregate idle time, not CGEventTap [C10] | With only Accessibility, idle grows/resets; independent of AX path | D5 |
| R13 | Crates compile + run on aarch64-apple-darwin ⚠️ unverified | uncertain | M | docs.rs renders accessibility-sys x86_64 only (display artifact); objc2 supports aarch64 [C9] | Build spike for aarch64 on real Apple Silicon; callbacks fire at runtime | D4, D5, D16 |
| R14 | Granted permission attaches to the dev binary (identity churn) | FWC | H | TCC keys to code-signing identity; rebuilds change cdhash [C11] | Stable dev-sign (ad-hoc stable id or Developer ID); AX trust persists across ≥2 rebuilds | D4, D22 |
| R15 | Chromium front-tab URL via `URL of active tab of front window` | feasible | H | Chromium scripting.sdef defines active tab + URL [C12] | osascript returns exact URL for Chrome/Brave/Arc; correct app names (Brave="Brave Browser", Arc="Arc") | D4 |
| R16 | Safari URL via `URL of front document` | feasible | H | Multi-browser gist + Safari refs [C13] | Returns front-tab URL; behavior with multiple windows/Tab Groups | D4 |
| R17 | **Incognito Chromium excluded by reading window `mode`** ⚠️ unverified readability | FWC | H | sdef defines `mode` ('normal'/'incognito') but says "set only once during creation" — readability on existing window unconfirmed [C12] | Confirm `mode of front window` is *readable* and returns "incognito"; check BEFORE any URL read | D8 |
| R18 | Safari Private Browsing detectable ⚠️ fragile | FWC | M | No AppleScript property; only System Events Window-menu inspection ("Move Tab to New Private Window"), locale/version-fragile [C14] | Reliable detection on current Safari; safe-default = no URL when uncertain (never record private) | D8 |
| R19 | Automation TCC: per-(client,target), -1743 on deny, needs NSAppleEventsUsageDescription | FWC | H | Mojave+ per-pair consent; missing usage string → -1743 with no prompt [C15] | One prompt per browser; granting persists; deny → -1743 → graceful per-browser fallback | D4, D21 |
| R20 | Apple Events sent from main Tauri process (simpler than helper) | FWC | M | Tauri supports custom Info.plist merge + entitlements; helper binary would need its own embedded plist + disclaim [C16] | Prompt names "UsageOS"; signing/entitlement combo works; prefer main process | D4, D16 |
| R21 | osascript shell-out latency acceptable for event-driven capture | feasible | M | osascript standard, process-isolated; no authoritative latency bench [C17] | Measure cold/warm latency; confirm no jank on rapid switches; decide NSAppleScript only if needed | D4, D5 |
| R22 | **proc_pidinfo cwd read from non-root unsandboxed process** ⚠️ unverified | uncertain | L | Struct/flavor correct; sources confirm only the *negative* (sandbox blocks); unsandboxed-no-root case undocumented [C18] | Prove EPERM-or-success on live iTerm2/Terminal PID with NO sudo; if EPERM, the whole cwd branch dies | D6 |
| R23 | VS Code/Cursor project folder in window title by default | FWC | H | window.title default includes `${rootName}`; Cursor is a fork [C19] | Measure on real title corpus what fraction has a parseable folder token = intended project; handle no-folder/multi-root/customized | D6 |
| R24 | iTerm2 cwd via `path` var / Terminal.app via tty→pid→cwd | FWC | M | iTerm2 `path` documented for Python API (AppleScript deprecated); Terminal exposes tty not cwd [C20] | At least one Rust-reachable route works; depends on R22 for the tty→cwd leg | D6 |
| R25 | Editor-integrated terminals have NO external cwd surface | blocked | H | Not independently scriptable; only the editor title's folder token [C19] | Confirm; document integrated-terminal attribution as title-only (known limitation) | D6 |
| R26 | GitHub owner/repo parseable from browser titles | FWC | M | GitHub titles embed `owner/repo`; depends on R4 (Chrome AX title) [C4] | Measure extractability; tolerate title-format drift | D6 |
| R27 | **Project-inference accuracy useful with light correction loop** ⚠️ unverified | uncertain | L | D6 flags accuracy unproven; no measured precision/recall exists [C21] | Multi-day real capture: precision/recall, false-positive rate (matters most), corrections-to-steady-state, abstain threshold | D6 |
| R28 | FoundationModels exists as public Swift framework (macOS 26) | feasible | H | Apple newsroom + dev docs, WWDC25 [C22] | `import FoundationModels` compiles/links on Xcode 26 → runnable stdio binary | D9, D16 |
| R29 | Hard req macOS 26 + Apple Silicon + AI-on (large fallback population) | FWC | H | SystemLanguageModel macOS 26.0+; `.deviceNotEligible` permanent [C23] | On ineligible/AI-off config, sidecar returns clean "unavailable" → Rust picks TemplateRecap; confirm `@available` annotation | D9, D11 |
| R30 | Availability via `SystemLanguageModel.default.availability` (3 reasons) | feasible | H | Documented .available/.unavailable(reason) [C24] | Exact enum spelling/nesting vs SDK; sidecar serializes a stable JSON tag | D9 |
| R31 | Session API (`LanguageModelSession(instructions:)`, `respond(to:)`, `prewarm()`) | feasible | H | Documented APIs [C25] | Exact async/throws signature; fresh per-recap session pattern; whether prewarm helps | D9, D11 |
| R32 | Structured output (`@Generable`/`@Guide`, `respond(to:generating:)`) reliable | FWC | H | Constrained decoding guarantees *structural* validity, not semantic; numbers must be passed as input, not generated [C26] | RecapFacts→prose round-trips a valid struct across ~20 days; numbers untouched (hard rule 6); prose ≥ template baseline | D9, D11 |
| R33 | On-device model ~3B, free, OS-managed, offline | feasible | H | Apple ML research [C27] | No per-app download; works with networking disabled (hard rule 1) | D9, D12 |
| R34 | ~4096-token context (input+output) with catchable overflow | FWC | H | `GenerationError.exceededContextWindowSize` documented; 4096 shared [C28] | Real token usage well under 4096; sidecar catches overflow → template | D9, D11 |
| R35 | Latency acceptable for lazy on-open recap | feasible | M | iPhone figures (TTFT ~0.6ms/tok, ~30 tok/s); Mac unmeasured [C29] | Wall-clock Rust→stdio→Swift→struct on target M-series; decide prewarm | D11 |
| R36 | Standalone Swift CLI can reach FM (no GUI bundle) | feasible | H | Shipping CLIs: afm-cli, apfel, Apple `fm` [C30] | Minimal CLI linking FoundationModels, spawned headlessly as a Tauri child, returns a response | D16 |
| R37 | **Embedded sidecar code-signs + notarizes** ⚠️ unverified | uncertain | M | OPEN Tauri bug #11992: externalBin → "signature invalid" on notarization [C31] | End-to-end: bundle sidecar, codesign hardened+entitlements, submit to notary, staple; determine if manual pre-sign via beforeBundleCommand is required | D16, D20 |
| R38 | Tauri sidecar mechanics (externalBin, triple suffix, ShellExt, stdio) | feasible | H | Official v2 sidecar docs [C32] | Persistent (not one-shot) sidecar handles multi round-trips of line-delimited JSON; partial-line buffering handled | D16 |
| R39 | On-device embeddings via NaturalLanguage NLEmbedding (offline) | feasible | H | Documented; sentenceEmbedding 512-d, no download, 7 langs [C33] | sentenceEmbedding returns non-nil 512-vec on real macOS, no download, sub-ms | D10 |
| R40 | Embeddings can live in Rust via `objc2-natural-language` 0.3.2 (not Swift) | FWC | M | Crate exposes NLEmbedding/NLContextualEmbedding; objc2 ≥0.6.2 <0.8 [C34] | Call from Rust, get Vec<f32>; verify Send/thread-safety off main thread; typed errors | D10, D16 |
| R41 | NLContextualEmbedding (better/multilingual) needs async asset download | FWC | H | requestAssets/load; confirmed *failing in iOS simulator* (FB22699606) [C35] | On real macOS, requestAssets succeeds; first-load latency; cached after; per-token mean-pool to stable vec | D10 |
| R42 | k-NN/centroid over exemplars cannot hallucinate a category | feasible | H | Closed-vocabulary classification — structurally incapable of inventing a label [C33] | Matcher only returns existing category_id or 'unclassified'; tune cosine threshold; defer to rules below it | D10, D7 |
| R43 | Embedding *quality/accuracy* on short app+title strings ⚠️ unverified | uncertain | L | "cannot hallucinate" ≠ "is accurate"; code/non-English titles unproven [C36] | Measure real categorization accuracy on the user's own activity_logs; tune threshold; defer-to-rules fallback | D10 |
| R44 | BLOB float32 vectors + brute-force cosine in Rust fast enough (no sqlite-vec) | feasible | H | Brute-force fine for thousands of vecs at personal scale [C37] | 2k–5k 512-d BLOBs, top-k cosine <10ms; lossless f32 round-trip; pin endianness/dim/version col | D10, D18 |
| R45 | FM has NO embedding API (so embeddings = NaturalLanguage) ⚠️ unverified | uncertain | M | FM docs cover generation/guided/tools; no embedding API surfaced [C38] | Confirm against Xcode 26 SDK; lock embeddings=NaturalLanguage, FM=narration only | D9, D10 |
| R46 | tauri-specta v2 works with Tauri v2 | feasible | H | crates.io latest 2.0.0-rc.25; targets Tauri v2 [C39] | `cargo build` resolves against existing tauri 2.x; generated TS compiles under strict tsc | D17 |
| R47 | **Exact-pin the RC trio; whole stack is pre-1.0** ⚠️ churn risk | FWC | H | specta v2 still RC, no stable since 2022; pin with `=` [C40] | Pin trio in Cargo.toml; record resolved versions in Cargo.lock; document the rc in decisions.md | D17 |
| R48 | serde_json::Value return + String errors + tuple returns must be rewritten | FWC | H | Value is NOT specta::Type → `get_watcher_status` fails to generate; tuples awkward [C41] | Typed AppError (thiserror) deriving Serialize+specta::Type; WatcherStatus struct; clean TS | D17, hard rule 2/3 |
| R49 | i64 timestamps cross as number via BigIntExportBehavior::Number | FWC | H | Default is bigint; Number safe below 2^53 (epoch-seconds are) [C42] | Configure Number; ActivityLog timestamps appear as `number`; round-trip a 2026 epoch | D17, D18 |
| R50 | Typed events (Rust→TS) ⚠️ known bug | uncertain | M | tauri-specta #211: rc.24 event payload infers to `never` [C43] | Event payload type is the struct not `never`; OR ship commands-only first (dial/recap are pull-based) | D17 |
| R51 | Binding-freshness gate (regenerate → git diff --exit-code) deterministic on CI | FWC | M | Standard idiom; needs deterministic formatter (biome=Rust, no node skew) [C44] | Export #[test] writes bindings; no-op run → ZERO diff on Linux leg; hand-edit makes CI fail | D17, hard rule 8 |
| R52 | Versioned migration system extends to v5+ (the right mechanism) | feasible | H | db.rs runner already idempotent, version-recorded, tested; D24 mandates it [C45] | New v5+ migrations compile; idempotency/version tests still pass | D18, D24 |
| R53 | SQLite 3.45.1 (bundled) supports all needed features | feasible | H | WAL/partial idx/generated cols/STRICT/UPSERT all predate 3.45 [C46] | Version sufficient; no spike | D18 |
| R54 | ALTER TABLE ADD COLUMN (url/site/project_id/context_id/is_private) safe + cheap | FWC | H | O(1) metadata-only if nullable/constant-default; mirrors v2 category_id [C47] | Run v5+ on a COPY of real populated DB; rows get NULL defaults; get_activity_logs still deserializes | D18 |
| R55 | New tables (contexts/projects/corrections/recaps/embeddings) fit typed repository | feasible | H | Identical shape to existing categories/rules/settings [C45] | Migrations + typed CRUD + round-trip tests | D18 |
| R56 | **Migration safety on an existing user DB** ⚠️ untested path | FWC | M | Runner does NOT wrap migrations in a transaction; all tests use fresh in-memory DBs [C48] | Wrap each migration+version-insert in one transaction; test migrate-populated-v4→v5 + simulated-failure-rolls-back | D18, D24 |
| R57 | WAL + dedicated writer; never block Tokio executor | FWC | H | WAL NOT yet set (only foreign_keys); writes currently run on the async runtime [C49] | Set journal_mode=WAL (persists); writer thread/spawn_blocking; concurrent read during write, no "locked" | D18 |
| R58 | is_private/exclusion enforced at WRITE time, not query time | FWC | H | D8 requires title omitted at INSERT; incognito never recorded [C50] | Write path omits title for excluded/private apps; incognito skipped before URL fetch | D8, D18 |
| R59 | Tray/menubar + main window (core Tauri TrayIconBuilder) | feasible | H | Core v2, official system-tray docs [C51] | Enable 'tray-icon' feature; Open/Quit menu; click opens/focuses main window | D13 |
| R60 | Background run + dock-hide (ActivationPolicy::Accessory) | feasible | H | Documented menubar pattern; set once in setup [C52] | No dock icon; app survives window close; watcher keeps running; reopen from tray | D13 |
| R61 | Launch-at-login (tauri-plugin-autostart 2.5.1) | feasible | H | Official plugin, current [C53] | Toggle enable/disable; relaunch after reboot ideally into background | D13 |
| R62 | Single-instance (tauri-plugin-single-instance 2.3.6) | feasible | H | Official plugin; register first [C54] | Second launch focuses existing; no double-run with autostart | D13 |
| R63 | **Evening recap ping must be scheduled IN-PROCESS** | FWC | H | Official notification plugin has NO desktop scheduling (iOS/Android only); fork is separate crate [C55] | tokio timer fires at user-set HH:MM; handle app-launched-after-time, TZ/DST, day rollover; document "app must be running" | D11 |
| R64 | Only network = user-initiated update check (tauri-plugin-updater 2.9.0) | FWC | H | No auto-check unless you call check(); pulls reqwest+TLS; needs signing keypair [C56] | NO network at startup/idle (verify with monitor); check() only on button; tampered artifact rejected | D12, D20, hard rule 1 |
| R65 | Notarized direct-distribution DMG via `tauri build` | feasible | H | Official Tauri signing/notarization flow [C57] | DMG passes spctl + stapler validate; Gatekeeper opens on clean machine | D20 |
| R66 | Hardened runtime + apple-events entitlement + Info.plist usage strings ⚠️ exact wiring | FWC | M | bundle.macOS.entitlements + Info.plist merge exist; exact keys unverified [C58] | codesign -d --entitlements shows hardened+apple-events+usage strings; notarization passes WITH them | D4, D20 |
| R67 | AX + Automation work under notarized non-sandboxed build | feasible | H | Sandbox-off → AX prompt appears, AXIsProcessTrusted works [C59] | Granting Accessibility flips trust true; AppleScript returns URL after per-app prompt | D4, D20 |
| R68 | MAS is impossible (sandbox forbids AX + broad Automation) | blocked (confirmed) | H | Sandbox-on → AXIsProcessTrusted always false, no prompt [C59] | No spike — treat as decided; do not spend budget | D20 |
| R69 | Apple Developer Program ($99/yr) is a hard external dependency | FWC | H | Developer ID cert + notarytool require enrollment [C57] | Enroll; provision Developer ID + App Store Connect API key before release work | D20 |
| R70 | CI signing/notarization on GitHub macOS runners ⚠️ untested | uncertain | M | Env-var driven (CI-friendly) but notarization latency variable (>1h reported) [C60] | macOS job imports cert to temp keychain, builds, notarizes, staples within timeout; add --skip-stapling fallback | D20, D25 |
| R71 | Homebrew cask pointing at notarized DMG | FWC | M | Cask cookbook; own-tap immediate, central repo stricter [C61] | Author cask; brew install from tap; audit/style pass; livecheck resolves; decide auto_updates | D20 |
| R72 | 24h SVG dial with no chart library | feasible | H | Complete working impl in prototype/index.html (polar/arcPath/draw-in) [C62] | Port to React 19 SVG from real tauri-specta data; edge cases (12/24 boundary, sub-min, full-day, 4am offset) | D3, D14 |
| R73 | Full day + 7 mini-dials performant in React/SVG | feasible | H | Coalesced day ~10–300 arcs; SVG fine <5000 nodes [C63] | Synthetic 500-arc worst case + minis at 60fps; keyed reconciliation no jank | D14 |
| R74 | Per-arc click/hover hit-testing without a chart lib | feasible | H | Each arc is a real DOM <path> [C62] | Reliable click/hover on thin+thick arcs; invisible wide hit-area; keyboard focus+Enter | D14 |
| R75 | Light+dark via CSS-variable tokens | FWC | H | Standard; D15 says dark must be designed not auto-inverted [C64] | Dark token set; runtime toggle via data-theme; re-run 3:1 contrast in dark | D15 |
| R76 | Anton + Jost bundled locally, zero font network (OFL 1.1) | feasible | H | Both OFL 1.1; bundling/self-host permitted with notice [C65] | Bundle woff2 via local @font-face; ZERO font requests in packaged build; ship OFL.txt; CI greps for CDN URLs | D7 (no-network), D19 |
| R77 | **Comms yellow #F2BC0C fails WCAG non-text contrast** | blocked | H | 1.47:1 vs paper, 1.17:1 vs track; needs ≥3:1; red marginal vs track (3.16) [C66] | Darken yellow OR arc outlines OR guaranteed non-color cue; re-run contrast both themes before locking palette | D14, D19 |
| R78 | Orchestrated load animation without a lib | feasible | H | Prototype uses CSS stroke-dashoffset + rAF [C62] | Reproduce in React; respect prefers-reduced-motion; no jarring re-trigger on date nav | D14, D19 |
| R79 | rusqlite in-memory + temp-file + migration-chain tests on 3-OS CI | feasible | H | Already implemented in db.rs (in-memory); bundled feature green on Windows [C67] | Add temp-file test (WAL needs a real file); fresh-vs-upgraded parity | D18, D25 |
| R80 | capture/ai behind mockable traits; cfg(target_os) gates objc2 | feasible | H | Canonical Rust; arch mandates it; current crates already cross-platform [C68] | Capture trait + Fake + cfg-gated objc2 impl; Linux/Windows green; domain tests run on Fake | hard rule 5, D16 |
| R81 | FM/AX untestable in hosted CI → compile-only + manual device | FWC | H | Hosted runners can't enable AI / run model / grant TCC [C69] | ai trait Fake + TemplateRecap fully tested headless; documented manual on-device gate (D25) | D9, D25, hard rule 5 |
| R82 | Merge gates (clippy -D, fmt --check, tsc) addable; **hard rule 3 currently VIOLATED** | blocked (today) | H | lib.rs .expect() ×5, db.rs/watcher.rs .expect("Time went backwards") [C70] | Convert setup to Result; replace SystemTime expects; add #![deny(clippy::unwrap/expect/panic)]; prove existing code passes clippy -D | hard rule 3/8 |
| R83 | "Nothing leaves the machine" enforceable in CI ⚠️ no turnkey gate | FWC | L | No single gate; proxies: cargo-deny ban net crates, CSP, socket-free test [C71] | Pick an enforceable proxy that actually fails when a network call is introduced | hard rule 1, D12 |

---

## 3. Per-area feasibility

### A. AX window titles (D4) — **make-or-break, FWC, M confidence**
The mechanism is right: titles come from the AX API gated by Accessibility, not from CGWindowList/`kCGWindowName` (Screen-Recording-gated) — which is exactly why titles are empty today. `objc2-application-services` 0.3.2 exposes the chain; attribute constants are not re-exported and must be built as CFStrings ("AXFocusedWindow"/"AXTitle"). **The one assumption that can sink the project: whether Chromium/Electron apps and editors (Chrome, Cursor, VS Code) return a real focused-window AXTitle.** These apps lazily build their a11y tree; the top-level window title *usually* survives, but no authoritative source confirms it. **Spike #1 (below) must measure this per-app before any product code.**

### B. Events/run-loop capture (D5) — **FWC/uncertain, M**
NSWorkspace activation + AXObserver give second-accurate boundaries; `user-idle` (already shipped) covers idle with no extra permission. Two real unknowns: (1) the **threading/run-loop model** — both notification delivery and the AXObserver source need a *running* CFRunLoop; choose main-run-loop vs a dedicated CFRunLoop thread and prove callbacks fire; AX is documented main-thread-only. (2) **chatty `kAXTitleChangedNotification`** (browser load states, progress bars) could turn "near-zero wakeups" into a storm — needs debounce/coalescing. AXObservers are per-PID and must be rebuilt on every app switch. Spikes: prove ONE threading model end-to-end; measure idle wakeups with `powermetrics`.

### C. Browser URLs (D4, D8) — **FWC, H/M**
Chromium `URL of active tab of front window` and Safari `URL of front document` are stable. **Incognito exclusion (D8) is non-negotiable and must be checked *before* reading the URL.** Chromium exposes a `mode` window property ("normal"/"incognito") — but the sdef hints it may be creation-write-only, so **readability on an existing window is unconfirmed** and must be proven (R17). Safari has *no* private-browsing property; the only route is fragile, locale-dependent System Events menu inspection — so the safe default is: when uncertain, store no URL. Automation is per-(client,target): one prompt per browser, -1743 on denial, requires `NSAppleEventsUsageDescription`. Prefer sending Apple Events from the main Tauri process (a helper binary needs its own embedded plist + disclaim). Start with osascript shell-out (measure latency) before reaching for in-process NSAppleScript.

### D. Permissions / TCC — **feasible, H (with the dev-identity footgun)**
AX = `kTCCServiceAccessibility`; Automation = `kTCCServiceAppleEvents`; both work under a notarized, hardened, **non-sandboxed** Developer ID build. Degraded mode (app name only, via NSWorkspace, no TCC) is achievable for D21. **MAS is confirmed impossible** (R68) — do not spend budget. Two uncertain specifics: the Automation deep-link anchor (`?Privacy_Automation`) is unverified for macOS 15/26, and macOS 26.1 reportedly hides non-bundle binaries from the Accessibility list — ensure the **.app bundle** (not the Swift sidecar) is the TCC client. The dev-build identity churn (R14) will bite every spike: stabilize the dev signature first or chase phantom permission bugs.

### E. Project inference (D6) — **uncertain, L — accuracy is a product risk**
Title parsing (VS Code/Cursor `rootName`, GitHub `owner/repo`) is the safe, high-yield path and should anchor v1 — but depends on R4 (AX returning rich titles, especially Chrome). External cwd is split: iTerm2 (`path` var) and Terminal.app (tty→pid→cwd) are FWC, editor-integrated terminals are **blocked** (title-only fallback). The cross-cutting unknown (R22): **can a non-root, unsandboxed process read another process's cwd via `proc_pidinfo`?** Sources confirm only that sandbox blocks it; the unsandboxed case is undocumented — prove FIRST, because EPERM kills the entire cwd branch. Above all, **measure precision/recall on the dev's real machine**; for a "calm mirror," false positives (wrong project) hurt trust more than misses — build an abstain threshold so low-confidence samples stay unassigned.

### F. Foundation Models (D9, D11) — **FWC, H**
An exact fit: free, on-device, structured output via guided generation, with a clean runtime availability check that maps onto "FM else template." **The fallback is the primary path** for every Intel/pre-26/AI-off Mac and unsupported language — build and test TemplateRecap first (matches D3 ship order). Honor hard rule 6 by passing pre-computed numbers as input and constraining the model to *prose only* — guided generation guarantees structure, not semantics. The ~4096-token shared budget is small; keep RecapFacts compact and catch `exceededContextWindowSize`. Exact Swift symbol names/`@available` annotation must be confirmed against the Xcode 26 SDK.

### G. Swift sidecar (D16, D20) — **FWC, with one uncertain blocker**
Standalone Swift CLIs reaching FM are proven (afm-cli, apfel). Tauri sidecar mechanics (externalBin, triple suffix, ShellExt, line-delimited stdio) are stable — note stdout arrives as byte chunks, so the Rust reader must buffer/split on newlines. The genuine risk is **R37: notarization of the embedded sidecar** — open Tauri bug #11992 reports `externalBin` breaking notarization. Plan for a manual `codesign -o runtime --entitlements` pre-sign via `beforeBundleCommand` as the likely workaround, and prove the full notarize+staple on real hardware with a real Developer ID cert before committing the distribution story. Add `tauri-plugin-shell` with a capability scoped to *exactly* the named sidecar (no general shell:allow-execute).

### H. Embeddings (D10) — **feasible-with-caveats, M**
Design correction worth locking: **NaturalLanguage need not live in the Swift sidecar** — `objc2-natural-language` 0.3.2 exposes NLEmbedding to Rust, so the categorization layer can live in `enrich/` behind a mockable trait, keeping Swift = FM-recap-only. `NLEmbedding.sentenceEmbedding` (512-d, no download, 7 langs) is the safe default; `NLContextualEmbedding` (better, ~27 langs) needs a one-time async asset download that *fails in the iOS simulator* — prove on real macOS or treat as opt-in. k-NN over BLOB-stored f32 vectors with brute-force cosine is fast enough; sqlite-vec is unnecessary for v1. Critical distinction: the matcher **cannot hallucinate** a category (true, R42) but **can mislabel** (accuracy unproven, R43) — keep these claims separate and tune the cosine threshold on real data with a defer-to-rules fallback so the dial always has colors (D7).

### I. IPC / tauri-specta (D17) — **feasible-with-caveats, the contract is on an RC**
The right tool for hard rule 2, but the whole stack is pre-1.0 RC (latest 2.0.0-rc.25) — **exact-pin the trio with `=` and move them together; record the pin in decisions.md.** Migration friction in the current code: `get_watcher_status` returns `serde_json::Value` (NOT a specta::Type — *will fail to generate*, must become a named struct); all 11 commands return `Result<_, String>` (migrate to a thiserror `AppError` deriving Serialize+specta::Type); tuple returns are awkward; i64 timestamps need `BigIntExportBehavior::Number`. Known bug #211 (event payload → `never` on rc.24) argues for **commands-only first** (the dial/recap are pull-based). The freshness gate needs a deterministic formatter (prefer biome — Rust binary, no node skew) and should run on one OS leg.

### J. Persistence / data model (D18) — **the lowest-risk area, feasible**
The repo already ships exactly the pattern needed: a tested, idempotent, versioned migration runner over rusqlite 0.31 (SQLite 3.45.1 bundled — sufficient for everything). Adding v5+ columns and tables is mechanical. Two things to prove: (1) **migration safety on a populated user DB** — wrap each migration + its version-insert in a single transaction (currently NOT wrapped; all tests use fresh in-memory DBs) and test the migrate-populated + simulated-failure-rolls-back paths; (2) **WAL + writer thread** — WAL is mandated but not yet set, and writes currently run on the Tokio executor (forbidden by architecture). is_private/exclusion (D8) is a write-path concern: omit titles at INSERT, never store-then-filter.

### K. App shell (D13) — **feasible, with one design constraint**
Tray, background/accessory mode, autostart, single-instance, and the user-initiated updater are all on current official plugins. The one constraint: **the evening recap ping has no OS-level desktop scheduling** in the official notification plugin — it must be a tokio in-process timer, and "the app must be running" is an inherent limitation to document (acceptable for D11's single sanctioned interruption). The updater must never fire on startup/timer (verify with a network monitor) to honor hard rule 1. Pin all four plugins exactly (none are in the repo yet).

### L. Distribution / notarization (D20) — **feasible-with-caveats, gated on the Apple cert**
Tauri's built-in signing/notarization is well-supported and CI-friendly (env-var driven). Hardened runtime + apple-events entitlement + Info.plist usage strings are needed (exact wiring unverified — R66). Tauri updater (minisign) over Sparkle is the right call. The hard external dependency is the **$99/yr Apple Developer Program** (lead time — provision before release work). The repo has **zero distribution config today** — all greenfield. The sidecar-notarization bug (R37) is the cross-cutting risk shared with area G.

### M. Dial UI / Bauhaus (D14, D15, D19) — **feasible, one accessibility blocker**
The signature dial is already de-risked by the working prototype; React port is low-risk and performant. Fonts (Anton, Jost) are OFL 1.1 and bundle locally with zero network. The **one blocker is contrast**: the locked Comms yellow fails WCAG non-text contrast badly (R77) and color is currently the *only* channel distinguishing contexts — fix the palette (or add arc outlines + a guaranteed non-color cue) and re-run the contrast check in both themes before freezing `design-system.md`. Also remove the unused `recharts` dependency to keep the "no chart library" rule honest, and honor prefers-reduced-motion.

### N. Testing / CI (D25) — **feasible-with-caveats, plus a standing hard-rule violation**
The 3-OS matrix + bundled rusqlite is solid. Native/AI legs are compile-only on hosted CI; real behavior is manual/device (D25). Two corrections from research: the architecture doc names the wrong AX crate (it's `objc2-application-services`, not `objc2-accessibility`), and the "28 TS tests / RTL patterns" claim is misleading — current TS tests are pure-logic node-env; **RTL is net-new**. Merge gates (clippy -D, fmt --check, standalone tsc) are addable, but **hard rule 3 is violated today** (R82): `.expect()` ×5 in lib.rs setup and `.expect("Time went backwards")` in db.rs/watcher.rs must be converted to Result before the clippy gate can pass.

---

## 4. What to spike in code, in order (make-or-break first)

> Spikes 1–4 are the native gate (D22) and must run on a real Apple-Silicon Mac, macOS 26, with a **stably-signed** binary (R14 first, or every result is suspect). Spikes 5+ can proceed in parallel on any platform.

1. **AX titles WITHOUT Screen Recording across Electron / terminals / browsers (R2, R3, R4, R7).** THE make-or-break. Grant ONLY Accessibility (Screen Recording explicitly OFF). For Chrome, Brave, Arc, Cursor, VS Code, iTerm2, Terminal.app, Safari, Finder: read focused-window AXTitle and record real / empty / nil per app. Quantify vs the old empty-title baseline. **If editors+browsers return empty, the capture redesign and project inference must be rethought before any further investment.**
2. **Run-loop / threading model + NSWorkspace + AXObserver (R6, R8, R9, R10, R11, R13).** Prove ONE model end-to-end (main run loop vs dedicated CFRunLoop thread); confirm activation + focus/title-change callbacks fire, the AXObserver is rebuilt on PID change, results marshal to the Tokio side over a Send channel without blocking the executor, and measure idle wakeups (debounce chatty titles). Confirm aarch64 runtime.
3. **Browser URL + incognito exclusion (R15, R16, R17, R18, R19, R20, R21).** osascript for Chrome/Brave/Arc/Safari; **prove `mode of front window` is readable and returns "incognito" and is checked BEFORE the URL read** (D8); Safari private-window safe default; per-browser Automation prompt + -1743 graceful fallback; measure latency.
4. **proc_pidinfo cwd read, non-root, unsandboxed (R22, R24).** The terminal-cwd branch's life-or-death test — do this early because EPERM kills the branch. Then iTerm2 `path` / Terminal tty→pid→cwd routes.
5. **Project-inference accuracy on real data (R23, R26, R27).** Multi-day capture; precision/recall, false-positive rate, abstain threshold. Decide the column semantics only after measuring. (Depends on spikes 1 & 3.)
6. **Swift FM sidecar happy-path + availability + sidecar notarization (R28–R38).** Minimal CLI links FoundationModels, spawned as a Tauri child, RecapFacts→prose round-trip, availability→template routing; **then the externalBin notarize+staple test (R37) — the distribution blocker.** Requires the Apple Developer cert.
7. **Embeddings round-trip + accuracy (R39, R40, R41, R42, R43, R44).** NLEmbedding from Rust via objc2-natural-language; BLOB f32 + brute-force cosine bench; **measure categorization accuracy on real activity_logs**; tune threshold + defer-to-rules.
8. **tauri-specta wiring + freshness gate (R46–R51).** Pin the trio; convert one command + AppError; WatcherStatus struct; BigIntExportBehavior::Number; freshness #[test] → git diff on one OS leg; decide commands-only vs events.
9. **Persistence v5+ on a populated DB + WAL/writer (R54, R56, R57, R58).** Transaction-wrapped migrations against a COPY of a real DB; WAL persistence; concurrent reader during write; write-path title omission for private apps.
10. **App-shell plumbing + in-process recap ping + user-initiated-only updater (R59–R64).** Verify zero network at startup/idle.
11. **Dial React port + contrast fix (R72, R74, R77, R78).** Port the prototype; **resolve the Comms-yellow contrast blocker** and re-run the check both themes before locking the palette; remove recharts; prefers-reduced-motion.
12. **Hard-rule-3 cleanup + merge gates (R82) and the no-network proxy (R83).** Remove the existing `.expect()`s; add clippy/fmt/tsc gates; pick an enforceable network-ban proxy.

---

## 5. Honest unknowns desk research could NOT resolve (must be proven on a real Mac)

All tagged **⚠️ unverified — spike required**. These are the items where secondary sources are silent, contradictory, or display-artifacts — provisional until proven on hardware:

- **AX focused-window title returns a real value for Chrome/Cursor/VS Code** (R4). The single highest-impact unknown; no authoritative source confirms it for lazily-built a11y trees.
- **The run-loop/threading model that actually delivers callbacks** (R6) — main run loop vs dedicated CFRunLoop thread; AX main-thread constraint vs off-main queries.
- **Both native crates compile AND run on aarch64-apple-darwin** (R13) — docs.rs renders x86_64 only (likely display artifact, but unproven at runtime).
- **Chromium `mode of front window` is *readable* on an existing window and returns "incognito"** (R17) — the sdef hints it may be creation-write-only; D8 depends on it.
- **Safari private-window detection is reliable on current Safari** (R18) — locale/version-fragile menu inspection only.
- **A non-root, unsandboxed process can read another process's cwd via `proc_pidinfo`** (R22) — sources confirm only that sandbox blocks it; the unsandboxed case is undocumented. May return EPERM.
- **Real project-inference accuracy** (R27) and **real embedding categorization accuracy on short app+title strings** (R43) — never measured; "cannot hallucinate" ≠ "is accurate."
- **Exact FoundationModels Swift symbol names / `@available` / enum nesting** (R30, R31, R34) — reconstructed from secondhand guides, not read off the Xcode 26 SDK; wrong names = compile failure.
- **FM has no embedding API** (R45) — could not be confirmed; if false, the AI layer could unify on FM.
- **Embedded `externalBin` sidecar notarizes** (R37) — open Tauri bug #11992; likely needs a manual pre-sign workaround. The distribution blocker.
- **Exact tauri.conf.json entitlements + Info.plist merge wiring** (R66) and **Automation deep-link anchor on macOS 15/26** — unverified.
- **Whether `mode`/AX/Automation behavior shifts on macOS 26.1+** (binary clients hidden from Accessibility list — R-permissions) — needs the actual target OS.
- **tauri-specta exact method signatures and the rc.24/25 event `never` bug** (R50) — primary docs could not be fetched verbatim.
- **CI notarization completes within job timeout** (R70) — latency reportedly variable (>1h).
- **NLContextualEmbedding asset download succeeds on real macOS** (R41) — confirmed *failing* in the iOS simulator.

---

## 6. Citations

Each maps a load-bearing claim to its source URL (from the area dossiers).

- **[C1]** objc2-application-services 0.3.2 AXUIElement — https://docs.rs/objc2-application-services/0.3.2/objc2_application_services/struct.AXUIElement.html
- **[C2]** active-win-pos-rs CGWindowList/kCGWindowName Screen-Recording gating — https://github.com/dimusic/active-win-pos-rs
- **[C3]** Apple AXUIElement docs — https://developer.apple.com/documentation/applicationservices/axuielement
- **[C4]** Electron/Chromium AXManualAccessibility unsettable externally — https://github.com/electron/electron/issues/37465 ; Chromium a11y tree not on by default — https://issues.chromium.org/issues/382525581
- **[C5]** objc2-application-services all-items (AXIsProcessTrusted*, kAXTrustedCheckOptionPrompt; constants absent) — https://docs.rs/objc2-application-services/0.3.2/objc2_application_services/all.html
- **[C6]** AX functions main-thread-only (Apple DTS) — https://developer.apple.com/forums/thread/94878
- **[C7]** objc2-app-kit NSWorkspace — https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWorkspace.html ; didActivateApplication — https://developer.apple.com/documentation/appkit/nsworkspace/didactivateapplicationnotification
- **[C8]** objc2-foundation NSNotificationCenter block observer — https://docs.rs/objc2-foundation/latest/x86_64-apple-darwin/objc2_foundation/struct.NSNotificationCenter.html
- **[C9]** accessibility-sys 0.2.0 AXObserver* + constants — https://docs.rs/accessibility-sys/latest/accessibility_sys/fn.AXObserverCreate.html
- **[C10]** CGEventTap vs idle-read; user-idle — https://hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-input-monitoring-screen-capture-accessibility.html ; https://lib.rs/crates/user-idle
- **[C11]** TCC keyed to code identity / dev rebuild — https://developer.apple.com/forums/thread/735204 ; https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive
- **[C12]** Chromium scripting.sdef (active tab, URL, `mode`) — https://chromium.googlesource.com/chromium/src.git/+/lkgr/chrome/browser/ui/cocoa/applescript/scripting.sdef
- **[C13]** Multi-browser AppleScript gist — https://gist.github.com/vitorgalvao/5392178
- **[C14]** Safari private-browsing detection — https://alexwlchan.net/2021/detect-private-browsing/
- **[C15]** Automation TCC / -1743 / NSAppleEventsUsageDescription — https://scriptingosx.com/2020/09/avoiding-applescript-security-and-privacy-requests/ ; https://www.felix-schwarz.org/blog/2018/08/new-apple-event-apis-in-macos-mojave
- **[C16]** CLI Apple Events / disclaim / Info.plist — https://steipete.me/posts/2025/applescript-cli-macos-complete-guide
- **[C17]** osascript crate — https://docs.rs/osascript/
- **[C18]** proc_pidinfo PROC_PIDVNODEPATHINFO — https://github.com/mmastrac/proc_pidinfo ; sandbox blocks inspection — https://developer.apple.com/library/archive/documentation/Security/Conceptual/System_Integrity_Protection_Guide/RuntimeProtections/RuntimeProtections.html
- **[C19]** VS Code window.title default — https://github.com/microsoft/vscode/issues/170398 ; Cursor fork — https://one-tip-a-week.beehiiv.com/p/one-tip-a-week-change-vs-code-s-title-bar
- **[C20]** iTerm2 `path` variable — https://iterm2.com/documentation-variables.html ; Terminal AppleScript — https://support.apple.com/guide/terminal/automate-tasks-using-applescript-and-terminal-trml1003/mac
- **[C21]** repo (inference accuracy unproven) — https://github.com/f-gozie/usage_os
- **[C22]** Apple FM newsroom — https://www.apple.com/newsroom/2025/09/apples-foundation-models-framework-unlocks-new-intelligent-app-experiences/
- **[C23]** SystemLanguageModel docs — https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel
- **[C24]** Availability fallback — https://dev.to/arshtechpro/how-to-fall-back-gracefully-when-apple-intelligence-isnt-available-48j
- **[C25]** Session API gist — https://gist.github.com/koher/214301df47eeeb5c426cbcfd72700a8e
- **[C26]** Guided Generation (WWDC25 301) — https://developer.apple.com/videos/play/wwdc2025/301/
- **[C27]** Apple ML 3B model — https://machinelearning.apple.com/research/introducing-third-generation-of-apple-foundation-models
- **[C28]** exceededContextWindowSize / 4096 — https://developer.apple.com/documentation/foundationmodels/languagemodelsession/generationerror/exceededcontextwindowsize(_:)
- **[C29]** FM latency report — https://arxiv.org/pdf/2507.13575
- **[C30]** afm-cli standalone CLI — https://github.com/CreevekCZ/afm-cli ; apfel — https://apfel.franzai.com/
- **[C31]** Tauri externalBin notarization bug #11992 — https://github.com/tauri-apps/tauri/issues/11992
- **[C32]** Tauri v2 sidecar docs — https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/sidecar.mdx
- **[C33]** NLEmbedding — https://developer.apple.com/documentation/naturallanguage/nlembedding ; sentenceEmbedding — https://developer.apple.com/documentation/naturallanguage/nlembedding/sentenceembedding(for:)
- **[C34]** objc2-natural-language — https://docs.rs/objc2-natural-language/latest/objc2_natural_language/
- **[C35]** NLContextualEmbedding asset failure (simulator) — https://developer.apple.com/forums/thread/799951
- **[C36]** Embedding quality on app+title — https://www.callstack.com/blog/on-device-ai-introducing-apple-embeddings-in-react-native
- **[C37]** Brute-force vs sqlite-vec — https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html
- **[C38]** FM docs (no embedding API surfaced) — https://developer.apple.com/documentation/foundationmodels/
- **[C39]** tauri-specta versions — https://crates.io/crates/tauri-specta/versions
- **[C40]** specta-typescript pin — https://crates.io/crates/specta-typescript
- **[C41]** tauri-specta repo (serde_json::Value, error patterns) — https://github.com/specta-rs/tauri-specta
- **[C42]** BigIntExportBehavior — https://github.com/specta-rs/website/blob/main/content/docs/tauri-specta/v2.mdx
- **[C43]** tauri-specta event `never` bug #211 — https://github.com/specta-rs/tauri-specta/issues/211
- **[C44]** binding-freshness gate idiom — https://github.com/specta-rs/website/blob/main/content/docs/tauri-specta/v2.mdx
- **[C45]** repo migration runner / tables — src-tauri/src/db.rs + context/architecture.md
- **[C46]** SQLite feature/version history — https://www.sqlite.org/changes.html
- **[C47]** ALTER TABLE ADD COLUMN — https://www.sqlite.org/lang_altertable.html
- **[C48]** DDL transactional in BEGIN/COMMIT — https://www.sqlite.org/lang_transaction.html
- **[C49]** WAL — https://www.sqlite.org/wal.html
- **[C50]** decisions.md D8 — context/decisions.md
- **[C51]** Tauri system tray — https://v2.tauri.app/learn/system-tray/
- **[C52]** ActivationPolicy::Accessory — https://github.com/tauri-apps/tauri/issues/9244
- **[C53]** tauri-plugin-autostart — https://crates.io/crates/tauri-plugin-autostart/
- **[C54]** tauri-plugin-single-instance — https://docs.rs/crate/tauri-plugin-single-instance/latest
- **[C55]** tauri-plugin-notification (no desktop scheduling) — https://v2.tauri.app/plugin/notification/ ; fork — https://crates.io/crates/tauri-plugin-notifications
- **[C56]** tauri-plugin-updater — https://v2.tauri.app/plugin/updater/
- **[C57]** Tauri macOS signing/notarization — https://v2.tauri.app/distribute/sign/macos/
- **[C58]** Hardened runtime config — https://developer.apple.com/documentation/xcode/configuring-the-hardened-runtime
- **[C59]** Sandbox vs AX (MAS impossibility) — https://lapcatsoftware.com/articles/hardened-runtime-sandboxing.html
- **[C60]** Tauri notarization latency — https://github.com/orgs/tauri-apps/discussions/8630
- **[C61]** Homebrew Acceptable Casks — https://docs.brew.sh/Acceptable-Casks ; Cookbook — https://docs.brew.sh/Cask-Cookbook
- **[C62]** prototype/index.html (dial impl) — /Users/favour/Documents/projects/usage_os/prototype/index.html
- **[C63]** SVG performance — https://www.svggenie.com/blog/svg-vs-canvas-vs-webgl-performance-2025 ; https://css-tricks.com/high-performance-svgs/
- **[C64]** design-system.md / D15 — context/design-system.md ; context/decisions.md
- **[C65]** OFL FAQ — https://openfontlicense.org/ofl-faq/ ; Anton — https://fonts.google.com/specimen/Anton ; Jost — https://www.fontsquirrel.com/license/jost
- **[C66]** WCAG 1.4.11 non-text contrast — https://www.w3.org/WAI/WCAG21/Techniques/general/G207 ; https://webaim.org/articles/contrast/
- **[C67]** rusqlite (bundled, open_in_memory) — https://crates.io/crates/rusqlite
- **[C68]** objc2 (Apple-target gating) — https://github.com/madsmtm/objc2
- **[C69]** FoundationModels (device-only live inference) — https://developer.apple.com/documentation/FoundationModels
- **[C70]** repo hard-rule-3 violations — src-tauri/src/lib.rs, db.rs, watcher.rs
- **[C71]** cargo-deny — https://github.com/EmbarkStudios/cargo-deny

---

_End of audit. This is a desk-research synthesis with no independent verification pass; all native/version/permission/latency claims are provisional and must be proven by the spikes in §4 on a real Apple-Silicon Mac running macOS 26._
