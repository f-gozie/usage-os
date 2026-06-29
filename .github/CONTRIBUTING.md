# Contributing to UsageOS

Thanks for the interest. UsageOS is a private, on-device macOS app (Tauri v2 — Rust core +
React/TS frontend + a small Swift Foundation-Models sidecar). It's **macOS-only** by design
(it needs Accessibility + Automation, which the Mac App Store sandbox forbids), so development
happens on a Mac.

The full contract — the architecture, the hard rules, and how the plans/decisions/handoffs
system works — lives in **[`CLAUDE.md`](../CLAUDE.md)**. Read that first; this file is just the
practical on-ramp.

## Setup

```bash
# Requirements: macOS, Rust (stable), Node.js 22+, Xcode Command Line Tools.
npm install
npm run tauri dev
```

The daily recap uses Apple's on-device Foundation Models (macOS 26); when that isn't available
the app falls back to a deterministic template recap, so dev works without it.

## Layout

```
src/                     React + TypeScript frontend
  components/ views/ hooks/ lib/ styles/   UI, state, helpers
  bindings.ts            GENERATED tauri-specta IPC client — never hand-edit
src-tauri/src/           Rust core
  capture/               objc2 capture (NSWorkspace + AX) behind a trait
  enrich/                project inference, site parsing, rules
  rollup.rs              aggregation into the day/week/timeline shapes
  db/  migrations.rs     rusqlite repository + versioned schema (all SQL lives here)
  ai/                    the recap trait: Swift sidecar OR template fallback
  permissions/           Accessibility / Automation grant flow
  lib.rs                 Tauri command handlers + app setup
sidecar/                 the Swift Foundation-Models helper (built by sidecar/build.sh)
context/                 vision, decisions, architecture, standards, plans
```

## Before a pull request

Branch from `main`. All of these are merge-blocking (they're what CI runs):

```bash
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings
cd src-tauri && cargo test
npx tsc --noEmit
npx vitest run
# IPC bindings must be fresh — regenerate and assert no diff:
cd src-tauri && cargo test export_bindings && cd .. && git diff --exit-code -- src/bindings.ts
```

If you add or change a `#[tauri::command]`, the bindings regenerate from Rust — don't edit
`src/bindings.ts` by hand. Keep all SQL in the `db` layer, and no `unwrap()`/`expect()`/`panic!`
in production paths (the [hard rules](../CLAUDE.md) explain why).

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`,
`refactor:`, `test:`, `chore:`. Keep the docs (plans/decisions) moving with the code.

## Issues

Open an issue with your macOS version, steps to reproduce, and what you expected vs saw.
