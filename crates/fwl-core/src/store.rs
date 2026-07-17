use crate::config::FwlConfig;
use crate::download::download_file;
use crate::error::{FwlError, Result};
use crate::instances::Instance;
use crate::paths::ensure_dir;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSearchResult {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub source: String,
    pub project_type: String,
}

pub async fn search_modrinth(
    query: &str,
    project_type: &str,
    limit: u32,
) -> Result<Vec<StoreSearchResult>> {
    let client = reqwest::Client::new();
    let facets = match project_type {
        "mod" => "[[\"project_type:mod\"]]",
        "shader" => "[[\"project_type:shader\"]]",
        "modpack" => "[[\"project_type:modpack\"]]",
        _ => "[[\"project_type:mod\"]]",
    };
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&limit={}&facets={}",
        urlencoding::encode(query),
        limit,
        urlencoding::encode(facets)
    );
    let resp: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "FreshwaterLauncher/0.1.0 (FWL)")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut out = Vec::new();
    if let Some(hits) = resp.get("hits").and_then(|v| v.as_array()) {
        for h in hits {
            out.push(StoreSearchResult {
                id: h.get("project_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                slug: h.get("slug").and_then(|v| v.as_str()).unwrap_or("").into(),
                title: h.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                description: h
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                icon_url: h
                    .get("icon_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                downloads: h.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                source: "modrinth".into(),
                project_type: project_type.into(),
            });
        }
    }
    Ok(out)
}

pub async fn search_curseforge(
    cfg: &FwlConfig,
    query: &str,
    class_id: u32,
    limit: u32,
) -> Result<Vec<StoreSearchResult>> {
    let key = cfg
        .curseforge_api_key
        .as_deref()
        .ok_or_else(|| FwlError::Store("未配置 CurseForge API Key（设置 FWL_CURSEFORGE_API_KEY）".into()))?;
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.curseforge.com/v1/mods/search?gameId=432&classId={class_id}&searchFilter={}&pageSize={limit}",
        urlencoding::encode(query)
    );
    let resp: serde_json::Value = client
        .get(&url)
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut out = Vec::new();
    if let Some(data) = resp.get("data").and_then(|v| v.as_array()) {
        for m in data {
            out.push(StoreSearchResult {
                id: m.get("id").map(|v| v.to_string()).unwrap_or_default(),
                slug: m.get("slug").and_then(|v| v.as_str()).unwrap_or("").into(),
                title: m.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                description: m
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                icon_url: m
                    .pointer("/logo/url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                downloads: m
                    .get("downloadCount")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as u64)
                    .unwrap_or(0),
                source: "curseforge".into(),
                project_type: match class_id {
                    6 => "mod",
                    6552 => "shader",
                    4471 => "modpack",
                    _ => "mod",
                }
                .into(),
            });
        }
    }
    Ok(out)
}

pub async fn install_modrinth_version_to_instance(
    cfg: &FwlConfig,
    instance: &Instance,
    version_id: &str,
    dest_kind: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.modrinth.com/v2/version/{version_id}");
    let ver: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "FreshwaterLauncher/0.1.0 (FWL)")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let files = ver
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or_else(|| FwlError::Store("版本无文件".into()))?;
    let primary = files
        .iter()
        .find(|f| f.get("primary").and_then(|v| v.as_bool()) == Some(true))
        .or_else(|| files.first())
        .ok_or_else(|| FwlError::Store("无可用文件".into()))?;
    let dl = primary
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FwlError::Store("缺少下载 URL".into()))?;
    let filename = primary
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("download.jar");
    let dest_dir = match dest_kind {
        "shader" => instance.shaderpacks_dir(&cfg.data_dir),
        "resourcepack" => instance.resourcepacks_dir(&cfg.data_dir),
        _ => instance.mods_dir(&cfg.data_dir),
    };
    ensure_dir(&dest_dir)?;
    let dest = dest_dir.join(filename);
    download_file(cfg, dl, &dest, None).await?;
    Ok(dest.to_string_lossy().into())
}

pub async fn get_modrinth_versions(project_id: &str) -> Result<Vec<serde_json::Value>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
    let list: Vec<serde_json::Value> = client
        .get(&url)
        .header("User-Agent", "FreshwaterLauncher/0.1.0 (FWL)")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(list)
}

/// Import Modrinth mrpack into a new/existing instance directory.
pub async fn import_mrpack(
    cfg: &FwlConfig,
    mrpack_path: &Path,
    instance: &Instance,
) -> Result<MrpackReport> {
    let file = std::fs::File::open(mrpack_path)?;
    let mut archive = ZipArchive::new(file).map_err(|e| FwlError::Store(e.to_string()))?;
    let mut index_text = String::new();
    {
        let mut index = archive
            .by_name("modrinth.index.json")
            .map_err(|_| FwlError::Store("不是有效的 mrpack（缺少 modrinth.index.json）".into()))?;
        index.read_to_string(&mut index_text)?;
    }
    let index: serde_json::Value = serde_json::from_str(&index_text)?;
    let files = index
        .get("files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let game_dir = instance.game_dir(&cfg.data_dir);
    ensure_dir(&game_dir)?;
    let mut downloaded = 0u32;
    let mut skipped = 0u32;
    let mut missing = Vec::new();

    for f in &files {
        let path = f.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let downloads = f
            .get("downloads")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let dest = game_dir.join(path);
        if let Some(url) = downloads.first().and_then(|v| v.as_str()) {
            match download_file(cfg, url, &dest, None).await {
                Ok(()) => downloaded += 1,
                Err(e) => {
                    skipped += 1;
                    missing.push(format!("{path}: {e}"));
                }
            }
        } else {
            skipped += 1;
            missing.push(format!("{path}: 无下载地址"));
        }
    }

    // overrides/
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| FwlError::Store(e.to_string()))?;
        let name = file.name().to_string();
        if let Some(rel) = name.strip_prefix("overrides/") {
            if rel.is_empty() || name.ends_with('/') {
                continue;
            }
            let dest = game_dir.join(rel);
            if let Some(parent) = dest.parent() {
                ensure_dir(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut file, &mut out)?;
            out.flush()?;
        }
    }

    Ok(MrpackReport {
        name: index
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("mrpack")
            .into(),
        downloaded,
        skipped,
        missing,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackReport {
    pub name: String,
    pub downloaded: u32,
    pub skipped: u32,
    pub missing: Vec<String>,
}

pub fn export_mrpack_stub(
    cfg: &FwlConfig,
    instance: &Instance,
    out_path: &Path,
) -> Result<()> {
    // Minimal export: zip mods folder + basic index
    let game = instance.game_dir(&cfg.data_dir);
    let file = std::fs::File::create(out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut files_json = Vec::new();
    let mods = game.join("mods");
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
            zip.start_file(format!("overrides/{rel}"), opts)?;
            let data = std::fs::read(e.path())?;
            zip.write_all(&data)?;
            files_json.push(serde_json::json!({
                "path": rel,
                "downloads": [],
                "env": { "client": "required", "server": "optional" }
            }));
        }
    }

    let index = serde_json::json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "fwl-export",
        "name": instance.name,
        "summary": "Exported by FreshwaterLauncher",
        "files": files_json,
        "dependencies": {
            "minecraft": instance.version_id
        }
    });
    zip.start_file("modrinth.index.json", opts)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    zip.finish().map_err(|e| FwlError::Store(e.to_string()))?;
    Ok(())
}
