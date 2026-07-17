use crate::config::FwlConfig;
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<Library>,
    pub downloads: Option<VersionDownloads>,
    pub assetIndex: Option<AssetIndexRef>,
    pub assets: Option<String>,
    pub javaVersion: Option<JavaVersionReq>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub inheritsFrom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionReq {
    pub component: String,
    pub majorVersion: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadArtifact {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub totalSize: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadArtifactPath>,
    pub classifiers: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadArtifactPath {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

pub async fn fetch_manifest(cfg: &FwlConfig) -> Result<VersionManifest> {
    let client = reqwest::Client::new();
    let url = cfg.manifest_url();
    let text = client.get(url).send().await?.error_for_status()?.text().await?;
    Ok(serde_json::from_str(&text)?)
}

pub async fn fetch_version_json(cfg: &FwlConfig, url: &str) -> Result<VersionJson> {
    let client = reqwest::Client::new();
    let rewritten = cfg.rewrite_download_url(url);
    let text = client
        .get(&rewritten)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(serde_json::from_str(&text)?)
}

pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn library_allowed(lib: &Library) -> bool {
    let Some(rules) = &lib.rules else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let os_match = match &rule.os {
            None => true,
            Some(os) => os.name.as_deref() == Some(current_os_name()),
        };
        if os_match {
            allowed = rule.action == "allow";
        }
    }
    allowed
}
