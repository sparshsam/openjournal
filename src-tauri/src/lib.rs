mod activity_tracker;
mod export;
mod models;
mod provider;
mod storage;
mod summarizer;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use activity_tracker::ActivityTracker;
use models::{ActivityEntry, AiSummary, AppStatus, SummaryBlock};
use provider::{create_provider, AiConfig, ConnectionTestResult};
use storage::Storage;
use summarizer::{aggregate_blocks, build_summary_prompt};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

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
    Ok(AppStatus {
        logging_paused: tracker.is_paused(),
        active_window: tracker.active_window_label(),
        db_path: state.storage.db_path().display().to_string(),
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

// -----------------------------------------------------------------------
// v0.2 AI summary commands
// -----------------------------------------------------------------------

#[tauri::command]
fn get_ai_config(state: State<'_, AppState>) -> Result<AiConfig, String> {
    state
        .storage
        .get_ai_config()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_ai_config(config: AiConfig, state: State<'_, AppState>) -> Result<(), String> {
    state
        .storage
        .set_ai_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_ai_connection(config: AiConfig) -> Result<ConnectionTestResult, String> {
    let provider = create_provider(&config).map_err(|e| e.to_string())?;
    // Spawn blocking I/O on a background thread
    let result = tokio::task::spawn_blocking(move || provider.test_connection())
        .await
        .map_err(|e| format!("test connection task failed: {e}"))?
        .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
async fn generate_ai_summary(
    day: String,
    block_index: i32,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.storage.get_ai_config().map_err(|e| e.to_string())?;
    if !config.enabled {
        return Err("AI summaries are disabled".to_string());
    }

    let activities = state
        .storage
        .activity_for_day(&day)
        .map_err(|e| e.to_string())?;
    let blocks = aggregate_blocks(&day, &activities);
    let block = blocks
        .into_iter()
        .find(|b| {
            let hour = (block_index as u32) * 3;
            // Match by block_start format "HH:MM"
            let label = format!("{:02}:00", hour);
            b.block_start == label
        })
        .ok_or_else(|| format!("No activity data for block index {block_index}"))?;

    if block.entries.is_empty() {
        return Err("No activity in this block".to_string());
    }

    let prompt = build_summary_prompt(&block);
    let provider = create_provider(&config).map_err(|e| e.to_string())?;

    let result = tokio::task::spawn_blocking(move || provider.generate_summary(&prompt))
        .await
        .map_err(|e| format!("summary task failed: {e}"))?
        .map_err(|e| e.to_string())?;

    // Store to DB
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
        generated_at: now,
        token_count: None,
        status: "completed".to_string(),
        error_message: None,
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

fn app_data_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn install_tray(app: &tauri::App, state: Arc<Mutex<ActivityTracker>>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show OpenJournal", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause/Resume logging", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &quit])?;

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
                if let Ok(mut tracker) = state.lock() {
                    let next = !tracker.is_paused();
                    let _ = tracker.set_paused(next);
                }
            }
            "quit" => app.exit(0),
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
            let tracker = Arc::new(Mutex::new(ActivityTracker::new(storage.clone())?));
            install_tray(app, tracker.clone())?;
            activity_tracker::spawn_poll_loop(tracker.clone());
            app.manage(AppState { storage, tracker });
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
            // v0.2 AI commands
            get_ai_config,
            set_ai_config,
            test_ai_connection,
            generate_ai_summary,
            get_ai_summaries,
            delete_ai_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenJournal");
}
