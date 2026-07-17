use crate::config::FwlConfig;
use crate::download::download_file;
use crate::error::{FwlError, Result};
use crate::instances::Instance;
use crate::paths::ensure_dir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const SYNC_PROTOCOL: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub protocol: u32,
    pub channel: String,
    pub revision: u64,
    pub mc: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub files: Vec<SyncFile>,
    #[serde(default)]
    pub remove: Vec<String>,
    #[serde(default)]
    pub rules: SyncRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRules {
    /// If true, delete mods not listed in manifest.
    pub strict_mods: bool,
}

impl Default for SyncRules {
    fn default() -> Self {
        Self {
            strict_mods: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDiff {
    pub revision: u64,
    pub to_download: Vec<SyncFile>,
    pub to_remove: Vec<String>,
    pub up_to_date: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub downloaded: Vec<String>,
    pub removed: Vec<String>,
    pub revision: u64,
}

pub async fn fetch_manifest(base_url: &str, channel: &str) -> Result<SyncManifest> {
    let base = base_url.trim_end_matches('/');
    let url = format!("{base}/v1/channels/{channel}/manifest.json");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "FreshwaterLauncher/0.1.0")
        .send()
        .await?
        .error_for_status()
        .map_err(|e| FwlError::Sync(format!("拉取清单失败: {e}")))?;
    Ok(resp.json().await?)
}

pub fn plan_sync(cfg: &FwlConfig, instance: &Instance, manifest: &SyncManifest) -> Result<SyncDiff> {
    let game = instance.game_dir(&cfg.data_dir);
    let mut to_download = Vec::new();
    let mut to_remove = Vec::new();

    for f in &manifest.files {
        let dest = game.join(&f.path);
        if dest.exists() {
            let hash = sha256_file(&dest)?;
            if hash.eq_ignore_ascii_case(&f.sha256) {
                continue;
            }
        }
        to_download.push(f.clone());
    }

    for r in &manifest.remove {
        let dest = game.join(r);
        if dest.exists() {
            to_remove.push(r.clone());
        }
    }

    if manifest.rules.strict_mods {
        let mods_dir = game.join("mods");
        if mods_dir.exists() {
            let listed: HashSet<String> = manifest
                .files
                .iter()
                .filter(|f| f.path.starts_with("mods/"))
                .map(|f| f.path.clone())
                .collect();
            for e in walkdir::WalkDir::new(&mods_dir).into_iter().flatten() {
                if !e.file_type().is_file() {
                    continue;
                }
                let rel = format!(
                    "mods/{}",
                    e.path()
                        .strip_prefix(&mods_dir)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                );
                if !listed.contains(&rel) && !to_remove.contains(&rel) {
                    to_remove.push(rel);
                }
            }
        }
    }

    let up_to_date = to_download.is_empty() && to_remove.is_empty();
    Ok(SyncDiff {
        revision: manifest.revision,
        to_download,
        to_remove,
        up_to_date,
    })
}

pub async fn apply_sync(
    cfg: &FwlConfig,
    instance: &Instance,
    manifest: &SyncManifest,
    diff: &SyncDiff,
) -> Result<SyncReport> {
    let game = instance.game_dir(&cfg.data_dir);
    ensure_dir(&game)?;
    let mut downloaded = Vec::new();
    let mut removed = Vec::new();

    for f in &diff.to_download {
        let dest = game.join(&f.path);
        let url = resolve_url(&manifest_base_hint(&f.url), &f.url);
        download_file(cfg, &url, &dest, None).await?;
        let hash = sha256_file(&dest)?;
        if !hash.eq_ignore_ascii_case(&f.sha256) {
            return Err(FwlError::Sync(format!("校验失败: {}", f.path)));
        }
        downloaded.push(f.path.clone());
    }

    for r in &diff.to_remove {
        let dest = game.join(r);
        if dest.exists() {
            std::fs::remove_file(&dest)?;
            removed.push(r.clone());
        }
    }

    // remember revision
    let meta = game.join(".fwl-sync.json");
    std::fs::write(
        meta,
        serde_json::to_string_pretty(&serde_json::json!({
            "revision": manifest.revision,
            "channel": manifest.channel
        }))?,
    )?;

    Ok(SyncReport {
        downloaded,
        removed,
        revision: manifest.revision,
    })
}

fn manifest_base_hint(url: &str) -> String {
    url.to_string()
}

fn resolve_url(_base: &str, url: &str) -> String {
    url.to_string()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

/// Build a manifest from an instance mods folder (for sync-server publish).
pub fn build_manifest_from_instance(
    instance_dir: &Path,
    channel: &str,
    revision: u64,
    mc: &str,
    file_base_url: &str,
) -> Result<SyncManifest> {
    let mods = instance_dir.join("mods");
    let mut files = Vec::new();
    if mods.exists() {
        for e in walkdir::WalkDir::new(&mods).into_iter().flatten() {
            if !e.file_type().is_file() {
                continue;
            }
            let rel = format!(
                "mods/{}",
                e.path()
                    .strip_prefix(&mods)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            let data = std::fs::read(e.path())?;
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let sha = hex::encode(hasher.finalize());
            let base = file_base_url.trim_end_matches('/');
            files.push(SyncFile {
                path: rel.clone(),
                sha256: sha,
                size: data.len() as u64,
                url: format!("{base}/files/{rel}"),
            });
        }
    }
    Ok(SyncManifest {
        protocol: SYNC_PROTOCOL,
        channel: channel.into(),
        revision,
        mc: mc.into(),
        loader: None,
        loader_version: None,
        files,
        remove: vec![],
        rules: SyncRules {
            strict_mods: true,
        },
    })
}

pub fn copy_instance_mods_to_publish(instance_dir: &Path, out_dir: &Path) -> Result<()> {
    let src = instance_dir.join("mods");
    let dst = out_dir.join("files").join("mods");
    ensure_dir(&dst)?;
    if src.exists() {
        for e in walkdir::WalkDir::new(&src).into_iter().flatten() {
            if e.file_type().is_file() {
                let rel = e.path().strip_prefix(&src).unwrap();
                let target = dst.join(rel);
                if let Some(p) = target.parent() {
                    ensure_dir(p)?;
                }
                std::fs::copy(e.path(), target)?;
            }
        }
    }
    Ok(())
}

pub fn local_sync_revision(cfg: &FwlConfig, instance: &Instance) -> Option<u64> {
    let meta = instance.game_dir(&cfg.data_dir).join(".fwl-sync.json");
    let text = std::fs::read_to_string(meta).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("revision")?.as_u64()
}

pub fn parse_platform_input(input: &str) -> (String, String) {
    // URL or "url#channel" or invite "fwl://sync?base=...&channel=..."
    let input = input.trim();
    if let Some(rest) = input.strip_prefix("fwl://sync?") {
        let mut base = String::new();
        let mut channel = "default".to_string();
        for part in rest.split('&') {
            if let Some((k, v)) = part.split_once('=') {
                match k {
                    "base" => base = urlencoding::decode(v).unwrap_or_default().into_owned(),
                    "channel" => channel = v.to_string(),
                    _ => {}
                }
            }
        }
        return (base, channel);
    }
    if let Some((base, channel)) = input.rsplit_once('#') {
        return (base.to_string(), channel.to_string());
    }
    (input.to_string(), "default".to_string())
}

pub fn invite_code(base: &str, channel: &str) -> String {
    format!(
        "fwl://sync?base={}&channel={}",
        urlencoding::encode(base),
        channel
    )
}

pub type PublishDir = PathBuf;
