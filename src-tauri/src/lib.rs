mod activity_tracker;
mod credential;
mod export;
mod models;
mod provider;
mod scheduler;
mod storage;
mod summarizer;

use scheduler::Scheduler;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use activity_tracker::ActivityTracker;
use credential::{ApiKeyStatus, CredentialKey};
use models::{
    ActivityEntry, AiSummary, AppStatus, AutostartSetting, SchedulerSettings, SummaryBlock,
};
use provider::{create_provider, AiConfig, ConnectionTestResult};
use storage::Storage;
use summarizer::{aggregate_blocks, build_summary_prompt};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvProviderStatus {
    pub deepseek_key_found: bool,
    pub deepseek_key_masked: String,
    pub openjournal_key_found: bool,
}

struct AppState {
    storage: Storage,
    tracker: Arc<Mutex<ActivityTracker>>,
}

#[tauri::command]
fn get_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let tracker = state.tracker.lock().map_err(|_| "tracker lock failed")?;
    let autostart = state.storage.get_autostart_setting();
    let in_tauri =
        cfg!(target_os = "windows") || cfg!(target_os = "macos") || cfg!(target_os = "linux");
    Ok(AppStatus {
        logging_paused: tracker.is_paused(),
        active_window: tracker.active_window_label(),
        db_path: state.storage.db_path().display().to_string(),
        app_mode: if in_tauri && option_env!("TAURI_ENV_DEBUG").is_none() {
            "Installed".to_string()
        } else if std::env::var("__TAURI__").is_ok() {
            "Dev".to_string()
        } else {
            "Browser preview".to_string()
        },
        tracker_running: true,
        autostart_enabled: autostart.enabled,
        last_write_at: state.storage.get_last_recovery_at(),
        last_recovery_at: state.storage.get_last_recovery_at(),
    })
}

#[tauri::command]
fn set_logging_paused(paused: bool, state: State<'_, AppState>) -> Result<AppStatus, String> {
    let mut tracker = state.tracker.lock().map_err(|_| "tracker lock failed")?;
    tracker
        .set_paused(paused)
        .map_err(|error| error.to_string())?;
    Ok(AppStatus {
        logging_paused: tracker.is_paused(),
        active_window: tracker.active_window_label(),
        db_path: state.storage.db_path().display().to_string(),
        app_mode: String::new(),
        tracker_running: true,
        autostart_enabled: false,
        last_write_at: String::new(),
        last_recovery_at: String::new(),
    })
}

