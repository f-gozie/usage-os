---
name: review
description: >-
  The standard pre-PR review for UsageOS. Reviews the current diff against the
  project's 8 hard rules, runs the real merge gates, gets an independent Codex
  cross-model second opinion, verifies every finding against real code, applies
  only safe fixes, and writes the report into the active plan folder so a
  feature's plan → impl-plan → review live in one place. Use before opening or
  updating any PR, or when the user says "review", "review this", "review the
  branch / PR / diff", or "/review".
argument-hint: "[branch|staged|uncommitted|rust|ui|sidecar|<path>] [--no-codex]"
---

# `/review` — UsageOS pre-PR review

A parallel review **panel** for UsageOS, then a mandatory **verify-against-real-code**
pass, then **consolidate + safe auto-fix**, then a report written **into the active
plan folder**. Built for *this* product, not a generic web stack.

UsageOS is one Tauri app — **Rust core + React/TS + a macOS-only Swift Foundation-Models
sidecar**, **local-only** (no network, no auth, no server). So its review surface is
its **8 hard rules** and its **plans/handoffs/decisions** lifecycle, not SQLi/XSS/CORS.
Two ideas carry over from the nudge `review-all` because they're the parts that work:

1. **An independent Codex cross-model agent** runs on the same diff by default — a
   genuinely different model, not Claude reviewing Claude's own work. It surfaces
   blind spots a single model misses. Skip with `--no-codex`; it degrades gracefully
   if the `codex` CLI is missing.
2. **Every Critical/Warning finding is verified against the actual code** before it
   reaches the report. Findings are advisory, not gospel.

> **Source of truth is the repo, not this file.** Read `CLAUDE.md` → *Hard rules* +
> *What this product is NOT* and `.github/workflows/ci.yml` (the gate list) **at review
> time** — they evolve, and this skill must follow them, never a hardcoded copy. If a
> rule here ever disagrees with `CLAUDE.md`, `CLAUDE.md` wins.

Use **Opus** and **high reasoning effort** for this review — correctness and the
verification pass matter more than speed.

---

## Arguments

Default scope is **`branch`** — the use case is per-PR.

| Arg | Scope |
|---|---|
| `branch` *(default)* | Changes since `main`: `git diff main...HEAD` |
| `staged` | `git diff --cached` |
| `uncommitted` | `git diff` (working tree) |
| `rust` | Only `src-tauri/**` in the scoped diff |
| `ui` | Only `src/**` in the scoped diff |
| `sidecar` | Only `sidecar/**` in the scoped diff |
| `<path>` | A specific file or directory |
| `--no-codex` | Skip the Codex cross-model lane (Lane D) |

---

## Phase 1 — Scope & context

1. **Compute the diff** for the chosen scope and list changed files:
   ```bash
   git diff --name-only main...HEAD        # branch (default)
   git diff --cached --name-only           # staged
   git diff --name-only                    # uncommitted
   ```
2. **Filter noise:** `target/`, `node_modules/`, `dist/`, `.vite/`, `package-lock.json`,
   `Cargo.lock`, icons/binaries.
3. **`src/bindings.ts` is GENERATED (hard rule 2).** Never review it as authored code —
   only check that it wasn't hand-edited (Phase 2, Lane A) and that it's fresh (Phase 2.5).
4. **Resolve the active plan** so the report has a home and plan-compliance can be checked:
   - Read `context/plans/README.md` → the row marked **`active`** → the plan folder.
   - In that folder, find the **`impl-plans/` entry** that matches this task (by date/slug,
     or the newest if obvious). Note its path — the review will pair with it.
   - If no active plan or no impl-plan matches, note it (the DoD check in Phase 6 will flag).

---

## Phase 2 — Launch the review panel (one message, in parallel)

Launch all lanes **in a single message** so they run concurrently. Lanes A–C are Agent
subtasks (or done inline if the diff is small); Lane D is a Bash call to Codex in that
same batch. Each lane gets the same scope and file list.

### Lane A — Hard-rules & architecture gate *(the UsageOS-specific lane)*

**Re-read `CLAUDE.md` → Hard rules + What this product is NOT now**, then check the diff
against each. This lane is where most real UsageOS findings live. *Security is folded in
here* — for a local-only app, the threat surface is the privacy rule + unsafe + SQL, not web.

