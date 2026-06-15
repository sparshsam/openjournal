# Privacy

OpenJournal is designed to be local-first and private by default.

## What OpenJournal Records

- Active application name (e.g., `Code.exe`, `WindowsTerminal.exe`).
- Active window title (e.g., "OpenJournal - lib.rs").
- Focus start time.
- Focus end time.
- Focus duration.

## What OpenJournal Does NOT Collect

- Keystrokes.
- Typed text.
- Passwords.
- Clipboard contents.
- Screenshots.
- Screen recordings.
- Browser page contents beyond the active window title.
- Cloud account identifiers.
- Microphone or camera input.

## Storage

All activity logs are stored locally in SQLite on the user's machine. There is no cloud sync.

## Blocklist

The blocklist is applied before activity is written to SQLite. If an app name, domain, or title fragment matches the blocklist, that focused window is skipped.

Suggested blocklist entries include password managers, banking domains, private browsers, medical portals, and messaging apps.

## AI Summaries (v0.2)

v0.2 introduces optional AI-powered 3-hour summaries. Key privacy guarantees:

- **AI is disabled by default.** No data is sent to any provider unless you explicitly enable AI and configure a provider.
- **Local providers first.** LM Studio and Ollama run entirely on your machine. No data leaves your device.
- **External providers require explicit consent.** OpenAI-compatible providers are scaffolded but disabled by default. You must opt in and configure them manually.
- **Provider data flow:** When you click "Generate summary," the 3-hour block's aggregated metadata (app names, window titles, durations) is sent to your configured provider via its API. The full prompt is shown in the app before sending.
- **No telemetry.** OpenJournal never sends usage data, crash reports, or analytics anywhere.

## User Controls

- Pause or resume logging at any time.
- Block private apps, domains, and title fragments.
- Export a day to Markdown or JSON.
- Delete any day's logs.
- Enable/disable AI summaries in Settings.
- Choose which provider to use (LM Studio, Ollama, or OpenAI-compatible).
- Test provider connection before generating.
- Generate, regenerate, or delete individual summary blocks.
