# Review — smappservice-login-item (login item branded as the app)

**Date:** 2026-07-02 · **Scope:** branch (`main...feat/smappservice-login-item`) · **Files:** 9
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [2026-07-01-smappservice-login-item.md](../impl-plans/2026-07-01-smappservice-login-item.md)
**Codex:** ran (structured output; one session-restart casualty re-run)

## Merge gates

| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 127 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 36 passed |
| bindings fresh (regen + clean diff) | ✅ (doc-comments-only changes) |

Re-run after every fix below; all green.

## Findings

**Verification:** 12 verified · 1 downgraded (speculative) · **1 cross-model confirmed**

### Critical (found by the review, fixed on the branch)

- `login_item.rs` `set_enabled(false)` — **[D + B · cross-model confirmed]** `unregister` kills the
  process launchd owns, and in steady-state use every session is agent-launched — **flipping the
  toggle off would have killed the app the user was sitting in** (Apple's header: a running
  service "will be killed"; Lane B quoted it from the vendored bindings). Fixed with the
  **trampoline**: the agent job respawns the app detached (`setsid`; plist env
  `USAGEOS_AGENT_LAUNCH` → `USAGEOS_DETACHED` handoff) and exits, so launchd never owns the real
  session. **Verified on-device:** a genuinely agent-launched session survived toggle-off and the
  agent unregistered.
- `lib.rs` duplicate-instance guard — **[D]** the `NSRunningApplication` snapshot could race two
  simultaneous launches. Replaced with an **instance flock** (atomic, lifetime-held, OS-released);
  a `--hidden` launch must win it or exit; the updater relaunch retries briefly
  (`USAGEOS_SHOW_AFTER_RESTART`) because it overlaps its exiting predecessor. Also removes the
  `NSRunningApplication` dependency entirely.
- `login_item.rs` `migrate_legacy` — **[D + B]** remove-then-register could silently lose the
  user's login item if registration failed. Reordered: register first ("already registered"
  counts as success), delete the legacy plist only once the new registration holds.

### Warnings (fixed)

- `THIRD-PARTY-LICENSES.html` stale after the dependency swap — **[A]** regenerated: `auto-launch`
  et al. out, `objc2-service-management` in.
- D69 lagged the code twice (guard's updater exception; then the trampoline/flock redesign) —
  **[A]** amended to describe the as-built design, including the on-device verification results.

### Info (accepted, recorded)

- `std::env::set_var` in `restart_app` races libc `getenv` in other threads for milliseconds
  before re-exec — **[B]** pre-existing pattern, window negligible; accepted.
- Migration re-registers an item the user disabled in System Settings (the legacy plist survives
  BTM-disable) — **[B]** conscious accept; lands approval-pending, not silently on. Comment
  updated to state the real invariant.
- Codex's "re-register the agent after app updates or it may not launch" — **[D]** downgraded:
  registrations are path-based and expected to survive in-place updates; recorded in D69 as
  watched, revisit if a release proves otherwise.
- `args_os()` hardening applied at both `--hidden` checks (non-UTF-8 argv can't panic) — **[B]**.

### Simplify (applied)

- `is_enabled()` returns `bool` (the `Result` was stub-parity theater) — **[C]**.
- SAFETY comments state preconditions only; duplicated rationale hoisted to the one call site;
  Cargo.toml dep comment trimmed of its dangling clause — **[C]**.
- Drive-by: reunited the `make_builder`/`restart_app` doc comment that a pre-existing insertion
  had split — **[C]**.

## Hard-rules gate (Lane A) — all 8 PASS

Vendored `objc2-service-management` read end-to-end: pure generated bindings, zero network;
Cargo.lock is a net **negative** dependency change (6 packages out, 1 in). Bindings diff is
doc-comments only; commands registered; no unwrap/expect/panic in prod paths; no SQL; the new
native surface follows the `glance_panel.rs` isolation pattern (cfg module + stubs, Linux CI
compiles); no UI changes; capabilities untouched.

## On-device verification (Developer-ID re-signed bundle)

Toggle ON → agent registered, **Login Items shows "UsageOS" with its own icon** (the point of
D69) · registration-time RunAtLoad and `launchctl kickstart` both ran the trampoline → exit 0, no
duplicate instances · agent-launched session (kickstart with nothing running) → detached child in
menu-bar mode, `open` Reopen works → **toggle OFF: session survives, agent unregistered** ·
migration verified in the previous round (register + legacy plist removed) · no bare plist in
`~/Library/LaunchAgents` at any point.

## Manual TODO

- [ ] Owner: real log-out/in with the toggle on (the one scenario a script can't do).
- [ ] Ships in v0.1.2 with PR #37; watch the first 0.1.1→0.1.2 update for the
      agent-survives-update expectation recorded in D69.

## Definition of Done

- [x] plan.md ticked for what landed
- [x] decisions.md D69 amended to the as-built design
- [x] impl-plan present · handoff to follow at session end
- [x] docs move with code (tripwire would not fire)

## Plan compliance

Alignment: **good, with one review-driven redesign** — the impl-plan's simple guard grew into
trampoline + flock after the panel proved `unregister` kills launchd-owned processes; the plan's
docs-rule (generic certificate-attribution wording, no personal names) is honored in every
artifact. Frontend untouched, exactly as planned.
