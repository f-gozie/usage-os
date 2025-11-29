# Technical Context & Architecture Plan

## Tech Stack
* **Core framework:** Tauri v2 (Beta) or v1. (Prefer v2 if stable enough, otherwise v1).
* **Backend Language:** Rust.
* **Frontend Framework:** React (Vite template).
* **Styling:** Tailwind CSS (essential for rapid UI dev).
* **UI Components:** Shadcn UI (optional, but good for nice-looking base components).
* **Charting Library:** Recharts (excellent React wrapper for D3).
* **Database:** SQLite via `rusqlite` crate.

## Database Schema (Proposed)
We need a simple time-series structure.

Table: `activity_logs`
| Column Name | Data Type | Description |
| :--- | :--- | :--- |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT | |
| `process_name` | TEXT | The executable name (e.g., "Code.exe", "Firefox") |
| `window_title` | TEXT | The specific tab or file open (can be identifying PII, handle carefully) |
| `start_time` | DATETIME/INTEGER | Unix timestamp of when this block started |
| `end_time` | DATETIME/INTEGER | Unix timestamp of when it ended. Updated constantly until app switch. |
| `is_idle` | BOOLEAN | True if this block represents AFK time. |

## Crucial Rust Crates (Research findings)
Cursor, we will likely need these crates to achieve the functionality:

1.  **Window Detection:** `active-win-pos-rs` (cross-platform, seems reliable).
2.  **Idle Detection:** `user-idle-time` (checks system inputs).
3.  **Database:** `rusqlite` (standard SQLite bindings) + `r2d2` (connection pooling, might be overkill for MVP but good practice) OR `tauri-plugin-sql` (easier integration). Let's start with `rusqlite` for direct control.
4.  **Async runtime:** `tokio` (Tauri uses this internally anyway).

## Architecture Diagram (Mental Model)

[OS Window Manager / Input Events]
       | (polls every 5s)
       v
[Rust Background Thread (The Watcher)]
       | (Detects change vs previous state)
       v
[Rust Database Handler (rusqlite)] -> [usage.db File]
       ^
       | (Tauri Command: fetch_stats)
       v
[Tauri Main Process]
       | (IPC)
[React Frontend (Dashboard / Recharts)]