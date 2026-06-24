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
