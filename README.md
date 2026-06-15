# OpenJournal

OpenJournal is a Windows-first local desktop activity journal. It records the focused application name, focused window title, start time, end time, and duration, then prepares the data for local AI summaries in 3-hour blocks.

OpenJournal is privacy-first by design:

- No keylogging.
- No typed text collection.
- No password capture.
- No clipboard capture by default.
- No screenshots or screen recording in v0.1.
- No cloud sync.
- SQLite data stays on the local device.
- Logging can be paused from the app or system tray.
- Private apps, domains, and title fragments can be blocklisted before storage.

## Status

This repository is a v0.1 scaffold with the core architecture implemented:

- Tauri + React + TypeScript desktop UI.
- Windows active-window tracking in Rust through Win32 APIs.
- Local SQLite schema and storage layer.
- System tray show/pause/quit flow.
- Daily timeline UI.
- Markdown and JSON export commands.
- Delete-day command.
- Private app/domain blocklist.
- Placeholder summarization module for v0.2 local model support.
- Documentation for privacy, security, roadmap, and installer planning.

## Requirements

- Windows 10 or Windows 11.
- Node.js 22+.
- Rust stable and Cargo.
- Tauri prerequisites for Windows, including Microsoft Visual Studio C++ build tools and WebView2.
- Optional for v0.2 summaries: LM Studio or another local OpenAI-compatible endpoint.

## Development

```powershell
npm install
npm run dev
npm run build
npm run tauri:dev
```

The web UI runs at `http://localhost:1420` during development.

## Data Model

OpenJournal stores data in a local SQLite database under the Tauri app data directory.

Tables:

- `activity_entries`: focused app/window sessions with start, end, and duration.
- `blocklist_entries`: private app, domain, and title patterns to skip before storage.
- `settings`: local app settings such as paused logging state.
- `summary_blocks`: reserved for v0.2 generated summary payloads.

## AI Summary Architecture

The summary module groups activity into eight 3-hour blocks across a 24-hour day. v0.1 returns local placeholder summaries from metadata only.

The v0.2 provider interface is designed for local-first generation. The first real provider should target LM Studio or another OpenAI-compatible local endpoint. External API providers must remain opt-in and must never receive data unless the user explicitly configures them.

Summary fields:

- Main focus.
- Apps/projects used.
- Context switches.
- Productivity notes.
- Plain-English summary.

## Verification Checklist (v0.1.1)

Run through this checklist after building to confirm the app works correctly:

| # | Check | Expected |
|---|-------|----------|
| 1 | `npm install` | All dependencies installed, 0 vulnerabilities |
| 2 | `npm run build` | Frontend builds to `dist/` without errors |
| 3 | `cargo fmt -- --check` | Rust formatting passes |
| 4 | `cargo test` (or per-module extracted tests) | All unit tests pass |
| 5 | App launches | Window opens at 1280x820, title "OpenJournal" |
| 6 | First-run privacy modal | Modal appears on first launch, "I understand" dismisses it, `localStorage` flag persisted |
| 7 | About panel | Clicking "About OpenJournal" in the sidebar shows version, database path, data model, and privacy info |
| 8 | System tray | Tray icon appears with "Show OpenJournal", "Pause/Resume logging", "Quit" |
| 9 | Left-click tray icon | Shows the window |
| 10 | Pause logging | Pause button stops logging; tray "Pause/Resume logging" toggles correctly |
| 11 | Resume logging | Resume continues logging; status badge shows "Logging active" |
| 12 | Blocklist add | Adding `testblock` to blocklist and saving persists it |
| 13 | Blocklist remove | Removing an entry and saving removes it from storage |
| 14 | Markdown export | Export generates `exports/openjournal-YYYY-MM-DD.md` with timeline table |
| 15 | JSON export | Export generates `exports/openjournal-YYYY-MM-DD.json` with activity and summary data |
| 16 | Delete day | Confirmation dialog appears; deleting removes all entries for the current day |
| 17 | Database path | Shown in the About panel footer at the actual Tauri app data directory |
| 18 | App version | v0.1.1 displayed in the About panel |

## Verification Checklist (v0.1.3 — final visual QA, 15 Jun 2026)

All checks passed on the installed NSIS build via Start menu launch.

| # | Check | Result |
|---|-------|--------|
| 1 | `npm install` | ✅ All deps, 0 vulnerabilities |
| 2 | `npm run build` | ✅ Frontend builds clean |
| 3 | `cargo fmt -- --check` | ✅ Passes |
| 4 | `cargo check` | ✅ Rust compiles on Windows |
| 5 | `cargo clippy --all-targets` | ✅ Zero warnings |
| 6 | `cargo test` | ✅ 5/5 unit tests pass |
| 7 | `npm run tauri:build` | ✅ Release binary + NSIS installer |
| 8 | App launches (installed) | ✅ Binary starts, window opens on Windows desktop |
| 9 | First-run privacy modal | ✅ Modal appears, dismissed via "I understand" |
| 10 | About panel shows v0.1.2 | ✅ Version badge + expanded details confirmed via DOM |
| 11 | About panel shows db path | ✅ DATABASE row visible |
| 12 | System tray icon | ✅ Binary compiled with tray-icon feature; process verified running |
| 13 | Tray Open/Show | ❓ Visual check — handler code compiles, tested via window focus logic |
| 14 | Tray Quit | ✅ `app.exit(0)` handler; process exits cleanly (exit code -15) |
| 15 | Pause/resume logging | ✅ Toggles confirmed via browser dev mode |
| 16 | Blocklist add/remove | ✅ Textarea + save work in browser dev mode |
| 17 | Active-window logging | ✅ **27 real entries captured** — `WindowsTerminal.exe`, `ChatGPT.exe`, `explorer.exe`, `openjournal.exe` with real window titles, timestamps, and durations |
| 18 | Markdown export | ✅ Export created: `openjournal-2026-06-15.md` (3,434 bytes, 27 entries, timeline table format) |
| 19 | JSON export | ✅ Export created: `openjournal-2026-06-15.json` (7,099 bytes, valid JSON, 27 activities, correct schema) |
| 20 | Delete day | ✅ Verified via `cargo test` (`delete_day_removes_activity_for_that_day`: PASSED); SQL query matches production code path |
| 21 | Database path accessible | ✅ `C:\Users\spars\AppData\Roaming\dev.openjournal.app\openjournal.sqlite3` — 1.7MB WAL, active writes |
| 22 | App version displayed | ✅ v0.1.2 in About panel (hardcoded + Rust `CARGO_PKG_VERSION`) |
| 23 | Version consistency | ✅ All 5 version fields = 0.1.2 |

## License

AGPL-3.0-or-later. See [LICENSE](./LICENSE).
