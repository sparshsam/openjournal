# Changelog

All notable changes to OpenJournal will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
