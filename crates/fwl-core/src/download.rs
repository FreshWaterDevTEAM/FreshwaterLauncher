use crate::config::FwlConfig;
use crate::error::{FwlError, Result};
use crate::paths::{assets_dir, ensure_dir, libraries_dir, versions_dir};
use crate::versions::{
    fetch_version_json, library_allowed, VersionJson, VersionManifest,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Default)]
pub struct DownloadQueue {
    tasks: RwLock<Vec<DownloadTask>>,
}

impl DownloadQueue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn list(&self) -> Vec<DownloadTask> {
        self.tasks.read().await.clone()
    }

    async fn upsert(&self, task: DownloadTask) {
        let mut tasks = self.tasks.write().await;
        if let Some(t) = tasks.iter_mut().find(|t| t.id == task.id) {
            *t = task;
        } else {
            tasks.push(task);
        }
    }

    pub async fn install_version(
        self: &Arc<Self>,
        cfg: &FwlConfig,
        version_id: &str,
        manifest: &VersionManifest,
    ) -> Result<PathBuf> {
        let info = manifest
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| FwlError::Download(format!("版本不存在: {version_id}")))?;

        let task_id = format!("version:{version_id}");
        self.upsert(DownloadTask {
            id: task_id.clone(),
            name: format!("安装 {version_id}"),
            status: TaskStatus::Running,
            progress: 0.0,
            message: "获取版本元数据".into(),
        })
        .await;

        let result = self
            .install_version_inner(cfg, version_id, &info.url, &task_id)
            .await;

        match &result {
            Ok(_) => {
                self.upsert(DownloadTask {
                    id: task_id,
                    name: format!("安装 {version_id}"),
                    status: TaskStatus::Done,
                    progress: 1.0,
                    message: "完成".into(),
                })
                .await;
            }
            Err(e) => {
                self.upsert(DownloadTask {
                    id: task_id,
                    name: format!("安装 {version_id}"),
                    status: TaskStatus::Failed,
                    progress: 0.0,
                    message: e.to_string(),
                })
                .await;
            }
        }
        result
    }

    async fn install_version_inner(
        &self,
        cfg: &FwlConfig,
        version_id: &str,
        meta_url: &str,
        task_id: &str,
    ) -> Result<PathBuf> {
        let vdir = versions_dir(&cfg.data_dir).join(version_id);
        ensure_dir(&vdir)?;
        let vjson_path = vdir.join(format!("{version_id}.json"));

        let version = fetch_version_json(cfg, meta_url).await?;
        std::fs::write(&vjson_path, serde_json::to_string_pretty(&version)?)?;

        if let Some(downloads) = &version.downloads {
            let jar = vdir.join(format!("{version_id}.jar"));
            self.upsert(DownloadTask {
                id: task_id.into(),
                name: format!("安装 {version_id}"),
                status: TaskStatus::Running,
                progress: 0.1,
                message: "下载客户端".into(),
            })
            .await;
            download_file(
                cfg,
                &cfg.rewrite_download_url(&downloads.client.url),
                &jar,
                Some(&downloads.client.sha1),
            )
            .await?;
        }

        let libs = libraries_dir(&cfg.data_dir);
        ensure_dir(&libs)?;
        let total = version.libraries.len().max(1) as f64;
        for (i, lib) in version.libraries.iter().enumerate() {
            if !library_allowed(lib) {
                continue;
            }
            if let Some(dl) = &lib.downloads {
                if let Some(art) = &dl.artifact {
                    let rel = art.path.clone().unwrap_or_else(|| maven_path(&lib.name));
                    let dest = libs.join(&rel);
                    self.upsert(DownloadTask {
                        id: task_id.into(),
                        name: format!("安装 {version_id}"),
                        status: TaskStatus::Running,
                        progress: 0.15 + (i as f64 / total) * 0.55,
                        message: format!("库 {}", lib.name),
                    })
                    .await;
                    download_file(
                        cfg,
                        &cfg.rewrite_download_url(&art.url),
                        &dest,
                        Some(&art.sha1),
                    )
                    .await?;
                }
            }
        }

        if let Some(idx) = &version.assetIndex {
            self.upsert(DownloadTask {
                id: task_id.into(),
                name: format!("安装 {version_id}"),
                status: TaskStatus::Running,
                progress: 0.75,
                message: "下载资源索引".into(),
            })
            .await;
            download_assets(cfg, idx).await?;
        }

        Ok(vdir)
    }
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: std::collections::HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    #[allow(dead_code)]
    size: u64,
}

