use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};

use crate::storage::Storage;

const PAUSED_SETTING: &str = "logging_paused";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FocusSnapshot {
    app_name: String,
    window_title: String,
}

#[derive(Debug, Clone)]
struct OpenEntry {
    id: i64,
    snapshot: FocusSnapshot,
    started_at: DateTime<Utc>,
}

pub struct ActivityTracker {
    storage: Storage,
    paused: bool,
    current: Option<OpenEntry>,
    blocklist: Vec<String>,
    active_label: String,
}

impl ActivityTracker {
    pub fn new(storage: Storage) -> anyhow::Result<Self> {
        let paused = storage.setting_bool(PAUSED_SETTING, false)?;
        let blocklist = storage.blocklist()?;
        Ok(Self {
            storage,
            paused,
            current: None,
            blocklist,
            active_label: "No active window yet".to_string(),
        })
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.tick_with_snapshot(active_window_snapshot()?)
    }

    fn tick_with_snapshot(&mut self, snapshot: Option<FocusSnapshot>) -> anyhow::Result<()> {
        if self.paused {
            self.flush_current()?;
            self.active_label = "Logging paused".to_string();
            return Ok(());
        }

        let Some(snapshot) = snapshot else {
            self.flush_current()?;
            self.active_label = "No foreground window".to_string();
            return Ok(());
        };

        self.active_label = format!("{} - {}", snapshot.app_name, snapshot.window_title);

        if self.is_blocked(&snapshot) {
            self.flush_current()?;
            return Ok(());
        }

        match &self.current {
            Some(open) if open.snapshot == snapshot => {}
            Some(_) => {
                self.flush_current()?;
                let started_at = Utc::now();
                let id = self.storage.start_activity(
                    &snapshot.app_name,
                    &snapshot.window_title,
                    started_at,
                )?;
                self.current = Some(OpenEntry {
                    id,
                    snapshot,
                    started_at,
                });
            }
            None => {
                let started_at = Utc::now();
                let id = self.storage.start_activity(
                    &snapshot.app_name,
                    &snapshot.window_title,
                    started_at,
                )?;
                self.current = Some(OpenEntry {
                    id,
                    snapshot,
                    started_at,
                });
            }
        }

        self.touch_current()?;
        Ok(())
    }

    pub fn set_paused(&mut self, paused: bool) -> anyhow::Result<()> {
        self.paused = paused;
        self.storage.set_setting_bool(PAUSED_SETTING, paused)?;
        if paused {
            self.flush_current()?;
        }
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn active_window_label(&self) -> String {
        self.active_label.clone()
    }

    pub fn reload_blocklist(&mut self) -> anyhow::Result<()> {
        self.blocklist = self.storage.blocklist()?;
        Ok(())
    }

    fn flush_current(&mut self) -> anyhow::Result<()> {
        if let Some(open) = self.current.take() {
            let ended_at = Utc::now();
            self.storage
                .update_activity_end(open.id, open.started_at, ended_at)?;
        }
        Ok(())
    }

    fn touch_current(&mut self) -> anyhow::Result<()> {
        if let Some(open) = &self.current {
            self.storage
                .update_activity_end(open.id, open.started_at, Utc::now())?;
        }
        Ok(())
    }

    fn is_blocked(&self, snapshot: &FocusSnapshot) -> bool {
        let haystack = format!(
            "{} {}",
            snapshot.app_name.to_lowercase(),
            snapshot.window_title.to_lowercase()
        );
        self.blocklist
            .iter()
            .any(|pattern| !pattern.is_empty() && haystack.contains(pattern))
    }
}

impl Drop for ActivityTracker {
    fn drop(&mut self) {
        let _ = self.flush_current();
    }
}

pub fn spawn_poll_loop(tracker: Arc<Mutex<ActivityTracker>>) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(mut tracker) = tracker.lock() {
                if let Err(error) = tracker.tick() {
                    eprintln!("OpenJournal tracker tick failed: {error}");
                }
            }
            tokio::time::sleep(StdDuration::from_secs(5)).await;
        }
    });
}

