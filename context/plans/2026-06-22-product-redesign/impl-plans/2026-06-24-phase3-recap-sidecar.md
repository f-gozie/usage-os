# Phase 3 step 2 — Recap sidecar (Foundation Models)

_Forward plan, 2026-06-24. Branch `phase3/recap-sidecar`. Builds on D48 (RecapFacts + template) and the `spikes/foundation-models/` spike (verdict: viable). Decision basis: D9/D16; conventions C1–C11/C19–C20 in `context/standards/foundation-models.md`. Will be annotated as-built and an ADR appended when it lands._

## What the spike settled (so we don't relitigate)
- Headless Swift CLI + FoundationModels compiles against the real macOS 26 SDK; availability gate, `@Generable` prose-only output, and the stdio line protocol all work.
- Latency ~5.4s cold / ~1–2s warm ⇒ `prewarm()` at launch + compute the recap **lazily, off the day-load path**.
- Quality needs: firm verbatim/second-person instructions (no personal examples in the OSS prompt), `temperature: 0.2`, **units spelled out by the Rust formatter**, template fallback for the rest.

## Chunks (smallest reviewable first)

### A. Rust AI seam — pure Rust, fake-tested
- `src-tauri/src/ai/mod.rs`: `AiError`; an **async `Narrator` trait** (`narrate(&self, prompt: &str) -> Result<String, AiError>`); a `FakeNarrator` (canned prose / forced error) for tests + CI (hard rule 5 / C19).
- Expose `RecapFacts` (pub + `Serialize`) from `rollup`; add `format_recap_prompt(&RecapFacts) -> String` — **units spelled out** ("47 minutes", never "47m"), clearly-labeled fields, numbers as strings (C9/C10).
- `build_recap(narrator, facts).await -> Recap`: `Ok` → `Recap { generated_by: "foundation-models" }`; `Err` → the D48 `render_template_recap` (C5).
- Tests: prompt formatting (units, labels), success path, fallback-on-error.

### B. Productionize the Swift sidecar
- `sidecar/usageos-ai/` (SwiftPM): the spike code, cleaned — generic verbatim prompt, `prewarm()` on start, `--serve` line loop, `temperature: 0.2`, stable status tags (`available`/`unavailable:<reason>`/`ok`/`error:<kind>`), no network entitlement (C8).
- Build → the Tauri `externalBin` binary named `usageos-ai-$TARGET_TRIPLE` (a small build script; documented).

### C. Tauri wiring + real Narrator
- Add `tauri-plugin-shell`; `externalBin` in `tauri.conf.json`; capability `shell:allow-spawn` with `sidecar: true` (C-shell mechanics).
- `SidecarNarrator`: `ShellExt::sidecar("usageos-ai").spawn()`, write facts line, **buffer stdout into lines** (C6), branch on the status tag (C4), **per-call timeout** (C7), fall back on anything (C5).
- `prewarm()` the model at app launch.

### D. Lazy async recap + frontend
- New async command `get_recap(start_time, end_time) -> Recap` (compute `RecapFacts` → `build_recap`). `get_day` keeps returning the **instant template** recap (no regression, no blocking).
- Frontend: `RecapCard` renders the template immediately, then calls `get_recap` and **upgrades** to the AI prose when it resolves; badge "⌁ Summarized on-device" vs "≡ Template" (from `design/day.html`).

### E. CI & deferred
- A **separate, non-blocking** macOS-26 Swift build lane (C20); cross-platform CI stays green via the `FakeNarrator` (C19).
- Deferred: opt-in evening "your day is ready" ping; prompt voice tuning to the copy bar (iterate on real days).

## Constraints carried
- Hard rule 6 — numbers in Rust; the model only phrases. Hard rule 1 — no network in the sidecar (entitlement-enforced). Hard rule 5 — `ai` behind a mockable trait. Generated IPC only.
- No personal data in the prompt template (OSS) — names arrive only as runtime facts, never as baked-in examples.

---

## As-built (chunks B–D landed 2026-06-25 — full ADR D51)

