# Architecture

## Layers

- `src/`: React + TypeScript desktop UI.
- `src-tauri/src/activity_tracker.rs`: Windows active-window polling and pause/blocklist enforcement.
- `src-tauri/src/storage.rs`: SQLite migration and local persistence.
- `src-tauri/src/export.rs`: Markdown and JSON exports.
- `src-tauri/src/summarizer.rs`: v0.2-ready summary provider interface and placeholder 3-hour grouping.

## Privacy Flow

1. Poll foreground window metadata through Win32 APIs.
2. If logging is paused, flush the current focus record and store nothing new.
3. Compare app name and window title against the local blocklist.
4. If blocked, skip storage.
5. If allowed, merge contiguous focus time for the same app/title.
6. Persist only app name, window title, timestamps, and duration.

## Summary Flow

1. Load one day of activity from SQLite.
2. Split activity into 3-hour blocks.
3. Build summary payloads with the `SummaryProvider` trait.
4. Use placeholder summaries in v0.1.
5. Add LM Studio/OpenAI-compatible local provider in v0.2.

External providers must remain disabled until the user explicitly configures them.
