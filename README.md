# UsageOS

A privacy-first desktop utility built with Tauri (Rust + React) that silently tracks computer usage activity and presents the data in a beautiful cyberpunk-styled dashboard.

## Features

- **Background Activity Tracking** - Polls active window every 5 seconds to track app usage
- **Idle Detection** - Automatically detects when you're away (3+ minutes of inactivity)
- **Smart Coalescing** - Updates duration instead of creating duplicate entries
- **Local SQLite Storage** - All data stays on your machine, stored in app data directory
- **Real-time Dashboard** - View today, yesterday, and weekly stats with live updates
- **Idle Time Visualization** - Track and visualize idle time separately from active usage
- **Cyberpunk UI** - Clean, professional dark mode interface with subtle neon accents

## Tech Stack

- **Tauri v2** - Cross-platform desktop framework
- **Rust** - Backend for window detection, idle tracking, and database operations
- **React + TypeScript** - Frontend dashboard with type safety
- **Tailwind CSS** - Utility-first styling
- **Shadcn UI** - Component foundation with heavy customization
- **Recharts** - Data visualization
- **SQLite** - Local time-series database

## Prerequisites

- **macOS**: Accessibility permissions required for window title detection
  - System Settings > Privacy & Security > Accessibility
- **Rust** (latest stable)
- **Node.js** (v18+)

## Installation

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Database Schema

Activity logs are stored in `~/Library/Application Support/com.favour.usage-os/usage.db` (macOS):

```sql
CREATE TABLE activity_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    process_name TEXT NOT NULL,
    window_title TEXT NOT NULL,
    start_time INTEGER NOT NULL,  -- Unix timestamp
    end_time INTEGER NOT NULL,    -- Unix timestamp
    is_idle INTEGER NOT NULL      -- Boolean (0 or 1)
);
```

## Architecture

- **Watcher** (`src-tauri/src/watcher.rs`) - Background thread polling active window
- **Database** (`src-tauri/src/db.rs`) - SQLite operations and coalescing logic
- **Dashboard** (`src/App.tsx`) - React UI with stats cards and activity charts
- **Stats** (`src/lib/stats.ts`) - Data processing and aggregation utilities

## Data Privacy

- 100% local storage - no cloud sync or telemetry
- No network requests for data storage
- All processing happens on your device
- You own your data completely

## Future Enhancements

See [02-FUTURE-IDEAS.md](02-FUTURE-IDEAS.md) for planned features:
- Categorization rules engine
- XP system and gamification
- Timeline view
- Export capabilities

## License

MIT
