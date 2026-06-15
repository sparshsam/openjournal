use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::{NaiveDate, Timelike};

use crate::models::{ActivityEntry, BlockActivity, SummaryBlock};
use crate::provider::{create_provider, AiConfig, GeneratedSummary};

// ---------------------------------------------------------------------------
// Prompt builder — separate from provider logic
// ---------------------------------------------------------------------------

/// Build a structured prompt for a single 3-hour block.
pub fn build_summary_prompt(block: &BlockActivity) -> String {
    let app_lines: Vec<String> = block
        .app_breakdown
        .iter()
        .map(|(app, secs)| {
            let mins = secs / 60;
            format!("  - {app}: {mins} minutes")
        })
        .collect();

    format!(
        r#"Analyze this 3-hour activity block and return a JSON object with these fields:
  - "block_start": "{start}"
  - "block_end": "{end}"
  - "main_focus": short title of the main work focus
  - "apps_projects": ["app1", "app2", ...]
  - "context_switches": number
  - "total_focus_minutes": total tracked minutes
  - "productivity_notes": ["note1", "note2"]
  - "plain_english_summary": 2-3 sentence summary

Activity data for this block:
  Total tracked time: {total_minutes} minutes
  Context switches (window changes): {switches}
  Idle (no tracked window): {idle} minutes

Apps used:
{apps}

Return ONLY valid JSON. No markdown fences."#,
        start = block.block_start,
        end = block.block_end,
        total_minutes = block.total_focus_seconds / 60,
        switches = block.context_switches,
        idle = block.idle_minutes,
        apps = app_lines.join("\n"),
    )
}

// ---------------------------------------------------------------------------
// Block aggregator
// ---------------------------------------------------------------------------

/// Aggregate activity entries into 3-hour blocks with computed statistics.
pub fn aggregate_blocks(day: &str, activities: &[ActivityEntry]) -> Vec<BlockActivity> {
    let Ok(date) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
        return Vec::new();
    };

    let mut groups: BTreeMap<u32, Vec<ActivityEntry>> = BTreeMap::new();
    for activity in activities {
        if let Ok(started_at) = chrono::DateTime::parse_from_rfc3339(&activity.started_at) {
            let block = (started_at.hour() / 3) * 3;
            groups.entry(block).or_default().push(activity.clone());
        }
    }

    let mut blocks = Vec::new();
    for hour in (0u32..24).step_by(3) {
        let entries = groups.remove(&hour).unwrap_or_default();
        let block_start = date
            .and_hms_opt(hour, 0, 0)
            .expect("valid summary block start");
        let block_end = block_start + chrono::Duration::hours(3);

        let total_focus_seconds: i64 = entries.iter().map(|e| e.duration_seconds).sum();

        let mut app_seconds: BTreeMap<String, i64> = BTreeMap::new();
        for entry in &entries {
            *app_seconds.entry(entry.app_name.clone()).or_default() += entry.duration_seconds;
        }
        let mut app_breakdown: Vec<(String, i64)> = app_seconds.into_iter().collect();
        app_breakdown.sort_by_key(|entry| Reverse(entry.1));

        // Idle: 3 hours in seconds minus tracked focus
        let block_seconds = 3 * 3600;
        let idle_seconds = (block_seconds - total_focus_seconds).max(0);

        let context_switches = entries.len().saturating_sub(1);

        blocks.push(BlockActivity {
            block_start: block_start.format("%H:%M").to_string(),
            block_end: block_end.format("%H:%M").to_string(),
            entries,
            total_focus_seconds,
            context_switches,
            app_breakdown,
            idle_minutes: idle_seconds / 60,
        });
    }
    blocks
}

// ---------------------------------------------------------------------------
// Placeholder (non-AI) summaries
// ---------------------------------------------------------------------------

pub fn placeholder_three_hour_blocks(day: &str, activities: &[ActivityEntry]) -> Vec<SummaryBlock> {
    aggregate_blocks(day, activities)
        .into_iter()
        .map(|block| summarize_placeholder(&block))
        .collect()
}

fn summarize_placeholder(block: &BlockActivity) -> SummaryBlock {
    if block.entries.is_empty() {
        return SummaryBlock {
            block_start: block.block_start.clone(),
            block_end: block.block_end.clone(),
            main_focus: "No activity logged".to_string(),
            apps_projects: Vec::new(),
            context_switches: 0,
            productivity_notes: vec![
                "No local summary was generated for this block.".to_string(),
            ],
            plain_english_summary: "OpenJournal did not record focused window activity during this period."
                .to_string(),
            provider: "placeholder".to_string(),
        };
    }

    let top_app = block
        .app_breakdown
        .first()
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| "your desktop".to_string());

    let app_names: Vec<String> = block
        .app_breakdown
        .iter()
        .map(|(name, _)| name.clone())
        .take(5)
        .collect();

    SummaryBlock {
        block_start: block.block_start.clone(),
        block_end: block.block_end.clone(),
        main_focus: format!("Focused work centered on {top_app}"),
        apps_projects: app_names,
        context_switches: block.context_switches,
        productivity_notes: vec![
            "Placeholder summary generated locally from metadata only.".to_string(),
            "Enable an LM Studio/OpenAI-compatible local endpoint in v0.2 for richer summaries."
                .to_string(),
        ],
        plain_english_summary: format!(
            "You used {} focused windows in this 3-hour block. No data was sent externally.",
            block.entries.len()
        ),
        provider: "placeholder".to_string(),
    }
}

// ---------------------------------------------------------------------------
// AI generation — runs in a spawned task
// ---------------------------------------------------------------------------

/// Generate a summary for one block using the configured provider.
/// Returns None if AI is disabled.
pub async fn generate_ai_summary(
    config: &AiConfig,
    block: &BlockActivity,
) -> anyhow::Result<GeneratedSummary> {
    if !config.enabled {
        anyhow::bail!("AI summaries are disabled");
    }
    let prompt = build_summary_prompt(block);
    let provider = create_provider(config)?;
    provider.generate_summary(&prompt)
}

/// Generate AI summaries for all blocks that have activity.
/// Returns (completed, failed) counts.
pub async fn generate_all_block_summaries(
    config: &AiConfig,
    blocks: &[BlockActivity],
) -> (usize, usize) {
    if !config.enabled || blocks.is_empty() {
        return (0, 0);
    }
    let mut completed = 0usize;
    let mut failed = 0usize;
    for block in blocks {
        if block.entries.is_empty() {
            continue;
        }
        match generate_ai_summary(config, block).await {
            Ok(_) => completed += 1,
            Err(_) => {
                failed += 1;
                if failed >= 2 {
                    // stop after 2 consecutive failures
                    break;
                }
            }
        }
    }
    (completed, failed)
}
