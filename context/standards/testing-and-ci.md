# Testing & CI Standards

_Scope: how UsageOS tests itself and what gates every merge. Generated in Phase 0 from a research audit of the existing repo (`testing-ci.json`). This research had **no independent verification pass**, so pinned versions and API specifics below are **provisional** — anything not confirmed by a cited authoritative source is parked under "⚠️ Open questions / verify in the Phase-0 spike". State a confirmed claim, cite it, and move on; do not assert version/API details you cannot cite._

This file serves CLAUDE.md hard rule #8:

> **Every merge is gated:** `cargo clippy -D warnings`, `cargo fmt --check`, `cargo test`, `tsc`, and a binding-freshness check must pass. Red = not merged.

And the architecture doc's "Build & gates" section:

> **CI already exists** ... **Extend it, don't replace it**, with: `cargo clippy -D warnings`, `cargo fmt --check`, the tauri-specta **binding-freshness check** (regenerate → assert no diff), and tests for the new `capture`/`ai` trait fakes + new repository functions. All merge-blocking.

---

## 0. The shape of the test pyramid

UsageOS is a Tauri v2 app: Rust core + React/TS frontend + a macOS-only Swift sidecar + macOS-only native capture. Those four surfaces have **four different testability ceilings**, and the design exists to keep as much logic as possible in the cheap, portable tiers.

| Tier | Where | Runs in CI? | Tooling |
|------|-------|-------------|---------|
| Rust unit + integration | `domain/`, `enrich/`, `store/`, trait fakes | Yes, all 3 OSes | `cargo test` |
| TS logic | `src/lib/*` (pure functions) | Yes, Node env | Vitest (`environment: 'node'`) |
| TS component (RTL) | Bauhaus dial / recap / settings | Yes, jsdom/happy-dom env | Vitest + React Testing Library — **net-new, see §6** |
| macOS native (AX/NSWorkspace) | `capture` objc2 impl | **Compile-only** on macOS runners | `cargo build` (cfg-gated) |
| Swift sidecar (Foundation Models) | `usageos-ai` | **Compile/interface-only** | `swift build`; live model = device/manual |

