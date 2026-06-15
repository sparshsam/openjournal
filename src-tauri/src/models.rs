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

/// AI-generated summary record stored in the DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummary {
    pub id: i64,
    pub day: String,
    pub block_index: i32, // 0-7 (8 x 3-hour blocks)
    pub block_start: String,
    pub block_end: String,
    pub summary_json: String, // full JSON payload from provider
    pub model_name: String,
    pub generated_at: String,
    pub token_count: Option<i64>,
    pub status: String, // pending | completed | failed
    pub error_message: Option<String>,
}

/// The aggregated data sent to the prompt builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockActivity {
    pub block_start: String,
    pub block_end: String,
    pub entries: Vec<ActivityEntry>,
    pub total_focus_seconds: i64,
    pub context_switches: usize,
    pub app_breakdown: Vec<(String, i64)>, // (app_name, total_seconds)
    pub idle_minutes: i64,
}