1. **① Privacy — nothing leaves the machine.** No network in the data path. Flag any new
   `reqwest`/`ureq`/`hyper`/`isahc`/`std::net`/`tokio::net`/`URLSession`/`fetch` reaching
   `src-tauri/src/**` or `sidecar/**`. The **only** permitted network is an explicit,
   user-initiated update check. A new outbound call with no clear user-initiated update
   purpose is **Critical**.
2. **② IPC is generated, never hand-written.** `src/bindings.ts` must not be hand-edited
   (Phase 2.5 freshness gate proves it). Every new `#[tauri::command]` must be registered
   in the `tauri-specta` builder so bindings regenerate. Hand-edited bindings = **Critical**.
3. **③ No `unwrap()`/`expect()`/`panic!`/`unreachable!`/`todo!` in production paths.** Allowed
   only in `#[test]`/`#[cfg(test)]` or with a comment proving the invariant is truly impossible.
   A new `unwrap`/`expect` on a real `Result`/`Option` in a prod path = **Critical**.
4. **④ All SQL in the repository layer.** SQL strings and `Connection`/`rusqlite` handles
   live only in `store`/`db.rs`/`migrations.rs`. A SQL string or DB handle leaking into a
   command handler or business logic = **Critical**.
5. **⑤ Native + AI surface stays isolated & mockable.** `capture` and `ai` are traits with
   fakes; `objc2`/Swift impls sit behind `#[cfg(target_os = "macos")]` and the trait, never
   imported directly elsewhere. A direct `objc2`/AX/`NSWorkspace` call outside `capture/macos/`,
   or AI/sidecar code that can't be faked, = **Warning→Critical**.
6. **⑥ The smart layer narrates, it never counts.** Recap/`ai` code may only phrase
   **pre-computed** `RecapFacts` aggregates — numbers are computed in Rust. The deterministic
   `TemplateRecap`/`build_recap` fallback must stay intact. Any place the model is handed raw
   events to count, or asked to produce a number, = **Critical** (it can fabricate counts).
7. **⑦ Design system, not vibes.** New colors/fonts/spacing must use tokens from
   `context/design-system.md` / `tokens.css`. Ad-hoc hex colors, raw px, or off-token
   fonts = **Warning**.

   **Unsafe sub-lens:** new `unsafe` (objc2) blocks are minimal and have a safety comment;
   rusqlite uses parameter binding (no `format!`/string-concatenated SQL); the Swift sidecar
   validates stdin before acting. Reference `decisions.md` so known-deliberate choices aren't
   flagged (e.g. `Arc<Mutex<Connection>>` is the documented interim — see D-notes; the
   `specta rc.20` pin is deliberate; `ActivityLog.category_id` keeps its legacy name by D42).

### Lane B — Correctness & bugs

Real logic bugs in the diff: error handling, edge cases, off-by-one, `None`/empty handling,
concurrency (the `Arc<Mutex<Connection>>` writer + WAL story — D-notes), migration safety
(fresh-vs-upgraded parity, idempotence), and **what the diff actually does** vs what it claims.
Trace changed functions; don't pattern-match.

### Lane C — Simplify & altitude

Reuse, dead code, over-abstraction, AI-tells (needless verbose comments, speculative
generality), naming, complexity (functions >~50 lines, nesting >3). **Match the surrounding
code's idiom** — comment density, naming, patterns. Also **microcopy voice:** any new
user-facing string must read human (no AI-tells, no jargon) — UsageOS copy is first-class.

### Lane D — Codex cross-model agent *(on by default; `--no-codex` to skip)*

Run an independent review with the Codex CLI **in the same parallel batch** — a different
model on the same diff.

