# Changelog

## v0.1.1 — runs in the menu bar (2026)

- **Close the window, keep the tracking** — closing UsageOS now also removes it from the Dock; it keeps recording quietly from the menu bar. Bring it back from the menu bar icon, or open it like any app.
- **Start at login** — a new opt-in toggle (Settings → Background, also offered during onboarding) starts UsageOS in the menu bar when you log in. Off by default, and System Settings → Login Items shows it as UsageOS, like any proper app.

## v0.1.0 — first public release (2026)

UsageOS is a private, on-device time tracker for macOS.

- **See your day** — a dial of where your screen time went, an on-device recap in plain words, plus week and timeline views.
- **Your categories** — a rules engine sorts apps and sites into categories you control.
- **What it reads** — the active app, its window title, and the current browser site. Both permissions are optional.
- **Local only** — one SQLite file, retention you set, CSV export, delete-all. No cloud, account, or telemetry.
- **Updates** — opt-in, off by default. A notarized Universal DMG, also on Homebrew.

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
