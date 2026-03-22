# Usage OS

[![CI](https://github.com/f-gozie/usage-os/actions/workflows/ci.yml/badge.svg)](https://github.com/f-gozie/usage-os/actions/workflows/ci.yml)

A privacy-first desktop activity tracker built with Tauri v2 (Rust + React). Silently monitors app usage and presents data in a cyberpunk-styled dashboard.

## Features

- **Background Activity Tracking** — Polls active window every 5 seconds
- **Idle Detection** — Detects inactivity after 3 minutes
- **Smart Coalescing** — Merges consecutive entries for the same app (30s gap threshold)
- **Category Rules Engine** — Classify apps by process name or window title patterns
- **Data Retention** — Configure automatic cleanup (30/60/90/180/365 days or keep all)
- **Local SQLite** — All data stays on your machine
- **Cyberpunk UI** — Dark mode with neon accents, pie charts, and stats cards

## Screenshots

<!-- TODO: Add screenshots -->

## Quick Start

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Platform Setup

### macOS

Grant Accessibility permissions when prompted:
**System Settings → Privacy & Security → Accessibility**

No additional dependencies needed.

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

### Running Tests

```bash
# Rust tests (database, coalescing, categorization, migrations)
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript tests (stats, time utilities)
npx vitest run

# Both
cargo test --manifest-path src-tauri/Cargo.toml && npx vitest run
```

### Dev Mode

```bash
npm run tauri dev
```

## Architecture

```
usage-os/
├── src/                        # React/TypeScript frontend
│   ├── components/
│   │   ├── ActivityChart.tsx    # Pie chart with top-N + "Other" bucket
│   │   ├── SettingsView.tsx     # Categories, rules, data retention
│   │   ├── StatsCard.tsx        # Duration/idle summary cards
│   │   └── TimeRangeSelector.tsx
│   ├── lib/
│   │   ├── stats.ts            # Duration calc, grouping, formatting
│   │   ├── time.ts             # Relative time formatting
│   │   ├── tauri.ts            # Tauri IPC bindings
│   │   └── utils.ts            # CN utility
│   └── App.tsx                 # Main dashboard layout
├── src-tauri/                  # Rust backend
│   └── src/
│       ├── db.rs               # SQLite, migrations, CRUD, coalescing logic
│       ├── watcher.rs          # Background window polling + idle detection
│       ├── lib.rs              # Tauri commands, app setup, data retention
│       └── main.rs             # Entry point
└── .github/workflows/ci.yml   # CI: test + build on Linux/macOS/Windows
```

### Key Design Decisions

- **Coalescing with 30s gap threshold**: If the same process/title/idle-state is active within 30s of the last entry, extend it instead of creating a new row. Prevents false inflation from app restarts.
- **Migration-based schema**: `schema_migrations` table tracks applied versions. Migrations run on startup, idempotent.
- **Category rules**: First matching rule wins (ordered by creation). Match on process name or window title, case-insensitive contains.

## Tech Stack

| Layer | Tech |
|-------|------|
| Framework | Tauri v2 |
| Backend | Rust, rusqlite (bundled SQLite) |
| Frontend | React 19, TypeScript, Vite 7 |
| Styling | Tailwind CSS, Radix UI |
| Charts | Recharts |
| Testing | `cargo test` (Rust), Vitest (TS) |

## Data Privacy

- 100% local storage — no cloud, no telemetry, no network requests
- SQLite database in your app data directory
- You own your data completely

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md).

## License

MIT
