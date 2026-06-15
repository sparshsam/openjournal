# Privacy

OpenJournal is designed to be local-first and private by default.

## What v0.1 Collects

- Active application name.
- Active window title.
- Focus start time.
- Focus end time.
- Focus duration.

## What v0.1 Does Not Collect

- Keystrokes.
- Typed text.
- Passwords.
- Clipboard contents by default.
- Screenshots.
- Screen recordings.
- Browser page contents beyond the active window title.
- Cloud account identifiers.

## Storage

All activity logs are stored locally in SQLite on the user's machine. There is no cloud sync in v0.1.

## Blocklist

The blocklist is applied before activity is written to SQLite. If an app name, domain, or title fragment matches the blocklist, that focused window is skipped.

Suggested blocklist entries include password managers, banking domains, private browsers, medical portals, and messaging apps.

## AI Summaries

v0.1 includes only a placeholder summarization service. No activity data is sent to any model provider.

v0.2 should support a local provider first, such as LM Studio through an OpenAI-compatible endpoint. External providers must be disabled unless the user explicitly configures one and accepts a clear data-sharing notice.

## User Controls

- Pause or resume logging at any time.
- Block private apps, domains, and title fragments.
- Export a day to Markdown or JSON.
- Delete any day's logs.
