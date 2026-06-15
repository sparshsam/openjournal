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
