# OpenJournal

<div align="center">

**Privacy-first local activity journal for Windows.**

[![License](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.96+-orange.svg)]()
[![Tauri](https://img.shields.io/badge/Tauri-2-green.svg)]()
[![Platform](https://img.shields.io/badge/Platform-Windows_10_|_11-lightgrey.svg)]()

![Hero screenshot placeholder](./src/assets/hero.png)

</div>

---

## Features

- **Automatic activity tracking** — Records focused app names, window titles, and duration locally
- **System tray integration** — Show window, pause/resume logging, or quit from the tray icon
- **Daily timeline** — Browse your day's focused windows with start/end times and durations
- **Private blocklist** — Skip sensitive apps, domains, or title patterns before anything is stored
- **Pause/resume logging** — One-click pause from the app or system tray
- **Markdown & JSON export** — Export your day's activity as a formatted timeline or structured data
- **Delete day** — Remove all activity for a selected day
- **Privacy-first from the ground up** — No cloud, no telemetry, no data leaving your device

## Privacy-First Principles

OpenJournal is built on a simple promise: **everything stays on your device.**

### What OpenJournal records

- The **name** of the focused application (e.g., `Code.exe`, `WindowsTerminal.exe`)
- The **title** of the focused window (e.g., "OpenJournal - lib.rs")
- When focus **started** and **ended**
- The calculated **duration** of each focus session

### What OpenJournal does NOT collect

| Feature | Status |
|---------|--------|
| Keystrokes | ❌ Never recorded |
| Clipboard contents | ❌ Never read |
| Passwords or typed text | ❌ Never recorded |
| Screenshots or screen recordings | ❌ Never captured |
| Microphone input | ❌ Never accessed |
| Camera input | ❌ Never accessed |
| Cloud data / network sync | ❌ Never sent |
| Telemetry or analytics | ❌ Never collected |
| External API calls | ❌ None in v0.1 (opt-in only in future versions) |
| Keylogging of any kind | ❌ Explicitly prevented |

### Data storage

All data is stored in a local SQLite database under the Tauri app data directory:

```
%APPDATA%\dev.openjournal.app\openjournal.sqlite3
```

## Installation

### Download

Download the latest installer from the [Releases page](https://github.com/sparshsam/openjournal/releases).

1. Download `OpenJournal_v0.1.3_x64-setup.exe`
2. Run the installer
3. Launch OpenJournal from the Start menu

### Build from source

```powershell
# Prerequisites
# - Windows 10 or Windows 11
# - Node.js 22+
# - Rust 1.96+ and Cargo
# - Visual Studio C++ build tools
# - WebView2 (included in Windows 10+)

git clone https://github.com/sparshsam/openjournal.git
cd openjournal
npm install
npm run tauri:build
```

## How Activity Tracking Works

OpenJournal uses the Windows Win32 API to monitor the foreground window.

1. Every **5 seconds**, OpenJournal calls `GetForegroundWindow()` to find the active window
2. It retrieves the **window title** via `GetWindowTextW()` and the **process image name** via `QueryFullProcessImageNameW()`
3. If the window or app matches a blocklist pattern, the activity is **discarded before storage**
4. If logging is **paused**, the current session is flushed and no new entries are created
5. If the window **changes**, the previous session is closed and a new one starts
6. Durations are calculated as `ended_at - started_at` and stored in seconds

No data is ever sent over the network.

## Export Examples

### Markdown

```markdown
| Start | End | Duration | App | Window |
| --- | --- | ---: | --- | --- |
| 2026-06-15T09:00:00Z | 2026-06-15T09:39:00Z | 2340s | Code.exe | openjournal - lib.rs |
| 2026-06-15T09:42:00Z | 2026-06-15T09:57:00Z | 900s | msedge.exe | Tauri system tray docs |
```

### JSON

```json
{
  "day": "2026-06-15",
  "activities": [
    {
      "id": 1,
      "app_name": "Code.exe",
      "window_title": "openjournal - lib.rs",
      "started_at": "2026-06-15T09:00:00Z",
      "ended_at": "2026-06-15T09:39:00Z",
      "duration_seconds": 2340
    }
  ],
  "summaries": []
}
```

## Data Model

| Table | Description |
|-------|-------------|
| `activity_entries` | Focused app/window sessions with start, end, and duration |
| `blocklist_entries` | Private app, domain, and title patterns to skip before storage |
| `settings` | Local app settings (e.g., paused logging state) |
| `summary_blocks` | Reserved for v0.2 generated AI summaries |

## Development

```powershell
# Install dependencies
npm install

# Run the web dev server (browser preview)
npm run dev

# Build the frontend
npm run build

# Run Tauri dev (desktop app with hot-reload)
npm run tauri:dev

# Build release installer
npm run tauri:build

# Rust quality checks
cd src-tauri
cargo fmt -- --check
cargo check
cargo clippy --all-targets
cargo test
```

## Roadmap

| Version | Focus |
|---------|-------|
| **v0.1** ✅ | Core architecture, local tracking, exports, privacy controls |
| **v0.2** 🚧 | **Local AI summaries** — LM Studio/Ollama provider, prompt builder, AI settings, manual generation controls |
| **v0.3** 🔜 | Multi-day views, calendar navigation, search |
| **v0.4** 🔜 | Optional encrypted backup / restore |

## AI Summaries (v0.2)

OpenJournal v0.2 introduces optional AI-powered 3-hour summaries. AI is **disabled by default** — no data is sent anywhere unless you explicitly enable it.

### Supported Providers

| Provider | Type | Default | Status |
|----------|------|---------|--------|
| **LM Studio** | Local (HTTP) | `http://localhost:1234/v1` | ✅ Implemented |
| **Ollama** | Local (HTTP) | `http://localhost:11434` | ✅ Implemented |
| **OpenAI-compatible** | Remote (opt-in) | — | Scaffolded, disabled by default |

### How It Works

1. Activity is aggregated into **8 x 3-hour blocks** per day
2. Each block's data (app names, durations, context switches) is combined into a structured prompt
3. The prompt is sent to your configured local provider
4. The AI returns a structured JSON summary (main focus, apps, context switches, notes, plain-English summary)
5. The summary is stored locally in SQLite

### Privacy

- ✅ **AI is off by default.** OpenJournal works exactly as before when no provider is configured.
- ✅ **LM Studio and Ollama run locally.** No data leaves your machine.
- ✅ **External providers are opt-in.** You must explicitly enable and configure them.
- ✅ **No telemetry.** OpenJournal never sends usage data.
- ✅ **All prompts are generated from local data only.** App names, window titles, and durations — no keystrokes, clipboard, or passwords.

### Setup

1. Install [LM Studio](https://lmstudio.ai/) or [Ollama](https://ollama.com/)
2. Load a model (e.g., `llama3.2` for Ollama, or any model in LM Studio)
3. In OpenJournal, go to **AI Settings**
4. Enable AI, select your provider, and enter the base URL
5. Click **Test Connection** to verify
6. Click **Generate Summary** on any 3-hour block

### DeepSeek Setup (Default)

OpenJournal ships with DeepSeek as the default AI provider. To use it:

1. Get an API key from [platform.deepseek.com](https://platform.deepseek.com/)
2. Set it as an environment variable:

```bash
# PowerShell
$env:OPENJOURNAL_DEEPSEEK_API_KEY = "sk-your-key-here"

# CMD
set OPENJOURNAL_DEEPSEEK_API_KEY=sk-your-key-here
```

Or use the standard `DEEPSEEK_API_KEY` variable.

3. Restart OpenJournal
4. Go to **AI Settings** and enable AI
5. The "Using env:" indicator should show a masked key
6. Click **Test Connection** to verify

### Provider Endpoints

| Provider | Default URL | API Format |
|----------|-------------|------------|
| LM Studio | `http://localhost:1234/v1` | OpenAI-compatible `/v1/chat/completions` |
| Ollama | `http://localhost:11434` | Ollama `/api/chat` |

See [ROADMAP.md](./ROADMAP.md) for the full plan.

## License

AGPL-3.0-or-later. See [LICENSE](./LICENSE).

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
