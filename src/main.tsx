import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  Activity,
  Ban,
  Brain,
  CalendarDays,
  Clock3,
  Download,
  ExternalLink,
  Info,
  Pause,
  Play,
  ShieldCheck,
  Trash2,
  X,
} from 'lucide-react';
import './style.css';

const APP_VERSION = '0.1.2';

type ActivityEntry = {
  id: number;
  app_name: string;
  window_title: string;
  started_at: string;
  ended_at: string | null;
  duration_seconds: number;
};

type SummaryBlock = {
  block_start: string;
  block_end: string;
  main_focus: string;
  apps_projects: string[];
  context_switches: number;
  productivity_notes: string[];
  plain_english_summary: string;
  provider: string;
};

type AppStatus = {
  logging_paused: boolean;
  active_window: string;
  db_path: string;
};

const sampleActivities: ActivityEntry[] = [
  {
    id: 1,
    app_name: 'Visual Studio Code',
    window_title: 'openjournal - activity_tracker.rs',
    started_at: new Date().toISOString(),
    ended_at: new Date(Date.now() + 39 * 60 * 1000).toISOString(),
    duration_seconds: 2340,
  },
  {
    id: 2,
    app_name: 'Microsoft Edge',
    window_title: 'Tauri system tray docs',
    started_at: new Date(Date.now() + 42 * 60 * 1000).toISOString(),
    ended_at: new Date(Date.now() + 57 * 60 * 1000).toISOString(),
    duration_seconds: 900,
  },
  {
    id: 3,
    app_name: 'Windows Terminal',
    window_title: 'npm run tauri dev',
    started_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
    ended_at: new Date(Date.now() + 81 * 60 * 1000).toISOString(),
    duration_seconds: 1260,
  },
];

const sampleSummaries: SummaryBlock[] = [
  {
    block_start: '09:00',
    block_end: '12:00',
    main_focus: 'Building the OpenJournal local tracker foundation',
    apps_projects: ['VS Code', 'Terminal', 'Edge'],
    context_switches: 5,
    productivity_notes: ['Mostly focused engineering work', 'A few documentation checks'],
    plain_english_summary:
      'You spent the morning wiring the local desktop app and validating the Windows tracking approach.',
    provider: 'placeholder',
  },
  {
    block_start: '12:00',
    block_end: '15:00',
    main_focus: 'No summary generated yet',
    apps_projects: [],
    context_switches: 0,
    productivity_notes: ['Configure LM Studio to generate local summaries.'],
    plain_english_summary: 'OpenJournal will summarize this block locally when a provider is enabled.',
    provider: 'not configured',
  },
];

const canUseTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!canUseTauri) throw new Error('Tauri runtime is not available in browser preview.');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

function formatDuration(seconds: number) {
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem ? `${hours}h ${rem}m` : `${hours}h`;
}

function formatTime(value: string | null) {
  if (!value) return 'now';
  return new Intl.DateTimeFormat([], { hour: 'numeric', minute: '2-digit' }).format(new Date(value));
}

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

function PrivacyModal({ onDismiss }: { onDismiss: () => void }) {
  const backdropRef = useRef<HTMLDivElement>(null);

  const handleOutsideClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === backdropRef.current) onDismiss();
    },
    [onDismiss],
  );

  return (
    <div className="modal-backdrop" ref={backdropRef} onClick={handleOutsideClick} role="dialog" aria-modal="true" aria-label="Privacy notice">
      <div className="modal-card">
        <button className="modal-close" onClick={onDismiss} type="button" aria-label="Close privacy notice">
          <X size={20} />
        </button>
        <div className="modal-header">
          <ShieldCheck size={28} />
          <h2>Welcome to OpenJournal</h2>
        </div>
        <div className="modal-body">
          <p>OpenJournal logs focused window activity <strong>entirely on your device.</strong></p>
          <ul>
            <li>No keylogging — never records typed keys or passwords.</li>
            <li>No clipboard capture — never reads your clipboard.</li>
            <li>No screenshots, screen recording, or screen capture.</li>
            <li>No cloud sync — all data stays in a local SQLite database.</li>
            <li>No external AI calls in v0.1 — summaries are placeholder templates.</li>
            <li>Private apps and domains can be blocklisted before anything is stored.</li>
            <li>Logging can be paused at any time from the app or system tray.</li>
          </ul>
          <p>OpenJournal only records:</p>
          <ul>
            <li>The name of the focused application (e.g., "Code.exe")</li>
            <li>The title of the focused window</li>
            <li>When focus started and ended</li>
          </ul>
        </div>
        <div className="modal-footer">
          <button className="primary-button full" onClick={onDismiss} type="button">
            I understand — start using OpenJournal
          </button>
        </div>
      </div>
    </div>
  );
}

