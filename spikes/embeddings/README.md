# Spike — embeddings for categorization (D26 / open Qs 15·16·19 / R43)

_Run 2026-06-24 on the dev Mac (Apple Silicon, macOS 26). Standalone crate; build/run with `cargo run --release` from this dir. macOS-only._

## Question

Phase-2 planned **embedding-based categorization + corrections memory** (D10), with embeddings running **in Rust** via `objc2-natural-language` so the Swift sidecar stays Foundation-Models-only (D26). Two things were unproven and gate any build:

1. **Mechanism** (Qs 15/16/19) — is `objc2-natural-language` 0.3.2's `NLEmbedding` actually *callable* from Rust, **off the main thread**, returning a **512-d** vector with **no asset download**, fast enough for a background pass?
2. **Accuracy** (R43) — does cosine similarity over `NLEmbedding.sentenceEmbedding` vectors sort real app/window titles into the right category well enough to be useful?

This needs **no Apple Intelligence** — NaturalLanguage ships in-OS (since macOS 10.15). Apple Intelligence / Foundation Models is a separate framework, used only for the recap (Phase 3).

## Verdict

- **Mechanism: ✅ PASS, cleanly.** The binding works exactly as D26 hoped.
- **Accuracy: ❌ FAIL for categorization.** Embedding similarity over app names / titles is **below the majority-class baseline**. Embeddings are **not** a viable categorizer for this product. **Shelve the embedding path; do not build it.**
- **Better answer found:** macOS's own **`LSApplicationCategoryType`** bundle metadata is a free, deterministic, accurate-where-present signal for suggesting a category for an *uncategorized app* — the actual user goal. Recommend pivoting Phase-2 categorization to it (+ the existing rules engine, which already *is* the app-level "corrections memory" via D44's Assign→rule flow).

## What was measured

### Mechanism (PASS)
- `NLEmbedding::sentenceEmbeddingForLanguage(&NSString::from_str("en"))` returns non-nil. (`NLLanguage` is an `NSString` typed-enum; passing the BCP-47 raw value `"en"` avoids depending on the constant's binding.)
- Cold model load **~55–85 ms**; `dimension() == 512`; **no download** (sub-100ms cold proves it's in-OS).
- `vectorForString:` → `NSArray<NSNumber>`, read out as `Vec<f32>`. First embed ~24 ms (warm-up), **warm embed ~3.3 ms**.
- **Off the main thread: PASS** — the model is created *and* used entirely inside a `std::thread::spawn` closure (nothing `!Send` crosses the boundary). No run-loop / main-thread requirement, unlike AX. This is the architecturally important result: the enrichment worker thread can own and use the model.

### Accuracy (FAIL)
Three probes, all in `src/main.rs`:

| Probe | What | Result |
|---|---|---|
| `title_probe` | hand-authored exemplars per category, classify 15 realistic window titles | **9/15 = 60%** |
| `app_name_loo_probe` | the user's **real** app→category map (44 apps, read from the live DB); leave-one-out, k=1 nearest neighbour | **17/44 = 39%** |
| `app_name_centroid_loo` | same map, leave-one-out vs nearest **category centroid** | **18/44 = 41%** |
| _baseline_ | always guess the largest category ("Work", 19/44) | **43%** |

Both ground-truthed app-name schemes **lose to the dumbest baseline**. The misses show why: the model keys on the **brand word's surface meaning**, which is unrelated to app function — `Cursor→Reddit`, `Steam→Xcode`, `Safari→YouTube`, `Figma→Netflix`, `Slack→Chrome`. Brand names are out-of-vocabulary noise. Title-based did better only where the title had rich English content, and still mis-fired on app-identity cases (`Steam — Football Manager → Work`).

**Why a better model won't fix it:** the failure is in the *input*, not the model. "Cursor is a code editor" is world knowledge, not language structure — `NLContextualEmbedding` (the multilingual, downloaded upgrade) was **not** tested because it cannot recover information the brand token doesn't contain. (Untested; noted for completeness.)

### The signal that works: `LSApplicationCategoryType`
Read from each app bundle's `Info.plist` (the `apps` module already reads bundles for icons — D43). Sampled across the user's apps:

- `developer-tools` → Cursor, VS Code, Xcode, TablePlus, DataGrip · `graphics-design` → Figma · `productivity` → Notion, Mail, Safari, Preview
- `business` → Slack · `social-networking` → Discord, Telegram, WhatsApp, FaceTime, Messages
- `music` → Spotify, Music · `video` → VLC
- **Not set:** Zed, Google Chrome, Brave, Steam (browsers commonly omit it).

It's not perfect (`productivity` spans Work/Browsing/Messaging for this user; browsers often unset), so it's a **suggested default**, not a hard classifier — but it's a far stronger prior than embeddings, free, deterministic, no model, no download, no Apple Intelligence.

## Recommendation

1. **Do not build embedding categorization.** Record the negative result (refines/supersedes the embedding half of D10/D26).
2. **App-level categorization stays rules-based** — already shipped, deterministic, retroactive. D44's "Uncategorized → Assign" flow *is* the app-level corrections memory (a correction becomes a rule, reprocessed over history).
3. **Add `LSApplicationCategoryType` → category mapping** to the `apps` module so the Uncategorized list can **pre-suggest** the right category (developer-tools→Work, social-networking→Messaging, music/video→Personal/Entertainment, games→Entertainment, browser/unset→Browsing). Deterministic, cheap, no new native surface beyond a plist read.
4. **Keep this crate** as the record that the embedding path was tried and measured, not assumed.

## Files
- `src/main.rs` — the three probes + the mechanism checks.
- `Cargo.toml` — `objc2 0.6`, `objc2-foundation 0.3.2`, `objc2-natural-language 0.3.2`.
