# Windows Installer Plan

OpenJournal should ship as a signed Windows installer built with Tauri's NSIS target.

## v0.1 Packaging

1. Install Windows prerequisites:
   - Rust stable.
   - Microsoft Visual Studio C++ build tools.
   - WebView2 runtime.
   - Node.js 22+.
2. Build the frontend:
   ```powershell
   npm run build
   ```
3. Build the desktop app:
   ```powershell
   npm run tauri:build
   ```
4. Confirm the installer is produced under `src-tauri\target\release\bundle\nsis`.

## Signing

1. Acquire an Authenticode code-signing certificate.
2. Configure Tauri signing environment variables in CI.
3. Sign the installer and executable.
4. Publish SHA-256 checksums with each release.

## Release Verification

- Fresh install on Windows 10 and Windows 11.
- Launches into system tray.
- Pause/resume works from tray and UI.
- SQLite database is created in the local app data directory.
- Activity logs include app name, window title, start, end, and duration.
- Blocklisted patterns are skipped before storage.
- Markdown and JSON export files are created.
- Delete-day removes the selected day's logs.
- No network requests occur unless a provider is explicitly configured in a future release.

## CI Plan

- Run `npm ci`.
- Run `npm run build`.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo test`.
- Build unsigned artifacts for pull requests.
- Build signed release artifacts only from protected tags.
