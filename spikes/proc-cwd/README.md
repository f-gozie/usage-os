# Spike ④ — read another process's cwd via `proc_pidinfo`

> **Status: ✅ RUN — PASS (2026-06-22, macOS / Apple Silicon).** A **non-root,
> unsandboxed** process **can** read another process's current working directory
> via `proc_pidinfo(PROC_PIDVNODEPATHINFO)` — **no `EPERM`**. Verified against
> interactive zsh shells it did not spawn, with values matching `lsof` exactly
> (a shell in `…/projects/usage_os` reads as that path; one in `…/projects/nudge`
> reads as that). The terminal-cwd branch of project inference is **viable**.
> This crate is **isolated** (not a workspace member) and depends only on `libc`.

## What this proves (and why it matters)

The "project" axis infers which repo/codebase you're in. Editors give it via the
window **title** (Spike #1: Cursor → `… — nudge`) and browsers via the **URL**
(Spike ③: `github.com/owner/repo`). **Terminals are the blind spot** — their title
is usually `zsh`, a hostname, or the running command. The real signal is the
shell's **current working directory**: `…/projects/usage_os` → project `usage_os`.

The capture layer would read that with `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, …)`.
The audit's **R22** is the kill-switch question, rated *uncertain* because sources
only confirm the **negative** (a *sandboxed* process is blocked); the
unsandboxed-non-root case is undocumented:

> Can a non-root, unsandboxed process read **another** process's cwd this way, or
> does it return `EPERM`? If `EPERM`, the clean terminal-cwd branch dies and we
> fall back to per-app routes (iTerm2's AppleScript `path`, Terminal's tty→cwd) — **R24**.

**Answer: it works.** So the universal `proc_pidinfo` path is viable and the R24
per-app fallbacks are **not needed for the common case**.

This binary touches no network and writes nothing. `libc::proc_pidinfo` /
`proc_vnodepathinfo` / `PROC_PIDVNODEPATHINFO` come straight from `libc` 0.2 — no
hand-rolled FFI. No `unwrap()`/`expect()`/`panic!` in the logic.

## What it does

1. **self** — reads its own cwd; cross-checks against `std::env::current_dir()`.
2. **spawned child** — `/bin/sleep` launched with `current_dir("/private/tmp")`;
   reads its cwd back (correctness on a pid we control, known answer).
3. **argv pids** — reads the cwd of any pids passed on the command line: real
   terminal shells / GUI apps it did **not** spawn — the genuine cross-process test.

It also prints each target's executable basename (`proc_pidpath`) for context.

## Build & run

```sh
cd spikes/proc-cwd
cargo build

# self + spawned-child tests, plus real interactive zsh shells (note: zsh does NOT
# word-split unquoted $vars — use xargs so each pid is a separate argument):
pgrep -x zsh | sort -n | head -6 | xargs ./target/debug/proc-cwd
```

- **Binary:** `target/debug/proc-cwd` — `Mach-O 64-bit executable arm64`.
- `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` green; crate
  root carries the hard-rule-3 `deny`. Single dependency: `libc`.
- **No TCC permission needed** — `proc_pidinfo` for same-uid processes requires no
  Accessibility/Automation grant.

### Observed results — PASS (2026-06-22)

`euid=501` (non-root), unsandboxed. `lsof` shown as ground truth — `proc-cwd`
matched it on every pid:

```
ground truth (lsof):                         proc-cwd read:
  pid 41937 → …/projects/usage_os              ✅ 41937  cwd=/Users/favour/Documents/projects/usage_os   [zsh]
  pid 58394 → …/projects/nudge                 ✅ 58394  cwd=/Users/favour/Documents/projects/nudge       [zsh]
  pid 41943 → /                                ✅ 41943  cwd=/                                            [zsh]

  self    ✅ cwd=…/spikes/proc-cwd  (== env::current_dir)          [proc-cwd]
  child   ✅ cwd=/private/tmp       (== the current_dir we set)    [sleep]
```

Also read **Cursor** (57241) and **Finder** (1029) — hardened-runtime GUI apps not
spawned by us — successfully (cwd `/`). **No `EPERM` / `EACCES` on any target.**

**Verdict: PASS.** Same-uid cross-process cwd reads work from a plain unsandboxed
CLI; values match `lsof`; shells in real project dirs yield exactly the project
signal we want.

#### Findings

1. **R22 resolved — the clean path works.** No per-app workaround is required for the
   read itself. **R24's iTerm2-`path` / Terminal-tty fallbacks are not needed** for the
   common case (keep them only as a belt-and-suspenders option).
2. **Correctness is exact** — `proc-cwd` matched `lsof -d cwd` on every pid, including
   the two shells sitting in real repos. The vnode path is the resolved absolute path
   (`/private/tmp`, not `/tmp`).
3. **Hardened-runtime GUI apps are readable too** (Cursor, Finder) — same-uid is the
   only gate that matters here; SIP/hardened-runtime does not block cwd introspection.
4. **The remaining work is pid *selection*, not feasibility.** Spike ② says *iTerm2 is
   frontmost (app pid)*; to label terminal time we must pick the **front tab's shell
   pid**. Options for Phase 1: `proc_listchildpids(terminalPid)` + a tty/recency
   heuristic, or iTerm2's AppleScript `path` of the current session (R24, precise but
   needs an Automation grant). This is mechanism, not a feasibility risk.
5. **`euid` matters, not entitlements.** The read succeeded with no entitlements and no
   TCC grant. In the shipped app this stays grant-free (a nice contrast to AX/Automation).

## Note for the capture-layer port

- Use `libc::proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, 0, &mut info, size)`; success is
  `ret == size_of::<proc_vnodepathinfo>()`. On `ret <= 0` read `errno`
  (`ESRCH` = process gone, `EPERM` = blocked) and fall back.
- `pvi_cdir.vip_path` is `[[c_char; 32]; 32]` (a split 1024-byte MAXPATHLEN buffer) —
  read it as 1024 contiguous bytes to the first NUL.
- Gate the whole module behind `#[cfg(target_os = "macos")]` and the `capture` trait
  (hard rule 5); a `Fake` returns canned cwds for CI.
- Treat cwd as **best-effort enrichment**: a shell at `~` or `/` carries no project
  signal — abstain rather than guess (feeds the project-inference abstain threshold).
