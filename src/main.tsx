import { createRoot } from 'react-dom/client';
import { useCallback, useEffect, useState } from 'react';
import {
  Activity, AlertTriangle, Ban, Brain, CalendarDays, Clock3, Download,
  ExternalLink, FolderOpen, Info, Power, RefreshCw, Save, Settings,
  ShieldCheck, Trash2, X, Zap,
} from 'lucide-react';
import './style.css';

const APP_VERSION = '0.3.7';

// ── Types ──
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
type AiConfig = {
  enabled: boolean; provider: string; base_url: string; api_key: string; model: string;
};
type AiSummary = {
  id: number; day: string; block_index: number; block_start: string; block_end: string;
  summary_json: string; model_name: string; generated_at: string; token_count: number | null;
  status: string; error_message: string | null; retry_count: number;
  last_attempt_at: string | null; generation_source: string; queue_status: string;
};
type ApiKeyStatus = { source: string; masked_key: string; has_env_var: boolean; has_credential: boolean };
type ConnectionTestResult = { success: boolean; message: string; model: string };
type AutostartSetting = { enabled: boolean };

const canUseTauri = !!(window as any).__TAURI__;
async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!canUseTauri) throw new Error('Tauri not available');
  return (window as any).__TAURI__.invoke(cmd, args);
}
const today = () => new Date().toISOString().slice(0, 10);
function fmtTime(secs: number): string {
  const h = Math.floor(secs / 3600); const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}
const BLOCK_LABELS = ['00:00–03:00','03:00–06:00','06:00–09:00','09:00–12:00','12:00–15:00','15:00–18:00','18:00–21:00','21:00–00:00'];

const PROVIDER_CONFIGS: Record<string, { base_url: string; model: string }> = {
  deepseek: { base_url: 'https://api.deepseek.com/v1', model: 'deepseek-chat' },
  lm_studio: { base_url: 'http://localhost:1234/v1', model: '' },
  ollama: { base_url: 'http://localhost:11434', model: '' },
  openai_compat: { base_url: '', model: '' },
};