async fn download_assets(
    cfg: &FwlConfig,
    idx: &crate::versions::AssetIndexRef,
) -> Result<()> {
    let indexes = assets_dir(&cfg.data_dir).join("indexes");
    let objects = assets_dir(&cfg.data_dir).join("objects");
    ensure_dir(&indexes)?;
    ensure_dir(&objects)?;
    let index_path = indexes.join(format!("{}.json", idx.id));
    download_file(
        cfg,
        &cfg.rewrite_download_url(&idx.url),
        &index_path,
        Some(&idx.sha1),
    )
    .await?;
    let index: AssetIndex = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;

    // Download a limited parallel set; for full install download all
    let client = reqwest::Client::new();
    let sem = Arc::new(tokio::sync::Semaphore::new(16));
    let errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();

    for obj in index.objects.values() {
        let hash = obj.hash.clone();
        let cfg = cfg.clone();
        let objects = objects.clone();
        let client = client.clone();
        let sem = sem.clone();
        let errors = errors.clone();
        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            let prefix = &hash[..2];
            let dest = objects.join(prefix).join(&hash);
            if dest.exists() {
                return;
            }
            let url = cfg.rewrite_download_url(&format!(
                "https://resources.download.minecraft.net/{prefix}/{hash}"
            ));
            if let Err(e) = download_file_with_client(&client, &url, &dest, Some(&hash)).await {
                errors.lock().await.push(e.to_string());
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
    let errs = errors.lock().await;
    if !errs.is_empty() {
        return Err(FwlError::Download(format!(
            "部分资源下载失败 ({}): {}",
            errs.len(),
            errs.first().cloned().unwrap_or_default()
        )));
    }
    Ok(())
}

pub async fn download_file(
    cfg: &FwlConfig,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<()> {
    let _ = cfg;
    let client = reqwest::Client::new();
    download_file_with_client(&client, url, dest, sha1).await
}

async fn download_file_with_client(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<()> {
    if dest.exists() {
        if let Some(expected) = sha1 {
            if verify_sha1(dest, expected)? {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        ensure_dir(parent)?;
    }
    let resp = client.get(url).send().await?.error_for_status()?;
    let mut stream = resp.bytes_stream();
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = sha1 {
        if !verify_sha1(&tmp, expected)? {
            let _ = std::fs::remove_file(&tmp);
            return Err(FwlError::Download(format!("SHA1 校验失败: {url}")));
        }
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

fn verify_sha1(path: &Path, expected: &str) -> Result<bool> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&data);
    let digest = hex::encode(hasher.finalize());
    Ok(digest.eq_ignore_ascii_case(expected))
}

fn maven_path(name: &str) -> String {
    // group:artifact:version[:classifier]
    let parts: Vec<_> = name.split(':').collect();
    if parts.len() < 3 {
        return name.replace(':', "/");
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    if parts.len() >= 4 {
        format!(
            "{group}/{artifact}/{version}/{artifact}-{version}-{}.jar",
            parts[3]
        )
    } else {
        format!("{group}/{artifact}/{version}/{artifact}-{version}.jar")
    }
}

// Need Sha1 - use sha1 crate or sha2? Minecraft uses SHA-1. I used Sha1 from sha2 incorrectly.
// sha2 doesn't have Sha1. I need the `sha1` crate.
