import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  Activity,
  AlertTriangle,
  Ban,
  Brain,
  CalendarDays,
  CheckCircle2,
  Clock3,
  Download,
  ExternalLink,
  Info,
  Pause,
  Play,
  RefreshCw,
  Save,
  Settings,
  ShieldCheck,
  Trash2,
  WifiOff,
  X,
  Zap,
} from 'lucide-react';
import './style.css';

const APP_VERSION = '0.2.1';

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------

type ActivityEntry = {
  id: number;
  app_name: string;
  window_title: string;
  started_at: string;
  ended_at: string | null;
  duration_seconds: number;
};

type AppStatus = {
  logging_paused: boolean;
  active_window: string;
  db_path: string;
};

type AiConfig = {
  enabled: boolean;
  provider: string;
  base_url: string;
  api_key: string;
  model: string;
};

type ConnectionTestResult = {
  success: boolean;
  message: string;
  models: string[];
};

type AiSummary = {
  id: number;
  day: string;
  block_index: number;
  block_start: string;
  block_end: string;
  summary_json: string;
  model_name: string;
  generated_at: string;
  token_count: number | null;
  status: string;
  error_message: string | null;
};

type ApiKeyStatus = {
  source: string;        // "env" | "credential" | "session" | "missing"
  masked_key: string;    // "sk-••••••••abcd" or empty
  has_env_var: boolean;
  has_credential: boolean;
};

// -----------------------------------------------------------------------
// Provider presets
// -----------------------------------------------------------------------

type ProviderPreset = {
  id: string;
  label: string;
  base_url: string;
  model: string;
  category: 'local' | 'external';
  badge: string;
};

const PROVIDER_PRESETS: ProviderPreset[] = [
  {
    id: 'deepseek',
    label: 'DeepSeek ⭐ Recommended',
    base_url: 'https://api.deepseek.com/v1',
    model: 'deepseek-chat',
    category: 'external',
    badge: 'External',
  },
  {
    id: 'lm_studio',
    label: 'LM Studio',
    base_url: 'http://localhost:1234/v1',
    model: '',
    category: 'local',
    badge: 'Local',
  },
  {
    id: 'ollama',
    label: 'Ollama',
    base_url: 'http://localhost:11434',
    model: '',
    category: 'local',
    badge: 'Local',
  },
  {
    id: 'openai_compatible',
    label: 'OpenAI-compatible (advanced)',
    base_url: '',
    model: '',
    category: 'external',
    badge: 'External',
  },
];

// -----------------------------------------------------------------------
// Tauri bridge
// -----------------------------------------------------------------------

const canUseTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!canUseTauri) throw new Error('Tauri runtime is not available in browser preview.');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

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

const BLOCK_LABELS = [
  '00:00 - 03:00', '03:00 - 06:00', '06:00 - 09:00', '09:00 - 12:00',
  '12:00 - 15:00', '15:00 - 18:00', '18:00 - 21:00', '21:00 - 00:00',
];

// -----------------------------------------------------------------------
// Sub-components
// -----------------------------------------------------------------------

function PrivacyModal({ onDismiss }: { onDismiss: () => void }) {
  const backdropRef = useRef<HTMLDivElement>(null);
  const handleClick = useCallback(
    (e: React.MouseEvent) => { if (e.target === backdropRef.current) onDismiss(); },
    [onDismiss],
  );
  return (
    <div className="modal-backdrop" ref={backdropRef} onClick={handleClick} role="dialog" aria-modal="true" aria-label="Privacy notice">
      <div className="modal-card">
        <button className="modal-close" onClick={onDismiss} type="button" aria-label="Close"><X size={20} /></button>
        <div className="modal-header"><ShieldCheck size={28} /><h2>Welcome to OpenJournal</h2></div>
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
          <button className="primary-button full" onClick={onDismiss} type="button">I understand — start using OpenJournal</button>
        </div>
      </div>
    </div>
  );
}