```bash
# 1. Capture the scoped diff (same scope as Phase 1).
git diff main...HEAD > /tmp/review-diff.patch     # or --cached / plain diff per scope

# 2. Codex: read-only, ephemeral, structured findings against the UsageOS schema.
{
  echo "You are a senior engineer reviewing a git diff for UsageOS — a PRIVATE, ON-DEVICE macOS app (Tauri v2: Rust backend, React/TS frontend, a macOS-only Swift Foundation-Models sidecar). It is LOCAL-ONLY: no network in the data path (the product's whole promise), no auth, no server. Enforce these hard rules and report violations: (1) nothing leaves the machine — no network calls in the data path; (2) the Rust<->TS IPC is generated by tauri-specta, never hand-edited (src/bindings.ts is generated); (3) no unwrap()/expect()/panic! in production paths; (4) all SQL lives in the repository/store layer; (5) capture + ai are mockable traits, objc2/Swift stay isolated behind cfg; (6) the smart layer NARRATES pre-computed numbers, it never counts — the model must never produce a count; (7) UI uses design-system tokens. Report only real, actionable findings tied to a changed file and line, as JSON per the schema. Do NOT invent speculative risks or propose large rewrites. severity: critical = bug/security/privacy/correctness, warning = should fix, info = minor. Diff:"
  echo
  cat /tmp/review-diff.patch
} | codex exec --ephemeral -C "$(pwd)" -s read-only \
      --output-schema .claude/skills/review/codex-schema.json \
      -o /tmp/review-codex.json -
```

Then read `/tmp/review-codex.json` and fold its `findings[]` into Phase 3 (verify) and
Phase 4 (consolidate). Map Codex `severity`/`category` onto the same buckets/lanes.

**Graceful degradation:** if `codex` isn't on PATH, exits non-zero, times out, or writes
no parseable JSON, add "Codex skipped: <reason>" to the report and continue with Lanes A–C.
**Never fail the whole review because Codex was unavailable.**

> Validated against `codex 0.130.0` (`--ephemeral`, `-s read-only`, `--output-schema`, `-o`).
> Schema: `.claude/skills/review/codex-schema.json`.

---

## Phase 2.5 — Run the real merge gates (don't eyeball them)

Hard rule #8: *red = not merged*. **Actually run** the gates CI runs — read the current set
from `.github/workflows/ci.yml` (don't trust the snapshot below if CI has moved on):

```bash
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings
cd src-tauri && cargo test
npx tsc --noEmit
npx vitest run
# binding freshness (hard rules 2 + 8):
cd src-tauri && cargo test export_bindings && cd .. && git diff --exit-code -- src/bindings.ts
```

- **Any red gate = Critical** in the report — it blocks merge by definition.
- **Degrade gracefully:** a command that doesn't exist yet (e.g. `swift build` off macOS,
  RTL not installed) is **noted, not failed**. State what was and wasn't run.
- **Tests-for-new-code heuristic** (flag as **Warning**, no invented %-threshold):
  a new `store`/repo fn → wants a store test; new `domain`/`enrich`/`rollup` logic →
  a unit test; a new trait impl → a fake-based test.

---

## Phase 3 — Verify every finding (MANDATORY)

Before consolidating, validate **every Critical and Warning** from all lanes (Claude **and**
Codex) against the real code. This is the discipline that keeps the report trustworthy.

For each finding:
1. **Open the cited `file:line` and confirm it's real.** If it's not in the changed set,
   drop it (scope guard).
2. **Reject speculative risks and over-engineered rewrites.** Can't tie it to a concrete
   code path? Mark "unverified" → downgrade to Info or drop.
