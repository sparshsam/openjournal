use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::models::ActivityEntry;

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
            "#,
        )?;
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
}
