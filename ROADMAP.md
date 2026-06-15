# OpenJournal Roadmap

## v0.1 ✅ (Released)

- Windows desktop app with Tauri.
- System tray app lifecycle.
- Active app and window-title logging.
- Start, end, and duration tracking per focused window.
- Local SQLite storage.
- Pause and resume logging.
- Private app/domain/title blocklist.
- Daily timeline page.
- Markdown and JSON export.
- Delete a day's logs.
- In-app privacy notice.
- No screenshots, keylogging, clipboard capture, typed-text capture, password capture, or cloud sync.

## v0.2 (In Progress)

### Completed

- ✅ Provider abstraction (`SummaryProvider` trait) with LM Studio, Ollama, and OpenAI-compatible implementations
- ✅ `ai_summaries` database table with status tracking (pending, completed, failed)
- ✅ `ai_config` table for persistent provider configuration
- ✅ Structured prompt builder for 3-hour activity blocks
- ✅ Block aggregation with total focus time, app breakdown, context switches, idle periods
- ✅ Provider test-connection command
- ✅ Manual generation, regenerate, and delete controls via Tauri commands
- ✅ Background generation with 30s timeout and 2-retry limit

### Remaining

- Frontend AI settings page (provider config, test button)
- Summary display UI with generation status indicators
- Warning modal for external providers
- Documentation updates for provider setup
- Full end-to-end testing with local provider

## v0.3 🔜

- Search and filters.
- Tags and manual annotations.
- Weekly review view.
- Local encrypted database option.
- App icon and signed Windows installer.
- Automatic update channel for signed releases.

## Future

- Optional external provider integrations with explicit consent.
- Import/export backup workflow.
- Advanced retention rules.
- Cross-platform active-window tracker implementations.
