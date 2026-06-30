# Changelog

## v0.1.0 — first public release (2026)

The first build available to download. UsageOS is a private, on-device time tracker for macOS:

- **Your day, played back** — a dial of where your screen time went, an on-device recap that puts it in words (Apple's Foundation Models on Apple Silicon, a plain template everywhere else), plus week and timeline views.
- **Your categories** — a deterministic rules engine sorts apps and sites into categories you control; you decide what counts as work.
- **What it reads** — the active app, its window title (Accessibility), and the current browser site (Automation). Both permissions are optional; without them tracking is just app-level.
- **100% on your machine** — one SQLite file, retention you set, CSV export, and one-click delete-all. No cloud, no account, no telemetry.
- **Updates** — opt-in and off by default; when on, a once-a-day check for a newer signed version. Distributed as a notarized, Universal (Intel + Apple Silicon) DMG and via Homebrew.

> The version is `0.1.0` because this is an early, honest first release — not a `1.0`. The pre-public prototype that also carried `0.1.0` never shipped, so there's no public history before this build.

---

UsageOS's history lives where the work actually happened, in more detail than a changelog could hold:

- **[`context/plans/`](context/plans/)** — the roadmap and per-session **handoffs** (the real narrative of how it was built)
- **[`context/decisions.md`](context/decisions.md)** — every architectural decision, `D1` onward, with the reasoning
- **[GitHub Releases](https://github.com/f-gozie/usage-os/releases)** — tagged, downloadable builds

UsageOS began as a gamified tracker (its original specs are kept in
[`context/history/`](context/history/)) and pivoted in mid-2026 to the current calm,
on-device "rear-view mirror" — the dial, the on-device recap, the Swift Foundation-Models
sidecar, and the generated Rust↔TS IPC. The decision log and handoffs trace that evolution
commit by commit.
