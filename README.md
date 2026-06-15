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

The React UI builds in this environment. Rust is not installed in this workspace, so the Tauri shell was scaffolded but not compiled here.

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

## License

AGPL-3.0-or-later. See [LICENSE](./LICENSE).