function ExternalProviderWarning({ provider, onConfirm, onCancel }: { provider: string; onConfirm: () => void; onCancel: () => void }) {
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-label="External provider warning">
      <div className="modal-card warning-card">
        <div className="modal-header" style={{ color: '#b42318' }}><AlertTriangle size={28} /><h2>Privacy Warning</h2></div>
        <div className="modal-body">
          <p>You are about to enable <strong>{provider}</strong>, which is an <strong>external</strong> provider.</p>
          <p>When generating summaries, OpenJournal will send aggregated activity metadata to:</p>
          <p className="provider-url">{PROVIDER_PRESETS.find(p => p.id === provider)?.base_url || 'remote server'}</p>
          <p>This includes: app names, window titles, and durations — <strong>never</strong> keystrokes, passwords, clipboard, or screenshots.</p>
          <div className="warning-list">
            <p><strong>OpenJournal will NEVER send:</strong></p>
            <ul>
              <li>Keystrokes or typed text</li>
              <li>Clipboard contents</li>
              <li>Passwords</li>
              <li>Screenshots or screen recordings</li>
              <li>File contents</li>
              <li>Microphone or camera data</li>
            </ul>
          </div>
        </div>
        <div className="modal-footer" style={{ display: 'flex', gap: 12 }}>
          <button className="danger-button" onClick={onCancel} type="button" style={{ flex: 1 }}>Cancel</button>
          <button className="primary-button" onClick={onConfirm} type="button" style={{ flex: 1 }}>I understand, enable {provider}</button>
        </div>
      </div>
    </div>
  );
}

function AboutPanel({ version, dbPath }: { version: string; dbPath: string }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <section className={`about-panel ${expanded ? 'expanded' : ''}`}>
      <button className="about-toggle" onClick={() => setExpanded(v => !v)} type="button" aria-label="Toggle about panel">
        <Info size={16} /><span>About OpenJournal</span><span className="about-version">{version}</span>
      </button>
      {expanded && (
        <div className="about-details">
          <div className="about-row"><span className="about-label">Version</span><span className="about-value">{version}</span></div>
          <div className="about-row"><span className="about-label">Database</span><span className="about-value about-path" title={dbPath}><ExternalLink size={12} />{dbPath}</span></div>
          <div className="about-row"><span className="about-label">Data model</span><span className="about-value">SQLite (local only)</span></div>
          <div className="about-row"><span className="about-label">Privacy</span><span className="about-value">All data stays on-device</span></div>
        </div>
      )}
    </section>
  );
}

// -----------------------------------------------------------------------
// AI Settings Panel
// -----------------------------------------------------------------------

