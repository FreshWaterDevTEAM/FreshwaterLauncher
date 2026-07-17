use crate::error::Result;
use std::path::{Path, PathBuf};

pub fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("FreshwaterLauncher")
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

pub fn minecraft_dir(root: &Path) -> PathBuf {
    root.join(".minecraft")
}

pub fn versions_dir(root: &Path) -> PathBuf {
    minecraft_dir(root).join("versions")
}

pub fn libraries_dir(root: &Path) -> PathBuf {
    minecraft_dir(root).join("libraries")
}

pub fn assets_dir(root: &Path) -> PathBuf {
    minecraft_dir(root).join("assets")
}

pub fn instances_dir(root: &Path) -> PathBuf {
    root.join("instances")
}

pub fn accounts_file(root: &Path) -> PathBuf {
    root.join("accounts.json")
}

pub fn config_file(root: &Path) -> PathBuf {
    root.join("config.toml.json")
}

pub fn java_dir(root: &Path) -> PathBuf {
    root.join("java")
}

pub fn logs_dir(root: &Path) -> PathBuf {
    root.join("logs")
}
