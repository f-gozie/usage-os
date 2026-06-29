# Product Requirements Document (PRD): UsageOS (MVP Phase)

## 1. Project Goal
To build a personal desktop utility app using Tauri (Rust/React) that silently tracks computer usage activity in the background. The data is stored locally and presented in a beautiful, gamified dashboard to help the user understand where their time goes.

## 2. Target Audience
Personal use by the developer (me). The user wants ownership of their data, low system resource usage, and a UI that feels more like a video game stats screen than an Excel spreadsheet.

## 3. Scope (MVP - The "Walking Skeleton")
The MVP focuses strictly on the reliable acquisition and storage of data, and a basic visualization.

**IN SCOPE for MVP:**
- [Backend] Polling the active window every N seconds.
- [Backend] Detecting when the user is idle (no mouse/keyboard movement).
- [Backend] Local SQLite storage of activity logs.
- [Backend] Basic "coalescing" logic (if the app hasn't changed in the last check, update the duration rather than creating a new row).
- [Frontend] A React-based dashboard showing total time tracked today.
- [Frontend] A simple pie chart showing top 5 apps used "Today" and "Past Week".

**OUT OF SCOPE for MVP (Future):**
- Manual categorization of apps (e.g., telling the app that "VS Code" = "Productive").
- XP systems, streaks, or boss fights (gamification).
- Cloud sync or exporting data.
- Detailed timeline views (e.g., "what was I doing at 2:00 PM specifically").
- OS support outside of your primary development OS (assume macOS or Windows for now).

## 4. Functional Requirements

### 4.1 The Watcher (Rust Backend)
1.  The app must spawn a background thread on startup.
2.  This thread wakes up every 5 seconds (configurable interval).
3.  It must query the OS for the currently active window's **Process Name** (e.g., `code`, `chrome`) and **Window Title** (e.g., `my_project.rs`, `YouTube`).
4.  It must check an "idle timer". If the user hasn't moved mouse/keyboard for > 3 minutes, the current activity is logged as "Idle".
5.  It must communicate this data to the database handler.

### 4.2 The Database Handler (Rust Backend)
1.  Must use SQLite stored locally on the disk.
2.  On receiving data from the Watcher:
    - *If the last logged entry is the same app and title:* Update the `end_time` of that entry to now.
    - *If the app has changed:* Insert a new row with `start_time` as now.

### 4.3 The Dashboard (React Frontend)
1.  A single-page view.
2.  Must fetch data via Tauri Commands from the Rust backend.
3.  **Component A: Summary Cards:** Show Total active time today vs yesterday.
4.  **Component B: Distribution Chart:** A donut/pie chart showing the percentage breakdown of time spent per application name for the selected time range.

## 5. Non-Functional Requirements
1.  **Resource Usage:** The background Watcher must use negligible CPU (< 1%) when the dashboard is closed.
2.  **Data Privacy:** 100% local. No network calls for data storage.
3.  **OS Permissions:** Must gracefully handle initial OS requests for accessibility/screen recording permissions needed to read window titles.

## 6. Aesthetics & Design Nudges
* **Vibe:** "Cyberpunk utility," "Video game HUD," dark mode default.
* **Layout:** "Bento Box" grid layout for the dashboard statistics.
* **Colors:** High contrast neon accents on dark grey backgrounds.
* **Typography:** Monospaced fonts for numbers/data, clean sans-serif for labels.