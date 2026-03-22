# Contributing to Usage OS

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

### Prerequisites

- **Rust** (stable, latest recommended)
- **Node.js** 22+
- **npm** 10+
- Platform-specific dependencies (see below)

### macOS

No extra system dependencies needed. Grant Accessibility permissions when prompted.

### Windows

No extra system dependencies needed.

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

## Getting Started

```bash
# Clone the repo
git clone https://github.com/f-gozie/usage-os.git
cd usage-os

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml
npx vitest run
```

## Project Structure

```
usage-os/
├── src/                    # React/TypeScript frontend
│   ├── components/         # UI components
│   ├── lib/                # Utility functions (stats, time, tauri IPC)
│   └── __mocks__/          # Test mocks
├── src-tauri/              # Rust backend
│   └── src/
│       ├── db.rs           # Database, migrations, CRUD, coalescing
│       ├── watcher.rs      # Background activity polling
│       ├── lib.rs          # Tauri commands and app setup
│       └── main.rs         # Entry point
├── .github/workflows/      # CI configuration
└── landing/                # Landing page
```

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat(scope): description` — new features
- `fix(scope): description` — bug fixes
- `test(scope): description` — adding or fixing tests
- `refactor(scope): description` — code restructuring
- `chore(scope): description` — maintenance tasks
- `docs(scope): description` — documentation changes

## Pull Requests

1. Create a feature branch from `main`
2. Make your changes
3. Ensure all tests pass: `cargo test && npx vitest run`
4. Submit a PR with a clear description

## Code Style

- **Rust**: Follow standard `rustfmt` conventions
- **TypeScript**: Follow existing patterns in the codebase
- **Tests**: Write tests for new functionality

## Reporting Issues

Open an issue with:
- OS and version
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs (check the console output)
