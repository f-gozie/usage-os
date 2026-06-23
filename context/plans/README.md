# Plans registry

The index of plans. A **plan** is a multi-session body of work (a roadmap). This
file is the entry point: find the plan you're working on, then read its folder.
(Workflow rules are authoritative in `CLAUDE.md` → _Dev workflow_; this is the map.)

## Registry

| Plan | Status | What |
|---|---|---|
| [`2026-06-22-product-redesign/`](2026-06-22-product-redesign/) | **active** | The recap + day-dial + local-AI redesign (Phases 0–5). Phase 0 ✅, Phase-1 backend capture ✅; UI remaining. |
| [`2026-03-22-tier1-oss-hygiene/`](2026-03-22-tier1-oss-hygiene/) | done | v0.1.0 + OSS hygiene (tests, migrations, CI, retention). The foundation the redesign builds on. |

_Status ∈ `active` · `paused` · `done`. Add a row when a plan starts; update its status as it moves. Not every session maps to a plan — one-off tasks need no plan folder._

## Anatomy of a plan folder

```
<date>-<slug>/
  plan.md          # the roadmap — the ONE living doc here (check off / annotate; don't rewrite history)
  handoffs/        # one file PER SESSION, append-only, NEVER overwritten:  YYYY-MM-DD-NN-slug.md
  impl-plans/      # the approved plan-mode plan for each task/PR (the detailed how):  YYYY-MM-DD-<task>.md
```

- **plan.md** — the durable roadmap + checkboxes. Living.
- **handoffs/** — the session journal. The newest file is "where we are now." Immutable once written, so the chain is the project's narrative history. (The pre-2026-06-23 handoffs were reconstructed from git.)
- **impl-plans/** — the approved implementation plan behind each PR (the as-built detail). One per task.
- The **why** lives separately in `context/decisions.md` (append-only ADRs, D1–…), cross-referenced from plans/handoffs.

## Finding "where we are now"

1. This registry → the `active` plan(s).
2. That plan's `plan.md` (roadmap) + the **newest** file in its `handoffs/` (current state, gotchas, next steps).
3. `context/decisions.md` for the rationale behind anything.
