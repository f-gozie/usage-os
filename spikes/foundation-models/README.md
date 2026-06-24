# Spike — Foundation Models recap sidecar (D9/D16, Phase 3 step 2)

_Run 2026-06-24 on the dev Mac (Apple M2 Pro, macOS 26.0, Swift 6.2.3, Xcode SDK 26.2). Standalone Swift CLI; build/run with `swift run usageos-ai` (or `-- --serve`). macOS 26 + Apple Intelligence only._

## Question

Can a headless Swift CLI reach Apple's **FoundationModels** to narrate the day's recap — availability gate, a single `@Generable` structured call (prose only, hard rule 6), usable latency/quality — before we build the real Tauri sidecar + Rust wiring? The `foundation-models.md` standard's API was researched but **never compiled** ("provisional until the spike").

## Verdict

**✅ VIABLE — build it.** Mechanism works on the first compile; the model is available on this machine and produces faithful prose. The real risks are in the *prompt/facts formatting*, not the mechanism, and each has a found mitigation.

## What was proven

| # | Question | Result |
|---|---|---|
| 1 | Headless CLI links FoundationModels, compiles vs the real SDK? | **PASS, first try.** Every provisional spelling in the standard was correct: `@Generable`/`@Guide`, `SystemLanguageModel.default.availability` (`.available` / `.unavailable(reason)`), `LanguageModelSession(instructions:)`, `respond(to:generating:options:)`, `result.content.text`, `LanguageModelSession.GenerationError`, `GenerationOptions(temperature:)`. |
| 2 | Availability on this machine? | **`available`** (M2 Pro, macOS 26, Apple Intelligence on). The gate + stable tag (C3) work. |
| 3 | Structured prose, numbers preserved (narrate-never-count)? | **PASS.** Prose-only `@Generable`; numbers came through exactly (no fabricated counts). |
| 4 | Latency? | **~5.4 s cold** (one-time model load), then **~0.8–2.3 s warm.** ⇒ `prewarm()` at app launch makes on-open recap viable (D11); else show a brief spinner. |
| 5 | stdio line protocol shape (C2/C4/C6)? | **Works** via `--serve` (one JSON-facts line in → one tagged JSON line out). |

## Quality findings (the spike's real value)

Out of the box (default temp, soft instructions) the model **mangled proper nouns and embellished**:
- `main project nudge` → "the main project was **nudged** throughout the day"
- `main project usage_os` → "predominantly used on **the operating system**"
- `all Personal` → "47 minutes of **personal training**"
- `47m` → "47 **million** personal minutes" (read the `m` abbreviation as "million")
- voice was third-person and stiff ("The day consisted of…")

**Mitigations found (all confirmed):**
1. **Firm instructions** — "use category/project names EXACTLY/verbatim, even if they look like code or contain underscores; do not translate/expand/interpret/invent; write in second person, plainly." → `usage_os` and `nudge` then come through verbatim; voice becomes "You spent…".
2. **Low temperature** (`GenerationOptions(temperature: 0.2)`) — less creative, less embellishment.
3. **Spell units out in the prompt** — pass "47 minutes", never "47m" (the Rust facts-formatter must do this). Fixed the "47 million" misread.
4. **Always-on template fallback** (D48) covers refusal / overflow / unavailable / mangled output.

Residual (minor, for the real prompt): occasional category↔project label conflation ("the leading project was Work" when Work is a category) — fixable with clearer field labels in the formatted facts.

## The recipe for the real implementation

- **Rust** formats `RecapFacts` (D48) into a compact prompt with **units spelled out** and **clearly labeled fields** (numbers as strings — C9/C10).
- **Swift sidecar**: firm verbatim/second-person instructions (C11), `temperature: 0.2`, prose-only `@Generable` (C9), `prewarm()` at startup, stateless per request (C2), stable status tags (C4).
- **Rust falls back to the D48 template** on any non-`ok` status (C5); per-call timeout (C7).
- **CI**: the Swift build runs in a separate non-blocking macOS-26 lane; the `ai` trait has a fake so cross-platform CI passes with no model (C19/C20).

## Files
- `Sources/usageos-ai/main.swift` — availability gate + one structured recap call + the `--serve` stdio loop.
- `Package.swift` — executable, `platforms: [.macOS("26.0")]`.

## Not done (real implementation, not spike)
Tauri `externalBin` wiring + `ShellExt::sidecar` spawn/read; the Rust `ai` trait + fake + `RecapFacts`→prompt formatter; `prewarm()`; the macOS-26 CI lane; prompt voice tuning to the product's copy bar.
