# Security

## Supported Version

OpenJournal is currently pre-release. Security fixes should target `main` until a stable release line exists.

## Reporting a Vulnerability

Please open a private security advisory on the public repository when available, or email the maintainers listed in the repository profile. Include:

- OpenJournal version or commit.
- Operating system version.
- Reproduction steps.
- Expected and actual behavior.
- Whether local data exposure is possible.

Do not include private activity logs, passwords, API keys, or screenshots in public issues.

## Security Boundaries

OpenJournal should never implement:

- Keylogging.
- Typed-text capture.
- Password capture.
- Silent clipboard capture.
- Screenshots or screen recording in v0.1.
- Cloud sync in v0.1.
- External AI data sharing without explicit user configuration.

## API Key Security (v0.2.2+)

API keys are managed with the following priority and storage rules:

1. **Environment variables** — `OPENJOURNAL_DEEPSEEK_API_KEY` (highest priority) or `DEEPSEEK_API_KEY`
2. **OS credential manager** — Windows Credential Manager, macOS Keychain, Linux Secret Service
3. **Session override** — in-memory only, lost on app restart

Rules:
- API keys are **never** stored in the OpenJournal SQLite database.
- API keys are **never** included in exports (Markdown/JSON).
- API keys are **never** sent to the frontend in plaintext.
- API keys are **never** logged.
- The UI shows only a masked representation (`sk-••••••••abcd`).
- Manual key entry is saved to the OS credential manager, not to SQLite.
- Existing plaintext keys from earlier versions are migrated to secure storage on first launch.

## Dependency Hygiene

- Keep Tauri, Rust crates, and npm packages current.
- Review Tauri command permissions before each release.
- Keep the command API narrow and typed.
- Prefer local-only providers and local files.
- Treat export files as sensitive user data.

## Release Checklist

- Run frontend build.
- Run Rust tests and `cargo clippy`.
- Verify Windows active-window tracking.
- Verify pause/resume from app and tray.
- Verify blocklist skips records before storage.
- Verify delete-day removes activity and summaries.
- Verify installer signature and checksum.
