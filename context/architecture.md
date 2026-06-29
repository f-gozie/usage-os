# Architecture

_Last updated: 2026-06-29. Detailed code conventions live in `context/standards/`. This doc is the shape + boundaries. **A few sections below describe the Phase-1 *plan* rather than the as-built code** — the schema kept `activity_logs`/`categories` (the `events`/`contexts` renames are deferred, D31) and on-device embeddings were trialled then shelved (D47); both are flagged inline._

## Layer map

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (React + TS + Vite + Tailwind)                      │
│   Bauhaus UI · dial / timeline / week / recap / settings      │
│   talks ONLY through generated tauri-specta client            │
└───────────────▲───────────────────────────────────────────────┘
                │  typed IPC (tauri-specta — generated, never hand-edited)
┌───────────────┴───────────────────────────────────────────────┐
│  Rust core (Tauri v2)                                          │
│                                                                │
│  commands/      thin handlers — no SQL, no business logic     │
│  domain/        contexts, projects, sessions, recap building   │
│  capture/  ◄── trait ── objc2: NSWorkspace events + AX titles  │
│  enrich/        project inference · site parsing · rules       │
│  ai/       ◄── trait ── stdio ──► Swift sidecar (FoundationModels)
│  store/         rusqlite repository (WAL, dedicated writer)    │
└───────────────────────────────┬───────────────────────────────┘
                                 │  stdio (JSON)
                    ┌────────────┴───────────────┐
                    │  usageos-ai (Swift)          │
                    │  Foundation Models ONLY      │
                    └──────────────────────────────┘
```

_(The folder names above are conceptual groupings; the as-built Rust modules are `lib.rs` (commands), `capture/`, `enrich/`, `rollup.rs`, `db/` (the repository + migrations), `ai/`, and `permissions/`.)_

## Boundaries (enforced)

- **Frontend → Rust:** only via the generated client. No fetch, no direct DB, no business logic in the UI.
- **`capture` is a trait.** The objc2 implementation is one impl; tests use a fake. The rest of the app never imports objc2.
- **`ai` is a trait** with two impls: `FoundationModelsAi` (Swift sidecar) and `TemplateRecap` (deterministic, always available). Selection is runtime (availability check / user setting).
- **`store` owns all SQL.** Repository functions are typed in / typed out. Nothing above it sees a SQL string or a `Connection`.
- **Numbers are computed in Rust** (`domain`), never by the model. The AI receives a `RecapFacts` struct and returns prose.

## Native layer (macOS)

- **App-switch detection:** `NSWorkspace` `didActivateApplicationNotification` (via objc2) on the main run loop → fires on every frontmost-app change.
- **Window title:** AX API — `AXUIElementCopyAttributeValue(focusedWindow, kAXTitleAttribute)` (crate: `objc2-application-services` — **including the `AXObserver` family**, so observers stay in the same objc2 crate; `accessibility-sys` is *not* needed — see `context/standards/capture-and-permissions.md`). Requires Accessibility permission (`AXIsProcessTrusted`). AX + the observer run-loop source run on the **main run loop** (no separate AX thread, no `NSApplication` required). **Proven:** AX returns real titles for Chromium/Electron apps + editors under Accessibility alone, Screen Recording OFF (R4, Spike #1 ✅); the event-driven activation + per-PID AXObserver model works end-to-end and marshals to the async side over a `Send` channel (R6/R8–R11/R13, Spike ② ✅).
- **Browser URL:** AppleScript / Apple Events to the active browser (Automation permission, prompted once per browser). Title-derived site is the fallback when URL is unavailable.
- **Idle:** existing idle detection + heartbeat to bound long-running/idle windows.
- **Gotchas to respect:** AX must be main-thread + trusted; NSWorkspace notifications need the run loop; in dev the binary identity differs so granted permissions may not attach (sign the dev build or grant the dev binary); never block the Tokio executor on SQLite (dedicated thread / `spawn_blocking`).

## Data model (extends the existing migration chain — finalize in Phase 1)

Schema is already managed by the versioned migration system (`schema_migrations`; current tables: `categories`, `rules`, `activity_logs`, `settings`). The redesign adds **new migrations (v5+)** — it does not recreate the schema. The `events` shape below is the evolved `activity_logs` (new columns) plus new tables:

- `events` — one row per coalesced activity span: `id, start, end, app, title, url, site, project_id?, context_id?, is_idle, is_private`. _(Built in Phase 1.1 as new columns on `activity_logs` — the rename to `events` is deferred to the UI rewrite, D31; `project_abstain_reason` (NULL | `no-signal` | `ambiguous`) persists why a span is unassigned, for Phase-2 correlation.)_
- `contexts` — `id, name, color` _(still the `categories` table for now; rename deferred — D31)_. `projects` — keyed on `canonical_key` (git remote `owner/repo`) with `display_name, remote_url`; folder/title/url aliases resolve to it via `project_aliases` (D30).
- `sites` — `id, host, display_name, kind` registry (`general`/`dashboard`/`project-host`); `exclusions` — `match_type, pattern, mode (exclude|private)` (D8).
- `rules` — app/title/site → context (smart defaults, user-editable).
- `corrections` — user reclassifications; reprocessing re-sorts past days to match.
- `recaps` — `date, facts_json, text, generated_by (template|fm)`.
- ~~`embeddings`~~ — **not built.** On-device embeddings were trialled and shelved (D47, below the rules baseline); no embeddings table ships.
- `settings` — permissions state, exclusions, day-start offset, recap-ping time, theme.

Storage: SQLite. **WAL is enabled today; a dedicated writer thread is the remaining Phase-1 target (R57/D58) — the connection is `Arc<Mutex<Connection>>`.** Titles raw-local; excluded/private apps store no title (omit at write time, never store-then-filter — R58). No network columns, no sync state — there is no server.

## The smart pipeline (batch enrichment, off the hot path)

Capture (real-time, cheap) → coalesce into spans → **enrichment**: project inference, site parsing, and rules-based category assignment (deterministic, in Rust — embeddings were trialled and shelved, D47) → on open / on schedule, build `RecapFacts` in Rust → AI sidecar (or template) phrases it. The model never runs on the capture tick, and never assigns a category.

## Frontend

Lean: generated tauri-specta client, local component state (small store only if needed — no data-fetching library; commands are local and fast). Custom SVG dial/mini-dial/timeline. Strict TS. Theming via CSS variables (light + dark from the design tokens).

## Build & gates

**CI exists** (GitHub Actions on Linux + macOS — Windows was dropped; it's a macOS-only product and the specta IPC stack won't link there) running the Rust + TS suites with `cargo clippy -D warnings`, `cargo fmt --check`, the tauri-specta **binding-freshness check** (regenerate → assert no diff), and the `capture`/`ai` trait-fake + repository tests. All merge-blocking.
