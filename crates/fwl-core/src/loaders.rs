use crate::config::FwlConfig;
use crate::download::{download_file, DownloadQueue};
use crate::error::{FwlError, Result};
use crate::paths::{ensure_dir, versions_dir};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Fabric,
    Quilt,
    Forge,
    Neoforge,
    Optifine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderInstallRequest {
    pub kind: LoaderKind,
    pub mc_version: String,
    pub loader_version: Option<String>,
}

/// Install Fabric loader profile into versions/ (uses Fabric meta API).
pub async fn install_fabric(
    cfg: &FwlConfig,
    queue: &Arc<DownloadQueue>,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let _ = queue;
    let client = reqwest::Client::new();
    let loader = if let Some(v) = loader_version {
        v.to_string()
    } else {
        let url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{mc_version}"
        );
        let list: serde_json::Value = client.get(&url).send().await?.error_for_status()?.json().await?;
        list.as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.pointer("/loader/version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| FwlError::msg("无法解析 Fabric loader 版本"))?
            .to_string()
    };

    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{mc_version}/{loader}/profile/json"
    );
    let profile_text = client
        .get(&profile_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let profile: serde_json::Value = serde_json::from_str(&profile_text)?;
    let id = profile
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("fabric-profile")
        .to_string();

    let vdir = versions_dir(&cfg.data_dir).join(&id);
    ensure_dir(&vdir)?;
    std::fs::write(vdir.join(format!("{id}.json")), serde_json::to_string_pretty(&profile)?)?;

    // Ensure inherited vanilla version exists if possible
    if let Some(parent) = profile.get("inheritsFrom").and_then(|v| v.as_str()) {
        let parent_json = versions_dir(&cfg.data_dir)
            .join(parent)
            .join(format!("{parent}.json"));
        if !parent_json.exists() {
            tracing::warn!("Fabric profile inherits {parent}; install that vanilla version first if launch fails");
        }
    }

    // Download libraries listed in profile
    if let Some(libs) = profile.get("libraries").and_then(|v| v.as_array()) {
        for lib in libs {
            if let Some(url) = lib.pointer("/url").and_then(|v| v.as_str()) {
                let name = lib.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let path = maven_path(name);
                let dest = crate::paths::libraries_dir(&cfg.data_dir).join(&path);
                let full_url = if url.ends_with('/') {
                    format!("{url}{path}")
                } else {
                    format!("{url}/{path}")
                };
                let rewritten = cfg.rewrite_download_url(&full_url);
                // fabric maven may not be on bmcl - try original on failure
                if download_file(cfg, &rewritten, &dest, None).await.is_err() {
                    download_file(cfg, &full_url, &dest, None).await?;
                }
            } else if let Some(art) = lib.pointer("/downloads/artifact") {
                let url = art.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let path = art
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        maven_path(lib.get("name").and_then(|v| v.as_str()).unwrap_or(""))
                    });
                let dest = crate::paths::libraries_dir(&cfg.data_dir).join(&path);
                let sha1 = art.get("sha1").and_then(|v| v.as_str());
                download_file(cfg, &cfg.rewrite_download_url(url), &dest, sha1).await?;
            }
        }
    }

    Ok(id)
}

pub async fn install_quilt(
    cfg: &FwlConfig,
    queue: &Arc<DownloadQueue>,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let _ = queue;
    let client = reqwest::Client::new();
    let loader = if let Some(v) = loader_version {
        v.to_string()
    } else {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mc_version}");
        let list: serde_json::Value = client.get(&url).send().await?.error_for_status()?.json().await?;
        list.as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.pointer("/loader/version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| FwlError::msg("无法解析 Quilt loader 版本"))?
            .to_string()
    };
    let profile_url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mc_version}/{loader}/profile/json"
    );
    let profile_text = client
        .get(&profile_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let profile: serde_json::Value = serde_json::from_str(&profile_text)?;
    let id = profile
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("quilt-profile")
        .to_string();
    let vdir = versions_dir(&cfg.data_dir).join(&id);
    ensure_dir(&vdir)?;
    std::fs::write(vdir.join(format!("{id}.json")), serde_json::to_string_pretty(&profile)?)?;
    Ok(id)
}

/// Forge / NeoForge / OptiFine: record intent + open installer guidance.
/// Full silent installers vary by version; we fetch recommended version lists.
pub async fn list_forge_versions(mc_version: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://bmclapi2.bangbang93.com/forge/minecraft/{mc_version}"
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let list: serde_json::Value = resp.json().await?;
    let mut out = Vec::new();
    if let Some(arr) = list.as_array() {
        for v in arr {
            if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                out.push(ver.to_string());
            }
        }
    }
    Ok(out)
}

pub async fn install_forge_profile(
    cfg: &FwlConfig,
    mc_version: &str,
    forge_version: &str,
) -> Result<String> {
    // BMCLAPI provides installer jar; we download installer for user-assisted or headless attempt
    let id = format!("{mc_version}-forge-{forge_version}");
    let vdir = versions_dir(&cfg.data_dir).join(&id);
    ensure_dir(&vdir)?;
    let installer_url = format!(
        "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{mc_version}-{forge_version}/forge-{mc_version}-{forge_version}-installer.jar"
    );
    let installer = vdir.join("forge-installer.jar");
    download_file(cfg, &installer_url, &installer, None).await?;
    // Write a stub marker profile pointing users/runtime to run installer --installClient
    let marker = serde_json::json!({
        "id": id,
        "fwl": {
            "pendingInstaller": installer.to_string_lossy(),
            "kind": "forge",
            "mc": mc_version,
            "loader": forge_version,
            "hint": "Run: java -jar forge-installer.jar --installClient"
        }
    });
    std::fs::write(vdir.join(format!("{id}.json")), serde_json::to_string_pretty(&marker)?)?;
    Ok(id)
}

pub async fn list_neoforge_versions(mc_version: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://bmclapi2.bangbang93.com/neoforge/list/{mc_version}"
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let list: serde_json::Value = resp.json().await?;
    let mut out = Vec::new();
    if let Some(arr) = list.as_array() {
        for v in arr {
            if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                out.push(ver.to_string());
            }
        }
    }
    Ok(out)
}

pub fn optifine_download_page(mc_version: &str) -> String {
    format!("https://optifine.net/downloads#version_{mc_version}")
}

fn maven_path(name: &str) -> String {
    let parts: Vec<_> = name.split(':').collect();
    if parts.len() < 3 {
        return format!("{}.jar", name.replace(':', "/"));
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
