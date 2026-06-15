use std::path::PathBuf;

use chrono::Local;

use crate::models::{ActivityEntry, ExportBundle, SummaryBlock};

fn export_dir() -> anyhow::Result<PathBuf> {
    let dir = std::env::current_dir()?.join("exports");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn export_json(
    day: &str,
    activities: &[ActivityEntry],
    summaries: &[SummaryBlock],
) -> anyhow::Result<PathBuf> {
    let path = export_dir()?.join(format!("openjournal-{day}.json"));
    let bundle = ExportBundle {
        day: day.to_string(),
        activities: activities.to_vec(),
        summaries: summaries.to_vec(),
    };
    std::fs::write(path.clone(), serde_json::to_string_pretty(&bundle)?)?;
    Ok(path)
}

pub fn export_markdown(
    day: &str,
    activities: &[ActivityEntry],
    summaries: &[SummaryBlock],
) -> anyhow::Result<PathBuf> {
    let path = export_dir()?.join(format!("openjournal-{day}.md"));
    let mut output = String::new();
    output.push_str(&format!("# OpenJournal activity for {day}\n\n"));
    output.push_str(&format!(
        "Exported locally at {}.\n\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    output.push_str("## 3-hour summaries\n\n");
    for summary in summaries {
        output.push_str(&format!(
            "### {} - {}\n\n",
            summary.block_start, summary.block_end
        ));
        output.push_str(&format!("- Main focus: {}\n", summary.main_focus));
        output.push_str(&format!(
            "- Apps/projects used: {}\n",
            if summary.apps_projects.is_empty() {
                "None".to_string()
            } else {
                summary.apps_projects.join(", ")
            }
        ));
        output.push_str(&format!(
            "- Context switches: {}\n",
            summary.context_switches
        ));
        output.push_str(&format!(
            "- Productivity notes: {}\n",
            summary.productivity_notes.join("; ")
        ));
        output.push_str(&format!(
            "- Plain-English summary: {}\n\n",
            summary.plain_english_summary
        ));
    }

    output.push_str("## Timeline\n\n");
    output.push_str("| Start | End | Duration | App | Window |\n");
    output.push_str("| --- | --- | ---: | --- | --- |\n");
    for activity in activities {
        output.push_str(&format!(
            "| {} | {} | {}s | {} | {} |\n",
            activity.started_at,
            activity
                .ended_at
                .clone()
                .unwrap_or_else(|| "now".to_string()),
            activity.duration_seconds,
            escape_pipe(&activity.app_name),
            escape_pipe(&activity.window_title)
        ));
    }

    std::fs::write(path.clone(), output)?;
    Ok(path)
}

fn escape_pipe(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_markdown_and_json_exports() {
        let day = "2099-01-02";
        let activities = vec![ActivityEntry {
            id: 1,
            app_name: "Code.exe".to_string(),
            window_title: "OpenJournal".to_string(),
            started_at: "2099-01-02T09:00:00Z".to_string(),
            ended_at: Some("2099-01-02T09:15:00Z".to_string()),
            duration_seconds: 900,
        }];
        let summaries = vec![SummaryBlock {
            block_start: "09:00".to_string(),
            block_end: "12:00".to_string(),
            main_focus: "OpenJournal verification".to_string(),
            apps_projects: vec!["Code.exe".to_string()],
            context_switches: 0,
            productivity_notes: vec!["Local placeholder".to_string()],
            plain_english_summary: "Verified local export behavior.".to_string(),
            provider: "placeholder".to_string(),
        }];

        let markdown = export_markdown(day, &activities, &summaries).expect("markdown export");
        let json = export_json(day, &activities, &summaries).expect("json export");

        assert!(markdown.exists());
        assert!(json.exists());
        assert!(std::fs::read_to_string(markdown)
            .expect("read markdown")
            .contains("OpenJournal verification"));
        assert!(std::fs::read_to_string(json)
            .expect("read json")
            .contains("\"day\": \"2099-01-02\""));
    }
}
