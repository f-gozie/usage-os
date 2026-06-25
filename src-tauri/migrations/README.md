# Database migrations

Hand-written SQL migrations, applied once each, in order, by the runner in
[`src/migrations.rs`](../src/migrations.rs). They are embedded into the binary at
compile time (`include_str!`), so this directory ships inside the app — there is no
separate migration step for users.

## Rules

1. **Forward-only.** There are no down-migrations. This is a local-first desktop DB
   that only ever rolls forward; a bad migration is fixed by a new migration, never a
   rollback.
2. **Append-only & immutable once shipped.** Never edit a migration that has been in a
   release. The runner stores a checksum of each applied migration and re-verifies it
   on every boot; editing a shipped migration is a loud startup error, not silent
   schema drift. To change the schema, add the next `NNNN_*.sql` file.
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
