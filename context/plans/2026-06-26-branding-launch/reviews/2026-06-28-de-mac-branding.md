# Review — de-Mac the brand positioning

**Date:** 2026-06-28 · **Scope:** branch (`claude/mac-branding-review-xzbzi5` vs `main`) · **Files:** 11 (+1 lockfile reverted)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** none (one-off owner-initiated copy pass; recorded as [D60](../../../decisions.md) + handoff `2026-06-28-02`)
**Codex:** skipped (`codex` not on PATH) — degraded to Lanes A–C + gates

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy -D warnings | N/A — no Rust source changed (only `Cargo.toml` `description` string) |
| cargo test | N/A — no Rust logic changed |
| tsc --noEmit | ✅ (only pre-existing `baseUrl` deprecation warning) |
| vitest | ✅ 32/32 |
| bindings fresh | N/A — no `#[tauri::command]` added/changed; `src/bindings.ts` untouched |
| astro build (landing) | ✅ |

## Findings
**Verification:** 1 verified · 0 dropped · 0 cross-model (Codex unavailable)

### Critical (must fix before merge)
- None.

### Warnings (should fix)
- None.

### Info
- `landing/src/pages/index.astro` — **[Lane C / microcopy]** trust strip introduced "On your **device**" while the rest of the page uses "**machine**" exclusively ("nothing leaves the machine", "one file on your machine"). **Auto-fixed** → "On your machine" for within-page consistency.
- Register variation across surfaces — "your **computer**" (vision thesis + onboarding hook) vs "your **machine**" (privacy/data lines) — is **intentional, not a bug**: "computer" is the relatable hook, "machine" the it's-all-local promise. Original was uniform ("Mac" everywhere); the split is a deliberate voice choice, left as-is.

## Hard-rules pass (Lane A)
All 8 clean — this is a copy/docs-only diff:
1. **Privacy/network** — no network code touched; no new `reqwest`/`fetch`/etc. ✅
2. **IPC generated** — `bindings.ts` untouched; no command added. ✅
3. **No unwrap/expect/panic** — no Rust logic changed. ✅
4. **SQL in repo** — n/a. ✅
5. **Native/AI isolated** — n/a. ✅
6. **Smart layer narrates** — n/a (recap/ai untouched). ✅
7. **Design tokens** — text-content changes only; no colors/fonts/spacing. ✅
8. **Gates green** — see table. ✅

## Claim-accuracy (D59 precedent — every public claim verified vs code)
- "macOS is the first platform it runs on" — **true** (v1 is macOS-only; CI builds macOS; `vision.md` "Out of scope (v1): Windows/Linux"). ✅
- "reveal it on disk" (was "in Finder") — still **accurate**: `revealDb()` reveals the DB file on disk; in-app button keeps the literal "Show in Finder". ✅
- "your machine/computer/device", "a private, on-device time tracker" — accurate, no overstatement. ✅
- No platform-expansion *promise* added anywhere (other platforms framed as "potentially later" only). ✅

## Auto-fixes applied
- `landing/src/pages/index.astro`: "On your device" → "On your machine" (strip consistency). Re-ran tsc + vitest + astro build → all green.

## Manual TODO
- [ ] None blocking. (Owner-side, outside repo: Twitter bio + GitHub repo "About"; visual assets `og.png` / `docs/banner.png` not inspected for Mac wording.)

## Definition of Done
- [x] plan.md ticked for what landed (cross-cutting de-Mac bullet under Milestones)
- [x] decisions.md ADR appended (D60)
- [x] impl-plan: none (one-off); handoff written (`handoffs/2026-06-28-02-de-mac-branding.md`)
- [x] docs move with code (pre-push tripwire would NOT fire — `context/plans/` + `decisions.md` both touched)

## Plan compliance
Alignment: **good** — fits the active branding-launch plan's launch front; an owner-initiated positioning correction, scoped to copy with no architecture/scope change. No creep (the one auto-fix was a consistency nit within the same change).