3. **Check it against project context** the reviewer may have missed — `CLAUDE.md` rules,
   `context/decisions.md` (the *why* — don't "fix" a documented deliberate choice), and
   memory. Examples that are **not** bugs: `Arc<Mutex<Connection>>` (documented interim),
   the `specta rc.20` pin, `ActivityLog.category_id`'s legacy name (D42).
4. **Sibling-instance expansion:** when you accept a bug-class finding, grep the diff and
   nearby code for the same shape and list every other instance. A pattern bug is never one line.
5. **Confirm any suggested fix won't break a gate** or violate a hard rule before it enters
   the auto-fix set.

**Cross-model agreement is signal:** a finding raised by **both** Codex and a Claude lane is
high-confidence — mark it *cross-model confirmed*. Track counts: verified / dropped / cross-model.

---

## Phase 4 — Consolidate

1. Merge findings from all lanes.
2. **Deduplicate** — same file + line range + category = one entry (keep most severe; mark
   cross-model-confirmed when Codex and a Claude lane agree).
3. Sort **Critical > Warning > Info**; group by file.
4. Separate: auto-fixed · needs-manual-fix · informational.

---

## Phase 5 — Safe auto-fix only

Apply **only provably-safe** fixes, then **re-run the affected gate** to confirm green:

**Safe to auto-fix:** `cargo fmt`, `cargo clippy --fix` (machine-applicable only), `prettier`,
dead/unused-import removal, trivial early-returns/renames Lane C flagged.

**NEVER auto-fix (report for manual fix):**
- Anything touching **network/privacy** code (hard rule 1)
- **Generated `src/bindings.ts`** (hard rule 2 — regenerate via the export test, never hand-fix)
- **`unsafe` blocks**, `objc2`/native code
- **Logic changes**, error-handling semantics, concurrency
- **Public command signatures / IPC shape**, migrations
- Anything the verification pass couldn't confirm

---

## Phase 6 — Definition-of-Done / lifecycle check

UsageOS's distinctive step — first-classes the `.githooks/pre-push` tripwire as a review lens
(*docs move in lockstep with code*):

- Does the diff change `src/` or `src-tauri/src/` **without** touching `context/plans/` or
  `context/decisions.md`? → flag (the tripwire condition).
- Is **`plan.md`** ticked/annotated for what landed?
- Was a decision made that needs a new **ADR** appended to `decisions.md`?
- Is there an **impl-plan** for this task, and will a **handoff** be written at session end?
- Reference the active plan + its impl-plan **by path** in the report.

---

## Phase 7 — Write the report into the plan lifecycle

Write to the **active plan folder**, named to **pair with the impl-plan**
(`impl-plans/<date>-<task>.md` ↔ `reviews/<date>-<task>.md`):

```
context/plans/<active-plan>/reviews/YYYY-MM-DD-<task>.md
```

Create the `reviews/` dir if absent. The header links `plan.md` + the impl-plan so a PR
reviewer reads **plan → impl-plan → review** as one narrative — the feature's mental map.

````markdown
# Review — <task>

**Date:** YYYY-MM-DD · **Scope:** <branch|staged|…> · **Files:** N
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [<date>-<task>.md](../impl-plans/<date>-<task>.md)
**Codex:** ran | skipped (<reason>)

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅/❌/skipped |
| cargo clippy -D warnings | ✅/❌ |
| cargo test | ✅/❌ |
| tsc --noEmit | ✅/❌ |
| vitest | ✅/❌ |
| bindings fresh | ✅/❌ |

## Findings
**Verification:** V verified · D dropped/downgraded · X cross-model confirmed

### Critical (must fix before merge)
- `path:line` — **[lane]** description · why it matters · fix

### Warnings (should fix)
- `path:line` — **[lane]** description

### Info
- …

## Auto-fixes applied
- …

## Manual TODO
- [ ] …

## Definition of Done
- [ ] plan.md ticked for what landed
- [ ] decisions.md ADR appended (if a decision was made)
- [ ] impl-plan present · handoff to follow
- [ ] docs move with code (pre-push tripwire would not fire)

## Plan compliance
Alignment: good/partial/poor — <one line on scope match / creep / deviation>
````

---

## Phase 8 — Present & offer

1. Show a short summary in chat (the gate table + finding counts + report path).
2. Highlight Criticals that **must** be fixed before merge.
3. Offer to fix the manual items one by one.
4. If everything's green: **"All gates pass, no Critical/Warning findings — ready to ship."**

---

## Notes for future-proofing (why this stays correct as the repo evolves)

- The hard-rules list and the gate list are **read from the repo at review time**
  (`CLAUDE.md`, `ci.yml`), never hardcoded here — so adding a 9th rule or a new gate is
  picked up automatically.
- **No tool versions are pinned** in this skill. Net-new tooling that may not exist yet
  (RTL component tests, `cargo-deny`, a network-gate proxy) is handled by graceful
  degradation, not assumed.
- Codex is **optional and self-degrading** — the review still works (Lanes A–C + gates) on
  a machine without the `codex` CLI.
- The skill is **self-contained** (`SKILL.md` + `codex-schema.json`) and carries no
  UsageOS secrets, so it's safe to keep in the public repo.
