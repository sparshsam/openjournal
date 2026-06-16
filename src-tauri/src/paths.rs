use std::path::PathBuf;

/// Root data directory.
/// Installed: %LOCALAPPDATA%/OpenJournal/
/// Portable: ./ (beside executable)
pub fn data_root_dir() -> anyhow::Result<PathBuf> {
    if portable_flag() {
        let exe = std::env::current_exe()?;
        Ok(exe.parent().unwrap_or(&exe).to_path_buf())
    } else if cfg!(target_os = "windows") {
        let local =
            std::env::var("LOCALAPPDATA").map_err(|_| anyhow::anyhow!("LOCALAPPDATA not set"))?;
        Ok(PathBuf::from(local).join("OpenJournal"))
    } else {
        dirs::data_dir()
            .map(|d| d.join("OpenJournal"))
            .ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))
    }
}

pub fn backups_dir() -> anyhow::Result<PathBuf> {
    let dir = data_root_dir()?.join("Backups");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Returns true if a `portable.flag` file exists beside the executable.
pub fn portable_flag() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("portable.flag")))
        .map(|f| f.exists())
        .unwrap_or(false)
}