#[tauri::command]
fn get_day_activity(day: String, state: State<'_, AppState>) -> Result<Vec<ActivityEntry>, String> {
    state
        .storage
        .activity_for_day(&day)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_summary_blocks(
    day: String,
    state: State<'_, AppState>,
) -> Result<Vec<SummaryBlock>, String> {
    let activities = state
        .storage
        .activity_for_day(&day)
        .map_err(|error| error.to_string())?;
    Ok(summarizer::placeholder_three_hour_blocks(&day, &activities))
}

#[tauri::command]
fn set_blocklist(entries: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    state
        .storage
        .replace_blocklist(&entries)
        .map_err(|error| error.to_string())?;
    state
        .tracker
        .lock()
        .map_err(|_| "tracker lock failed")?
        .reload_blocklist()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn export_day(day: String, format: String, state: State<'_, AppState>) -> Result<String, String> {
    let activities = state
        .storage
        .activity_for_day(&day)
        .map_err(|error| error.to_string())?;
    let summaries = summarizer::placeholder_three_hour_blocks(&day, &activities);
    let path = match format.as_str() {
        "markdown" => export::export_markdown(&day, &activities, &summaries),
        "json" => export::export_json(&day, &activities, &summaries),
        _ => return Err("unsupported export format".to_string()),
    }
    .map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn delete_day(day: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .storage
        .delete_day(&day)
        .map_err(|error| error.to_string())
}

// v0.2 AI summary commands

#[tauri::command]
fn get_ai_config(state: State<'_, AppState>) -> Result<AiConfig, String> {
    let mut config = state.storage.get_ai_config().map_err(|e| e.to_string())?;
    config.api_key = String::new();
    Ok(config)
}

#[tauri::command]
fn set_ai_config(config: AiConfig, state: State<'_, AppState>) -> Result<(), String> {
    if !config.api_key.is_empty() {
        credential::save_credential(&CredentialKey::DeepSeek, &config.api_key)
            .map_err(|e| format!("Failed to save API key: {e}"))?;
    }
    let mut safe = config.clone();
    safe.api_key = String::new();
    state
        .storage
        .set_ai_config(&safe)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_ai_connection(config: AiConfig) -> Result<ConnectionTestResult, String> {
    let provider = create_provider(&config).map_err(|e| e.to_string())?;
    let result = tokio::task::spawn_blocking(move || provider.test_connection())
        .await
        .map_err(|e| format!("task failed: {e}"))?
        .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
async fn generate_ai_summary(
    day: String,
    block_index: i32,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut config = state.storage.get_ai_config().map_err(|e| e.to_string())?;
    if !config.enabled {
        return Err("AI summaries are disabled".to_string());
    }
    let (resolved_key, _source) = credential::resolve_api_key(&config.api_key);
    if resolved_key.is_empty() {
        return Err("No API key found.".to_string());
    }
    config.api_key = resolved_key;
    let activities = state
        .storage
        .activity_for_day(&day)
        .map_err(|e| e.to_string())?;
    let blocks = aggregate_blocks(&day, &activities);
    let block = blocks
        .into_iter()
        .find(|b| {
            let label = format!("{:02}:00", block_index * 3);
            b.block_start == label
        })
        .ok_or_else(|| format!("No data for block index {block_index}"))?;
    if block.entries.is_empty() {
        return Err("No activity in this block".to_string());
    }
    let prompt = build_summary_prompt(&block);
    let provider = create_provider(&config).map_err(|e| e.to_string())?;
    let result = tokio::task::spawn_blocking(move || provider.generate_summary(&prompt))
        .await
        .map_err(|e| format!("task failed: {e}"))?
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let summary_json = serde_json::to_string(&result).unwrap_or_default();
    let ai_summary = AiSummary {
        id: 0,
        day: day.clone(),
        block_index,
        block_start: result.block_start.clone(),
        block_end: result.block_end.clone(),
        summary_json,
        model_name: result.model_used.clone(),
        generated_at: now.clone(),
        token_count: None,
        status: "completed".to_string(),
        error_message: None,
        retry_count: 0,
        last_attempt_at: Some(now),
        generation_source: "manual".to_string(),
        queue_status: "idle".to_string(),
    };
    state
        .storage
        .upsert_ai_summary(&ai_summary)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&result).unwrap_or_default())
}

#[tauri::command]
fn get_ai_summaries(day: String, state: State<'_, AppState>) -> Result<Vec<AiSummary>, String> {
    state
        .storage
        .get_ai_summaries_for_day(&day)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_ai_summary(summary_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state
        .storage
        .delete_ai_summary(summary_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_environment_provider_status() -> Result<EnvProviderStatus, String> {
    let ds = std::env::var("DEEPSEEK_API_KEY").ok();
    let oj = std::env::var("OPENJOURNAL_DEEPSEEK_API_KEY").ok();
    let preferred = oj.clone().or_else(|| ds.clone());
    let masked = preferred
        .as_ref()
        .map(|k| {
            if k.len() > 8 {
                format!("sk-••••••••{}", &k[k.len() - 4..])
            } else {
                "sk-••••".to_string()
            }
        })
        .unwrap_or_default();
    Ok(EnvProviderStatus {
        deepseek_key_found: ds.is_some(),
        deepseek_key_masked: masked,
        openjournal_key_found: oj.is_some(),
    })
}

#[tauri::command]
fn get_masked_api_key() -> Result<String, String> {
    let key = std::env::var("OPENJOURNAL_DEEPSEEK_API_KEY")
        .ok()
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok());
    match key {
        Some(k) if !k.is_empty() => {
            if k.len() > 8 {
                Ok(format!("sk-••••••••{}", &k[k.len() - 4..]))
            } else {
                Ok("sk-••••".to_string())
            }
        }
        _ => Ok(String::new()),
    }
}

#[tauri::command]
fn get_api_key_status() -> Result<ApiKeyStatus, String> {
    Ok(credential::get_api_key_status(""))
}

#[tauri::command]
fn save_credential_api_key(key: String) -> Result<(), String> {
    credential::save_credential(&CredentialKey::DeepSeek, &key)
        .map_err(|e| format!("Failed to save: {e}"))
}

#[tauri::command]
fn delete_credential_api_key() -> Result<(), String> {
    credential::delete_credential(&CredentialKey::DeepSeek)
        .map_err(|e| format!("Failed to delete: {e}"))
}

// v0.3 Scheduler commands

#[tauri::command]
fn get_scheduler_settings(state: State<'_, AppState>) -> Result<SchedulerSettings, String> {
    state
        .storage
        .get_scheduler_settings()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_scheduler_settings(
    settings: SchedulerSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .storage
        .set_scheduler_settings(&settings)
        .map_err(|e| e.to_string())
}

// v0.3.1 Autostart commands

#[tauri::command]
fn get_autostart_setting(state: State<'_, AppState>) -> Result<AutostartSetting, String> {
    Ok(state.storage.get_autostart_setting())
}

#[tauri::command]
fn set_autostart_setting(
    setting: AutostartSetting,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .storage
        .set_autostart_setting(&setting)
        .map_err(|e| e.to_string())
}

fn app_data_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Enable or disable Windows autostart via HKCU registry Run key.
fn set_windows_autostart(exe_path: &str, enabled: bool) {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let key = r"Software\Microsoft\Windows\CurrentVersion\Run";
        if let Ok(run) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(key, winreg::enums::KEY_SET_VALUE)
        {
            if enabled {
                let _ = run.set_value("OpenJournal", &exe_path);
            } else {
                let _ = run.delete_value("OpenJournal");
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (exe_path, enabled);
    }
}

fn install_tray(
    app: &tauri::App,
    storage: Storage,
    tracker: Arc<Mutex<ActivityTracker>>,
) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open OpenJournal", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause logging", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume logging", true, None::<&str>)?;
    let generate = MenuItem::with_id(
        app,
        "generate",
        "Generate summaries now",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &generate, &sep, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("OpenJournal")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "pause" => {
                if let Ok(mut t) = tracker.lock() {
                    let _ = t.set_paused(true);
                }
            }
            "resume" => {
                if let Ok(mut t) = tracker.lock() {
                    let _ = t.set_paused(false);
                }
            }
            "generate" => {
                // Trigger scheduler tick
                let sched = Scheduler::new(storage.clone());
                tauri::async_runtime::spawn(async move {
                    let _ = sched.tick().await;
                });
            }
            "quit" => {
                // Flush tracker before exit
                if let Ok(mut t) = tracker.lock() {
                    let _ = t.flush_current();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app_data_dir(app.handle())?;
            let storage = Storage::open(data_dir.join("openjournal.sqlite3"))?;
            storage.migrate()?;
            let _ = storage.migrate_plaintext_keys();
            // Crash recovery: finalize any open entries from last session
            let recovered = storage.recover_open_entries().unwrap_or(0);
            if recovered > 0 {
                eprintln!(
                    "[OpenJournal] Recovered {recovered} activity entries from previous session."
                );
            }
            let tracker = Arc::new(Mutex::new(ActivityTracker::new(storage.clone())?));
            install_tray(app, storage.clone(), tracker.clone())?;
            activity_tracker::spawn_poll_loop(tracker.clone());
            let scheduler = Arc::new(Scheduler::new(storage.clone()));
            scheduler.spawn_background_loop();
            scheduler.spawn_startup_catchup();
            app.manage(AppState {
                storage: storage.clone(),
                tracker: tracker.clone(),
            });

            // Close to tray: hide window instead of quitting
            // In installed (release) mode, start hidden — tray-first UX
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                // Hide on launch for installed builds
                if !cfg!(debug_assertions) {
                    let _ = window.hide();
                }
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = window_clone.hide();
                        api.prevent_close();
                    }
                });
            }

            // Sync autostart setting on launch
            let auto_setting = storage.get_autostart_setting();
            if let Ok(exe) = std::env::current_exe() {
                set_windows_autostart(&exe.display().to_string(), auto_setting.enabled);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_status,
            set_logging_paused,
            get_day_activity,
            get_summary_blocks,
            set_blocklist,
            export_day,
            delete_day,
            get_ai_config,
            set_ai_config,
            test_ai_connection,
            generate_ai_summary,
            get_ai_summaries,
            delete_ai_summary,
            get_environment_provider_status,
            get_masked_api_key,
            get_api_key_status,
            save_credential_api_key,
            delete_credential_api_key,
            get_scheduler_settings,
            set_scheduler_settings,
            get_autostart_setting,
            set_autostart_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenJournal");
}