function AiSettingsPanel({
  config,
  apiKeyStatus,
  connectionResult,
  testing,
  saving,
  onSave,
  onTest,
  onClear,
  onConfigChange,
  onSaveApiKey,
  onDeleteApiKey,
}: {
  config: AiConfig;
  apiKeyStatus: ApiKeyStatus | null;
  connectionResult: ConnectionTestResult | null;
  testing: boolean;
  saving: boolean;
  onSave: () => void;
  onTest: () => void;
  onClear: () => void;
  onConfigChange: (cfg: AiConfig) => void;
  onSaveApiKey: (key: string) => void;
  onDeleteApiKey: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const preset = PROVIDER_PRESETS.find(p => p.id === config.provider);

  const handleProviderChange = (id: string) => {
    const p = PROVIDER_PRESETS.find(pr => pr.id === id);
    onConfigChange({
      ...config,
      provider: id,
      base_url: p?.base_url || config.base_url,
      model: p?.model || config.model,
    });
  };

  return (
    <section className="ai-settings-panel">
      <button className="ai-settings-toggle" onClick={() => setExpanded(v => !v)} type="button">
        <Brain size={16} /><span>AI Summaries</span>
        {config.enabled ? <span className="ai-badge-enabled">ON</span> : <span className="ai-badge-disabled">OFF</span>}
      </button>
      {expanded && (
        <div className="ai-settings-body">
          {/* Enable toggle */}
          <div className="ai-toggle-row">
            <span className="ai-label">Enable AI summaries</span>
            <button
              className={`toggle-switch ${config.enabled ? 'on' : ''}`}
              onClick={() => onConfigChange({ ...config, enabled: !config.enabled })}
              type="button"
              role="switch"
              aria-checked={config.enabled}
            >
              <span className="toggle-knob" />
            </button>
          </div>

          {config.enabled && (
            <>
              {/* Provider dropdown */}
              <div className="ai-field">
                <span className="ai-label">Provider</span>
                <select
                  className="ai-select"
                  value={config.provider}
                  onChange={e => handleProviderChange(e.target.value)}
                >
                  {PROVIDER_PRESETS.map(p => (
                    <option key={p.id} value={p.id}>{p.label}</option>
                  ))}
                </select>
              </div>

              {/* Provider badge */}
              {preset && (
                <div className="ai-badge-row">
                  <span className={`provider-badge ${preset.category}`}>{preset.badge}</span>
                  {preset.category === 'external' && <span className="provider-badge external-warning">Data sent externally</span>}
                </div>
              )}

              {/* Base URL */}
              <div className="ai-field">
                <span className="ai-label">Base URL</span>
                <input
                  className="ai-input"
                  type="text"
                  value={config.base_url}
                  onChange={e => onConfigChange({ ...config, base_url: e.target.value })}
                  placeholder="https://api.deepseek.com/v1"
                />
              </div>

              {/* API key */}
              <div className="ai-field">
                <span className="ai-label">API Key</span>
                <div className="ai-key-status">
                  <span className={`key-source-badge source-${apiKeyStatus?.source || 'missing'}`}>
                    {apiKeyStatus?.source === 'env' && <><Zap size={12} /> Environment</>}
                    {apiKeyStatus?.source === 'credential' && <>🔑 Credential Manager</>}
                    {apiKeyStatus?.source === 'session' && <>Session only</>}
                    {(!apiKeyStatus || apiKeyStatus.source === 'missing') && <>❌ Missing</>}
                  </span>
                  {apiKeyStatus?.masked_key && (
                    <span className="key-masked">{apiKeyStatus.masked_key}</span>
                  )}
                </div>
                {apiKeyStatus?.has_env_var && (
                  <div className="ai-env-indicator">
                    <Zap size={12} />
                    <span>Using environment variable</span>
                  </div>
                )}
                {apiKeyStatus?.has_credential && (
                  <div className="ai-env-indicator" style={{color:'#5b21b6',background:'#f3e8ff'}}>
                    <span>Key stored in OS credential manager</span>
                  </div>
                )}
                <div className="ai-key-actions">
                  <input
                    className="ai-input"
                    type="password"
                    value={config.api_key}
                    onChange={e => onConfigChange({ ...config, api_key: e.target.value })}
                    placeholder="Enter new API key to save..."
                  />
                  <div className="ai-key-buttons">
                    <button className="primary-button small" onClick={() => onSaveApiKey(config.api_key)} disabled={!config.api_key} type="button">
                      Save Key
                    </button>
                    <button className="danger-button small" onClick={onDeleteApiKey} type="button">
                      Remove Key
                    </button>
                  </div>
                </div>
              </div>
              <div className="ai-note">
                API keys are never stored in the OpenJournal database.
                Keys are saved to your OS credential manager.
              </div>

              {/* Model */}
              <div className="ai-field">
                <span className="ai-label">Model</span>
                <input
                  className="ai-input"
                  type="text"
                  value={config.model}
                  onChange={e => onConfigChange({ ...config, model: e.target.value })}
                  placeholder="deepseek-chat"
                />
              </div>

              {/* Action buttons */}
              <div className="ai-actions">
                <button className="primary-button small" onClick={onTest} disabled={testing} type="button">
                  {testing ? 'Testing...' : 'Test Connection'}
                </button>
                <button className="primary-button small" onClick={onSave} disabled={saving} type="button">
                  <Save size={14} />{saving ? 'Saving...' : 'Save Settings'}
                </button>
                <button className="danger-button small" onClick={onClear} type="button">
                  Reset
                </button>
              </div>

              {/* Connection test result */}
              {connectionResult && (
                <div className={`ai-test-result ${connectionResult.success ? 'success' : 'error'}`}>
                  {connectionResult.success ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
                  <span>{connectionResult.message}</span>
                </div>
              )}

              {!apiKeyStatus?.has_env_var && !apiKeyStatus?.has_credential && apiKeyStatus?.source === 'missing' && config.provider === 'deepseek' && (
                <div className="ai-env-hint">
                  Set <code>OPENJOURNAL_DEEPSEEK_API_KEY</code> or <code>DEEPSEEK_API_KEY</code> in your environment, or save a key below.
                </div>
              )}
            </>
          )}
        </div>
      )}
    </section>
  );
}

// -----------------------------------------------------------------------
// Summary Block Card
// -----------------------------------------------------------------------

function SummaryCard({
  blockIndex,
  summary,
  generating,
  onGenerate,
  onDelete,
}: {
  blockIndex: number;
  summary: AiSummary | null;
  generating: boolean;
  onGenerate: () => void;
  onDelete: () => void;
}) {
  const label = BLOCK_LABELS[blockIndex] || `${blockIndex * 3}:00`;
  const [start, end] = label.split(' - ');

  // Parse summary_json if completed
  let parsed: any = null;
  if (summary?.status === 'completed' && summary.summary_json) {
    try { parsed = JSON.parse(summary.summary_json); } catch { /* ignore */ }
  }

  const generatedTime = summary?.generated_at
    ? new Intl.DateTimeFormat([], { hour: 'numeric', minute: '2-digit' }).format(new Date(summary.generated_at))
    : null;

  return (
    <article className={`summary-card status-${summary?.status || 'none'}`}>
      <div className="summary-card-header">
        <div className="summary-time">{start} - {end}</div>
        <div className="summary-card-actions">
          {summary?.status === 'completed' && (
            <button className="icon-button" onClick={onGenerate} title="Regenerate" type="button">
              <RefreshCw size={14} />
            </button>
          )}
          {summary && (
            <button className="icon-button" onClick={onDelete} title="Delete summary" type="button">
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Empty / Disabled */}
      {!summary && (
        <div className="summary-card-body empty">
          <p>No summary yet.</p>
          <button className="text-button" onClick={onGenerate} disabled={generating} type="button">
            {generating ? 'Generating...' : 'Generate summary'}
          </button>
        </div>
      )}

      {/* Pending */}
      {summary?.status === 'pending' && (
        <div className="summary-card-body pending">
          <div className="spinner" />
          <p>Generating summary...</p>
        </div>
      )}

      {/* Completed */}
      {summary?.status === 'completed' && parsed && (
        <div className="summary-card-body completed">
          <h3>{parsed.main_focus || 'Work session'}</h3>
          <p className="summary-text">{parsed.plain_english_summary || 'No summary text.'}</p>
          {parsed.apps_projects && parsed.apps_projects.length > 0 && (
            <div className="summary-meta"><span className="meta-label">Apps:</span> {parsed.apps_projects.join(', ')}</div>
          )}
          <div className="summary-meta"><span className="meta-label">Context switches:</span> {parsed.context_switches ?? '—'}</div>
          {parsed.total_focus_minutes != null && (
            <div className="summary-meta"><span className="meta-label">Focus time:</span> {formatDuration(parsed.total_focus_minutes * 60)}</div>
          )}
          <div className="summary-footer">
            <span className="summary-model">{summary.model_name || 'unknown model'}</span>
            {generatedTime && <span className="summary-time-label">{generatedTime}</span>}
            {summary.token_count != null && <span className="summary-tokens">{summary.token_count} tokens</span>}
          </div>
        </div>
      )}

      {/* Failed */}
      {summary?.status === 'failed' && (
        <div className="summary-card-body failed">
          <AlertTriangle size={16} />
          <p>Generation failed</p>
          {summary.error_message && <p className="error-detail">{summary.error_message}</p>}
          <button className="text-button" onClick={onGenerate} type="button">Retry</button>
        </div>
      )}
    </article>
  );
}

// -----------------------------------------------------------------------
// Main App
// -----------------------------------------------------------------------

function App() {
  const [status, setStatus] = useState<AppStatus>({
    logging_paused: false, active_window: 'Browser preview mode', db_path: 'Local app data folder',
  });
  const [activities, setActivities] = useState<ActivityEntry[]>([]);
  const [blocklistText, setBlocklistText] = useState('1Password\nBitwarden\nbankofamerica.com\nchase.com');
  const [notice, setNotice] = useState('Ready');
  const [showPrivacyModal, setShowPrivacyModal] = useState(false);
  const [generatingBlock, setGeneratingBlock] = useState<number | null>(null);

  // AI state
  const [aiConfig, setAiConfig] = useState<AiConfig>({
    enabled: false, provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', api_key: '', model: 'deepseek-chat',
  });
  const [apiKeyStatus, setApiKeyStatus] = useState<ApiKeyStatus | null>(null);
  const [connectionResult, setConnectionResult] = useState<ConnectionTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [aiSummaries, setAiSummaries] = useState<AiSummary[]>([]);
  const [showExternalWarning, setShowExternalWarning] = useState<string | null>(null);
  const [pendingAiConfig, setPendingAiConfig] = useState<AiConfig | null>(null);

  const day = todayIso();
  const totalSeconds = useMemo(() => activities.reduce((s, e) => s + e.duration_seconds, 0), [activities]);
  const appCount = useMemo(() => new Set(activities.map(e => e.app_name)).size, [activities]);

  // Initialize
  useEffect(() => {
    const dismissed = localStorage.getItem('openjournal_privacy_dismissed');
    if (!dismissed) setShowPrivacyModal(true);
    loadAiState();
    if (canUseTauri) refreshActivities();
  }, []);

  const dismissPrivacy = useCallback(() => {
    localStorage.setItem('openjournal_privacy_dismissed', 'true');
    setShowPrivacyModal(false);
  }, []);

  // --- Data loading ---
  async function loadAiState() {
    // Load env status (safe in browser preview)
    if (canUseTauri) {
      try {
        const [keyStatus, summaries] = await Promise.all([
          invokeCommand<ApiKeyStatus>('get_api_key_status'),
          invokeCommand<AiSummary[]>('get_ai_summaries', { day: todayIso() }),
        ]);
        setApiKeyStatus(keyStatus);
        setAiSummaries(summaries);
        const cfg = await invokeCommand<AiConfig>('get_ai_config');
        setAiConfig(cfg);
      } catch { /* preview mode fallback */ }
    }
  }

  async function refreshActivities() {
    if (!canUseTauri) return;
    try {
      const [s, a] = await Promise.all([
        invokeCommand<AppStatus>('get_status'),
        invokeCommand<ActivityEntry[]>('get_day_activity', { day }),
      ]);
      setStatus(s);
      setActivities(a);
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Unable to refresh.');
    }
  }

  useEffect(() => {
    if (!canUseTauri) return;
    refreshActivities();
    const timer = setInterval(refreshActivities, 15_000);
    return () => clearInterval(timer);
  }, []);

  // --- AI settings ---
  async function handleSaveAiConfig() {
    setSaving(true);
    try {
      await invokeCommand('set_ai_config', { config: aiConfig });
      setNotice('AI settings saved.');
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Failed to save.');
    }
    setSaving(false);
  }

  async function handleTestConnection() {
    setTesting(true);
    setConnectionResult(null);
    try {
      const result = await invokeCommand<ConnectionTestResult>('test_ai_connection', { config: aiConfig });
      setConnectionResult(result);
    } catch (e) {
      setConnectionResult({ success: false, message: e instanceof Error ? e.message : 'Connection failed', models: [] });
    }
    setTesting(false);
  }

  async function handleClearAiSettings() {
    const defaults: AiConfig = {
      enabled: false, provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', api_key: '', model: 'deepseek-chat',
    };
    setAiConfig(defaults);
    setConnectionResult(null);
    if (canUseTauri) {
      try {
        await invokeCommand('set_ai_config', { config: defaults });
        setNotice('AI settings reset.');
      } catch { /* */ }
    }
  }

  async function handleSaveApiKey(key: string) {
    if (!canUseTauri || !key) return;
    try {
      await invokeCommand('save_credential_api_key', { key });
      setNotice('API key saved to credential manager.');
      const status = await invokeCommand<ApiKeyStatus>('get_api_key_status');
      setApiKeyStatus(status);
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Failed to save API key.');
    }
  }

  async function handleDeleteApiKey() {
    if (!canUseTauri) return;
    try {
      await invokeCommand('delete_credential_api_key');
      setNotice('API key removed from credential manager.');
      const status = await invokeCommand<ApiKeyStatus>('get_api_key_status');
      setApiKeyStatus(status);
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Failed to delete API key.');
    }
  }

  // --- Handle enabling AI with external provider check ---
  function handleToggleAi(enabled: boolean) {
    if (enabled) {
      const preset = PROVIDER_PRESETS.find(p => p.id === aiConfig.provider);
      if (preset?.category === 'external') {
        setShowExternalWarning(aiConfig.provider);
        setPendingAiConfig({ ...aiConfig, enabled: true });
        return;
      }
    }
    setAiConfig(c => ({ ...c, enabled }));
  }

  function confirmExternalProvider() {
    if (pendingAiConfig) {
      setAiConfig(pendingAiConfig);
      setShowExternalWarning(null);
      setPendingAiConfig(null);
    }
  }

  function cancelExternalProvider() {
    setShowExternalWarning(null);
    setPendingAiConfig(null);
  }

  // --- Summary generation ---
  async function handleGenerateSummary(blockIndex: number) {
    if (!canUseTauri) {
      setNotice('Generation requires Tauri runtime.');
      return;
    }
    setGeneratingBlock(blockIndex);
    try {
      await invokeCommand<string>('generate_ai_summary', { day, blockIndex });
      // Refresh summaries
      const updated = await invokeCommand<AiSummary[]>('get_ai_summaries', { day });
      setAiSummaries(updated);
      setNotice('Summary generated.');
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Generation failed.');
      // Refresh to show failed status
      try {
        const updated = await invokeCommand<AiSummary[]>('get_ai_summaries', { day });
        setAiSummaries(updated);
      } catch { /* */ }
    }
    setGeneratingBlock(null);
  }

  async function handleDeleteSummary(summaryId: number) {
    if (!canUseTauri) return;
    try {
      await invokeCommand('delete_ai_summary', { summaryId });
      const updated = await invokeCommand<AiSummary[]>('get_ai_summaries', { day });
      setAiSummaries(updated);
      setNotice('Summary deleted.');
    } catch (e) {
      setNotice(e instanceof Error ? e.message : 'Delete failed.');
    }
  }

  // --- Core actions ---
  async function toggleLogging() {
    if (!canUseTauri) { setStatus(s => ({ ...s, logging_paused: !s.logging_paused })); return; }
    const next = await invokeCommand<AppStatus>('set_logging_paused', { paused: !status.logging_paused });
    setStatus(next);
  }

  async function saveBlocklist() {
    if (!canUseTauri) { setNotice('Blocklist saved in preview state.'); return; }
    await invokeCommand('set_blocklist', { entries: blocklistText.split('\n').map(e => e.trim()).filter(Boolean) });
    setNotice('Blocklist saved locally.');
  }

  async function exportDay(format: 'markdown' | 'json') {
    if (!canUseTauri) { setNotice(`Preview export simulated for ${format}.`); return; }
    const path = await invokeCommand<string>('export_day', { day, format });
    setNotice(`Exported to ${path}`);
  }

  async function deleteDay() {
    if (!window.confirm(`Delete all logs for ${day}? This cannot be undone.`)) return;
    if (!canUseTauri) { setActivities([]); return; }
    await invokeCommand('delete_day', { day });
    await refreshActivities();
  }

  // Mock activities for browser preview
  useEffect(() => {
    if (!canUseTauri && activities.length === 0) {
      setActivities([
        { id: 1, app_name: 'Visual Studio Code', window_title: 'openjournal - main.tsx', started_at: new Date().toISOString(), ended_at: new Date(Date.now() + 39 * 60 * 1000).toISOString(), duration_seconds: 2340 },
        { id: 2, app_name: 'Microsoft Edge', window_title: 'Tauri docs', started_at: new Date(Date.now() + 42 * 60 * 1000).toISOString(), ended_at: new Date(Date.now() + 57 * 60 * 1000).toISOString(), duration_seconds: 900 },
        { id: 3, app_name: 'Windows Terminal', window_title: 'npm run tauri dev', started_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(), ended_at: new Date(Date.now() + 81 * 60 * 1000).toISOString(), duration_seconds: 1260 },
      ]);
    }
  }, []);

  // Get summary for a block
  const getSummaryForBlock = (idx: number) => aiSummaries.find(s => s.block_index === idx) || null;

  return (
    <>
      {showPrivacyModal && <PrivacyModal onDismiss={dismissPrivacy} />}
      {showExternalWarning && (
        <ExternalProviderWarning provider={showExternalWarning} onConfirm={confirmExternalProvider} onCancel={cancelExternalProvider} />
      )}
      <main className="app-shell">
        {/* Sidebar */}
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-mark">OJ</div>
            <div><strong>OpenJournal</strong><span>Local activity journal</span></div>
          </div>

          <nav className="nav-list" aria-label="Primary">
            <a className="nav-item active" href="#today"><CalendarDays size={18} /> Today</a>
            <a className="nav-item" href="#privacy"><ShieldCheck size={18} /> Privacy</a>
            <a className="nav-item" href="#blocklist"><Ban size={18} /> Blocklist</a>
            <a className="nav-item" href="#summaries"><Brain size={18} /> AI Summaries</a>
            <a className="nav-item" href="#ai-settings"><Settings size={18} /> AI Settings</a>
          </nav>

          <AiSettingsPanel
            config={aiConfig}
            apiKeyStatus={apiKeyStatus}
            connectionResult={connectionResult}
            testing={testing}
            saving={saving}
            onSave={handleSaveAiConfig}
            onTest={handleTestConnection}
            onClear={handleClearAiSettings}
            onConfigChange={(cfg) => {
              if (cfg.enabled !== aiConfig.enabled) {
                handleToggleAi(cfg.enabled);
              } else {
                setAiConfig(cfg);
              }
            }}
            onSaveApiKey={handleSaveApiKey}
            onDeleteApiKey={handleDeleteApiKey}
          />

          <section className="privacy-box" id="privacy">
            <ShieldCheck size={20} /><h2>Privacy</h2>
            <p>Everything stays on this device. OpenJournal records app names, window titles, and focus durations only.</p>
            <button className="text-button" onClick={() => setShowPrivacyModal(true)} type="button">View full privacy notice</button>
          </section>

          <AboutPanel version={APP_VERSION} dbPath={status.db_path} />
        </aside>

        {/* Content */}
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
              </span>
              <button className="primary-button" onClick={toggleLogging} type="button">
                {status.logging_paused ? <Play size={18} /> : <Pause size={18} />}
                {status.logging_paused ? 'Resume logging' : 'Pause logging'}
              </button>
            </div>
          </header>

          <section className="stats-grid" aria-label="Daily activity statistics">
            <div className="stat"><span>Tracked time</span><strong>{formatDuration(totalSeconds)}</strong></div>
            <div className="stat"><span>Apps used</span><strong>{appCount}</strong></div>
            <div className="stat"><span>Focused windows</span><strong>{activities.length}</strong></div>
            <div className="stat"><span>Storage</span><strong>SQLite</strong></div>
          </section>

          <section className="timeline-panel">
            <div className="panel-heading">
              <div><h2>Daily timeline</h2><p>Focused windows are merged into duration records.</p></div>
              <div className="button-row">
                <button onClick={() => exportDay('markdown')} type="button"><Download size={16} /> Export Markdown</button>
                <button onClick={() => exportDay('json')} type="button"><Download size={16} /> Export JSON</button>
                <button className="danger-button" onClick={deleteDay} type="button"><Trash2 size={16} /> Delete day</button>
              </div>
            </div>
            <div className="timeline">
              {activities.length === 0 ? (
                <p className="empty-state">No activity has been logged for this day.</p>
              ) : (
                activities.map(entry => (
                  <article className="timeline-row" key={entry.id}>
                    <div className="time-cell"><Clock3 size={16} /><span>{formatTime(entry.started_at)} - {formatTime(entry.ended_at)}</span></div>
                    <div className="row-main"><strong>{entry.app_name}</strong><span>{entry.window_title || 'Untitled window'}</span></div>
                    <div className="duration">{formatDuration(entry.duration_seconds)}</div>
                  </article>
                ))
              )}
            </div>
          </section>
        </section>

        {/* Right rail */}
        <aside className="right-rail">
          <section className="summary-panel" id="summaries">
            <div className="panel-heading compact">
              <h2>3-hour summaries</h2>
              {aiConfig.enabled ? <span className="ai-active-badge">AI active</span> : <span className="ai-disabled-badge">Disabled</span>}
            </div>

            {!aiConfig.enabled && (
              <div className="ai-empty-state">
                <Brain size={24} />
                <p>AI summaries are disabled.</p>
                <p className="ai-empty-hint">Enable in <a href="#ai-settings">AI Settings</a> to get AI-powered 3-hour block summaries.</p>
              </div>
            )}

            {aiConfig.enabled && !aiConfig.api_key && !apiKeyStatus?.has_env_var && !apiKeyStatus?.has_credential && (
              <div className="ai-empty-state warning">
                <AlertTriangle size={16} />
                <p>DeepSeek API key not found.</p>
                <p className="ai-empty-hint">Set <code>OPENJOURNAL_DEEPSEEK_API_KEY</code> or configure another provider in AI Settings.</p>
              </div>
            )}

            {aiConfig.enabled && (aiConfig.api_key || apiKeyStatus?.has_env_var || apiKeyStatus?.has_credential) && (
              <div className="ai-summary-list">
                {BLOCK_LABELS.map((_label, idx) => (
                  <SummaryCard
                    key={idx}
                    blockIndex={idx}
                    summary={getSummaryForBlock(idx)}
                    generating={generatingBlock === idx}
                    onGenerate={() => handleGenerateSummary(idx)}
                    onDelete={() => {
                      const s = getSummaryForBlock(idx);
                      if (s) handleDeleteSummary(s.id);
                    }}
                  />
                ))}
              </div>
            )}

            {aiConfig.enabled && !apiKeyStatus?.has_env_var && !apiKeyStatus?.has_credential && !aiConfig.api_key && aiConfig.provider === 'lm_studio' && (
              <div className="ai-empty-state">
                <WifiOff size={16} />
                <p>Configure LM Studio or Ollama to summarize locally.</p>
              </div>
            )}
          </section>

          {/* Blocklist */}
          <section className="blocklist-panel" id="blocklist">
            <h2>Blocklist</h2>
            <p>Skip private apps, domains, or title fragments before anything is stored.</p>
            <textarea
              aria-label="Private app and domain blocklist"
              value={blocklistText}
              onChange={e => setBlocklistText(e.target.value)}
            />
            <button className="primary-button full" onClick={saveBlocklist} type="button">Save blocklist</button>
          </section>

          {/* Footer */}
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
  <React.StrictMode><App /></React.StrictMode>,
);
