use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::{NaiveDate, Timelike};

use crate::models::{ActivityEntry, SummaryBlock};

#[allow(dead_code)]
pub trait SummaryProvider {
    fn summarize_block(&self, entries: &[ActivityEntry]) -> anyhow::Result<SummaryBlock>;
}

#[allow(dead_code)]
pub struct LocalOpenAiCompatibleProvider {
    pub endpoint: String,
    pub model: String,
}

impl SummaryProvider for LocalOpenAiCompatibleProvider {
    fn summarize_block(&self, _entries: &[ActivityEntry]) -> anyhow::Result<SummaryBlock> {
        anyhow::bail!(
            "local summarization provider is scaffolded for v0.2; configure LM Studio endpoint {} and model {} before enabling",
            self.endpoint,
            self.model
        )
    }
}

pub fn placeholder_three_hour_blocks(day: &str, activities: &[ActivityEntry]) -> Vec<SummaryBlock> {
    let Ok(date) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
        return Vec::new();
    };

    let mut groups: BTreeMap<u32, Vec<ActivityEntry>> = BTreeMap::new();
    for activity in activities {
        let parsed = chrono::DateTime::parse_from_rfc3339(&activity.started_at);
        if let Ok(started_at) = parsed {
            let block = (started_at.hour() / 3) * 3;
            groups.entry(block).or_default().push(activity.clone());
        }
    }

    (0u32..24)
        .step_by(3)
        .map(|hour| {
            let entries = groups.remove(&hour).unwrap_or_default();
            let block_start = date
                .and_hms_opt(hour, 0, 0)
                .expect("valid summary block start");
            let block_end = block_start + chrono::Duration::hours(3);
            summarize_placeholder(
                block_start.format("%H:%M").to_string(),
                block_end.format("%H:%M").to_string(),
                &entries,
            )
        })
        .collect()
}

fn summarize_placeholder(
    block_start: String,
    block_end: String,
    entries: &[ActivityEntry],
) -> SummaryBlock {
    if entries.is_empty() {
        return SummaryBlock {
            block_start,
            block_end,
            main_focus: "No activity logged".to_string(),
            apps_projects: Vec::new(),
            context_switches: 0,
            productivity_notes: vec!["No local summary was generated for this block.".to_string()],
            plain_english_summary:
                "OpenJournal did not record focused window activity during this period.".to_string(),
            provider: "placeholder".to_string(),
        };
    }

    let mut app_seconds: BTreeMap<String, i64> = BTreeMap::new();
    for entry in entries {
        *app_seconds.entry(entry.app_name.clone()).or_default() += entry.duration_seconds;
    }
    let mut apps: Vec<_> = app_seconds.into_iter().collect();
    apps.sort_by_key(|entry| Reverse(entry.1));
    let app_names: Vec<String> = apps.iter().map(|(name, _)| name.clone()).take(5).collect();
    let top_app = app_names
        .first()
        .cloned()
        .unwrap_or_else(|| "your desktop".to_string());

    SummaryBlock {
        block_start,
        block_end,
        main_focus: format!("Focused work centered on {top_app}"),
        apps_projects: app_names,
        context_switches: entries.len().saturating_sub(1),
        productivity_notes: vec![
            "Placeholder summary generated locally from metadata only.".to_string(),
            "Enable an LM Studio/OpenAI-compatible local endpoint in v0.2 for richer summaries."
                .to_string(),
        ],
        plain_english_summary: format!(
            "You used {} focused windows in this 3-hour block. No data was sent externally.",
            entries.len()
        ),
        provider: "placeholder".to_string(),
    }
}
