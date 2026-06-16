use std::sync::Arc;
use std::time::Duration;

use chrono::Timelike;
use chrono::Utc;

use crate::credential::resolve_api_key;
use crate::models::AiSummary;
use crate::provider::{create_provider, AiConfig};
use crate::storage::Storage;
use crate::summarizer::{aggregate_blocks, build_summary_prompt};

/// Background scheduler for automatic summary generation.
/// Runs every 15 minutes, never blocks tracking.

pub struct Scheduler {
    storage: Storage,
}

impl Scheduler {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub fn spawn_background_loop(self: &Arc<Self>) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(900)).await; // 15 min
                let _ = scheduler.tick().await;
            }
        });
    }

    pub fn spawn_startup_catchup(self: &Arc<Self>) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await; // wait for app init
            let settings = scheduler.storage.get_scheduler_settings().ok();
            if settings.map_or(false, |s| s.generate_on_startup) {
                let _ = scheduler.generate_missing_days(7).await;
            }
        });
    }

    /// Main scheduler tick: find completed blocks and generate missing summaries.
    pub async fn tick(&self) -> anyhow::Result<()> {
        let settings = self.storage.get_scheduler_settings()?;
        if !settings.auto_generate {
            return Ok(());
        }
        let config = self.storage.get_ai_config()?;
        if !config.enabled {
            return Ok(());
        }

        let (resolved_key, _source) = resolve_api_key("");
        if resolved_key.is_empty() {
            return Ok(()); // no key available, skip this tick
        }

        let pending = self.storage.find_pending_summary_blocks(1)?;
        for (day, block_index) in pending {
            if block_index < 0 {
                continue; // skip block -3 (first incomplete block's previous)
            }
            if self.is_block_active(day.as_str(), block_index) {
                continue; // never generate for active/incomplete blocks
            }
            if self.has_reached_retry_limit(day.as_str(), block_index) {
                continue;
            }

            let mut ai_config = config.clone();
            ai_config.api_key = resolved_key.clone();
            let result = self
                .generate_block_summary(&day, block_index, &ai_config)
                .await;
            match result {
                Ok(_) => {
                    eprintln!(
                        "[OpenJournal Scheduler] Generated summary for {}/block-{}",
                        day, block_index
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[OpenJournal Scheduler] Failed to generate {}/block-{}: {}",
                        day, block_index, e
                    );
                    self.increment_retry(day.as_str(), block_index, &e.to_string())
                        .ok();
                }
            }
        }
        Ok(())
    }

    /// Generate missing summaries for the last N days (startup catch-up).
    pub async fn generate_missing_days(&self, days_back: i64) -> anyhow::Result<()> {
        let config = self.storage.get_ai_config()?;
        if !config.enabled {
            return Ok(());
        }
        let (resolved_key, _source) = resolve_api_key("");
        if resolved_key.is_empty() {
            return Ok(());
        }

        let pending = self.storage.find_pending_summary_blocks(days_back)?;
        let mut ai_config = config.clone();
        ai_config.api_key = resolved_key;

        for (day, block_index) in &pending {
            if *block_index < 0 {
                continue;
            }
            if self.has_reached_retry_limit(day, *block_index) {
                continue;
            }
            if self
                .generate_block_summary(day, *block_index, &ai_config)
                .await
                .is_err()
            {
                // Continue with next block — best-effort
                continue;
            }
            // Small delay between generations to avoid rate limits
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Ok(())
    }

    /// Generate a summary for one block and store it.
    async fn generate_block_summary(
        &self,
        day: &str,
        block_index: i32,
        config: &AiConfig,
    ) -> anyhow::Result<()> {
        let activities = self.storage.activity_for_day(day)?;
        let blocks = aggregate_blocks(day, &activities);
        let block = blocks
            .into_iter()
            .find(|b| b.block_start == format!("{:02}:00", block_index * 3))
            .ok_or_else(|| anyhow::anyhow!("No data for block"))?;

        if block.entries.is_empty() {
            return Ok(()); // skip empty blocks
        }

        let prompt = build_summary_prompt(&block);
        let provider = create_provider(config)?;

        let result = tokio::task::spawn_blocking(move || provider.generate_summary(&prompt))
            .await
            .map_err(|e| anyhow::anyhow!("Task failed: {e}"))?
            .map_err(|e| anyhow::anyhow!("Generation failed: {e}"))?;

        let now = Utc::now().to_rfc3339();
        let summary_json = serde_json::to_string(&result).unwrap_or_default();
        let summary = AiSummary {
            id: 0,
            day: day.to_string(),
            block_index,
            block_start: result.block_start,
            block_end: result.block_end,
            summary_json,
            model_name: result.model_used,
            generated_at: now,
            token_count: None,
            status: "completed".to_string(),
            error_message: None,
            retry_count: 0,
            last_attempt_at: Some(Utc::now().to_rfc3339()),
            generation_source: "automatic".to_string(),
            queue_status: "idle".to_string(),
        };
        self.storage.upsert_ai_summary(&summary)?;
        Ok(())
    }

    fn is_block_active(&self, day: &str, block_index: i32) -> bool {
        let now = Utc::now();
        let block_hour = (block_index as u32) * 3;
        let block_end_hour = block_hour + 3;
        let day_match = now.format("%Y-%m-%d").to_string() == day;
        let hour = now.hour();
        day_match && hour < block_end_hour
    }

    fn has_reached_retry_limit(&self, day: &str, block_index: i32) -> bool {
        if let Ok(summaries) = self.storage.get_ai_summaries_for_day(day) {
            for s in &summaries {
                if s.block_index == block_index && s.retry_count >= 2 {
                    return true;
                }
            }
        }
        false
    }

    fn increment_retry(&self, day: &str, block_index: i32, error: &str) -> anyhow::Result<()> {
        let mut summaries = self.storage.get_ai_summaries_for_day(day)?;
        if let Some(s) = summaries.iter_mut().find(|s| s.block_index == block_index) {
            s.retry_count += 1;
            s.last_attempt_at = Some(Utc::now().to_rfc3339());
            s.error_message = Some(error.to_string());
            if s.retry_count >= 2 {
                s.status = "failed".to_string();
            }
            self.storage.upsert_ai_summary(s)?;
        }
        Ok(())
    }
}
