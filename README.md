# Usage OS

[![CI](https://github.com/f-gozie/usage-os/actions/workflows/ci.yml/badge.svg)](https://github.com/f-gozie/usage-os/actions/workflows/ci.yml)

**A private, on-device time mirror for macOS.** Usage OS quietly records where your time goes and shows you the story of your day — calmly, with nothing leaving your machine.

> **Status — in redesign.** Today Usage OS ships as a local activity tracker (see _Available today_). It's being rebuilt around a 24-hour **day-dial**, a daily **recap**, and optional **on-device AI**, in a calm Bauhaus design. The full direction is in [`context/vision.md`](context/vision.md) and the plan is in [`context/plans/`](context/plans/). This README evolves with it.

## What it is

Your Mac already knows how you spent today. Usage OS turns that into _understanding_ — where you focused, what pulled you away, where the hours actually went — as a calm rear-view mirror you check on your own terms, not a productivity coach that nags you. 100% local.

## Features

**Available today (v0.1.0)**
- Background activity tracking (active window, 5s polling)
- Idle detection (3-minute threshold) + smart coalescing (30s gap)
- Category rules engine (process name / window-title matching)
- Configurable data retention (30 / 60 / 90 / 180 / 365 days, or keep all)
- Local SQLite with versioned migrations
- Dashboard: distribution chart, stats cards, today / yesterday / week

**In active redesign** (see [the plan](context/plans/2026-06-22-product-redesign.md))
- A 24-hour **day-dial** + a week-of-dials — the new signature
- A daily **recap**: an honest, plain-English summary of your day
- Two-axis model — **context** (Deep work / Research / Comms / Breaks) × **project** (auto-inferred)
- Reliable capture via macOS **Accessibility + Automation** (window titles + browser URLs)
- Optional **on-device AI** (Apple Foundation Models) for the recap and categorization — never the cloud
- A **Bauhaus** design language, light & dark

## Privacy

- **Nothing leaves your machine.** No cloud, no account, no telemetry, no network calls in the data path — and it's open source, so you can verify it.
- Data lives in a local SQLite database in your app data directory.
- Capture uses **Accessibility + Automation** only — **never Screen Recording**.
- You own your data, completely.

## Quick start

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Platform setup

The shipped tracker builds cross-platform; the **redesign targets macOS first** (capture relies on macOS APIs).

### macOS
Grant Accessibility permissions when prompted:
**System Settings → Privacy & Security → Accessibility**. No additional dependencies needed.

### Windows
No additional dependencies needed.

### Linux
```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxss-dev \
  libxdo-dev
```

## Development

### Prerequisites
- Rust (stable, latest)
- Node.js 22+
- Platform dependencies (see above)

### Running tests
```bash
# Rust tests (database, coalescing, categorization, migrations)
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript tests (stats, time utilities)
npx vitest run
```

## Architecture

The shipped layout: a React/TypeScript frontend (`src/`), a Rust/Tauri backend (`src-tauri/` — `db.rs` SQLite + migrations + coalescing, `watcher.rs` window polling + idle detection, `lib.rs` Tauri commands), and CI across Linux/macOS/Windows.

The **target architecture** for the redesign — a Rust core (objc2 capture) + a thin Swift Foundation Models sidecar, a generated tauri-specta IPC contract, and the rusqlite repository — is documented in [`context/architecture.md`](context/architecture.md).

## Tech stack

| Layer | Today | Redesign adds |
|-------|-------|---------------|
| Framework | Tauri v2 | — |
| Backend | Rust, rusqlite (bundled SQLite), versioned migrations | objc2 capture, Swift Foundation Models sidecar |
| Frontend | React 19, TypeScript, Vite 7 | Bauhaus design system, custom SVG day-dial |
| IPC | hand-written bindings | tauri-specta (generated, type-safe) |
| Styling | Tailwind CSS, Radix UI | — |
| Charts | Recharts | custom SVG dial (no chart library) |
| Testing | `cargo test` (Rust), Vitest (TS) | + capture/AI trait fakes |

## Project docs

- **Vision & spec** → [`context/vision.md`](context/vision.md)
- **Decisions (ADR log)** → [`context/decisions.md`](context/decisions.md)
- **Architecture** → [`context/architecture.md`](context/architecture.md)
- **Design system** → [`context/design-system.md`](context/design-system.md)
- **Plan** → [`context/plans/`](context/plans/)
- **Agent contract** → [`CLAUDE.md`](CLAUDE.md)

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md).

## License

MIT
