import { createRoot } from 'react-dom/client';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Activity,
  Ban,
  CalendarDays,
  Clock3,
  Download,
  ExternalLink,
  FolderOpen,
  Info,
  Power,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  X,
} from 'lucide-react';
import './style.css';

const APP_VERSION = '0.3.4';

// Types
type ActivityEntry = {
  id: number; app_name: string; window_title: string;
  started_at: string; ended_at: string | null; duration_seconds: number;
};

type AppStatus = {
  logging_paused: boolean; active_window: string; db_path: string;
  app_mode: string; tracker_running: boolean; autostart_enabled: boolean;
  last_write_at: string; last_recovery_at: string;
  data_path: string; exports_path: string; logs_path: string;
  tray_active: boolean; storage_backend: string;
};

type DayStats = {
  day: string; tracked_seconds: number; apps_used: number;
  focused_windows: number; storage_backend: string;
  last_activity_write_at: string; active_entry_duration_seconds: number;
};

type AutostartSetting = { enabled: boolean };

// Tauri bridge
const canUseTauri = !!(window as any).__TAURI__;
async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!canUseTauri) throw new Error('Tauri not available');
  return (window as any).__TAURI__.invoke(cmd, args);
}

const today = () => new Date().toISOString().slice(0, 10);

function fmtTimeClock(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

// ---- App ----
function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [dayStats, setDayStats] = useState<DayStats | null>(null);
  const [activities, setActivities] = useState<ActivityEntry[]>([]);
  const [notice, setNotice] = useState('');
  const [aboutExpanded, setAboutExpanded] = useState(false);
  const [diagRefreshing, setDiagRefreshing] = useState(false);

  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshState = useCallback(async () => {
    if (!canUseTauri) return;
    try {
      const [s, stats] = await Promise.all([
        invokeCommand<AppStatus>('get_status'),
        invokeCommand<DayStats>('get_day_stats_cmd', { day: today() }),
      ]);
      setStatus(s);
      setDayStats(stats);
    } catch { /* */ }
  }, []);

  // Poll while window is active
  useEffect(() => {
    refreshState();
    pollRef.current = setInterval(refreshState, 10000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [refreshState]);

  // Refresh on focus regain (window re-opened from tray)
  useEffect(() => {
    const onFocus = () => refreshState();
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [refreshState]);

  // Day stats
  const trackedTimeDisplay = dayStats
    ? fmtTimeClock(dayStats.tracked_seconds + dayStats.active_entry_duration_seconds)
    : activities.length > 0
      ? fmtTimeClock(activities.reduce((s, a) => s + a.duration_seconds, 0))
      : '—';

  const appsCount = dayStats?.apps_used ?? new Set(activities.map(a => a.app_name)).size;
  const windowsCount = dayStats?.focused_windows ?? new Set(activities.map(a => `${a.app_name}|${a.window_title}`)).size;

  // Autostart
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  useEffect(() => {
    if (canUseTauri) {
      invokeCommand<AutostartSetting>('get_autostart_setting').then(s => setAutostartEnabled(s.enabled)).catch(() => {});
    }
  }, []);

  async function toggleAutostart(enabled: boolean) {
    if (!canUseTauri) return;
    try {
      await invokeCommand('set_autostart_setting', { setting: { enabled } });
      setAutostartEnabled(enabled);
      setNotice(enabled ? 'Autostart enabled. Will launch at login.' : 'Autostart disabled.');
    } catch (e) { setNotice('Failed to update autostart.'); }
  }

  // Open folder buttons
  async function openFolder(cmd: string, label: string) {
    if (!canUseTauri) return;
    try {
      const path = await invokeCommand<string>(cmd);
      setNotice(`Opened ${label}: ${path}`);
    } catch { setNotice(`Could not open ${label}.`); }
  }

  // Diagnostics refresh
  async function refreshDiagnostics() {
    setDiagRefreshing(true);
    await refreshState();
    if (canUseTauri) {
      try {
        const s = await invokeCommand<AutostartSetting>('get_autostart_setting');
        setAutostartEnabled(s.enabled);
      } catch {}
    }
    setDiagRefreshing(false);
  }

  // ---- Render ----
  return (
    <div className="app-layout">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon"><Activity size={20} /></div>
          <div>
            <strong>OpenJournal</strong>
            <span className="brand-sub">Local activity journal</span>
          </div>
        </div>

        <nav className="nav-links">
          <a className="nav-item active"><CalendarDays size={16} /> Today</a>
          <a className="nav-item"><Ban size={16} /> Blocklist</a>
        </nav>

        {/* About / Diagnostics panel */}
        <section className="about-panel">
          <button className="about-toggle" onClick={() => setAboutExpanded(v => !v)} type="button" aria-label="Toggle about panel">
            <span className="about-version">v{APP_VERSION}</span>
            {aboutExpanded ? '▲' : '▼'}
          </button>
          {aboutExpanded && (
            <div className="about-details diagnostics-panel">
              <div className="diag-row"><span className="diag-label">Version</span><span className="diag-value">{APP_VERSION}</span></div>
              <div className="diag-row"><span className="diag-label">App mode</span><span className="diag-value">{status?.app_mode || 'Browser preview'}</span></div>
              <div className="diag-row"><span className="diag-label">Database</span><span className="diag-value diag-path">{status?.data_path || status?.db_path || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Exports</span><span className="diag-value diag-path">{status?.exports_path || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Logs</span><span className="diag-value diag-path">{status?.logs_path || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Storage</span><span className="diag-value">{status?.storage_backend || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Tracker</span><span className="diag-value">{status?.logging_paused ? 'Paused' : status ? 'Active' : '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Autostart</span><span className="diag-value">{autostartEnabled ? 'Enabled' : 'Disabled'}</span></div>
              <div className="diag-row"><span className="diag-label">Tray</span><span className="diag-value">{status?.tray_active ? 'Active' : 'Unavailable'}</span></div>
              <div className="diag-row"><span className="diag-label">Last write</span><span className="diag-value">{dayStats?.last_activity_write_at?.slice(0, 19) || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Recovery</span><span className="diag-value">{status?.last_recovery_at?.slice(0, 19) || '—'}</span></div>
              <div className="diag-row"><span className="diag-label">Active entry</span><span className="diag-value">{dayStats ? fmtTimeClock(dayStats.active_entry_duration_seconds) : '—'}</span></div>
              <div className="diag-actions">
                <button className="text-button small" onClick={refreshDiagnostics} disabled={diagRefreshing}>
                  <RefreshCw size={12} /> {diagRefreshing ? 'Refreshing...' : 'Refresh'}
                </button>
                <button className="text-button small" onClick={() => openFolder('open_data_folder', 'Data')}>
                  <FolderOpen size={12} /> Data
                </button>
                <button className="text-button small" onClick={() => openFolder('open_exports_folder', 'Exports')}>
                  <Download size={12} /> Exports
                </button>
                <button className="text-button small" onClick={() => openFolder('open_logs_folder', 'Logs')}>
                  <Info size={12} /> Logs
                </button>
                {canUseTauri && (
                  <a className="text-button small" href="https://github.com/sparshsam/openjournal/releases" target="_blank" rel="noopener noreferrer">
                    <ExternalLink size={12} /> Updates
                  </a>
                )}
              </div>

              {/* Autostart toggle */}
              {canUseTauri && (
                <div className="autostart-toggle">
                  <label className="toggle-row">
                    <span className="toggle-label">
                      <Power size={14} /> Launch at startup
                    </span>
                    <span className={`toggle-switch ${autostartEnabled ? 'on' : ''}`}
                          onClick={() => toggleAutostart(!autostartEnabled)} role="switch" aria-checked={autostartEnabled} tabIndex={0}>
                      <span className="toggle-knob" />
                    </span>
                  </label>
                </div>
              )}
            </div>
          )}
        </section>

        <div className="local-note">
          <ShieldCheck size={14} />
          {canUseTauri
            ? <span className="status-text">All data stays on your device.</span>
            : <span className="status-text"><small>Browser preview — some features disabled.</small></span>}
        </div>
      </aside>

      {/* Main */}
      <main className="main">
        <header className="main-header">
          <div className="date-nav">
            <h1>Today</h1>
            <div className="status-info">
              <span className={`status-badge ${status?.logging_paused ? 'paused' : 'active'}`}>
                {status?.logging_paused ? 'Logging paused' : 'Logging active'}
              </span>
              {status && <span className="status-info-label">Focused: {status.active_window}</span>}
            </div>
          </div>
        </header>

        {/* Stats cards — authoritative from backend DayStats */}
        <div className="stats-grid">
          <div className="stat-card">
            <div className="stat-value">{trackedTimeDisplay}</div>
            <div className="stat-label"><Clock3 size={12} /> Tracked time</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{appsCount}</div>
            <div className="stat-label"><Activity size={12} /> Apps used</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{windowsCount}</div>
            <div className="stat-label"><CalendarDays size={12} /> Focused windows</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{dayStats?.storage_backend || 'SQLite'}</div>
            <div className="stat-label"><Save size={12} /> Storage</div>
          </div>
        </div>

        {/* Pause / Export / Delete */}
        <div className="action-bar">
          {canUseTauri && (
            <button className="primary-button small"
              onClick={async () => {
                try {
                  const s = await invokeCommand<AppStatus>('set_logging_paused', { paused: !status?.logging_paused });
                  setStatus(s);
                } catch { setNotice('Failed to toggle logging.'); }
              }}>
              {status?.logging_paused ? '▶ Resume logging' : '⏸ Pause logging'}
            </button>
          )}
          <button className="primary-button small"
            onClick={async () => {
              try {
                const path = await invokeCommand<string>('export_day', { day: today(), format: 'markdown' });
                setNotice(`Exported Markdown: ${path}`);
              } catch (e) { setNotice('Export failed.'); }
            }} disabled={!canUseTauri}>
            <Download size={14} /> Markdown export
          </button>
          <button className="primary-button small"
            onClick={async () => {
              try {
                const path = await invokeCommand<string>('export_day', { day: today(), format: 'json' });
                setNotice(`Exported JSON: ${path}`);
              } catch (e) { setNotice('Export failed.'); }
            }} disabled={!canUseTauri}>
            <Download size={14} /> JSON export
          </button>
          <button className="danger-button small"
            onClick={async () => {
              if (!confirm('Delete all activity for this day?')) return;
              try {
                await invokeCommand('delete_day', { day: today() });
                setActivities([]);
                setNotice('Day deleted.');
              } catch { setNotice('Delete failed.'); }
            }} disabled={!canUseTauri}>
            <Trash2 size={14} /> Delete day
          </button>
        </div>

        {/* Notice */}
        {notice && <div className="notice-bar"><span>{notice}</span><button onClick={() => setNotice('')}><X size={14} /></button></div>}

        {/* Timeline */}
        <div className="timeline">
          <h2>Timeline</h2>
          <div className="timeline-entries">
            {[...activities].reverse().slice(0, 20).map((entry) => (
              <div className="entry" key={entry.id}>
                <div className="entry-time">{entry.started_at.slice(11, 19)}</div>
                <div className="entry-body">
                  <div className="entry-app">{entry.app_name}</div>
                  <div className="entry-title">{entry.window_title || '(no title)'}</div>
                  <div className="entry-duration">{entry.duration_seconds}s {entry.ended_at ? '' : '(active)'}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </main>
    </div>
  );
}

createRoot(document.getElementById('root')!).render(<App />);
