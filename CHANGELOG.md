# Changelog

All notable changes to OpenJournal will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-06-15

### Added

- Secure credential storage via OS Credential Manager / Keychain / Secret Service
- `credential` Rust module wrapping the `keyring` crate with cross-platform support
- API key source indicator in UI: Environment, Credential Manager, Session, or Missing
- `save_credential_api_key`, `delete_credential_api_key`, `get_api_key_status` Tauri commands
- Plaintext API key migration on startup (detect → move to credential store → delete)
- Masked API key display (`sk-••••••••abcd`) never exposing full keys
- SECURITY.md note on API key protection

### Changed

- **Security**: API keys are never stored in SQLite — stripped before `set_ai_config`
- `get_ai_config` now returns empty `api_key` field to frontend
- `generate_ai_summary` resolves API key from env → credential store → session
- AI Settings UI replaced API key input with source badge + save/remove/session-override actions
- Added `ai-note` explaining keys are never stored in the database
- Added key source CSS badges (env green, credential purple, session amber, missing red)
- Version bumped to 0.2.2 across all files
- `.gitignore` updated with release artifact patterns (`*.exe.sha256`, `*.exe`, `*.msi`)
- Cleaned tracked `OpenJournal_0.1.3_x64-setup.exe.sha256` from git
- Added `#[allow(dead_code)]` with doc comments on 4 reserved functions

### Fixed

- Plaintext API keys from earlier versions are migrated to secure storage on first launch
- Tests now save/restore environment variables to avoid cross-test contamination

## [0.2.1] - Unreleased

### Added

- DeepSeek as default AI provider (⭐ Recommended)
- Full AI Settings UI with provider dropdown (DeepSeek, LM Studio, Ollama, OpenAI-compatible)
- Environment variable detection (`OPENJOURNAL_DEEPSEEK_API_KEY`, `DEEPSEEK_API_KEY`)
- Masked API key display in UI (sk-••••••••abcd)
- External provider privacy warning modal with explicit consent
- Summary cards for each 3-hour block with status (disabled/pending/completed/failed)
- Manual controls: generate, regenerate, delete per block
- Provider badges (Local / External) and status indicators
- `get_environment_provider_status` and `get_masked_api_key` Tauri commands
- Toggle switch for enabling/disabling AI
- Provider presets with automatic URL/model population
- Empty states for all AI configurations

### Changed

- Frontend `APP_VERSION` bumped to 0.2.1
- `AiConfig` defaults changed to DeepSeek provider
- Backend `create_provider` now supports `deepseek` provider type
- Removed unused `SummaryBlock` type from frontend
- Separated summary display from placeholder generation

## [0.2.0] - Unreleased

### Added

- AI summary provider abstraction with LM Studio and Ollama support
- AI settings page (configurable via Tauri commands)
- `ai_summaries` database table for storing generated summaries
- `ai_config` database table for persistent AI provider configuration
- Structured prompt builder for 3-hour activity blocks
- Provider test-connection command
- Manual summary generation, regenerate, and delete controls
- Background generation with 30s timeout, 2-retry limit
- Local-first design: all data stays on device; providers are opt-in

### Changed

- Rewrote `summarizer.rs` with block aggregation and prompt builder
- Updated `storage.rs` with AI config and summary CRUD
- Updated `lib.rs` with 6 new AI-specific Tauri commands
- Added `reqwest` dependency for provider HTTP calls

## [0.1.3] - 2026-06-15

### Changed

- Bumped version to 0.1.3 for first public release
- Added repository documentation: CODE_OF_CONDUCT, CONTRIBUTING, SUPPORT, CHANGELOG
- Added issue templates (bug report + feature request) and PR template
- Rewrote README with public release polish
- Updated verification checklist with full QA results

## [0.1.2] - 2026-06-15

### Changed

- Aligned all version fields to 0.1.2 (package.json, Cargo.toml, tauri.conf.json, APP_VERSION)
- Built and installed NSIS installer (`OpenJournal_0.1.2_x64-setup.exe`)
- Verified installed-app runtime: active-window logging confirmed (27 real entries), exports verified, delete-day confirmed

### Fixed

- Version consistency across all source files (package.json was still at 0.0.0 scaffold default)

## [0.1.1] - 2026-06-15

### Added

- First-run privacy modal with localStorage dismissal
- About panel showing app version, database path, data model, and privacy info
- Active-window tooltip on status badge

### Changed

- README verification checklist section
- `.gitignore` — added SQLite database patterns
- Version consistency verification across all files

## [0.1.0] - 2026-06-14

### Added

- Tauri + React + TypeScript desktop UI scaffold
- Windows active-window tracking in Rust through Win32 APIs
- Local SQLite schema and storage layer (WAL mode, foreign keys)
- System tray icon with Show / Pause/Resume logging / Quit menu
- Daily timeline UI with time, duration, app, and window title columns
- Markdown export command (timeline table format)
- JSON export command (activity + summary bundle)
- Delete-day command with confirmation dialog
- Private app/domain blocklist (filtered before storage)
- Placeholder summarization module (v0.2-ready)
- Privacy-first documentation (PRIVACY.md, SECURITY.md, ROADMAP.md)
- Pause/resume logging with persistent state
- Architecture documentation (ARCHITECTURE.md, INSTALLER_PLAN.md)