function AboutPanel({ version, dbPath }: { version: string; dbPath: string }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <section className={`about-panel ${expanded ? 'expanded' : ''}`}>
      <button className="about-toggle" onClick={() => setExpanded((v) => !v)} type="button" aria-label="Toggle about panel">
        <Info size={16} />
        <span>About OpenJournal</span>
        <span className="about-version">{version}</span>
      </button>
      {expanded && (
        <div className="about-details">
          <div className="about-row">
            <span className="about-label">Version</span>
            <span className="about-value">{version}</span>
          </div>
          <div className="about-row">
            <span className="about-label">Database</span>
            <span className="about-value about-path" title={dbPath}>
              <ExternalLink size={12} />
              {dbPath}
            </span>
          </div>
          <div className="about-row">
            <span className="about-label">Data model</span>
            <span className="about-value">SQLite (local only)</span>
          </div>
          <div className="about-row">
            <span className="about-label">Privacy</span>
            <span className="about-value">All data stays on-device</span>
          </div>
        </div>
      )}
    </section>
  );
}

function App() {
  const [status, setStatus] = useState<AppStatus>({
    logging_paused: false,
    active_window: 'Browser preview mode',
    db_path: 'Local app data folder',
  });
  const [activities, setActivities] = useState<ActivityEntry[]>(sampleActivities);
  const [summaries, setSummaries] = useState<SummaryBlock[]>(sampleSummaries);
  const [blocklistText, setBlocklistText] = useState('1Password\nBitwarden\nbankofamerica.com\nchase.com');
  const [notice, setNotice] = useState('Ready');
  const [showPrivacyModal, setShowPrivacyModal] = useState(false);

  useEffect(() => {
    const dismissed = localStorage.getItem('openjournal_privacy_dismissed');
    if (!dismissed) {
      setShowPrivacyModal(true);
    }
  }, []);

  const day = todayIso();
  const totalSeconds = useMemo(
    () => activities.reduce((sum, entry) => sum + entry.duration_seconds, 0),
    [activities],
  );
  const appCount = useMemo(() => new Set(activities.map((entry) => entry.app_name)).size, [activities]);

  async function refresh() {
    if (!canUseTauri) return;
    try {
      const [nextStatus, nextActivities, nextSummaries] = await Promise.all([
        invokeCommand<AppStatus>('get_status'),
        invokeCommand<ActivityEntry[]>('get_day_activity', { day }),
        invokeCommand<SummaryBlock[]>('get_summary_blocks', { day }),
      ]);
      setStatus(nextStatus);
      setActivities(nextActivities);
      setSummaries(nextSummaries);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : 'Unable to refresh local data.');
    }
  }

  useEffect(() => {
    refresh();
    const timer = window.setInterval(refresh, 15_000);
    return () => window.clearInterval(timer);
  }, []);

  const handleDismissPrivacy = useCallback(() => {
    localStorage.setItem('openjournal_privacy_dismissed', 'true');
    setShowPrivacyModal(false);
  }, []);

  async function toggleLogging() {
    if (!canUseTauri) {
      setStatus((current) => ({ ...current, logging_paused: !current.logging_paused }));
      return;
    }
    const next = await invokeCommand<AppStatus>('set_logging_paused', {
      paused: !status.logging_paused,
    });
    setStatus(next);
  }

  async function saveBlocklist() {
    if (!canUseTauri) {
      setNotice('Blocklist saved in preview state.');
      return;
    }
    await invokeCommand('set_blocklist', {
      entries: blocklistText
        .split('\n')
        .map((entry) => entry.trim())
        .filter(Boolean),
    });
    setNotice('Private app/domain blocklist saved locally.');
  }

  async function exportDay(format: 'markdown' | 'json') {
    if (!canUseTauri) {
      setNotice(`Preview export simulated for ${format}.`);
      return;
    }
    const path = await invokeCommand<string>('export_day', { day, format });
    setNotice(`Exported to ${path}`);
  }

  async function deleteDay() {
    if (!window.confirm(`Delete all logs for ${day}? This cannot be undone.`)) return;
    if (!canUseTauri) {
      setActivities([]);
      setSummaries([]);
      return;
    }
    await invokeCommand('delete_day', { day });
    await refresh();
  }

  return (
    <>
      {showPrivacyModal && <PrivacyModal onDismiss={handleDismissPrivacy} />}
      <main className="app-shell">
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-mark">OJ</div>
            <div>
              <strong>OpenJournal</strong>
              <span>Local activity journal</span>
            </div>
          </div>

          <nav className="nav-list" aria-label="Primary">
            <a className="nav-item active" href="#today">
              <CalendarDays size={18} /> Today
            </a>
            <a className="nav-item" href="#privacy">
              <ShieldCheck size={18} /> Privacy
            </a>
            <a className="nav-item" href="#blocklist">
              <Ban size={18} /> Blocklist
            </a>
            <a className="nav-item" href="#summaries">
              <Brain size={18} /> 3-hour summaries
            </a>
          </nav>

          <section className="privacy-box" id="privacy">
            <ShieldCheck size={20} />
            <h2>Privacy</h2>
            <p>
              Everything stays on this device. OpenJournal records app names, window titles, and focus
              durations only. It never keylogs, reads typed text, captures clipboard data, records your
              screen, or syncs to the cloud.
            </p>
            <button className="text-button" onClick={() => setShowPrivacyModal(true)} type="button">
              View full privacy notice
            </button>
          </section>

          <AboutPanel version={APP_VERSION} dbPath={status.db_path} />
        </aside>

        <section className="content" id="today">
          <header className="topbar">
            <div>
              <h1>Today</h1>
              <p>{new Intl.DateTimeFormat([], { dateStyle: 'full' }).format(new Date())}</p>
            </div>
            <div className="topbar-actions">
              <span className={status.logging_paused ? 'status paused' : 'status'}>
                <Activity size={16} />
                {status.logging_paused ? 'Logging paused' : 'Logging active'}
                {canUseTauri && <span className="status-window" title={status.active_window}>{status.active_window}</span>}
              </span>
              <button className="primary-button" onClick={toggleLogging} type="button">
                {status.logging_paused ? <Play size={18} /> : <Pause size={18} />}
                {status.logging_paused ? 'Resume logging' : 'Pause logging'}
              </button>
            </div>
          </header>

          <section className="stats-grid" aria-label="Daily activity statistics">
            <div className="stat">
              <span>Tracked time</span>
              <strong>{formatDuration(totalSeconds)}</strong>
            </div>
            <div className="stat">
              <span>Apps used</span>
              <strong>{appCount}</strong>
            </div>
            <div className="stat">
              <span>Focused windows</span>
              <strong>{activities.length}</strong>
            </div>
            <div className="stat">
              <span>Storage</span>
              <strong>SQLite</strong>
            </div>
          </section>

          <section className="timeline-panel">
            <div className="panel-heading">
              <div>
                <h2>Daily timeline</h2>
                <p>Focused windows are merged into duration records.</p>
              </div>
              <div className="button-row">
                <button onClick={() => exportDay('markdown')} type="button">
                  <Download size={16} /> Export Markdown
                </button>
                <button onClick={() => exportDay('json')} type="button">
                  <Download size={16} /> Export JSON
                </button>
                <button className="danger-button" onClick={deleteDay} type="button">
                  <Trash2 size={16} /> Delete day
                </button>
              </div>
            </div>

            <div className="timeline">
              {activities.length === 0 ? (
                <p className="empty-state">No activity has been logged for this day.</p>
              ) : (
                activities.map((entry) => (
                  <article className="timeline-row" key={entry.id}>
                    <div className="time-cell">
                      <Clock3 size={16} />
                      <span>
                        {formatTime(entry.started_at)} - {formatTime(entry.ended_at)}
                      </span>
                    </div>
                    <div className="row-main">
                      <strong>{entry.app_name}</strong>
                      <span>{entry.window_title || 'Untitled window'}</span>
                    </div>
                    <div className="duration">{formatDuration(entry.duration_seconds)}</div>
                  </article>
                ))
              )}
            </div>
          </section>
        </section>

        <aside className="right-rail">
          <section className="summary-panel" id="summaries">
            <div className="panel-heading compact">
              <h2>3-hour summaries</h2>
              <span>v0.2-ready</span>
            </div>
            {summaries.map((summary) => (
              <article className="summary-block" key={`${summary.block_start}-${summary.block_end}`}>
                <div className="summary-time">
                  {summary.block_start} - {summary.block_end}
                </div>
                <h3>{summary.main_focus}</h3>
                <p>{summary.plain_english_summary}</p>
                <dl>
                  <div>
                    <dt>Apps/projects used</dt>
                    <dd>{summary.apps_projects.length ? summary.apps_projects.join(', ') : 'None yet'}</dd>
                  </div>
                  <div>
                    <dt>Context switches</dt>
                    <dd>{summary.context_switches}</dd>
                  </div>
                  <div>
                    <dt>Provider</dt>
                    <dd>{summary.provider}</dd>
                  </div>
                </dl>
              </article>
            ))}
          </section>

          <section className="blocklist-panel" id="blocklist">
            <h2>Blocklist</h2>
            <p>Skip private apps, domains, or title fragments before anything is stored.</p>
            <textarea
              aria-label="Private app and domain blocklist"
              value={blocklistText}
              onChange={(event) => setBlocklistText(event.target.value)}
            />
            <button className="primary-button full" onClick={saveBlocklist} type="button">
              Save blocklist
            </button>
          </section>

          <footer className="local-note">
            <strong>Local database</strong>
            <span>{status.db_path}</span>
            <small>{notice}</small>
          </footer>
        </aside>
      </main>
    </>
  );
}

createRoot(document.querySelector<HTMLDivElement>('#root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
