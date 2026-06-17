# OpenJournal — AI Agent Instructions

## Purpose

Privacy-first local activity journal for Windows. Tauri + React + TypeScript desktop app with Rust backend for Windows active-window tracking.

## Rules

1. **Privacy first.** Never introduce telemetry, analytics, cloud sync, or network calls without explicit user consent.
2. **Local by default.** All data stays on the user's device. External AI providers must be opt-in.
3. **AGPL-3.0.** All contributions are licensed under AGPL-3.0-or-later.
4. **Branch naming:** `feat/*`, `fix/*`, `docs/*`, `refactor/*`, `chore/*`.
5. **No direct pushes to master.** Open a PR for all changes.
6. **Rust quality gates:** `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -D warnings`, `cargo test` must pass.
7. **No API keys in SQLite.** Keys go to environment variables or OS credential manager.
8. **Architecture first.** Read ARCHITECTURE.md and the relevant source files before making changes.

## Key Files

| Path | Description |
|------|-------------|
| `src/` | React + TypeScript frontend |
| `src-tauri/src/` | Rust backend (tracking, storage, export, summarizer) |
| `ARCHITECTURE.md` | System design overview |
| `PRIVACY.md` | Privacy model documentation |
| `INSTALLER_PLAN.md` | Windows installer and signing plan |
