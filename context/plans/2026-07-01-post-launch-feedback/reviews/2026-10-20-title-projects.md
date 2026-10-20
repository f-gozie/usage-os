# Review — projects from window titles (D77)

**Date:** 2026-10-20 · **Scope:** branch `fix/title-projects` · **Files:** 8
**Plan:** [plan.md](../plan.md) · **Impl-plan:** none (one-off fix, no plan-mode plan)
**Codex:** ran (codex-cli 0.153.4) — "patch is correct", no findings

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy -D warnings | ✅ |
| cargo test | ✅ 191 passed, 1 ignored |
| tsc --noEmit | ✅ |
| vitest | ✅ 97 passed |
| bindings fresh | ✅ (no IPC change) |

## Findings
**Verification:** 0 Critical/Warning raised by any lane · 0 dropped · 0 cross-model

### Critical
- None.

### Warnings
- None.

### Info
- `src-tauri/migrations/0010_clear_title_projects.sql` — a span with a `file://` URL outside a
  `projects/` folder whose project came from its title (old builds read the title even with a
  URL) keeps that label. Not seen on the owner's database; left as is.
- `src-tauri/src/enrich/project.rs` — a VS Code window on a non-default profile ends its title
  with the profile name, which would be taken as the folder. Recorded under D77 consequences.

## Auto-fixes applied
- `classify_title`: a `[first, .., last]` slice pattern replaces the separate length check.

## Manual TODO
- [ ] On device: the migration on the upgrading launch; a Zed and a VS Code span get the folder.

## Definition of Done
- [x] plan.md ticked for what landed
- [x] decisions.md ADR appended (D77)
- [ ] impl-plan present — none, one-off fix · handoff left to the owning session
- [x] docs move with code (pre-push tripwire would not fire)

## Plan compliance
Alignment: good — the change is limited to title inference, the one-time cleanup and its tests.
Checked against a copy of the real database: 245 projects → 44.
