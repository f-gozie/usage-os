# Spike #5 — project-inference accuracy + the abstain threshold

> **Status: ✅ RUN — PASS (2026-06-22, snapshot).** On a corpus of real signals from
> this machine, the inference heuristic made **no false assignments** — every project
> it emitted was correct, and everything else **abstained** (general browsing, a shell
> at `/`, project-ambiguous dev tools). It cleanly identified the 3 active projects
> (`eyemark_frontend`, `usage_os`, `nudge`) and, critically, surfaced two design
> decisions: **project identity must be canonicalized** (the same project arrives as
> both `f-gozie/usage-os` and `usage_os`), and **"ambiguous-but-work" is a distinct
> third state** (localhost / dashboards) worth correlating later. This crate is
> isolated; pure std + shelled `git`. _Snapshot ≠ multi-day recall — see Limitations._

## What this proves (a *quality* spike, not a capability one)

Spikes ①–④ proved we can **capture** the three project signals — window **title** (①),
browser **URL** (③), terminal **cwd** (④). This one asks the different question: **how
accurately do those become a "project", and when must we abstain?** For a calm mirror,
a **wrong** project label costs more trust than a missing one — so the decision this
spike settles is the **abstain threshold** (R23/R26/R27): emit a project only on an
unambiguous, high-precision signal; otherwise leave it **unassigned**.

## The heuristic

| Signal | Rule | Result |
|--------|------|--------|
| **cwd** in a git repo | `git -C <cwd> remote get-url origin` → `owner/repo` (canonical), else folder name | **PROJECT** · HIGH · `cwd-git-remote` |
| **browser URL** `github.com/owner/repo` (R26) | first two path segments, minus `/settings`, `/notifications`, … | **PROJECT** · HIGH · `github-url` |
| **browser URL** `file://…/projects/<name>/…` | the segment after `projects/` | **PROJECT** · MED · `local-file` |
| **window title** `… — <project>` | trailing chunk, unless it's a bare app name | **PROJECT** · MED · `window-title` |
| `localhost:port` | clearly work, project unknown | **abstain · ambiguous** |
| dev dashboards (posthog, grafana, cloudflare, App Store Connect, …) | work, project unknown | **abstain · ambiguous** |
| youtube / x / gmail / google / docs / SO / … | not a project | **abstain · no-signal** |
| shell at `/`, github non-repo page, bare app name | no signal | **abstain · no-signal** |

**Abstain threshold:** assign only at HIGH (`cwd-git-remote`, `github-url`) or MED
(`local-file`, `window-title`) confidence **and** unambiguous. `localhost` + dashboards
abstain even though they're obviously work — guessing their project is the main
false-positive risk.

## Run

```sh
cd spikes/project-inference
cargo build && ./target/debug/project-inference
```

Pure std; shells `git` to resolve cwd→repo live. `clippy -D warnings` + `fmt` green;
crate root carries the hard-rule-3 `deny`.

### Observed results — PASS (2026-06-22)

Corpus: real terminal cwds (resolved live), representative real browser tabs (from the
85 normal-window tabs gathered in Spike ③), and editor titles.

```
cwd …/projects/combined/eyemark_frontend  → PROJECT zst-tech/eyemark_frontend  [HIGH cwd-git-remote]
cwd …/projects/usage_os                   → PROJECT f-gozie/usage-os           [HIGH cwd-git-remote]
cwd …/projects/nudge                      → PROJECT usenudgeai/nudge           [HIGH cwd-git-remote]
cwd /                                      → abstain·no-signal  (not inside a git repo)
url github.com/usenudgeai/nudge            → PROJECT usenudgeai/nudge           [HIGH github-url]
url github.com/f-gozie/usage-os/pull/6     → PROJECT f-gozie/usage-os           [HIGH github-url]
url github.com/notifications               → abstain·no-signal  (github non-repo page)
url localhost:3002                         → abstain·ambiguous  (local dev server)
url file://…/projects/nudge/…/mockup.html  → PROJECT nudge                      [MED local-file]
url youtube / x / gmail / google / docs    → abstain·no-signal  (general browsing)
url posthog / grafana / cloudflare / ASC   → abstain·ambiguous  (dev dashboard)
title "Browser Tab — nudge"                → PROJECT nudge                      [MED window-title]
title "Claude" / "Spotify Premium"         → abstain·no-signal  (bare app name)

Summary: 22 signals → 8 assigned · 9 abstain·no-signal · 5 abstain·ambiguous
```

**No false positives.** Every assignment was correct; every abstain was appropriate.
(Ground truth: `eyemark_frontend`, `usage_os`, `nudge` are the active projects; the
rest is general browsing or project-ambiguous tooling.)

## Findings

1. **`cwd → git remote` is the anchor signal** — deterministic, canonical (`owner/repo`),
   zero false positives. `github-url` corroborates it (R26 extraction is trivial and
   reliable). When cwd and URL agree (nudge + usage-os appeared in **both**), confidence
   is very high.
2. **Project identity must be canonicalized — the headline design finding.** The same
   project arrived under different ids: `f-gozie/usage-os` (remote) vs `usage_os`
   (folder/title); `usenudgeai/nudge` (url) vs `nudge` (folder/title). Without
   reconciliation, **one project fragments into several**. → The `projects` table's
   canonical key should be the **git remote `owner/repo`**, with the folder name,
   title-derived name, and any github URL stored as **aliases** that resolve to it.
3. **Abstaining is the common case, and that's correct.** On the full 85-tab browser
   sample, ~90% was general browsing; on this 22-signal corpus, **14/22 abstained**
   (9 no-signal + 5 ambiguous). A calm mirror should show a lot of "unassigned" rather
   than confident-but-wrong labels.
4. **"Ambiguous-but-work" is a real third state**, distinct from "no signal." `localhost`
   and dev dashboards (posthog / grafana / cloudflare / App Store Connect) are clearly
   work but project-unknown. The spike abstains; **Phase 2 can temporally correlate**
   them to the project active in the editor/terminal at the same time — a high-value
   enhancement because dashboards are frequent. Store the *ambiguous* reason distinctly
   so this is possible later.
5. **`github.com/owner/repo` extraction (R26) works**, with a non-repo denylist
   (`/settings`, `/notifications`, `/marketplace`, …) to avoid treating GitHub-the-site
   as a project.

## Limitations (be honest)

- **Snapshot, not longitudinal.** This measures **precision** + the abstain threshold,
  not multi-day **recall** or time-weighted volume. Recall needs the Phase-1 capture
  pipeline persisting signals over days — re-measure then.
- **Window-title parsing is the weakest rung** (MED) — folder-name only, and real editor
  titles vary (some won't carry the project). Prefer cwd/URL when available; treat title
  as a fallback.
- **Temporal correlation** (ambiguous → active project) is designed here but **not
  measured** — it needs concurrent multi-signal capture.
- Vocabularies (general hosts, dev dashboards, github non-repo) are seeded from this
  machine; they'll grow. Keep them data-driven, not hardcoded forever.

## Note for the capture/enrich-layer port

- **`projects` table keyed on the canonical git remote** (`owner/repo`); folder name,
  title-name, and github URL are aliases resolving to it (finding #2).
- Encode the **abstain threshold** in the inference fn; persist an explicit
  `unassigned` rather than a low-confidence guess.
- Persist the **abstain *kind*** (`no-signal` vs `ambiguous`) so Phase 2 can correlate
  the ambiguous ones; never correlate `no-signal`.
- Resolve cwd→repo with `git rev-parse --show-toplevel` + `remote get-url origin` (cache
  per repo root — it's stable).
