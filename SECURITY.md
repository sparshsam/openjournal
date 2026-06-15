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
