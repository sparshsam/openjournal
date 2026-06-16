use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::credential::{self, CredentialKey};
use crate::models::ActivityEntry;
use crate::models::AiSummary;
use crate::models::AutostartSetting;
use crate::models::SchedulerSettings;
use crate::provider::AiConfig;

#[derive(Clone)]
pub struct Storage {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&path)?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.path
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS activity_entries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              app_name TEXT NOT NULL,
              window_title TEXT NOT NULL,
              started_at TEXT NOT NULL,
              ended_at TEXT,
              duration_seconds INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_activity_started_at
              ON activity_entries(started_at);

            CREATE TABLE IF NOT EXISTS blocklist_entries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              pattern TEXT NOT NULL UNIQUE,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS summary_blocks (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              day TEXT NOT NULL,
              block_start TEXT NOT NULL,
              block_end TEXT NOT NULL,
              provider TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(day, block_start, provider)
            );

            -- v0.2: AI summary storage
            CREATE TABLE IF NOT EXISTS ai_summaries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              day TEXT NOT NULL,
              block_index INTEGER NOT NULL,
              block_start TEXT NOT NULL,
              block_end TEXT NOT NULL,
              summary_json TEXT NOT NULL DEFAULT '{}',
              model_name TEXT NOT NULL DEFAULT '',
              generated_at TEXT,
              token_count INTEGER,
              status TEXT NOT NULL DEFAULT 'pending',
              error_message TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(day, block_index, model_name)
            );

            -- v0.2: AI provider config (singleton row)
            CREATE TABLE IF NOT EXISTS ai_config (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );

            -- v0.3: Additional fields for ai_summaries (safe ALTER TABLE)
            -- Must be separate statements because SQLite batch may fail on duplicate ALTER.
            "#,
        )?;
        // v0.3: Add columns if they don't exist (each ALTER TABLE in its own exec)
        let _ = connection.execute(
            "ALTER TABLE ai_summaries ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE ai_summaries ADD COLUMN last_attempt_at TEXT",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE ai_summaries ADD COLUMN generation_source TEXT NOT NULL DEFAULT 'manual'",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE ai_summaries ADD COLUMN queue_status TEXT NOT NULL DEFAULT 'idle'",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE activity_entries ADD COLUMN last_seen_at TEXT",
            [],
        );
        Ok(())
    }

    pub fn start_activity(
        &self,
        app_name: &str,
        window_title: &str,
        started_at: DateTime<Utc>,
    ) -> anyhow::Result<i64> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"
            INSERT INTO activity_entries
              (app_name, window_title, started_at, ended_at, duration_seconds)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                app_name,
                window_title,
                started_at.to_rfc3339(),
                started_at.to_rfc3339(),
                0
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    pub fn update_activity_end(
        &self,
        id: i64,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let duration_seconds = (ended_at - started_at).num_seconds().max(0);
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"
            UPDATE activity_entries
            SET ended_at = ?1, duration_seconds = ?2
            WHERE id = ?3
            "#,
            params![ended_at.to_rfc3339(), duration_seconds, id],
        )?;
        Ok(())
    }

    pub fn activity_for_day(&self, day: &str) -> anyhow::Result<Vec<ActivityEntry>> {
        let date = NaiveDate::parse_from_str(day, "%Y-%m-%d")?;
        let start = date.and_hms_opt(0, 0, 0).expect("valid midnight").and_utc();
        let end = start + Duration::days(1);
        let connection = self.connection.lock().expect("storage lock failed");
        let mut statement = connection.prepare(
            r#"
            SELECT id, app_name, window_title, started_at, ended_at, duration_seconds
            FROM activity_entries
            WHERE started_at >= ?1 AND started_at < ?2
            ORDER BY started_at ASC
            "#,
        )?;
        let rows = statement.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            Ok(ActivityEntry {
                id: row.get(0)?,
                app_name: row.get(1)?,
                window_title: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    pub fn delete_day(&self, day: &str) -> anyhow::Result<()> {
        let date = NaiveDate::parse_from_str(day, "%Y-%m-%d")?;
        let start = date.and_hms_opt(0, 0, 0).expect("valid midnight").and_utc();
        let end = start + Duration::days(1);
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            "DELETE FROM activity_entries WHERE started_at >= ?1 AND started_at < ?2",
            params![start.to_rfc3339(), end.to_rfc3339()],
        )?;
        connection.execute("DELETE FROM summary_blocks WHERE day = ?1", params![day])?;
        Ok(())
    }

    pub fn replace_blocklist(&self, entries: &[String]) -> anyhow::Result<()> {
        let mut connection = self.connection.lock().expect("storage lock failed");
        let tx = connection.transaction()?;
        tx.execute("DELETE FROM blocklist_entries", [])?;
        for entry in entries {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                tx.execute(
                    "INSERT OR IGNORE INTO blocklist_entries(pattern) VALUES (?1)",
                    params![trimmed],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn blocklist(&self) -> anyhow::Result<Vec<String>> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut statement =
            connection.prepare("SELECT pattern FROM blocklist_entries ORDER BY pattern ASC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?.to_lowercase());
        }
        Ok(entries)
    }

    pub fn setting_bool(&self, key: &str, fallback: bool) -> anyhow::Result<bool> {
        let connection = self.connection.lock().expect("storage lock failed");
        let value = connection.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match value {
            Ok(value) => Ok(value == "true"),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(fallback),
            Err(error) => Err(error.into()),
        }
    }

    pub fn set_setting_bool(&self, key: &str, value: bool) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"
            INSERT INTO settings(key, value, updated_at)
            VALUES (?1, ?2, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
            "#,
            params![key, if value { "true" } else { "false" }],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // AI config
    // -----------------------------------------------------------------------

    pub fn get_ai_config(&self) -> anyhow::Result<AiConfig> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection.prepare("SELECT value FROM ai_config WHERE key = ?1")?;
        let json_str: Option<String> = stmt.query_row(params!["config"], |row| row.get(0)).ok();
        drop(stmt);
        match json_str {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(AiConfig::default()),
        }
    }

    pub fn set_ai_config(&self, config: &AiConfig) -> anyhow::Result<()> {
        // Strip API key — never store in SQLite
        let mut safe = config.clone();
        safe.api_key = String::new();
        let json = serde_json::to_string(&safe)?;
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"
            INSERT INTO ai_config(key, value)
            VALUES ('config', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![json],
        )?;
        Ok(())
    }

    /// Migrate any existing plaintext API key from the old ai_config table
    /// into the OS credential store, then clear it.
    pub fn migrate_plaintext_keys(&self) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection.prepare("SELECT value FROM ai_config WHERE key = 'config'")?;
        let json_str: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        drop(stmt);

        if let Some(json) = json_str {
            if let Ok(config) = serde_json::from_str::<AiConfig>(&json) {
                if !config.api_key.is_empty() {
                    // Save to credential store
                    credential::save_credential(&CredentialKey::DeepSeek, &config.api_key)?;
                    // Clear from SQLite
                    let mut safe = config.clone();
                    safe.api_key = String::new();
                    let clean_json = serde_json::to_string(&safe)?;
                    let conn = self.connection.lock().expect("storage lock failed");
                    conn.execute(
                        "UPDATE ai_config SET value = ?1 WHERE key = 'config'",
                        rusqlite::params![clean_json],
                    )?;
                    eprintln!("[OpenJournal] Migrated plaintext API key to OS credential store.");
                }
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // AI summaries CRUD
    // -----------------------------------------------------------------------

    pub fn upsert_ai_summary(&self, summary: &AiSummary) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"
            INSERT INTO ai_summaries
              (day, block_index, block_start, block_end, summary_json,
               model_name, generated_at, token_count, status, error_message,
               retry_count, last_attempt_at, generation_source, queue_status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(day, block_index, model_name) DO UPDATE SET
              summary_json = excluded.summary_json,
              generated_at = excluded.generated_at,
              token_count = excluded.token_count,
              status = excluded.status,
              error_message = excluded.error_message,
              retry_count = excluded.retry_count,
              last_attempt_at = excluded.last_attempt_at,
              generation_source = excluded.generation_source,
              queue_status = excluded.queue_status
            "#,
            params![
                summary.day,
                summary.block_index,
                summary.block_start,
                summary.block_end,
                summary.summary_json,
                summary.model_name,
                summary.generated_at,
                summary.token_count,
                summary.status,
                summary.error_message,
                summary.retry_count,
                summary.last_attempt_at,
                summary.generation_source,
                summary.queue_status,
            ],
        )?;
        Ok(())
    }

    pub fn get_ai_summaries_for_day(&self, day: &str) -> anyhow::Result<Vec<AiSummary>> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection.prepare(
            r#"
            SELECT id, day, block_index, block_start, block_end,
                   summary_json, model_name, generated_at, token_count,
                   status, error_message,
                   retry_count, last_attempt_at, generation_source, queue_status
            FROM ai_summaries
            WHERE day = ?1
            ORDER BY block_index ASC
            "#,
        )?;
        let rows = stmt.query_map(params![day], |row| {
            Ok(AiSummary {
                id: row.get(0)?,
                day: row.get(1)?,
                block_index: row.get(2)?,
                block_start: row.get(3)?,
                block_end: row.get(4)?,
                summary_json: row.get(5)?,
                model_name: row.get(6)?,
                generated_at: row.get(7)?,
                token_count: row.get(8)?,
                status: row.get(9)?,
                error_message: row.get(10)?,
                retry_count: row.get(11)?,
                last_attempt_at: row.get(12)?,
                generation_source: row.get(13)?,
                queue_status: row.get(14)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn delete_ai_summary(&self, id: i64) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute("DELETE FROM ai_summaries WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Scheduler
    // -----------------------------------------------------------------------

    pub fn get_scheduler_settings(&self) -> anyhow::Result<SchedulerSettings> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection.prepare("SELECT value FROM ai_config WHERE key = ?1")?;
        let json_str: Option<String> = stmt.query_row(params!["scheduler"], |row| row.get(0)).ok();
        drop(stmt);
        match json_str {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(SchedulerSettings::default()),
        }
    }

    pub fn set_scheduler_settings(&self, settings: &SchedulerSettings) -> anyhow::Result<()> {
        let json = serde_json::to_string(settings)?;
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"INSERT INTO ai_config(key, value) VALUES ('scheduler', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value"#,
            params![json],
        )?;
        Ok(())
    }

    /// Find blocks that are completed (past their end time) but have no summary or need retry.
    /// Returns (day, block_index) pairs.
    pub fn find_pending_summary_blocks(
        &self,
        days_back: i64,
    ) -> anyhow::Result<Vec<(String, i32)>> {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection.prepare(
            r#"
            SELECT DISTINCT activity.date_str, (CAST(strftime('%H', activity.max_start) AS INTEGER) / 3) * 3
            FROM (
              SELECT substr(started_at, 1, 10) AS date_str,
                     MAX(started_at) AS max_start
              FROM activity_entries
              GROUP BY date_str
              HAVING date_str >= date('now', ?1)
            ) activity
            LEFT JOIN ai_summaries summary
              ON summary.day = activity.date_str
             AND summary.block_index = (CAST(strftime('%H', activity.max_start) AS INTEGER) / 3) * 3 - 3
            WHERE summary.id IS NULL
               OR (summary.status = 'failed' AND summary.retry_count < 2)
            ORDER BY activity.date_str ASC, 2 ASC
            "#,
        )?;
        let since = format!("-{} days", days_back);
        let rows = stmt.query_map(params![since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Crash recovery and durability
    // -----------------------------------------------------------------------

    /// Finalize any activity rows that were open when the app last shut down.
    /// Handles both clean shutdown and crash recovery.
    pub fn recover_open_entries(&self) -> anyhow::Result<i64> {
        let connection = self.connection.lock().expect("storage lock failed");
        let now_rfc = Utc::now().to_rfc3339();
        // Update rows where ended_at == started_at (crashed before first refresh tick)
        // or duration_seconds < 3 (crashed mid-tick)
        let recovered = connection.execute(
            r#"
            UPDATE activity_entries
            SET ended_at = ?1,
                duration_seconds = CAST(
                  (julianday(?1) - julianday(started_at)) * 86400 AS INTEGER
                )
            WHERE ended_at = started_at
               OR (duration_seconds < 3
                   AND CAST(
                     (julianday(?1) - julianday(started_at)) * 86400 AS INTEGER
                   ) > duration_seconds)
            "#,
            params![now_rfc],
        )?;
        let _ = connection.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES ('last_startup_recovery', ?1, CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![now_rfc],
        );
        Ok(recovered as i64)
    }

    /// Touch last_seen_at for an active activity row.
    pub fn touch_last_seen(&self, activity_id: i64) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            "UPDATE activity_entries SET last_seen_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), activity_id],
        )?;
        Ok(())
    }

    pub fn get_last_recovery_at(&self) -> String {
        let connection = self.connection.lock().expect("storage lock failed");
        connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'last_startup_recovery'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "never".to_string())
    }

    // -----------------------------------------------------------------------
    // Autostart setting
    // -----------------------------------------------------------------------

    pub fn get_autostart_setting(&self) -> AutostartSetting {
        let connection = self.connection.lock().expect("storage lock failed");
        let mut stmt = connection
            .prepare("SELECT value FROM ai_config WHERE key = 'autostart'")
            .expect("prepare");
        let val: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        drop(stmt);
        match val {
            Some(j) => serde_json::from_str(&j).unwrap_or(AutostartSetting { enabled: true }),
            None => AutostartSetting { enabled: true },
        }
    }

    pub fn set_autostart_setting(&self, setting: &AutostartSetting) -> anyhow::Result<()> {
        let json = serde_json::to_string(setting)?;
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute(
            r#"INSERT INTO ai_config(key, value) VALUES ('autostart', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value"#,
            params![json],
        )?;
        Ok(())
    }

    /// Delete all AI summaries for a day. Reserved for future batch operations.
    #[allow(dead_code)]
    pub fn delete_ai_summaries_for_day(&self, day: &str) -> anyhow::Result<()> {
        let connection = self.connection.lock().expect("storage lock failed");
        connection.execute("DELETE FROM ai_summaries WHERE day = ?1", params![day])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_storage(name: &str) -> Storage {
        let path = std::env::temp_dir().join(format!(
            "openjournal-storage-{name}-{}.sqlite3",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let storage = Storage::open(path).expect("open test storage");
        storage.migrate().expect("migrate test storage");
        storage
    }

    #[test]
    fn delete_day_removes_activity_for_that_day() {
        let storage = test_storage("delete-day");
        let started_at = Utc::now();
        let id = storage
            .start_activity("Code.exe", "OpenJournal", started_at)
            .expect("start activity");
        storage
            .update_activity_end(id, started_at, started_at + Duration::seconds(30))
            .expect("update activity");

        let day = started_at.format("%Y-%m-%d").to_string();
        assert_eq!(storage.activity_for_day(&day).expect("before").len(), 1);
        storage.delete_day(&day).expect("delete day");
        assert_eq!(storage.activity_for_day(&day).expect("after").len(), 0);
    }

    #[test]
    fn recover_open_entries_finalizes_stale_rows() {
        let storage = test_storage("recover");
        let started_at = Utc::now() - Duration::minutes(30);
        // Simulate a crash: activity started but ended_at == started_at
        let id = storage
            .start_activity("Code.exe", "Crashed session", started_at)
            .expect("start activity");
        // ended_at is same as started_at — simulate crash before first touch
        storage
            .update_activity_end(id, started_at, started_at)
            .expect("update with same time");

        let day = started_at.format("%Y-%m-%d").to_string();
        assert_eq!(storage.activity_for_day(&day).expect("before").len(), 1);
        let entry = &storage.activity_for_day(&day).expect("entries")[0];
        assert_eq!(entry.duration_seconds, 0); // crash left 0 duration

        // Recover
        let recovered = storage.recover_open_entries().expect("recover");
        assert!(recovered > 0, "Should have recovered at least one entry");

        let entries = storage.activity_for_day(&day).expect("after");
        assert_eq!(entries.len(), 1, "Should still have the same entry");
        let entry = &entries[0];
        assert!(
            entry.duration_seconds > 0,
            "Duration should be updated after recovery, got {}",
            entry.duration_seconds
        );
        assert!(entry.ended_at.is_some());
        assert_ne!(entry.ended_at, Some(entry.started_at.clone()));
    }

    #[test]
    fn repeated_startup_does_not_duplicate_rows() {
        let storage = test_storage("dedup");
        let started_at = Utc::now() - Duration::hours(2);
        let id = storage
            .start_activity("Code.exe", "First session", started_at)
            .expect("start");
        storage
            .update_activity_end(id, started_at, started_at)
            .expect("crash");

        let day = started_at.format("%Y-%m-%d").to_string();
        assert_eq!(storage.activity_for_day(&day).expect("before").len(), 1);

        // First recovery
        let r1 = storage.recover_open_entries().expect("recover 1");
        assert!(r1 > 0);

        // Second recovery — should find nothing
        let r2 = storage.recover_open_entries().expect("recover 2");
        assert_eq!(r2, 0, "Repeated recovery should not find stale rows");

        assert_eq!(storage.activity_for_day(&day).expect("after").len(), 1);
    }
}
