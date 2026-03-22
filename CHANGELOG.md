# Changelog

All notable changes to Usage OS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive Rust test suite (24 tests): coalescing logic, category matching, CRUD, migrations, cleanup
- TypeScript test suite (28 tests): stats calculations, time formatting, grouping, color generation
- Migration-based schema management with `schema_migrations` table
- Data retention settings (automatic cleanup of old activity logs)
- Settings UI with data retention dropdown
- CI pipeline (GitHub Actions) for Linux, macOS, and Windows
- CONTRIBUTING.md with setup instructions
- This changelog

### Changed
- Refactored `init_database` from ad-hoc schema creation to versioned migrations
- Chart top-N now configurable (default increased from 5 to 8)

## [0.1.0] - 2026-03-21

### Added
- Initial release
- Background activity tracking with 5s polling interval
- Idle detection (3-minute threshold)
- Smart coalescing with 30s gap threshold
- SQLite local storage
- Category rules engine (process name / window title matching)
- Reprocess logs functionality
- Dashboard with pie chart, stats cards, time range selector
- Today / Yesterday / Week views
- Cyberpunk-themed dark mode UI
- Landing page