// ── App ──
function App() {
  const [page, setPage] = useState('today');
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [dayStats, setDayStats] = useState<DayStats | null>(null);
  const [activities, setActivities] = useState<ActivityEntry[]>([]);
  const [notice, setNotice] = useState('');

  // AI state
  const [aiConfig, setAiConfig] = useState<AiConfig>({ enabled: false, provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', api_key: '', model: 'deepseek-chat' });
  const [apiKeyStatus, setApiKeyStatus] = useState<ApiKeyStatus | null>(null);
  const [connectionResult, setConnectionResult] = useState<ConnectionTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [aiSummaries, setAiSummaries] = useState<AiSummary[]>([]);
  const [generatingBlock, setGeneratingBlock] = useState<number | null>(null);
  const [showExtWarning, setShowExtWarning] = useState(false);
  const [pendingEnable, setPendingEnable] = useState(false);
  const [sessionKey, setSessionKey] = useState('');
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  // ── Lifecycle ──
  const refreshState = useCallback(async () => {
    if (!canUseTauri) return;
    try {
      const [s, ds] = await Promise.all([
        invokeCommand<AppStatus>('get_status'),
        invokeCommand<DayStats>('get_day_stats_cmd', { day: today() }),
      ]);
      setStatus(s); setDayStats(ds);
    } catch { /* */ }
  }, []);

  useEffect(() => {
    refreshState();
    const iv = setInterval(refreshState, 10000);
    return () => clearInterval(iv);
  }, [refreshState]);

  useEffect(() => {
    const onFocus = () => refreshState();
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [refreshState]);

  useEffect(() => {
    if (canUseTauri) {
      invokeCommand<AutostartSetting>('get_autostart_setting').then(s => setAutostartEnabled(s.enabled)).catch(() => {});
    }
  }, []);

  // Load AI state when page = ai-settings or ai-summaries
  const loadAiState = useCallback(async () => {
    if (!canUseTauri) return;
    try {
      const [cfg, keyStatus, summaries] = await Promise.all([
        invokeCommand<AiConfig>('get_ai_config'),
        invokeCommand<ApiKeyStatus>('get_api_key_status'),
        invokeCommand<AiSummary[]>('get_ai_summaries', { day: today() }),
      ]);
      setAiConfig(cfg); setApiKeyStatus(keyStatus); setAiSummaries(summaries);
    } catch { /* */ }
  }, []);

  useEffect(() => { if (page === 'ai-settings' || page === 'ai-summaries') loadAiState(); }, [page, loadAiState]);

  const trackedTimeDisplay = dayStats ? fmtTime(dayStats.tracked_seconds + dayStats.active_entry_duration_seconds) : '—';
  const appsCount = dayStats?.apps_used ?? 0;
  const windowsCount = dayStats?.focused_windows ?? 0;

  // ── AI handlers ──
  function handleToggleAi(enabled: boolean) {
    if (enabled && (aiConfig.provider === 'deepseek' || aiConfig.provider === 'openai_compat')) {
      setPendingEnable(true); setShowExtWarning(true); return;
    }
    setAiConfig(c => ({ ...c, enabled }));
  }

  function confirmExternal() {
    setShowExtWarning(false);
    if (pendingEnable) setAiConfig(c => ({ ...c, enabled: true }));
  }

  async function handleSaveAiConfig() {
    setSaving(true);
    try {
      // If user entered a session key, pass it as api_key for one-time override
      const cfg = { ...aiConfig, api_key: sessionKey || '' };
      await invokeCommand('set_ai_config', { config: cfg });
      setNotice('AI settings saved.');
      setSessionKey('');
      loadAiState();
    } catch (e) { setNotice(e instanceof Error ? e.message : 'Failed to save.'); }
    setSaving(false);
  }

  async function handleTestConnection() {
    setTesting(true); setConnectionResult(null);
    try {
      const cfg = { ...aiConfig, api_key: sessionKey || '' };
      const result = await invokeCommand<ConnectionTestResult>('test_ai_connection', { config: cfg });
      setConnectionResult(result);
      setNotice(result.success ? `Connected: ${result.model}` : `Failed: ${result.message}`);
    } catch (e) { setConnectionResult({ success: false, message: String(e), model: '' }); setNotice('Connection test failed.'); }
    setTesting(false);
  }

  async function handleClearAi() {
    const d: AiConfig = { enabled: false, provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', api_key: '', model: 'deepseek-chat' };
    setAiConfig(d); setConnectionResult(null);
    if (canUseTauri) { try { await invokeCommand('set_ai_config', { config: d }); setNotice('AI settings reset.'); } catch { /* */ } }
  }

  async function handleSaveApiKey(key: string) {
    if (!canUseTauri || !key) return;
    try { await invokeCommand('save_credential_api_key', { key }); setNotice('API key saved to credential manager.'); loadAiState(); }
    catch (e) { setNotice(e instanceof Error ? e.message : 'Failed to save key.'); }
  }

  async function handleDeleteApiKey() {
    if (!canUseTauri) return;
    try { await invokeCommand('delete_credential_api_key'); setNotice('API key removed.'); loadAiState(); }
    catch (e) { setNotice(e instanceof Error ? e.message : 'Failed to delete key.'); }
  }

  async function handleGenerateBlock(blockIndex: number) {
    setGeneratingBlock(blockIndex);
    try {
      await invokeCommand<string>('generate_ai_summary', { day: today(), blockIndex });
      setNotice(`Summary generated for block ${BLOCK_LABELS[blockIndex]}.`);
      loadAiState();
    } catch (e) { setNotice(e instanceof Error ? e.message : 'Generation failed.'); }
    setGeneratingBlock(null);
  }

  async function handleDeleteSummary(summaryId: number) {
    try { await invokeCommand('delete_ai_summary', { summaryId }); setNotice('Summary deleted.'); loadAiState(); }
    catch (e) { setNotice('Failed to delete.'); }
  }

  async function handleGenerateAll() {
    for (let i = 0; i < 8; i++) {
      if (aiSummaries.find(s => s.block_index === i && s.status === 'completed')) continue;
      setGeneratingBlock(i);
      try { await invokeCommand<string>('generate_ai_summary', { day: today(), blockIndex: i }); }
      catch { /* best effort */ }
    }
    setGeneratingBlock(null);
    loadAiState();
  }

  async function toggleAutostart(enabled: boolean) {
    if (!canUseTauri) return;
    try {
      await invokeCommand('set_autostart_setting', { setting: { enabled } });
      setAutostartEnabled(enabled);
      setNotice(enabled ? 'Autostart enabled.' : 'Autostart disabled.');
    } catch { setNotice('Failed to update autostart.'); }
  }

  async function openFolder(cmd: string, label: string) {
    if (!canUseTauri) return;
    try { const p = await invokeCommand<string>(cmd); setNotice(`Opened ${label}: ${p}`); }
    catch { setNotice(`Could not open ${label}.`); }
  }

  const providerBadge = (p: string) => {
    if (p === 'lm_studio' || p === 'ollama') return 'Local';
    return 'External';
  };

  // ── Render helpers ──
  function renderSummaryCard(idx: number) {
    const s = aiSummaries.find(x => x.block_index === idx);
    const label = BLOCK_LABELS[idx];
    const isGenerating = generatingBlock === idx;
    const enabled = aiConfig.enabled && (apiKeyStatus?.has_env_var || apiKeyStatus?.has_credential || sessionKey);
    return (
      <div key={idx} className={`summary-card ${s?.status === 'failed' ? 'failed' : ''}`}>
        <div className="summary-header">
          <span className="summary-time">{label}</span>
          {s?.generation_source === 'automatic' && <span className="summary-source-badge">Generated automatically</span>}
          {s ? (
            <span className={`summary-status status-${s.status}`}>{s.status}</span>
          ) : (
            <span className="summary-status status-pending">{enabled ? 'Pending' : 'Disabled'}</span>
          )}
        </div>
        {s?.status === 'completed' && (
          <div className="summary-body">
            <p className="summary-text">{(() => { try { return JSON.parse(s.summary_json)?.summary || s.summary_json; } catch { return s.summary_json; } })()}</p>
            <div className="summary-meta">
              <span>Model: {s.model_name}</span>
              {s.generated_at && <span>At: {s.generated_at.slice(11, 19)}</span>}
              {s.token_count != null && <span>{s.token_count} tokens</span>}
            </div>
          </div>
        )}
        {s?.status === 'failed' && <div className="summary-error">{s.error_message || 'Generation failed'}</div>}
        {s?.status === 'pending' && <div className="summary-pending"><RefreshCw size={14} className="spinner" /> Generating...</div>}
        <div className="summary-actions">
          {canUseTauri && enabled && (
            <button className="text-button small" onClick={() => handleGenerateBlock(idx)} disabled={isGenerating}>
              {s ? 'Regenerate' : 'Generate'}
            </button>
          )}
          {s && <button className="text-button small danger" onClick={() => handleDeleteSummary(s.id)}><Trash2 size={12} /></button>}
        </div>
      </div>
    );
  }

  // ── Main render ──
  return (
    <div className="app-layout">
      {/* ── Sidebar ── */}
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon"><Activity size={20} /></div>
          <div><strong>OpenJournal</strong><span className="brand-sub">Local activity journal</span></div>
        </div>
        <nav className="nav-links">
          {['today','privacy','blocklist','ai-summaries','ai-settings','diagnostics'].map(s => (
            <a key={s} className={`nav-item ${page === s ? 'active' : ''}`} onClick={() => setPage(s)}>
              {s === 'today' && <CalendarDays size={16} />}
              {s === 'privacy' && <ShieldCheck size={16} />}
              {s === 'blocklist' && <Ban size={16} />}
              {s === 'ai-summaries' && <Brain size={16} />}
              {s === 'ai-settings' && <Settings size={16} />}
              {s === 'diagnostics' && <Info size={16} />}
              {s === 'ai-summaries' ? 'AI Summaries' : s === 'ai-settings' ? 'AI Settings' : s.charAt(0).toUpperCase() + s.slice(1)}
            </a>
          ))}
        </nav>

        <div className="local-note"><ShieldCheck size={14} /><small>All data stays on your device.</small></div>
      </aside>

      {/* ── Main content ── */}
      <main className="main">
        {notice && <div className="notice-bar"><span>{notice}</span><button onClick={() => setNotice('')}><X size={14} /></button></div>}

        {/* ── Today page ── */}
        {page === 'today' && (
          <>
            <header className="main-header">
              <h1>Today</h1>
              <div className="status-info">
                <span className={`status-badge ${status?.logging_paused ? 'paused' : 'active'}`}>
                  {status?.logging_paused ? 'Logging paused' : 'Logging active'}
                </span>
                {status && <span className="status-info-label">{status.active_window}</span>}
              </div>
            </header>
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
            <div className="action-bar">
              {canUseTauri && <button className="primary-button small" onClick={async () => {
                try { const s = await invokeCommand<AppStatus>('set_logging_paused', { paused: !status?.logging_paused }); setStatus(s); }
                catch { setNotice('Failed to toggle.'); }
              }}>{status?.logging_paused ? '▶ Resume' : '⏸ Pause'}</button>}
              <button className="primary-button small" onClick={async () => {
                try { const p = await invokeCommand<string>('export_day', { day: today(), format: 'markdown' }); setNotice(`Exported: ${p}`); }
                catch { setNotice('Export failed.'); }
              }} disabled={!canUseTauri}><Download size={14} /> Markdown</button>
              <button className="primary-button small" onClick={async () => {
                try { const p = await invokeCommand<string>('export_day', { day: today(), format: 'json' }); setNotice(`Exported: ${p}`); }
                catch { setNotice('Export failed.'); }
              }} disabled={!canUseTauri}><Download size={14} /> JSON</button>
              <button className="danger-button small" onClick={async () => {
                if (!confirm('Delete all activity for today?')) return;
                try { await invokeCommand('delete_day', { day: today() }); setActivities([]); setNotice('Day deleted.'); }
                catch { setNotice('Delete failed.'); }
              }} disabled={!canUseTauri}><Trash2 size={14} /> Delete</button>
            </div>
            <div className="timeline">
              <h2>Timeline</h2>
              <div className="timeline-entries">
                {[...activities].reverse().slice(0, 30).map(e => (
                  <div className="entry" key={e.id}>
                    <div className="entry-time">{e.started_at.slice(11, 19)}</div>
                    <div className="entry-body">
                      <div className="entry-app">{e.app_name}</div>
                      <div className="entry-title">{e.window_title || '(no title)'}</div>
                      <div className="entry-duration">{e.duration_seconds}s</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        {/* ── Privacy page ── */}
        {page === 'privacy' && (
          <div className="page-section">
            <h2>Privacy</h2>
            <div className="privacy-card">
              <h3>What OpenJournal Records</h3>
              <ul><li>Active application name</li><li>Active window title</li><li>Start and end timestamps</li><li>Duration per focused window</li></ul>
              <h3>What OpenJournal Does NOT Collect</h3>
              <ul><li>✗ Keystrokes</li><li>✗ Clipboard contents</li><li>✗ Passwords</li><li>✗ Screenshots</li><li>✗ Microphone input</li><li>✗ Camera input</li><li>✗ File contents</li><li>✗ Cloud data</li></ul>
              <h3>AI Summaries</h3>
              <p>AI is disabled by default. When enabled, only aggregated per-block metadata (app names, durations, context switches) is sent to your configured provider. API keys are never stored in SQLite.</p>
            </div>
          </div>
        )}

        {/* ── Blocklist page ── */}
        {page === 'blocklist' && (
          <div className="page-section">
            <h2>Blocklist</h2>
            <p className="page-hint">Windows matching these patterns will not be logged.</p>
            <textarea className="blocklist-textarea" rows={8} placeholder="One pattern per line (e.g., 1Password, bank.com)" defaultValue={'1Password\nBitwarden\nbankofamerica.com\nchase.com'} />
            <button className="primary-button small" disabled={!canUseTauri}>Save blocklist</button>
          </div>
        )}

        {/* ── AI Summaries page ── */}
        {page === 'ai-summaries' && (
          <div className="page-section">
            <h2>3-Hour Summaries</h2>
            {!aiConfig.enabled && <div className="ai-empty-state"><Brain size={20} /><p>AI summaries are disabled.</p><p className="hint">Enable AI in AI Settings.</p></div>}
            {aiConfig.enabled && !apiKeyStatus?.has_env_var && !apiKeyStatus?.has_credential && !aiConfig.api_key && (
              <div className="ai-empty-state warning"><AlertTriangle size={16} /><p>DeepSeek API key not found. Set OPENJOURNAL_DEEPSEEK_API_KEY or save a key in AI Settings.</p></div>
            )}
            {aiConfig.enabled && (apiKeyStatus?.has_env_var || apiKeyStatus?.has_credential) && (
              <>
                <button className="primary-button small" onClick={handleGenerateAll} disabled={generatingBlock !== null} style={{ marginBottom: 12 }}><Zap size={14} /> Generate all missing</button>
                <div className="summary-list">{[0,1,2,3,4,5,6,7].map(renderSummaryCard)}</div>
              </>
            )}
          </div>
        )}

        {/* ── AI Settings page ── */}
        {page === 'ai-settings' && (
          <div className="page-section">
            <h2>AI Settings</h2>
            <div className="ai-settings-card">
              {/* AI enabled toggle */}
              <div className="ai-field toggle-row">
                <span className="ai-label">AI Summaries</span>
                <span className={`toggle-switch ${aiConfig.enabled ? 'on' : ''}`}
                      onClick={() => handleToggleAi(!aiConfig.enabled)} role="switch" tabIndex={0}>
                  <span className="toggle-knob" />
                </span>
              </div>

              {/* Provider */}
              <div className="ai-field">
                <span className="ai-label">Provider</span>
                <select className="ai-input" value={aiConfig.provider}
                  onChange={e => {
                    const p = e.target.value;
                    const cfg = PROVIDER_CONFIGS[p];
                    setAiConfig(c => ({ ...c, provider: p, base_url: cfg.base_url, model: cfg.model }));
                    setConnectionResult(null);
                  }}>
                  <option value="deepseek">DeepSeek ⭐ Recommended</option>
                  <option value="lm_studio">LM Studio</option>
                  <option value="ollama">Ollama</option>
                  <option value="openai_compat">OpenAI-compatible (advanced)</option>
                </select>
                <span className={`provider-badge ${aiConfig.provider === 'deepseek' || aiConfig.provider === 'openai_compat' ? 'external' : 'local'}`}>
                  {providerBadge(aiConfig.provider)}
                </span>
              </div>

              {/* Base URL */}
              <div className="ai-field">
                <span className="ai-label">Base URL</span>
                <input className="ai-input" value={aiConfig.base_url} onChange={e => setAiConfig(c => ({ ...c, base_url: e.target.value }))} />
              </div>

              {/* Model */}
              <div className="ai-field">
                <span className="ai-label">Model</span>
                <input className="ai-input" value={aiConfig.model} onChange={e => setAiConfig(c => ({ ...c, model: e.target.value }))} placeholder={aiConfig.provider === 'deepseek' ? 'deepseek-chat' : ''} />
              </div>

              {/* API key */}
              <div className="ai-field">
                <span className="ai-label">API Key</span>
                <div className="ai-key-status">
                  {apiKeyStatus && (
                    <>
                      <span className={`key-source-badge source-${apiKeyStatus.source}`}>
                        {apiKeyStatus.source === 'env' && <Zap size={12} />}
                        {apiKeyStatus.source === 'credential' && <ShieldCheck size={12} />}
                        {apiKeyStatus.source}
                      </span>
                      {apiKeyStatus.masked_key && <span className="key-masked">{apiKeyStatus.masked_key}</span>}
                    </>
                  )}
                </div>
                <div className="ai-key-actions">
                  {apiKeyStatus?.source !== 'env' && (
                    <div className="ai-key-buttons">
                      {apiKeyStatus?.has_credential ? (
                        <button className="text-button small danger" onClick={handleDeleteApiKey}>Remove stored key</button>
                      ) : (
                        <button className="text-button small" onClick={() => {
                          const k = prompt('Enter API key:');
                          if (k) handleSaveApiKey(k);
                        }}>Save key securely</button>
                      )}
                    </div>
                  )}
                  <div className="ai-key-buttons">
                    <input className="ai-input" type="password" placeholder="Override for this session..." value={sessionKey} onChange={e => setSessionKey(e.target.value)} style={{ width: 220 }} />
                  </div>
                </div>
                <div className="ai-note">API keys are never stored in the OpenJournal database. They use your OS credential manager or environment variables.</div>
              </div>

              {/* Actions */}
              <div className="ai-field ai-actions">
                <button className="primary-button small" onClick={handleSaveAiConfig} disabled={saving || !canUseTauri}>
                  <Save size={14} /> {saving ? 'Saving...' : 'Save settings'}
                </button>
                <button className="primary-button small" onClick={handleTestConnection} disabled={testing || !canUseTauri}>
                  {testing ? 'Testing...' : 'Test connection'}
                </button>
                <button className="danger-button small" onClick={handleClearAi}>Reset</button>
              </div>

              {connectionResult && (
                <div className={`connection-result ${connectionResult.success ? 'success' : 'error'}`}>
                  {connectionResult.success ? '✅ ' : '❌ '}{connectionResult.message}
                  {connectionResult.model && <small> Model: {connectionResult.model}</small>}
                </div>
              )}
            </div>
          </div>
        )}

        {/* ── Diagnostics page ── */}
        {page === 'diagnostics' && (
          <div className="page-section">
            <h2>Diagnostics</h2>
            <div className="diagnostics-panel" style={{ maxWidth: 500 }}>
              {[
                ['Version', APP_VERSION],
                ['App mode', status?.app_mode || 'Browser preview'],
                ['Database', status?.data_path || status?.db_path || '—'],
                ['Exports', status?.exports_path || '—'],
                ['Logs', status?.logs_path || '—'],
                ['Storage', status?.storage_backend || '—'],
                ['Tracker', status?.logging_paused ? 'Paused' : status ? 'Active' : '—'],
                ['Autostart', autostartEnabled ? 'Enabled' : 'Disabled'],
                ['Tray', status?.tray_active ? 'Active' : 'Unavailable'],
                ['Last write', dayStats?.last_activity_write_at?.slice(0,19) || '—'],
                ['Recovery', status?.last_recovery_at?.slice(0,19) || '—'],
                ['Active entry', dayStats ? fmtTime(dayStats.active_entry_duration_seconds) : '—'],
              ].map(([l, v]) => (
                <div className="diag-row" key={l}>
                  <span className="diag-label">{l}</span>
                  <span className={`diag-value ${l === 'Database' || l === 'Exports' || l === 'Logs' ? 'diag-path' : ''}`}>{v}</span>
                </div>
              ))}
              <div className="diag-actions">
                <button className="text-button small" onClick={refreshState}><RefreshCw size={12} /> Refresh</button>
                <button className="text-button small" onClick={() => openFolder('open_data_folder', 'Data')}><FolderOpen size={12} /> Data</button>
                <button className="text-button small" onClick={() => openFolder('open_exports_folder', 'Exports')}><Download size={12} /> Exports</button>
                <button className="text-button small" onClick={() => openFolder('open_logs_folder', 'Logs')}><Info size={12} /> Logs</button>
                <a className="text-button small" href="https://github.com/sparshsam/openjournal/releases" target="_blank" rel="noopener noreferrer"><ExternalLink size={12} /> Updates</a>
              </div>
              {canUseTauri && (
                <div className="autostart-section" style={{ marginTop: 12 }}>
                  <label className="toggle-row">
                    <span className="toggle-label"><Power size={14} /> Launch at startup</span>
                    <span className={`toggle-switch ${autostartEnabled ? 'on' : ''}`}
                          onClick={() => toggleAutostart(!autostartEnabled)} role="switch" tabIndex={0}>
                      <span className="toggle-knob" />
                    </span>
                  </label>
                </div>
              )}
            </div>
          </div>
        )}
      </main>

      {/* ── External provider warning modal ── */}
      {showExtWarning && (
        <div className="modal-backdrop" onClick={() => setShowExtWarning(false)}>
          <div className="modal-card" onClick={e => e.stopPropagation()}>
            <h3>External Provider Warning</h3>
            <p>You are enabling an external AI provider. Aggregated activity metadata (app names, window titles, durations) will be sent to:</p>
            <p><strong>{aiConfig.base_url}</strong></p>
            <div className="warning-list">
              <p><strong>Data sent:</strong></p>
              <ul>
                <li>App names and window titles (no raw text)</li>
                <li>Durations per window</li>
                <li>Context switch counts</li>
              </ul>
              <p><strong>Never sent:</strong></p>
              <ul>
                <li>✗ Keystrokes</li>
                <li>✗ Clipboard contents</li>
                <li>✗ Passwords</li>
                <li>✗ Screenshots</li>
                <li>✗ File contents</li>
              </ul>
            </div>
            <div className="modal-actions">
              <button className="primary-button small" onClick={confirmExternal}>I understand — enable</button>
              <button className="text-button small" onClick={() => setShowExtWarning(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

createRoot(document.getElementById('root')!).render(<App />);
