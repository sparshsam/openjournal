# OpenJournal — Claude Code Instructions

## Overview

Privacy-first local activity journal for Windows. Tauri 2 desktop app with React + TypeScript frontend and Rust backend.

## Commands

```bash
npm install          # Install frontend dependencies
npm run dev          # Vite dev server
npm run build        # Build frontend
npm run tauri:dev    # Tauri dev (desktop app with hot-reload)
npm run tauri:build  # Build release installer
cd src-tauri && cargo check          # Rust type checking
cd src-tauri && cargo clippy --all-targets -- -D warnings  # Lint
cd src-tauri && cargo test           # Run tests
cd src-tauri && cargo fmt -- --check # Format check
```

## Rules

1. Privacy-first: no telemetry, analytics, or cloud sync. External AI providers must be opt-in.
2. Local-first: all data stays on the user's device.
3. API keys: never stored in SQLite — use env vars or OS credential manager.
4. Rust quality gates must pass before committing.
5. No direct pushes to master. Open a PR.
6. Read ARCHITECTURE.md and PRIVACY.md before making architectural changes.
