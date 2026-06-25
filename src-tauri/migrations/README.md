# Database migrations

Hand-written SQL migrations, applied once each, in order, by the runner in
[`src/migrations.rs`](../src/migrations.rs). They are embedded into the binary at
compile time (`include_str!`), so this directory ships inside the app — there is no
separate migration step for users.

## Rules

1. **Forward-only.** There are no down-migrations. This is a local-first desktop DB
   that only ever rolls forward; a bad migration is fixed by a new migration, never a
   rollback.
2. **Append-only & immutable once shipped** (D35/D54). Never change a shipped migration's
   **SQL**. The runner stores a checksum of each applied migration and re-verifies it on every
   boot. The checksum is over **normalized** SQL (comments + whitespace stripped, D54), so
   editing a migration's *comments* or reformatting it is fine — only a real statement change is
   drift. On real drift: a **release** build is a loud startup error (a shipped install must never
   silently run divergent DDL); **dev** (`tauri dev`/tests) self-heals — it rebaselines the local
   checksum with a one-line warning, so editing a migration never blocks the dev loop (dev DBs are
   throwaway). A *downgrade* (the DB has a migration this build doesn't ship) errors in both. To
   change the schema in a shipped migration, add the next `NNNN_*.sql` file instead.
3. **One concern per file**, numbered with a zero-padded prefix (`0001_`, `0002_`, …).
   The numeric prefix is the apply order; register the file in the `MIGRATIONS` array
   in `src/migrations.rs` with its version + name.
4. **Each migration runs in a transaction** (SQLite has transactional DDL), so a
   failure leaves the database untouched rather than half-migrated.

## Why hand-written SQL (not an ORM / derived migrations)

SQLite's `ALTER TABLE` is deliberately limited (column rename/drop/constraint changes
need the manual "create new table → copy → drop → rename" dance), and the project keeps
raw SQL in a typed repository layer by design. Auto-derived DDL would be both less
correct here and a heavier dependency than the audit-the-source ethos wants. So the SQL
is authored by hand; the runner just sequences and guards it.

## Current chain

| Version | File | What |
|---|---|---|
| 1 | `0001_initial_schema.sql` | Baseline schema (clean-slate squash of pre-1.0 v1–v8). |
| 2 | `0002_seed_canonical_contexts.sql` | Seed the 4 canonical contexts (deep/research/comms/breaks). |
| 3 | `0003_seed_default_rules.sql` | Starter categorization rules so the dial is meaningful out-of-the-box. |
| 4 | `0004_recategorize_claude_as_deep.sql` | Re-point the Claude rule (and past Claude spans) from Research to Deep work. |
| 5 | `0005_drop_away_app_spans.sql` | Delete historical lock-screen/screensaver spans (see D41). |
| 6 | `0006_relatable_default_categories.sql` | Fresh-install five-category model: Work · Browsing · Messaging · Entertainment · Personal (see D47). |

> **Deferred (audit B3):** adding `CHECK` constraints to the enum-like columns in `0001`
> (`mode`, `match_type`, `alias_kind`, abstain reason) needs a table rebuild or would change a
> shipped migration's recorded checksum and trip the drift guard — left for a future migration.
