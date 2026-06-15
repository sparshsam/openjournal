# Contributing to OpenJournal

Thank you for your interest in OpenJournal! This project is privacy-first, local-first, and community-driven.

## How to Contribute

### Reporting Bugs

Open an issue using the **Bug Report** template. Include:

- OpenJournal version
- Windows version (10 or 11)
- Steps to reproduce
- Expected vs actual behavior
- Screenshots if applicable

### Requesting Features

Open an issue using the **Feature Request** template. Include:

- What problem you're trying to solve
- How you envision the feature working
- Whether it aligns with OpenJournal's privacy-first principles

### Pull Requests

1. Fork the repository.
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Make your changes.
4. Run all quality checks:
   ```bash
   npm install
   npm run build
   cd src-tauri
   cargo fmt -- --check
   cargo check
   cargo clippy --all-targets
   cargo test
   ```
5. Commit with a clear message.
6. Push and open a pull request.

### Development Setup

See [README.md](./README.md#development) for setup instructions.

## Guidelines

- **Privacy first**: No telemetry, analytics, or cloud sync. External APIs must be opt-in.
- **Local by default**: All data stays on the user's device.
- **No breaking changes**: v0.1.x is feature-frozen except for release-critical fixes.
- **Tests required**: New features must include tests.
- **AGPL-3.0**: All contributions are licensed under AGPL-3.0-or-later.

## Code of Conduct

Please read and follow our [Code of Conduct](./CODE_OF_CONDUCT.md).