**The load-bearing principle:** because `capture` and `ai` are traits (hard rule #5), the entire app above those seams is testable with **fakes**, with zero macOS permissions and no live model. CI never depends on a granted Accessibility permission or an Apple Intelligence runtime — those do not exist on hosted runners.

> Hard rule #5: _"Capture lives behind a `capture` trait; the Swift AI sidecar behind an `ai` trait. Both must be mockable so the rest of the app is testable without macOS permissions or a model."_

---

## 1. Current baseline (what actually exists today)

Grounding fact, so we extend reality rather than the README's aspiration:

- **Rust tests** live in `src-tauri/src/db.rs` and exercise the migration chain in-memory (`Connection::open_in_memory()` → `run_migrations()`), asserting version `== 4` and recorded migration names. _(citation: crates.io/rusqlite)_
- **TS tests** are **two pure-logic Node-env files** — `src/lib/stats.test.ts` and `src/lib/time.test.ts`. `vitest.config.ts` sets `environment: 'node'`, `include: src/**/*.test.ts`.
- There is **no** `@testing-library/react`, `jsdom`, or `happy-dom` in `package.json`. **RTL is net-new** — despite CLAUDE.md saying "28 TS tests" and "RTL patterns", there are no RTL patterns to match yet.
- `ci.yml` runs `cargo test`, `cargo build`, and `npx vitest run`. It does **not** run `clippy`, `fmt --check`, or a standalone `tsc --noEmit`. `tsc` runs only transitively inside the `build` script (`tsc && vite build`), which CI never invokes.
- A Tauri invoke mock already exists at `src/__mocks__/tauri.ts` (aliased in `vitest.config.ts`) — this is the seam component tests use to fake commands.

**Consequence:** four of the five hard-rule-#8 gates are presently aspirational. Phase 0 wires them in. Expect the first clippy run to surface lints, and expect `expect(...)`/`println!` cleanup (see §4 and §7).

---

## 2. Rust: repository & migration tests

### 2.1 In-memory tests (fast, isolated) — the default

One-line rationale: in-memory DBs are instant and need no filesystem, so every logic test that touches the schema uses them.

```rust
// in src-tauri/src/store/ (or db.rs today)
#[test]
fn migrations_reach_head_and_are_idempotent() -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    run_migrations(&conn)?;
    run_migrations(&conn)?; // idempotent: running twice is a no-op
    let v: i64 = conn.query_row(
        "SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0),
    )?;
    assert_eq!(v, EXPECTED_HEAD_VERSION);
    Ok(())
}
```

### 2.2 Temp-file tests (WAL, persistence, reopen) — required for anything WAL-shaped

One-line rationale: **`:memory:` databases ignore WAL** (`journal_mode` stays `"memory"`), so any test that asserts WAL, foreign-key persistence, or survives-reopen **must** use a real file.

```rust
#[test]
fn wal_and_data_survive_reopen() -> rusqlite::Result<()> {
    let dir = tempfile::tempdir().unwrap();          // tempfile crate
    let path = dir.path().join("usageos.sqlite");
    {
        let conn = rusqlite::Connection::open(&path)?;
        let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        assert_eq!(mode, "wal");                      // a file DB honors WAL
        run_migrations(&conn)?;
        // ... insert a known row ...
    }
    let conn = rusqlite::Connection::open(&path)?;    // reopen
    // ... assert the row + schema_migrations head survived ...
    Ok(())
}
```

### 2.3 Fresh-vs-upgraded parity

One-line rationale: a user upgrading from v2 must land on byte-identical schema to a fresh install, or migrations have drifted.

Build one DB straight to head, build another stopped at an intermediate version then run the remaining migrations, and assert both produce the same schema (compare `sqlite_master` SQL or a dumped schema). This protects the "v5+ migrations extend, never recreate" rule from the architecture doc.

### 2.4 `store` owns all SQL

> Hard rule #4: _"All SQL lives in the repository layer. No raw SQL or DB handles leak into command handlers or business logic. Repository functions are typed in, typed out."_

Tests reinforce this: repository tests call typed repo functions (`repo.insert_event(&Event { .. })`), never raw SQL from a command test. If a test outside `store/` contains a SQL string, that is the bug.

---

## 3. Rust: the `capture` and `ai` trait fakes

### 3.1 Capture

One-line rationale: domain/coalescing logic must be provable without macOS, so capture is a trait with a scripted fake.

```rust
pub trait Capture {
    fn current(&self) -> Option<WindowInfo>;
}

pub struct FakeCapture { frames: Vec<Option<WindowInfo>>, /* cursor */ }
// returns scripted frames; zero permission prompts.

#[cfg(target_os = "macos")]
pub struct Objc2Capture { /* NSWorkspace + AX */ }
```

The objc2 impl and its dependencies live behind `#[cfg(target_os = "macos")]` **and** under `[target.'cfg(target_os = "macos")'.dependencies]` in `Cargo.toml`. If either gate is missing, the Linux/Windows `cargo build` legs fail to compile. The rest of the app imports the trait, never objc2.

### 3.2 AI

> Hard rule #6: _"The smart layer narrates, it never counts. Recap models receive pre-computed aggregates and may only phrase them. Numbers are computed in Rust. A deterministic template recap is always available as fallback."_

Two impls behind one trait (per the architecture doc): `FoundationModelsAi` (Swift sidecar over stdio) and `TemplateRecap` (deterministic, always available).

```rust
pub trait Ai {
    fn phrase(&self, facts: &RecapFacts) -> Result<String, AiError>;
}

// Test 1: FakeAi returns canned prose — proves the pipeline wiring.
// Test 2: TemplateRecap is DETERMINISTIC — same RecapFacts in, same string out.
#[test]
fn template_recap_is_deterministic() {
    let facts = RecapFacts { /* fixed */ };
    assert_eq!(TemplateRecap.phrase(&facts).unwrap(),
               TemplateRecap.phrase(&facts).unwrap());
}
```

Both run on all three OSes with **no Swift and no model**. Live-AI tests (real phrasing on real hardware) are `#[ignore]` or feature-gated — see §5.

---

## 4. The five merge gates (hard rule #8)

Run the Rust lint/format/type gates on **one OS leg** (Linux) to save matrix minutes — they are platform-independent. `cargo test` runs on all three.

```yaml
# extend the existing ci.yml — do not replace it
- name: clippy (deny warnings)
  run: cargo clippy --all-targets -- -D warnings

- name: rustfmt check
  run: cargo fmt --all --check

- name: cargo test
  run: cargo test --all

- name: tsc (type check, no emit)
  run: npx tsc --noEmit            # NOT currently run by CI; build script only

- name: vitest
  run: npx vitest run
```

`clippy`/`rustfmt` components come from `dtolnay/rust-toolchain@stable`. _(citation: github.com/dtolnay/rust-toolchain)_

**Expect first-run friction:** enabling `clippy -D warnings` on a previously-unlinted crate surfaces lints. `watcher.rs`/`db.rs` use `.expect("Time went backwards")` and `println!`/`eprintln!`; hard rule #3 (no `unwrap`/`expect`/`panic!` in production paths) plus the gate mean those become typed `Result` handling before the gate can go green. Do that cleanup in the same PR that enables the gate.

---

## 5. The binding-freshness gate (hard rule #2)

> Hard rule #2: _"The IPC contract is generated, never hand-written. All Rust↔TS types/clients come from `tauri-specta`. Editing generated bindings by hand is forbidden."_

The gate: regenerate `src/bindings.ts` from the Rust commands, then fail if it differs from what's committed.

```yaml
- name: bindings are fresh
  run: |
    cargo test export_bindings          # or whatever triggers the export
    git diff --exit-code src/bindings.ts
```

```rust
// the export is conventionally guarded so it only runs at build/test time
#[cfg(debug_assertions)]
#[test]
fn export_bindings() {
    tauri_specta::Builder::new()
        .commands(/* … */)
        .export(specta_typescript::Typescript::default(), "../src/bindings.ts")
        .expect("export bindings");
}
```

Rules for keeping this gate stable:
- **Run it on ONE OS leg only** (Linux). Line-ending and formatter drift across OSes produces false diffs.
- **Pin the generator with `=`** so output stays byte-stable across RC bumps (versions are provisional — see Open Questions).
- The generator must run **deterministically** (same prettier/formatter settings) or the diff is noisy.
- To prove the gate works: hand-edit `bindings.ts` and confirm CI goes red.

⚠️ tauri-specta is **not yet in the repo** — this is net-new wiring, and the published line is a pre-1.0 release candidate. Treat the exact version pins below as provisional.

---

## 6. TS frontend tests

### 6.1 Logic tests stay in Node env

`src/lib/*` pure functions keep `environment: 'node'`. Do not move them.

### 6.2 Component tests (RTL) are net-new

One-line rationale: the Bauhaus dial/recap/settings components need a DOM to render into, so component tests get a `jsdom`/`happy-dom` environment while logic tests stay on Node.

Keep the two environments separate — either per-file pragma or a second Vitest project:

```ts
// @vitest-environment jsdom        ← top of a component test file
```

```ts
// or in vitest.config.ts: a projects array, one node project (lib/*),
// one jsdom project (component tests), so logic tests stay fast.
```

Component tests render against the existing Tauri invoke mock (`src/__mocks__/tauri.ts`) so a dial reading from a faked command needs no backend. _(citation: vitest.dev/guide/environment)_

⚠️ RTL deps (`@testing-library/react` ≥16 for React 19, `@testing-library/jest-dom`, and `jsdom` or `happy-dom`) are **not installed yet**. See Open Questions for version specifics.

---

## 7. macOS-only legs: compile, don't behave

The single highest-risk testing fact in this project:

- **Foundation Models cannot run on GitHub-hosted runners** — there is no Apple Intelligence model runtime there. Live inference needs an Apple-Silicon device with Apple Intelligence enabled. Hosted runners can **build the Swift binary and validate its interface only**. _(citation: developer.apple.com/documentation/FoundationModels)_
- **AX calls + NSWorkspace observers** need the **main thread** and a **trusted (signed/granted) binary**. In unattended CI they prompt or return false. So the macOS native legs are **compile-only**, never behavior-gated.

Therefore:
- Real AI behavior tests and AX behavior tests are **device/manual**, marked `#[ignore]` or feature-gated so unattended CI never blocks on them.
- The availability check (`SystemLanguageModel` available vs `.unavailable(reason)`) must route to `TemplateRecap`. This is exercised either via Xcode's "simulate Foundation Models unavailability" scheme setting or an injected availability flag — provably, on any machine.

```yaml
- name: build Swift sidecar (compile/interface validation only)
  if: runner.os == 'macOS'
  run: swift build -c release       # builds; does NOT run the live model

- name: macOS native compile check
  if: runner.os == 'macOS'
  run: cargo build                  # objc2 impl is present but cfg-gated;
                                    # AX behavior tests are #[ignore]
```

The `if: runner.os == 'macOS'` guard is already an established pattern in `ci.yml` (used for Linux `apt` deps). The cross-platform legs stay green precisely because objc2 + Swift are cfg-/step-gated out.

---

## 8. "Nothing leaves the machine" — no turnkey gate yet

> Hard rule #1: _"Nothing leaves the machine. No network calls in the data path, ever. ... The only permitted network is an explicit, user-initiated update check."_

There is **no single CI check** that proves this, and one does not exist in the repo. It is primarily an architectural/review discipline. Candidate enforceable proxies, to be decided in the spike (§9):
- `cargo-deny` `bans` denying HTTP-client crates (`reqwest`, etc.) in the core crate. _(citation: github.com/EmbarkStudios/cargo-deny)_
- A Tauri config CSP that forbids remote `connect-src`.
- A sandboxed integration test asserting no socket is opened during a capture+recap cycle.

At least one of these must actually fail when a network call is introduced, or it is theater. Until one is wired, this rule is enforced by code review (the `code-review` skill / `/code-review`) per the dev workflow.

---

## 9. ⚠️ Open questions / verify in the Phase-0 spike

Everything in this section is **provisional** — asserted by the research summary but **not independently verified**. Confirm against an authoritative source (or a real compile) before relying on it; correct this doc and the architecture doc when the spike resolves each item.

**Version pins (provisional — confirm against crates.io/docs.rs at spike time, then pin):**
- `rusqlite` — repo pins `0.31` (bundled); research says latest is `0.40.1` embedding SQLite `3.53.2`. The `0.31 → 0.40` upgrade is a separate low-risk task (`open`/`open_in_memory`/`execute`/`query_map` are stable across the range). Verify before bumping.
- `tauri-specta = "=2.0.0-rc.24"`, `specta = "=2.0.0-rc.24"`, `specta-typescript = "=0.0.11"` — **pre-1.0 release candidates**. Output formatting can shift between RCs; pin exact and re-verify byte-stability.
- `objc2-application-services = "0.3.1"` — claimed to provide `AXUIElement` + `HIServices` (default features). **`objc2-accessibility` does NOT expose `AXUIElement`** (per madsmtm/objc2#624), so the architecture doc's implied crate is wrong — fix it when the spike confirms. The exact 0.3.1 symbol signatures (`AXUIElementCopyAttributeValue`, `AXIsProcessTrusted`, `AXUIElementCreateSystemWide`) were **not confirmed**; prove them by a real macOS compile+smoke spike.
- `objc2` core / `objc2-app-kit` / `objc2-foundation` — `~0.6` line; framework crates self-gate to Apple targets but still place under `[target.'cfg(target_os = "macos")'.dependencies]`.
- RTL: `@testing-library/react` ≥16, `@testing-library/jest-dom`, `jsdom` or `happy-dom`; Vitest `^4.1.0`, React `^19.1.0`, TS `~5.8.3`, Node 22 in CI. None of the RTL deps installed yet — confirm React-19-compatible versions at install time.

**Behaviors the spike must prove (from the research `spikeMustProve` items):**
1. **Temp-file + WAL test**: file DB in WAL, spawn a writer thread + concurrent reader, assert reads aren't blocked during a write and no "database is locked" escapes. Also decide: keep the current `Arc<Mutex<Connection>>` (serializes everything — **not** the "dedicated writer thread" the arch doc describes) or move to a real writer thread + channel **before** writing this test.
2. **Capture trait gating**: `cargo build` + `cargo test` pass on Linux/Windows with the objc2 impl cfg-gated out; domain tests run against `FakeCapture` with zero permission prompts; the objc2 impl compiles on macOS runners.
3. **AX symbols exist**: a macOS-only spike that actually links `objc2-application-services` 0.3.1 + `objc2-app-kit` and calls `AXIsProcessTrusted()`, `AXUIElementCreateSystemWide()`, `AXUIElementCopyAttributeValue(_, kAXFocusedWindowAttribute/kAXTitleAttribute)`, and an `NSWorkspace` `didActivateApplicationNotification` observer — on the main thread, with minimal/wrapped unsafe. Fall back to a thin `extern` block or `core-foundation-sys` if symbols are missing.
4. **AI fallback**: Rust `Ai` trait test with `FakeAi` + a deterministic `TemplateRecap` test; confirm the availability check routes absent-model → `TemplateRecap` via simulated unavailability or an injected flag; confirm `swift build` of the sidecar succeeds on a macOS-26 runner **or document that no such hosted runner exists yet** and the Swift leg is compile-on-device-only.
5. **Matrix stays green**: full 3-OS run green with objc2 present-but-gated; AX-dependent tests `#[ignore]`/feature-gated so unattended CI never hits a permission prompt.
6. **Binding freshness**: wire tauri-specta, generate `src/bindings.ts`, prove byte-identical regeneration on a clean Linux checkout, and prove a hand-edit makes CI fail.
7. **Gates pass on existing code**: prove the current crate passes `clippy -D warnings`, `fmt --check`, `tsc --noEmit` — fixing the `expect(...)`/`println!` usages flagged by hard rule #3 first.
8. **RTL works headless**: add the RTL deps, render one Bauhaus dial/recap component against the mocked `invoke`, prove it passes headless in CI and the existing node-env logic tests still pass unchanged.
9. **Network gate**: pick one enforceable proxy (cargo-deny ban-list / CSP / socket-free integration test) and prove it fails when a network call is introduced.

---

## Citations

- rusqlite (versions, `open_in_memory`, `bundled`): https://crates.io/crates/rusqlite · https://docs.rs/crate/rusqlite/latest
- tauri-specta v2 (TS bindings export, version line, Tauri v2 requirement): https://crates.io/crates/tauri-specta/2.0.0-rc.1 · https://github.com/specta-rs/tauri-specta
- objc2-application-services (AXUIElement + HIServices, manifest): https://lib.rs/crates/objc2-application-services/features · https://docs.rs/crate/objc2-application-services/latest/source/Cargo.toml.orig
- AXUIElement absent from objc2-accessibility: https://github.com/madsmtm/objc2/issues/624
- objc2 framework bindings, Apple-target gating: https://github.com/madsmtm/objc2
- Foundation Models (on-device, OS 26 / macOS Tahoe, device-only live inference, simulated availability): https://developer.apple.com/documentation/FoundationModels
- Foundation Models CI (compile/interface validation vs device-only live model): https://github.com/rudrankriyam/Foundation-Models-Framework-Lab
- rust-toolchain (clippy/rustfmt components): https://github.com/dtolnay/rust-toolchain
- Vitest environment switching (node vs jsdom): https://vitest.dev/guide/environment.html
- cargo-deny (bans / network-crate enforcement proxy): https://github.com/EmbarkStudios/cargo-deny
