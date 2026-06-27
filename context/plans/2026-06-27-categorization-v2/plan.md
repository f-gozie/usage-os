# UsageOS — Categorization v2 (rules: sites, precedence, default exclusions)

**Created:** 2026-06-27 · **Status:** 🅿️ backlog — not started (owner will pick up later) · **Owner:** Favour
**Origin:** surfaced 2026-06-27 while verifying landing-page claims against the code (branding-launch session). The landing/README/Settings were corrected to match *current* behavior; this plan is to close the gaps the verification exposed.

> Source of truth is the code. The "current behavior" below was read directly from the
> repo on 2026-06-27 — re-confirm before building (it may have moved).

## Goal
Make the rule engine match how people actually think about categorizing: let a **specific site or window-title override a broad app rule**, let rules **target sites directly**, and ship sensible **default exclusions** so the privacy promise is true out of the box. Stay calm, local, deterministic, user-owned — no behavior that needs the cloud or a model.

## Background — current behavior (verified 2026-06-27, cite before trusting)
- **Rules match on two fields only:** `process` (app-name substring) or `title` (window-title substring), case-insensitive `.contains`. The parsed **`site`/host is captured and stored but is NOT a category match field.** ([`db/categories.rs:105-131`](../../../src-tauri/src/db/categories.rs), [`capture/mod.rs:235`](../../../src-tauri/src/capture/mod.rs) calls `find_category(app, title)` — site never passed.)
- **Precedence = first matching rule in `id` order (earliest-created wins).** No specificity ranking; `process` vs `title` have no inherent priority. Both paths agree: live `find_category` returns the first `.contains` match in id order; bulk `reprocess_logs` applies rules in id order with `WHERE category_id IS NULL` so an earlier rule's claim sticks ([`db/events.rs:190-210`](../../../src-tauri/src/db/events.rs)).
- **Consequence (the user-facing gotcha):** browsers are **seeded** as `process → research` (Browsing) defaults with the lowest ids ([`migrations/0003_seed_default_rules.sql`](../../../src-tauri/src/migrations/0003_seed_default_rules.sql): `('research','Chrome')`, Safari, Arc, Firefox, Brave, …). So a title/site rule a user adds *later* (higher id) is evaluated *after* the Chrome rule and never fires for Chrome windows. **Today you cannot override an app's category for a specific site or title within that app.**
- **Exclusions** already support an `app | site | title` match type (and `exclude` vs `private` modes) — but **none are seeded by default**; every `1Password`/`Banking` exclusion in the tree is test-only. The "auto-exclude password managers/banking" promise is **not implemented** (claim was removed from landing + README on 2026-06-27).

## Work items

### W1 — Site-based category rules
Let a category rule match the parsed **site/host** (e.g. `youtube.com → Browsing`, `github.com → Work`), not just the app or window title.
- **Why:** the #1 ask — "group specific sites." The data is already captured (`activity_logs.site`), and **exclusions already match on `site`**, so the plumbing exists.
- **Sketch:** add `"site"` as a valid `match_field`; thread `site` into `find_category(process, title, site)` and add a `site` column branch in `reprocess_logs`; extend the category editor UI (the exclusion modal's App/Website/Title segmented control is the precedent to mirror). No schema migration needed (`match_field` is a free string; just validate the new value in UI + backend).
- **Caveat to document:** `site` is only present when Automation is granted and the window isn't private/incognito — site rules silently won't match in degraded mode. Surface this in the editor copy.

### W2 — Rule precedence / specificity (the override problem)
Make "a specific site or title beat a broad app rule" actually work. **Needs a decision (ADR) before building** — pick one:
- **(a) Field specificity (recommended default):** most-specific field wins — `site` > `title` > `process`; within a field, longest pattern (or most-recent) breaks ties. Solves the user's scenario with zero UI ("YouTube title rule beats the Chrome app rule" just works). Changes `find_category` from first-match to best-match, and `reprocess_logs` to apply passes in specificity order.
- **(b) Explicit priority:** a `priority` column + drag-to-reorder in the editor; first by priority, then id. More control, more UI, more user burden.
- **(c) Hybrid:** (a) as the default, (b) as an optional manual override.
- **Watch-outs:** the existing first-match-wins tests will change — update them and add specificity tests. Keep `reprocess_logs` and `find_category` resolving **identically** (they're deliberately kept in step — see `events.rs` comment). Re-run a full reprocess after the change so historical data re-sorts under the new rule.

### W3 — Default exclusions (close the "out of the box" promise)
Ship a curated default exclusion list so password managers (and ideally banking) are excluded without setup.
- **Sketch:** seed common password managers as `app`/`exclude` (1Password, Bitwarden, Dashlane, LastPass, KeePassXC, Proton Pass, Keychain Access). Banking is mostly *sites*, not apps — depends on W1 (site rules/exclusions exist) and is region-specific, so seed a small starter set or leave to the user; decide scope.
- **Migration nuance (decide):** a seed migration applies on **upgrade** too — don't clobber users who've already configured exclusions. Options: seed only on **fresh install**, or gate behind a one-time settings flag, or seed-if-empty. Pick and document.
- **Closes the loop:** once shipped, restore "password managers & banking excluded out of the box" on the landing + README (both were softened on 2026-06-27).

### Reference — embeddings-assisted categorization (NOT this plan)
The vision's planned smart layer (on-device NaturalLanguage embeddings matching new activity to your past corrections) is the *longer* arc and is **out of scope here** — this plan is about the deterministic rule engine. Note it so v2 rules aren't mistaken for the end state.

## Decisions to make (ADRs, when work starts)
1. **W2 precedence model** — (a) field specificity / (b) explicit priority / (c) hybrid. *(Recommend (a).)*
2. **W3 seed-on-upgrade behavior** — fresh-install-only vs seed-if-empty vs flag-gated.
3. **W3 banking scope** — apps only, or a starter site list too (gated on W1).

## Suggested sequencing
1. **W1 (site rules)** first — additive, unlocks W3's banking story, low risk.
2. **W2 (precedence)** next — the higher-design-risk piece; decide the ADR, then change both resolvers together + tests + a full reprocess.
3. **W3 (default exclusions)** — small once W1 lands; then restore the landing/README claims.

## Non-goals
No cloud/model categorization (that's the separate embeddings arc). No gamification. No per-event manual tagging UI (rules stay the model). Keep it deterministic and local.

## Lifecycle
Backlog. When picked up: register as `active` in [`../README.md`](../README.md), write impl-plans per task, `/usageos-review` each PR, append the ADRs above to `context/decisions.md`.
