use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: i64,
    pub app_name: String,
    pub window_title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub logging_paused: bool,
    pub active_window: String,
    pub db_path: String,
    pub app_mode: String,
    pub tracker_running: bool,
    pub autostart_enabled: bool,
    pub last_write_at: String,
    pub last_recovery_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartSetting {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryBlock {
    pub block_start: String,
    pub block_end: String,
    pub main_focus: String,
    pub apps_projects: Vec<String>,
    pub context_switches: usize,
    pub productivity_notes: Vec<String>,
    pub plain_english_summary: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub day: String,
    pub activities: Vec<ActivityEntry>,
    pub summaries: Vec<SummaryBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummary {
    pub id: i64,
    pub day: String,
    pub block_index: i32,
    pub block_start: String,
    pub block_end: String,
    pub summary_json: String,
    pub model_name: String,
    pub generated_at: String,
    pub token_count: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub last_attempt_at: Option<String>,
    pub generation_source: String,
    pub queue_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSettings {
    pub auto_generate: bool,
    pub generate_on_startup: bool,
    pub retry_failed: bool,
}

impl Default for SchedulerSettings {
    fn default() -> Self {
        Self {
            auto_generate: true,
            generate_on_startup: true,
            retry_failed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockActivity {
    pub block_start: String,
    pub block_end: String,
    pub entries: Vec<ActivityEntry>,
    pub total_focus_seconds: i64,
    pub context_switches: usize,
    pub app_breakdown: Vec<(String, i64)>,
    pub idle_minutes: i64,
}
