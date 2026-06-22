# Standards — AI sidecar: Foundation Models + embeddings

_Last updated: 2026-06-22. Scope: the `ai` layer — the Swift Foundation Models recap sidecar (D9, D16) and the on-device embedding path for categorization (D10). This document is grounded in a research pass that had **no independent verification**; version/API specifics below are **provisional** and flagged. Anything not confirmed by a cited authoritative source lives under [Open questions / verify in the Phase-0 spike](#-open-questions--verify-in-the-phase-0-spike), not in the asserting body. Boring, explicit, auditable._

> Read after `CLAUDE.md`, `context/decisions.md` (D9–D11, D16), and `context/architecture.md` (the `ai` trait + smart pipeline). This is the conventions layer for that boundary.

---

## What this layer is, in one paragraph

UsageOS computes the day's numbers in Rust and asks the AI layer only to **phrase** them. Two distinct on-device Apple capabilities sit behind two Rust traits:

1. **Recap narration** — Apple's **Foundation Models** framework (Swift-only, `import FoundationModels`), reached through a standalone Swift sidecar binary (`usageos-ai`) over line-delimited JSON on stdio. This is the only Swift in the project (D16).
2. **Categorization embeddings** — Apple's **NaturalLanguage** framework (`NLEmbedding` / `NLContextualEmbedding`). The research found these are reachable **directly from Rust** via the `objc2-natural-language` crate, so embeddings need **not** cross the Swift stdio boundary. See [Where embeddings live](#where-embeddings-live).

Both are fully on-device. Both must be mockable. A deterministic, always-available fallback (`TemplateRecap`; the existing rules engine for categorization) is the primary path for a large fraction of users, not a nicety.

---

## The hard rules this layer must honor (quoted)

These are from `CLAUDE.md`. They are not aspirational here — they are the design constraints that shape every convention below.

> **Hard rule 1 — Nothing leaves the machine.** "No network calls in the data path, ever. […] The only permitted network is an explicit, user-initiated update check."

Foundation Models and NaturalLanguage are on-device inference. The sidecar must be signed with an entitlements posture that grants **no** outbound network, so the guarantee is *enforced and auditable*, not merely observed. See [Sidecar security posture](#sidecar-security-posture).

> **Hard rule 3 — No `unwrap()` / `expect()` / `panic!` in production paths.** "Errors are typed and propagated (`Result`)."

Every fallible boundary here is fallible *by design*: model unavailable, model not ready, context-window overflow, guardrail refusal, spawn failure, timeout, malformed JSON, `nil` embedding. All map to typed `Result` and a fallback — never a panic.

> **Hard rule 5 — The native + AI surface stays minimal and isolated.** "Capture lives behind a `capture` trait; the Swift AI sidecar behind an `ai` trait. Both must be mockable so the rest of the app is testable without macOS permissions or a model."

The real Foundation Models path **cannot run in CI** (needs macOS 26 + Apple Silicon + Apple Intelligence on). Therefore the `ai` trait has a fake; CI tests `TemplateRecap` + the fake; the real sidecar is verified manually on-device (D25).

> **Hard rule 6 — The smart layer narrates, it never counts.** "Recap models receive pre-computed aggregates and may only phrase them. Numbers are computed in Rust. A deterministic template recap is always available as fallback."

This is the single most important constraint on the prompt/output design. Guided Generation guarantees *structural* validity, **not** semantic correctness — the model can still emit a wrong number if you let it. So: **never give the model a numeric output field.** Pass numbers in as pre-formatted strings; constrain the output to prose only. See [Narrate, never count](#narrate-never-count-the-load-bearing-pattern).

---

## Confirmed facts (cited)

Each row is a claim a cited authoritative source supports. Treat exact symbol spellings as provisional anyway (see Open questions) — the research had no compile pass.

| Fact | Value | Citation |
|---|---|---|
| Framework + platform | `import FoundationModels`; ships in macOS 26 "Tahoe" (and iOS/iPadOS/visionOS 26) for third-party apps | [apple newsroom](https://www.apple.com/newsroom/2025/09/apples-foundation-models-framework-unlocks-new-intelligent-app-experiences/) |
| Availability gate | `SystemLanguageModel.default.availability` → `.available` \| `.unavailable(reason)`; reasons `.deviceNotEligible` (permanent), `.appleIntelligenceNotEnabled` (user-fixable), `.modelNotReady` (transient) | [developer.apple.com](https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel) |
| Session API | `LanguageModelSession(instructions:)`, `respond(to:)`, `respond(to:generating:)`, `prewarm()` | [developer.apple.com](https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel) |
| Structured output | `@Generable` builds a schema the model is **constrained** to; `@Guide` constrains fields; constrained decoding guarantees structural validity | [WWDC25 session 301](https://developer.apple.com/videos/play/wwdc2025/301/) |
| Context overflow error | `LanguageModelSession.GenerationError.exceededContextWindowSize` is catchable | [developer.apple.com](https://developer.apple.com/documentation/foundationmodels/languagemodelsession/generationerror/exceededcontextwindowsize(_:)) |
| On-device model | ~3B-parameter dense model, 2-bit QAT, OS-managed (no per-app download), free, offline | [Apple ML Research](https://machinelearning.apple.com/research/introducing-third-generation-of-apple-foundation-models) |
| Standalone CLI works | Foundation Models is reachable from a plain command-line binary (afm-cli, apfel), not just a `.app` bundle | [afm-cli](https://github.com/CreevekCZ/afm-cli) |
| Tauri sidecar mechanics | `externalBin` + `-$TARGET_TRIPLE` suffix; `ShellExt::sidecar(...).spawn()` → `(rx, child)`; read `CommandEvent::Stdout`; capability `shell:allow-spawn` with `sidecar: true` | [tauri v2 sidecar docs](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/sidecar.mdx) |
| Embeddings (sentence) | `NLEmbedding.sentenceEmbedding(for:)` → single 512-d vector, no asset download, 7 languages (en/es/fr/de/it/pt/zh-Hans), since macOS 10.15 era | [developer.apple.com](https://developer.apple.com/documentation/naturallanguage/nlembedding/sentenceembedding(for:)) |
| Embeddings (contextual) | `NLContextualEmbedding` ~27 languages, Latin model 512-d, **per-token** output (mean-pool yourself), one-time async asset download (`requestAssets`/`load`), macOS 14+ | [react-native-ai](https://www.react-native-ai.dev/docs/apple/embeddings) |
| Embeddings in Rust | `objc2-natural-language` (v0.3.2) exposes `NLEmbedding` + `NLContextualEmbedding`; depends on `objc2 >=0.6.2,<0.8.0` | [docs.rs](https://docs.rs/objc2-natural-language/latest/objc2_natural_language/) |
| Brute-force is enough | Cosine over thousands of stored vectors is fast enough; sqlite-vec stays brute-force below ~4k rows anyway | [sqlite-vec](https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html) |

---

## Conventions

Each convention is a rule + a one-line rationale.

### Recap sidecar (Foundation Models)

**C1 — The sidecar is a standalone Swift CLI, not an app bundle.** _Rationale: it must be a Tauri `externalBin` child spawned headlessly; shipping CLIs prove model access works without a GUI session._

**C2 — One request, one response, stateless per recap.** Each recap is a fresh `LanguageModelSession` (or a session seeded only with fixed instructions). No multi-turn history is retained. _Rationale: the sidecar may crash and be restarted; statelessness makes restart transparent and keeps each call inside the small context window._

**C3 — Availability is checked first, every run, and serialized to a stable JSON tag.** Rust branches on the tag, never on free text. _Rationale: a large fraction of users hit `.deviceNotEligible` / `.appleIntelligenceNotEnabled`; the template fallback is the primary path and must be selected deterministically._

**C4 — The protocol carries an explicit status on every message.** `available` / `unavailable:<reason>` / `ok` / `error:<kind>` / `refused`. _Rationale: guardrail refusals, overflow, and empty responses must each route to the template fallback rather than surfacing a broken recap (hard rule 6's "always available fallback")._

**C5 — Rust falls back to `TemplateRecap` on ANY non-success.** Spawn failure, timeout, non-zero exit, malformed JSON, every `unavailable` reason, `refused`, `exceededContextWindowSize`. _Rationale: the dial/recap must always render; the AI is an optional upgrade (D9)._

**C6 — `CommandEvent::Stdout` is buffered and split on newlines; never assume one event == one message.** _Rationale: stdout arrives as arbitrary byte chunks; line-delimited JSON only works if the Rust reader reassembles lines._

**C7 — A per-recap timeout wraps the sidecar call.** On timeout, kill nothing the OS won't reclaim, return `Err`, fall back. _Rationale: a wedged model call must not hang the on-open render (D11)._

**C8 — The sidecar grants no network entitlement.** _Rationale: makes hard rule 1 enforced and auditable, not just observed._

### Narrate, never count (the load-bearing pattern)

**C9 — The model output struct contains ONLY narration fields (prose). Never a numeric field.** Numbers are pre-formatted into the *input* prompt by Rust. _Rationale: Guided Generation guarantees structure, not semantics; giving the model a number field is a footgun that lets it fabricate counts (hard rule 6)._

**C10 — `RecapFacts` is aggregates, never raw event logs, and is kept compact.** _Rationale: the context window is small and shared input+output (~4096 tokens, provisional); a verbose payload throws `exceededContextWindowSize`._

**C11 — Instructions are short and fixed.** _Rationale: every instruction token competes with the prose output for the shared budget._

### Embeddings / categorization

**C12 — Embeddings live in the Rust `enrich`/`ai` layer via `objc2-natural-language`, behind a mockable trait — NOT in the Swift sidecar.** _Rationale: keeps Swift = Foundation Models recap only (D16 intent), avoids a stdio round-trip per title, and lets categorization work on Macs that can't run Foundation Models. (This is a design correction vs. the architecture diagram — confirm and record in `decisions.md`; see Open questions.)_

**C13 — `NLEmbedding.sentenceEmbedding` is the safe default; `NLContextualEmbedding` is an opt-in upgrade.** _Rationale: sentence embeddings need no asset download and never block; the contextual model needs a one-time fetch that is confirmed to fail in the simulator and is unproven on real macOS._

**C14 — `NLContextualEmbedding` output is mean-pooled per-token into one fixed-length vector before storage/comparison.** _Rationale: it returns per-token vectors; comparing variable-length results by cosine is meaningless._

**C15 — Vectors are stored as little-endian `f32` BLOBs with an explicit dimension + model-revision column.** _Rationale: a silent endianness/length/model-revision mismatch corrupts cosine similarity with no error; the revision column prevents mixing incompatible vectors after a model update._

**C16 — Classification is closed-vocabulary k-NN/centroid with a cosine threshold; below threshold, defer to the rules engine.** _Rationale: a nearest-exemplar match structurally cannot hallucinate a category (D10), and the threshold preserves the "always have a deterministic answer" principle._

**C17 — "Cannot hallucinate" and "is accurate" are kept as separate claims.** _Rationale: the matcher can still confidently mislabel; accuracy on short code-heavy/non-English titles is the real, unproven product risk and must be measured on real `activity_logs`._

**C18 — Brute-force cosine in Rust over BLOBs for v1; no sqlite-vec / usearch.** _Rationale: hundreds-to-low-thousands of exemplars at personal scale is sub-millisecond; an index adds a native-extension build concern for no benefit until ~10k+ rows._

### CI & verification (hard rule 5)

**C19 — The `ai` trait and the embedding trait both have fakes; CI passes with no sidecar and no model.** _Rationale: GitHub-hosted runners can't enable Apple Intelligence or guarantee macOS 26 + Xcode 26._

**C20 — The Swift build runs in a separate, non-blocking macOS-26 lane (self-hosted/local), never gating cross-platform CI.** _Rationale: the model path is untestable on standard runners; don't let it block merges._

---

## Copy-pasteable patterns

> These are **shape** references, grounded in the cited research. Exact symbol names, async/throws signatures, and `@available` annotations are **provisional** — compile against the real Xcode 26 SDK in the spike before trusting them.

### Swift sidecar — availability + a single structured recap call

```swift
import Foundation
import FoundationModels

// Output is PROSE ONLY. No numeric fields — Rust pre-formats every number (hard rule 6).
@Generable
struct RecapProse {
    @Guide(description: "2-4 calm, factual sentences. Do not invent, compute, or alter any number.")
    var text: String
}

@available(macOS 26, *)   // VERIFY exact annotation against the SDK
func runRecap(promptJSON: String) async -> String {
    // C3: check availability first; serialize a stable tag for Rust.
    switch SystemLanguageModel.default.availability {
    case .available:
        break
    case .unavailable(let reason):
        // C4/C5: emit a tagged status; Rust falls back to TemplateRecap.
        emit(status: "unavailable", detail: tag(for: reason))
        return ""
    }

    let session = LanguageModelSession(
        instructions: "Narrate the day's recap from the given facts. Never compute or alter numbers."
    )

    do {
        // C9: structured output, prose field only.
        let result = try await session.respond(to: promptJSON, generating: RecapProse.self)
        emit(status: "ok", text: result.content.text)
        return result.content.text
    } catch let e as LanguageModelSession.GenerationError {
        // C5: overflow / guardrail refusal -> tagged error -> Rust template fallback.
        emit(status: "error", detail: String(describing: e))
        return ""
    } catch {
        emit(status: "error", detail: "unknown")
        return ""
    }
}

func tag(for reason: SystemLanguageModel.Availability /* VERIFY type */) -> String {
    // VERIFY exact enum nesting/spelling in the SDK.
    // Expected: .deviceNotEligible | .appleIntelligenceNotEnabled | .modelNotReady
    return String(describing: reason)
}
```

### Swift sidecar — line-delimited JSON loop on stdio

```swift
// One JSON object per line in, one per line out. Stateless per request (C2).
while let line = readLine(strippingNewline: true) {
    guard let data = line.data(using: .utf8),
          let req = try? JSONDecoder().decode(Request.self, from: data) else {
        emit(status: "error", detail: "malformed-json")   // C4
        continue
    }
    // ... dispatch req, write a single JSON line to stdout, flush ...
}
```

### Rust — spawn the sidecar, buffer stdout into lines, fall back on anything

```rust
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

/// Build the recap. On ANY non-success, returns the deterministic template (C5).
pub async fn build_recap(app: &tauri::AppHandle, facts: &RecapFacts) -> Recap {
    match try_fm_recap(app, facts).await {
        Ok(prose) => Recap::from_fm(prose),
        Err(_)    => TemplateRecap::render(facts), // hard rules 5 & 6: always available
    }
}

async fn try_fm_recap(app: &tauri::AppHandle, facts: &RecapFacts) -> Result<String, AiError> {
    let (mut rx, mut child) = app
        .shell()
        .sidecar("usageos-ai")      // matches externalBin name
        .map_err(AiError::Spawn)?
        .spawn()
        .map_err(AiError::Spawn)?;

    let req = serde_json::to_string(&AiRequest::recap(facts)).map_err(AiError::Encode)?;
    child.write(format!("{req}\n").as_bytes()).map_err(AiError::Io)?;

    // C6: stdout arrives in arbitrary byte chunks — reassemble lines.
    let mut buf = String::new();
    // C7: wrap the whole read in a timeout at the call site.
    while let Some(CommandEvent::Stdout(bytes)) = rx.recv().await {
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(nl) = buf.find('\n') {
            let line: String = buf.drain(..=nl).collect();
            let resp: AiResponse = serde_json::from_str(line.trim())
                .map_err(AiError::Decode)?;
            // C4/C5: branch on the explicit status tag, never free text.
            return match resp.status.as_str() {
                "ok"          => Ok(resp.text.unwrap_or_default()),
                "unavailable" => Err(AiError::Unavailable(resp.detail)),
                _             => Err(AiError::Model(resp.detail)),
            };
        }
    }
    Err(AiError::NoResponse)
}
```

### Rust — embeddings via `objc2-natural-language`, stored as f32 BLOB

```toml
# Cargo.toml — reuse the SAME objc2 version as the capture layer (no duplicate versions).
objc2 = ">=0.6.2,<0.8.0"
objc2-natural-language = "0.3.2"   # VERIFY method surface + Send-safety in the spike
```

```rust
/// Behind a mockable trait (C12, C19). Returns typed Result — never unwrap (hard rule 3).
fn embed(text: &str) -> Result<Vec<f32>, EmbedError> {
    // NLEmbedding.sentenceEmbedding(for: .english) -> vector(for: text)
    // vector(for:) can be nil (OOV / unsupported language) -> typed error, not panic.
    // For NLContextualEmbedding: mean-pool per-token vectors into one fixed-length vec (C14).
    todo!("objc2-natural-language call; verify thread/Send constraints off the main thread")
}

// C15: store little-endian f32 with a dim + model-revision column.
fn store_vector(v: &[f32]) -> Vec<u8> {
    bytemuck::cast_slice::<f32, u8>(v).to_vec()
}
```

### Tauri config — sidecar registration + tightly scoped capability

```jsonc
// tauri.conf.json — bundle the sidecar. File on disk: src-tauri/binaries/usageos-ai-aarch64-apple-darwin
"bundle": {
  "externalBin": ["binaries/usageos-ai"],
  "macOS": {
    "hardenedRuntime": true,          // default true; VERIFY it signs the NESTED binary (bug #11992)
    "entitlements": "./entitlements.plist"
  }
}
```

```jsonc
// capability file — scope to EXACTLY the named sidecar; no general shell:allow-execute (hard rule 1).
{ "identifier": "shell:allow-spawn", "allow": [{ "name": "binaries/usageos-ai", "sidecar": true }] }
```

---

## Sidecar security posture

- On-device only. Foundation Models + NaturalLanguage do no network I/O (hard rule 1).
- The sidecar's entitlements plist must **omit** `com.apple.security.network.client` (or equivalent), so the no-network guarantee is *enforced* and auditable in the open source, not merely observed.
- `tauri-plugin-shell` widens the attack surface (arbitrary command exec). The capability is scoped to the one named sidecar with `sidecar: true` — never a general `shell:allow-execute`.
- Each recap is stateless (C2); a crashed sidecar is restarted with no lost state.

---

## Where embeddings live

The architecture diagram (`architecture.md`) currently draws the embedding surface near the Swift/AI boundary. The research found `objc2-natural-language` exposes `NLEmbedding`/`NLContextualEmbedding` directly to Rust, so **embeddings should live in the Rust `enrich`/`ai` layer behind a mockable trait**, leaving the Swift sidecar for Foundation Models recap only. This:

- keeps Swift minimal and isolated (D16, hard rule 5),
- avoids a stdio round-trip per title during the enrichment pass,
- lets categorization run on Macs that cannot run Foundation Models (Intel, AI-off).

This is a design decision to **confirm in the spike and record in `decisions.md`** (it refines D10/D16) — see Open questions. Until confirmed, treat it as the leaning, not a lock.

---

## ⚠️ Open questions / verify in the Phase-0 spike

The research pass had **no compile/run verification**. Everything below is provisional and must be proven on a real macOS 26 + Apple Silicon + Apple Intelligence-on machine with Xcode 26 before it is relied on. Do not assert any of these as fact in code or other docs until the spike closes them.

**Foundation Models — symbols & signatures**
1. Exact enum nesting/spelling: `SystemLanguageModel.Availability`, its `UnavailableReason`, and cases `.deviceNotEligible` / `.appleIntelligenceNotEnabled` / `.modelNotReady`. Read off the Xcode 26 SDK headers — secondhand guides may be wrong; wrong names = compile failure.
2. Exact async/throws signature and response wrapper type of `respond(to:generating:)` and the plain `respond(to:)` (`.content` shape).
3. The literal `@available` annotation the sidecar needs (expected `macOS 26.0`); confirm the minimum-deployment target doesn't conflict with the app launching degraded on older/Intel Macs.
4. The full catchable path `LanguageModelSession.GenerationError.exceededContextWindowSize` and the other `GenerationError` cases (esp. guardrail/refusal).

**Foundation Models — behavior & limits**
5. Context window is exactly ~4096 tokens **shared** input+output for the installed macOS 26.x (26.4 changed context management) — pin the current number and confirm a representative `RecapFacts` + instructions + output stays well under it with headroom.
6. `SystemLanguageModel.default` resolves to the ~3B **on-device** model (not a server/larger tier); confirm no per-app download beyond the OS-managed asset and that it runs with networking fully disabled (proves hard rule 1).
7. End-to-end wall-clock latency (Rust → stdio → Swift session → parsed struct → back) on a target M-series Mac; decide whether `prewarm()` meaningfully helps the lazy on-open recap (D11). (Found latency figures are iPhone-class, not Mac.)
8. Supported-language behavior outside the bounded set (EN/FR/DE/IT/PT-BR/ES/zh-Hans/JA/KO + growing): does it error, refuse, or degrade? Confirm those users silently get the template recap.
9. Prove the prose round-trip returns a valid `@Generable` struct **every time** across ~20 varied days, and that numbers are passed through untouched (never regenerated).

**Sidecar build, sign, distribute**
10. **Highest-impact open item:** does Tauri's bundler apply a valid hardened-runtime + inherited-entitlement signature to the **nested** sidecar binary, or does notarization fail per open Tauri bug [#11992](https://github.com/tauri-apps/tauri/issues/11992)? End-to-end: `externalBin` → codesign with a real Developer ID Application cert + hardened runtime → submit to Apple notary → staple. Determine whether a manual pre-sign (`codesign -o runtime --entitlements …` via `beforeBundleCommand`) is required.
11. Can SwiftPM (`Package.swift`, `.macOS(26)` target) link `FoundationModels`, or is `xcodebuild`/`.xcodeproj` required (afm-cli used `xcodebuild`)? Affects build/CI integration.
12. Confirm `tauri-plugin-shell` sidecar supports a **persistent, bidirectional** stdio process with multiple request/response round-trips (docs show `spawn` + `write`; multi-round-trip persistence is unproven for this use).
13. Confirm `import FoundationModels` works when the binary is spawned **headlessly as a Tauri sidecar child** (CLIs prove terminal-launched binaries work; the headless-child case is the one that matters).
14. `tauri-plugin-shell` is not yet in `Cargo.toml` (only `tauri-plugin-opener`); adding it and the target-triple naming (`rustc --print host-tuple`, Rust 1.84+) must be wired up.

**Embeddings**
15. `objc2-natural-language` v0.3.2 actually exposes callable `NLEmbedding.sentenceEmbedding` / `NLContextualEmbedding.embeddingResult` / `requestAssets`, returns a `Vec<f32>`, and is **Send-safe off the main thread** (the enrichment pass runs on a worker thread). If the bindings are missing/not-callable/not-Send, the "embeddings in Rust" plan breaks.
16. `NLEmbedding.sentenceEmbedding(for: .english)` returns non-nil with a 512-element vector, **no download**, sub-ms, on the target macOS 26.
17. `NLContextualEmbedding.requestAssets()` succeeds on **real** macOS (it fails in the simulator — FB22699606), first-load latency, and that subsequent loads are cached and work offline. Decide whether to ship it at all for v1.
18. Confirm `rusqlite` 0.31 BLOB columns round-trip 512-d f32 vectors losslessly via little-endian bytes; microbenchmark N≈2000 vectors cosine-ranked in Rust < 10ms.
19. Confirm Foundation Models has **no** embedding API (so embeddings must come from NaturalLanguage). If it does, the AI layer could unify.
20. **Categorization accuracy** (the real product risk, not the API): on real `activity_logs`, do short app+title / code-heavy / non-English strings cluster usefully? Tune the cosine threshold and the defer-to-rules fallback. Decide the language-selection strategy (`NLLanguageRecognizer` per-title vs fixed English).

**Decision to record**
21. Lock and write into `decisions.md` (refining D10/D16): embeddings live in Rust via `objc2-natural-language`; the Swift sidecar is Foundation Models recap only.

---

## Citations

- Apple newsroom — Foundation Models framework: https://www.apple.com/newsroom/2025/09/apples-foundation-models-framework-unlocks-new-intelligent-app-experiences/
- `SystemLanguageModel` (availability, session APIs): https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel
- Graceful fallback / unavailable reasons: https://dev.to/arshtechpro/how-to-fall-back-gracefully-when-apple-intelligence-isnt-available-48j
- API symbols (session, `@Generable`, `@Guide`): https://gist.github.com/koher/214301df47eeeb5c426cbcfd72700a8e
- WWDC25 session 301 — Guided Generation / constrained decoding: https://developer.apple.com/videos/play/wwdc2025/301/
- Apple ML Research — third-gen on-device model (~3B): https://machinelearning.apple.com/research/introducing-third-generation-of-apple-foundation-models
- `GenerationError.exceededContextWindowSize`: https://developer.apple.com/documentation/foundationmodels/languagemodelsession/generationerror/exceededcontextwindowsize(_:)
- macOS 26.4 context-window management: https://www.infoq.com/news/2026/03/apple-foundation-models-context/
- Latency (TTFT / tok-s, iPhone-class): https://arxiv.org/pdf/2507.13575
- Supported languages: https://developer.apple.com/documentation/foundationmodels/supporting-languages-and-locales-with-foundation-models
- afm-cli (standalone CLI proof): https://github.com/CreevekCZ/afm-cli
- apfel (standalone CLI proof): https://apfel.franzai.com/
- createwithswift — exploring Foundation Models (requirements): https://www.createwithswift.com/exploring-the-foundation-models-framework/
- Tauri v2 sidecar docs: https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/sidecar.mdx
- Tauri v2 sidecar (target triple): https://v2.tauri.app/develop/sidecar/
- Tauri notarization bug #11992 (externalBin signature invalid): https://github.com/tauri-apps/tauri/issues/11992
- Tauri hardened-runtime signing (PR #10199): https://github.com/tauri-apps/tauri/pull/10199
- Tauri macOS signing/notarization keys: https://v2.tauri.app/distribute/sign/macos/
- `NLEmbedding` overview: https://developer.apple.com/documentation/naturallanguage/nlembedding
- `NLEmbedding.sentenceEmbedding(for:)`: https://developer.apple.com/documentation/naturallanguage/nlembedding/sentenceembedding(for:)
- NLEmbedding 7-language coverage: https://markbrownsword.com/2020/12/23/natural-language-framework-sentence-embedding-with-swift/
- `NLContextualEmbedding` (languages, 512-d, on-device): https://www.react-native-ai.dev/docs/apple/embeddings
- `NLContextualEmbedding.load()` (asset workflow): https://developer.apple.com/documentation/naturallanguage/nlcontextualembedding/load()
- Contextual embedding simulator failure (FB22699606): https://developer.apple.com/forums/thread/799951
- `objc2-natural-language` crate (v0.3.2): https://docs.rs/objc2-natural-language/latest/objc2_natural_language/
- Brute-force cosine / sqlite-vec scale: https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html
- Mean pooling: https://ashraf-bhuiyan.com/blog/embed-02-pooling/
- Foundation Models has no surfaced embedding API: https://developer.apple.com/documentation/foundationmodels/
- callstack — Apple embeddings (language coverage): https://www.callstack.com/blog/on-device-ai-introducing-apple-embeddings-in-react-native
- macOS 27 `fm` CLI + Python SDK (future, not v1): https://blakecrosley.com/blog/foundation-models-python-fm-cli