#[cfg(target_os = "windows")]
fn active_window_snapshot() -> anyhow::Result<Option<FocusSnapshot>> {
    use std::path::Path;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(None);
        }

        let title_len = GetWindowTextLengthW(hwnd);
        let mut title_buffer = vec![0u16; title_len as usize + 1];
        let copied = GetWindowTextW(hwnd, &mut title_buffer);
        let window_title = String::from_utf16_lossy(&title_buffer[..copied as usize])
            .trim()
            .to_string();

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)?;

        let mut module_buffer = vec![0u16; MAX_PATH as usize];
        let mut name_len = module_buffer.len() as u32;
        let image_name = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(module_buffer.as_mut_ptr()),
            &mut name_len,
        )
        .map(|_| String::from_utf16_lossy(&module_buffer[..name_len as usize]));
        let _ = CloseHandle(process);

        let app_name = image_name
            .ok()
            .and_then(|path| {
                Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| format!("Process {process_id}"));

        Ok(Some(FocusSnapshot {
            app_name,
            window_title,
        }))
    }
}

#[cfg(not(target_os = "windows"))]
fn active_window_snapshot() -> anyhow::Result<Option<FocusSnapshot>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_storage(name: &str) -> Storage {
        let path = std::env::temp_dir().join(format!(
            "openjournal-{name}-{}.sqlite3",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let storage = Storage::open(path).expect("open test storage");
        storage.migrate().expect("migrate test storage");
        storage
    }

    fn snapshot(app_name: &str, window_title: &str) -> FocusSnapshot {
        FocusSnapshot {
            app_name: app_name.to_string(),
            window_title: window_title.to_string(),
        }
    }

    fn today() -> String {
        Utc::now().format("%Y-%m-%d").to_string()
    }

    #[test]
    fn records_and_updates_focused_window() {
        let storage = test_storage("records");
        let mut tracker = ActivityTracker::new(storage.clone()).expect("tracker");
        tracker
            .tick_with_snapshot(Some(snapshot("Code.exe", "OpenJournal")))
            .expect("first tick");
        tracker
            .tick_with_snapshot(Some(snapshot("Code.exe", "OpenJournal")))
            .expect("second tick");

        let entries = storage.activity_for_day(&today()).expect("activity");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].app_name, "Code.exe");
        assert_eq!(entries[0].window_title, "OpenJournal");
        assert!(entries[0].ended_at.is_some());
    }

    #[test]
    fn pause_stops_and_resume_restarts_logging() {
        let storage = test_storage("pause-resume");
        let mut tracker = ActivityTracker::new(storage.clone()).expect("tracker");
        tracker
            .tick_with_snapshot(Some(snapshot("Code.exe", "OpenJournal")))
            .expect("first tick");
        tracker.set_paused(true).expect("pause");
        tracker
            .tick_with_snapshot(Some(snapshot("Notepad.exe", "Private note")))
            .expect("paused tick");
        tracker.set_paused(false).expect("resume");
        tracker
            .tick_with_snapshot(Some(snapshot("Notepad.exe", "Public note")))
            .expect("resumed tick");

        let entries = storage.activity_for_day(&today()).expect("activity");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].app_name, "Code.exe");
        assert_eq!(entries[1].app_name, "Notepad.exe");
        assert_eq!(entries[1].window_title, "Public note");
    }

    #[test]
    fn blocklist_skips_matching_app_or_title_before_storage() {
        let storage = test_storage("blocklist");
        storage
            .replace_blocklist(&["private".to_string(), "bank.com".to_string()])
            .expect("blocklist");
        let mut tracker = ActivityTracker::new(storage.clone()).expect("tracker");
        tracker
            .tick_with_snapshot(Some(snapshot("PrivateApp.exe", "Dashboard")))
            .expect("blocked app");
        tracker
            .tick_with_snapshot(Some(snapshot("Browser.exe", "https://bank.com")))
            .expect("blocked title");
        tracker
            .tick_with_snapshot(Some(snapshot("Code.exe", "OpenJournal")))
            .expect("allowed title");

        let entries = storage.activity_for_day(&today()).expect("activity");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].app_name, "Code.exe");
    }
}
