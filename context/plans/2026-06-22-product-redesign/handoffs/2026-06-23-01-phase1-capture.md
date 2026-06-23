# Handoff — start here for the next session

_Updated 2026-06-23 at the end of the **Phase-1 capture session**. Read this, then `CLAUDE.md`, then the redesign plan (`context/plans/2026-06-22-product-redesign.md`), then `context/decisions.md` (now D1–D33). Supersedes the prior (native-spikes + tauri-specta) handoff._

## 1. Where we are (TL;DR)

**Phase 0 ✅ and Phase 1's entire backend capture pipeline ✅ — built, CI-green, and on-device-verified.** The app now records real activity event-driven: window titles (AX), browser URLs (incognito-safe), and terminal/browser **projects** (D30 canonicalization), into SQLite. **Everything remaining in Phase 1 is UI** (the dial + the settings/exclusions screens), which is **gated on the design system** — so the design-system track is now the critical-path blocker, not backend.

`main` is at `00b5a1e`, clean, synced, **no open PRs, no stray branches**. CI is **Ubuntu + macOS only** (Windows was dropped — see §8). **58 Rust + 28 TS tests**, all green.

## 2. What shipped this session (5 PRs, all merged to `main`)

| PR | What | Decision |
|---|---|---|
| [#9](https://github.com/f-gozie/usage-os/pull/9) | tauri-specta IPC migration **+ dropped Windows from CI** | D27 |
| [#10](https://github.com/f-gozie/usage-os/pull/10) | **Phase 1.1** data model — migrations v5–v8 + typed repository | D31 |
| [#11](https://github.com/f-gozie/usage-os/pull/11) | **Phase 1.2a** capture seam (trait + Fake + polling) | D32 |
| [#12](https://github.com/f-gozie/usage-os/pull/12) | **Phase 1.2b** event-driven macOS capture + enrichment **+ incognito fix** | D33 |

## 3. Decisions locked this session (`context/decisions.md`)

- **Windows dropped from CI** (in `CLAUDE.md` + a comment in `.github/workflows/ci.yml`). The `tauri-specta`/`specta` stack fails to link on Windows (`STATUS_ENTRYPOINT_NOT_FOUND`); the product is macOS-only, so CI = **Ubuntu (freshness gate + platform-agnostic core) + macOS (real target)**.
- **D31** — Phase-1.1 data model: append-only migrations v5–v8 (`projects`+`project_aliases`, `sites`, `exclusions`, event-enrichment columns on `activity_logs`); WAL on; `db` is a `pub mod`; table renames `activity_logs`→`events` / `categories`→`contexts` **deferred** to the UI rewrite.
- **D32** — Capture split into the **seam** (1.2a, CI-tested) and the **macOS impl** (1.2b, on-device). `CaptureSource` trait isolates all native code; `FakeCapture` (CI) + `PollingCapture` (fallback) + `MacosCapture` (real).
- **D33** — Event-driven macOS capture; **enrichment runs inline at capture** (cwd is ephemeral); capture channel is `std::mpsc` + a **dedicated consumer thread** (SQLite + `git`-shell block the async executor); terminal pid-selection is best-effort; Safari URL not read (R18). Includes the **on-device verification results** + the **incognito-title fix** + the **dev-TCC-responsibility-attribution finding** (§8).

## 4. The capture pipeline as built (the mental model)

```
MacosCapture (capture/macos/) ── FocusEvent ──▶ std::mpsc ──▶ consume() on a dedicated thread
  NSWorkspace activation + per-PID AXObserver (main run loop, D29)        │
  browser::inspect → url + privacy state (incognito-safe, D8)            ▼
  terminal::front_cwd → cwd signal                          process_focus_event():
                                                              1. D8 exclusions (Exclude drops / Private = time only)
                                                              2. enrich::parse_site(url) + enrich::infer_project (D30)
                                                              3. db::log_focus (coalescing write → activity_logs)
```
- **`capture/`** (`mod.rs` trait+`FocusEvent`+`process_focus_event`+`consume`; `fake.rs`; `polling.rs`; `macos/{mod,browser,terminal}.rs` `#[cfg(macos)]`).
- **`enrich/`** (`mod.rs` `parse_site`; `project.rs` the ported inference heuristic → `db::resolve_or_create_project`). Cross-platform, CI-tested (incl. a temp-git-repo test).
- **`db.rs`** (the repository): migrations 1–8, `log_focus(&NewEvent)` coalescing write, `insert_event`, project/site/exclusion CRUD, `match_exclusion`, `now_unix` (pub).
- **`lib.rs`** `setup`: creates the channel, `capture::default_source().start(tx)` on the **main thread**, `std::thread::spawn(capture::consume(db_conn, rx))`. `get_watcher_status` reads `capture::get_error_count`.

## 5. On-device verification — what was proven (and what wasn't)

**✅ Verified live** (`cargo tauri dev`, 2026-06-23): real AX titles incl. **Electron** (Slack, Notion — R4 retired in the real app); in-place `AXTitleChanged`; Chrome normal → url + `site` + `project_id`; **incognito → empty title + NULL url + `is_private=1`** (D8, after the fix); iTerm2 cwd → correct `project_id`; **D30 no-fragmentation** (github-url + cwd + folder/title names all → one project `f-gozie/usage-os`); migrations 1→8 on a fresh DB.

**❌ NOT yet verified on-device (residuals):**
- **Idle handling** — the heartbeat timer (C2) is **deferred**; idle is only sampled per event, so "user walks away" doesn't bound the open span yet.
- **Terminal.app** cwd (only iTerm2 tested); the `pgrep`+highest-pid fallback heuristic is unproven there.
- **Safari** (no-url by design) and the **`-1743` Automation-deny** fallback.
- **`powermetrics`** idle-wakeups (R11 quantitative check).
- **The first-run permission prompt** — can't be exercised via `tauri dev` (§8).

## 6. Current blockers / dependencies

- **Design system is now THE critical-path blocker.** All remaining Phase-1 work (dial + settings UI) needs it locked. **Fix the R77 blocker first:** Comms-yellow `#F2BC0C` fails WCAG non-text contrast (1.47:1) and color is the only context channel — fix the palette / add a non-color cue, re-check both themes. The full Claude Design push needs `/design-login`, **not available in this CLI env** — the user drives that; in-repo design against `context/design-system.md` + mockups (the `frontend-design` / `impeccable:*` skills, `mcp__visualize__*`) can proceed here.
- **Apple Developer Program ($99/yr)** — external, has enrollment lead time; gates Phase 3 (Foundation Models sidecar) AND Phase 5 (notarized distribution). **Enroll now** so it's never the blocker. Also unblocks proper TCC (bundle-keyed grants, the prompt flow, signing per C11/R14).

## 7. TODO — remaining work (priority order)

**Phase 1 (finish the dial loop):**
1. **Design system** (parallel, now critical) — full Bauhaus, both themes, all states; fix R77; reconcile `context/design-system.md`. *Gates 2 + 3.*
2. **Phase 1.4 — the fixed-24h dial** from real data, click-to-inspect (D14). Custom SVG, no chart library. The capture data is ready and real.
3. **Phase 1.3 UI tail** — backend sensitive handling is DONE (exclusions schema + `match_exclusion` + incognito private at capture). What remains is the **settings UI** to add/manage exclusions + per-app Private list.

**Deferred capture refinements (small, off critical path):**
4. **Heartbeat timer** (C2) — a main-run-loop `CFRunLoopTimer` to bound long idle spans + re-sample idle when the user is away.
5. **Terminal.app cwd** — verify/improve the pgrep heuristic on-device; iTerm2 is solid.
6. **Permission priming/onboarding** (Phase 4 / D21) — primed first-run, deep-link, run-degraded.

**Later phases:** Phase 2 (embedding categorization via `objc2-natural-language` D26; contexts/rules editor; week view + timeline) → Phase 3 (recap: `RecapFacts` → `TemplateRecap` → Swift FM sidecar) → Phase 4 (menubar shell, dark-mode parity, perf) → Phase 5 (notarized DMG + auto-update + Homebrew cask).

**Latent hardening (noted, not urgent):** the migrator isn't robust to a DB whose schema was created outside the migration chain (a legacy v1 DB with `category_id` already present crashes migration 2). No real users hit this; the user's legacy dev DB was dropped this session.

## 8. Gotchas / environment (READ — several discovered this session)

- **Dev-build TCC = responsibility attribution, NOT a bundle grant.** `tauri dev` runs the bare `target/debug/usage-os` from the terminal, so macOS attributes its Accessibility to the **launching terminal** (iTerm, already granted). Consequences: the app is trusted with **no prompt**, the grant **survives rebuilds**, `tccutil reset Accessibility com.favour.usage-os` → **"No such bundle identifier,"** and `usage-os` never appears in the Accessibility list. **The first-run prompt only fires for the notarized `.app`** (its own responsible process, launched from Finder/Dock) or if the terminal's AX is revoked. Plan onboarding (D21) + signing (C11/R14) around this.
- **Windows is gone from CI** — don't re-add it. macOS-only product; specta won't link there.
- **CI native code:** the macOS job **compiles + clippy's** `capture/macos/` (objc2), but **cannot run** it (no TCC/GUI) — native runtime is the on-device manual gate. Linux excludes it via `#[cfg]` and uses the `Fake`.
- **App DB path:** `~/Library/Application Support/com.favour.usage-os/usage.db` (identifier `com.favour.usage-os`). The user's legacy DB was dropped — next launch creates a fresh, clean DB.
- **objc2 pins** (unify cleanly with Tauri 2.9.3): `objc2` 0.6.4, `objc2-*` 0.3.2, `block2` 0.6.2, `libc`; macOS-gated under `[target.'cfg(target_os = "macos")'.dependencies]`.
- **specta trio:** `=2.0.0-rc.20` (NOT rc.24 — it doesn't build against tauri 2.9.3). Don't float them.
- **No IPC change since #9** — `bindings.ts` is byte-identical; the Linux freshness gate enforces it. Only `ActivityLog` (in #10) ever changed shape.
- From the prior handoff, still true: **zsh does NOT word-split unquoted `$vars`** (use arrays/xargs); **`gh pr edit` is broken** here (Projects-classic GraphQL) — use `gh api … -X PATCH`; retarget a stacked child to `main` *before* deleting the parent branch.
- I **cannot toggle TCC / security settings** (safety rule) — the user does any grant/revoke.

## 9. Testing / CI status

- **58 Rust tests** (db migrations + repository + project canonicalization incl. temp-git-repo + capture spine + D8 paths incl. `private_flag_blanks_title_and_url`) + **28 TS** (vitest, pure-logic). CI green on Ubuntu + macOS.
- Local gate (mirrors CI), from `src-tauri/`: `cargo test` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo fmt --all -- --check` · `cargo test export_bindings` then (repo root) `git diff --exit-code src/bindings.ts` · `npx tsc --noEmit` · `npx vitest run`.
- On-device run: `npm run tauri dev` (vite on :1420 + the Rust app). DB inspect: `sqlite3 ~/Library/Application\ Support/com.favour.usage-os/usage.db`.

## 10. Recommended next session

**Pick the lane:** the backend is done, so the highest-leverage move is the **design system** (it unblocks both the dial and the settings UI — the rest of Phase 1). Concretely: fix R77, lock the Bauhaus tokens/components in `context/design-system.md` (+ mockups via the design skills), and the user handles the Claude Design `/design-login` push. **Then Phase 1.4 — the dial** from the now-real capture data, which is the product's soul (D3). If you'd rather keep momentum on backend, the small deferred capture refinements (heartbeat, Terminal.app cwd) are available, but they're off the critical path.

**Read order:** this file → `CLAUDE.md` → `context/plans/2026-06-22-product-redesign.md` (Phase 0 ✅, Phase 1 backend ✅, UI remaining) → `context/decisions.md` (D1–D33) → `context/design-system.md` + `context/standards/capture-and-permissions.md` as needed.
