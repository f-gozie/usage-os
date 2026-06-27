# Plans registry

The index of plans. A **plan** is a multi-session body of work (a roadmap). This
file is the entry point: find the plan you're working on, then read its folder.
(Workflow rules are authoritative in `CLAUDE.md` → _Dev workflow_; this is the map.)

## Registry

| Plan | Status | What |
|---|---|---|
| [`2026-06-27-categorization-v2/`](2026-06-27-categorization-v2/) | paused | **Backlog (not started).** Rule-engine gaps found while verifying landing claims: site-based category rules, rule precedence/specificity (let a specific site/title override a broad app rule), and default exclusions (auto-exclude password managers/banking). |
| [`2026-06-26-branding-launch/`](2026-06-26-branding-launch/) | **active** | Phase-5 launch front: finalize the logo (the **Contexts** dial-O), scatter it across every surface, build the landing page (Astro/Cloudflare), README/OSS face, notarized DMG. |
| [`2026-06-22-product-redesign/`](2026-06-22-product-redesign/) | **active** | The recap + day-dial + local-AI redesign (Phases 0–6). Phases 0–4 ✅, Phase 6 ✅; **Phase 5 launch spun out → branding-launch plan.** |
| [`2026-03-22-tier1-oss-hygiene/`](2026-03-22-tier1-oss-hygiene/) | done | v0.1.0 + OSS hygiene (tests, migrations, CI, retention). The foundation the redesign builds on. |

_Status ∈ `active` · `paused` · `done`. Add a row when a plan starts; update its status as it moves. Not every session maps to a plan — one-off tasks need no plan folder._

## Anatomy of a plan folder

```
<date>-<slug>/
  plan.md          # the roadmap — the ONE living doc here (check off / annotate; don't rewrite history)
  impl-plans/      # the approved plan-mode plan for each task/PR (the detailed how):  YYYY-MM-DD-<task>.md
  reviews/         # the /usageos-review report for each task/PR, paired to its impl-plan:     YYYY-MM-DD-<task>.md
  handoffs/        # one file PER SESSION, append-only, NEVER overwritten:             YYYY-MM-DD-NN-slug.md
```

- **plan.md** — the durable roadmap + checkboxes. Living.
- **impl-plans/** — the approved implementation plan behind each PR (the as-built detail). One per task.
- **reviews/** — the `/usageos-review` report for each PR (merge-gate results, verified findings, plan-compliance), named to **pair** with its impl-plan (`impl-plans/<date>-<task>.md` ↔ `reviews/<date>-<task>.md`). The `/usageos-review` skill writes these. Together the folder reads as the feature's whole lifecycle — **plan → impl-plan → review → handoff** — so a PR reviewer can see the thought process in one place.
- **handoffs/** — the session journal. The newest file is "where we are now." Immutable once written, so the chain is the project's narrative history. (The pre-2026-06-23 handoffs were reconstructed from git.)
- The **why** lives separately in `context/decisions.md` (append-only ADRs, D1–…), cross-referenced from plans/handoffs.

## Finding "where we are now"

1. This registry → the `active` plan(s).
2. That plan's `plan.md` (roadmap) + the **newest** file in its `handoffs/` (current state, gotchas, next steps).
3. `context/decisions.md` for the rationale behind anything.