What changed vs the forward plan above, and what the build surfaced:

- **B (sidecar).** Built `sidecar/usageos-ai/` as planned. **Two production fixes the spike's TTY run hid:** stdout must be **unbuffered** (a Tauri child's stdout is a pipe → Swift `print` fully buffers and the read hangs → write via `FileHandle.standardOutput`); and the request is **JSON-wrapped** `{"prompt":"…"}` because `format_recap_prompt` is multi-line and a raw newline would split the line-delimited protocol. Added `--prewarm`. Empty `entitlements.plist` (C8).
- **C (wiring).** `SidecarNarrator` spawns **one-shot per recap** (stateless C2; persistence is open-Q12-unproven), line-buffers stdout (C6), 20 s timeout (C7), branches on the status tag (C4/C5). `prewarm()` off the main thread at launch. Capability scoped to the one sidecar. **`tauri-plugin-shell` pinned `=2.2.1`** — 2.3.5 forces tauri ≥ 2.10, whose `tauri-runtime-wry 2.10.1` + `wry 0.54.2` don't compile (Send/Sync break); 2.2.1 keeps the proven tauri 2.9.3 stack.
- **D (lazy + UI).** New `pub(crate) rollup::build_recap_facts` (shares `build_day_view`'s aggregation); async `get_recap` reads+drops the DB lock **before** the await. `useRecap` fetches once per day range (not polled), card upgrades template→AI in place. **Fixed a pre-existing `RecapCard` badge bug** (`"fm"` vs the Rust `"foundation-models"`).
- **E (CI).** **`externalBin` is validated at compile time on every platform** (tauri-build), so cross-platform CI stages a **stub** sidecar before the Rust steps (app never runs in CI — `FakeNarrator`, C19); a **non-blocking macOS lane** attempts the real Swift build, skipping green when the SDK < 26 (C20). Built binaries gitignored under `src-tauri/binaries/`; `sidecar/build.sh` produces them.
- **Gates:** 111 Rust + 23 TS tests, clippy `-D warnings`/fmt/tsc/vitest, bindings fresh. Sidecar verified on-device (prose returned, `usage_os` verbatim, prewarm + malformed paths). **Deferred:** prompt voice tuning to the copy bar; evening "your day is ready" ping; nested-binary notarization signing (Phase 5, open-Q10).

### Live-verification fixes (2026-06-25)
Running the real app surfaced one bug the unit tests + standalone sidecar couldn't: the
`SidecarNarrator` spawned with name `"binaries/usageos-ai"`, but Tauri's `new_sidecar` joins
the name *literally* to the exe dir (no triple re-appended) and the bundler/dev-copy places
the binary at `<exe_dir>/usageos-ai` (basename). So every spawn hit ENOENT → `Unavailable` →
silent template. Fix: the name must be the **basename `"usageos-ai"`** (externalBin stays
`binaries/usageos-ai`, the source path). Also: subtle fade-up animation on the template→AI
upgrade (reduced-motion-safe); `narrate` now returns immediately on a dead child instead of
waiting the timeout. Verified live: badge flips to "⌁ Summarized on-device", `usage_os` verbatim.

### Recap cache (D52) — added after a `/debate`
Two independent reviewers (Codex + Opus) converged: **persist successful AI recaps in SQLite,
keyed by a content fingerprint of the facts**; **today settles on open + manual ↻ (no poll, no
throttle)**; the recap is captured-derived, so it's wiped by `delete_all_data` + pruned by
retention. Built: migration `0007_recap_cache`, `db::{get,put}_cached_recap`,
`rollup::recap_fingerprint` (FNV-1a of `"v{RECAP_CACHE_VERSION}\n"` + the facts prompt — the
version covers the deferred voice tuning), `get_recap` cache check/store (only the model
result, never the template). Past days = instant cache hit (no spawn/battery); a reprocess
yields a new fingerprint → re-narrate once. 115 Rust tests (+4: cache roundtrip/wipe/prune,
fingerprint stability). Full ADR: D52.
