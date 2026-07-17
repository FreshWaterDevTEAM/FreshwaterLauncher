use crate::error::Result;
use crate::paths::{config_file, default_data_dir, ensure_dir};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Public Azure client ID for FreshwaterLauncher (not a secret).
pub const DEFAULT_MS_CLIENT_ID: &str = "9e27bb17-91c2-49ce-b7b4-d667665e82da";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FwlConfig {
    pub data_dir: PathBuf,
    pub ms_client_id: String,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: String,
    pub download_source: DownloadSource,
    pub close_after_launch: bool,
    pub selected_instance: Option<String>,
    pub selected_account: Option<String>,
    pub language: String,
    pub curseforge_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSource {
    Official,
    Bmclapi,
}

impl Default for DownloadSource {
    fn default() -> Self {
        Self::Bmclapi
    }
}

impl Default for FwlConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            ms_client_id: std::env::var("FWL_MS_CLIENT_ID")
                .unwrap_or_else(|_| DEFAULT_MS_CLIENT_ID.to_string()),
            max_memory_mb: 4096,
            min_memory_mb: 512,
            java_path: None,
            jvm_args: String::new(),
            download_source: DownloadSource::Bmclapi,
            close_after_launch: false,
            selected_instance: None,
            selected_account: None,
            language: "zh-CN".into(),
            curseforge_api_key: std::env::var("FWL_CURSEFORGE_API_KEY").ok(),
        }
    }
}

impl FwlConfig {
    pub fn load_or_default() -> Result<Self> {
        let dir = default_data_dir();
        ensure_dir(&dir)?;
        let path = config_file(&dir);
        if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&text)?)
        } else {
            let cfg = Self::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<()> {
        ensure_dir(&self.data_dir)?;
        let path = config_file(&self.data_dir);
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn manifest_url(&self) -> &'static str {
        match self.download_source {
            DownloadSource::Official => {
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
            }
            DownloadSource::Bmclapi => {
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"
            }
        }
    }

    pub fn rewrite_download_url(&self, url: &str) -> String {
        match self.download_source {
            DownloadSource::Official => url.to_string(),
            DownloadSource::Bmclapi => url
                .replace(
                    "https://piston-meta.mojang.com",
                    "https://bmclapi2.bangbang93.com",
                )
                .replace(
                    "https://launchermeta.mojang.com",
                    "https://bmclapi2.bangbang93.com",
                )
                .replace(
                    "https://launcher.mojang.com",
                    "https://bmclapi2.bangbang93.com",
                )
                .replace(
                    "https://libraries.minecraft.net",
                    "https://bmclapi2.bangbang93.com/maven",
                )
                .replace(
                    "https://resources.download.minecraft.net",
                    "https://bmclapi2.bangbang93.com/assets",
                )
                .replace(
                    "https://piston-data.mojang.com",
                    "https://bmclapi2.bangbang93.com",
                ),
        }
    }
}
