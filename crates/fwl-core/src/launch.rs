use crate::accounts::Account;
use crate::config::FwlConfig;
use crate::error::{FwlError, Result};
use crate::instances::Instance;
use crate::java::resolve_java;
use crate::paths::{assets_dir, libraries_dir, logs_dir, versions_dir};
use crate::versions::{library_allowed, VersionJson};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, serde::Serialize)]
pub struct LaunchPlan {
    pub java: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub main_class: String,
}

fn load_resolved_version(cfg: &FwlConfig, version_id: &str) -> Result<VersionJson> {
    let vdir = versions_dir(&cfg.data_dir).join(version_id);
    let vjson_path = vdir.join(format!("{version_id}.json"));
    if !vjson_path.exists() {
        return Err(FwlError::Launch(format!(
            "版本未安装: {version_id}，请先下载"
        )));
    }
    let mut version: VersionJson = serde_json::from_str(&std::fs::read_to_string(&vjson_path)?)?;
    if let Some(parent_id) = version.inheritsFrom.clone() {
        let parent = load_resolved_version(cfg, &parent_id)?;
        // merge libraries
        let mut libs = parent.libraries;
        libs.extend(version.libraries.drain(..));
        version.libraries = libs;
        if version.main_class.is_empty() {
            version.main_class = parent.main_class;
        }
        if version.downloads.is_none() {
            version.downloads = parent.downloads;
        }
        if version.assetIndex.is_none() {
            version.assetIndex = parent.assetIndex;
        }
        if version.assets.is_none() {
            version.assets = parent.assets;
        }
        if version.javaVersion.is_none() {
            version.javaVersion = parent.javaVersion;
        }
        if version.arguments.is_none() {
            version.arguments = parent.arguments;
        }
        if version.minecraft_arguments.is_none() {
            version.minecraft_arguments = parent.minecraft_arguments;
        }
        // client jar lives in parent version dir — rewrite id for classpath jar lookup
        if !versions_dir(&cfg.data_dir)
            .join(&version.id)
            .join(format!("{}.jar", version.id))
            .exists()
        {
            // keep id for display but classpath builder uses version.id for jar name;
            // point jar to parent by temporarily using parent id in path helper below
            version.id = parent.id;
        }
    }
    Ok(version)
}

pub fn build_launch_plan(
    cfg: &FwlConfig,
    instance: &Instance,
    account: &Account,
) -> Result<LaunchPlan> {
    let version_id = &instance.version_id;
    let version = load_resolved_version(cfg, version_id)?;
    let jar_id = version.id.clone();
    let vdir = versions_dir(&cfg.data_dir).join(&jar_id);
    let required = version
        .javaVersion
        .as_ref()
        .map(|j| j.majorVersion)
        .unwrap_or(8);
    let java = resolve_java(cfg, required)?;

    let game_dir = instance.game_dir(&cfg.data_dir);
    std::fs::create_dir_all(&game_dir)?;

    let natives_dir = game_dir.join("natives");
    std::fs::create_dir_all(&natives_dir)?;

    let classpath = build_classpath(cfg, &version, &vdir)?;
    let assets = assets_dir(&cfg.data_dir);

    let mut args = Vec::new();
    args.push(format!("-Xms{}M", cfg.min_memory_mb));
    args.push(format!("-Xmx{}M", cfg.max_memory_mb));
    if !cfg.jvm_args.trim().is_empty() {
        args.extend(cfg.jvm_args.split_whitespace().map(|s| s.to_string()));
    }
    args.push(format!(
        "-Djava.library.path={}",
        natives_dir.to_string_lossy()
    ));
    args.push("-cp".into());
    args.push(classpath);
    args.push(version.main_class.clone());

    // game args (legacy + modern simplified)
    let game_args = expand_game_args(&version, cfg, instance, account, &assets, &vdir)?;
    args.extend(game_args);

    Ok(LaunchPlan {
        java: java.path,
        args,
        cwd: game_dir.to_string_lossy().into(),
        main_class: version.main_class,
    })
}

fn build_classpath(cfg: &FwlConfig, version: &VersionJson, vdir: &Path) -> Result<String> {
    let libs = libraries_dir(&cfg.data_dir);
    let mut entries = Vec::new();
    for lib in &version.libraries {
        if !library_allowed(lib) {
            continue;
        }
        if let Some(dl) = &lib.downloads {
            if let Some(art) = &dl.artifact {
                let rel = art
                    .path
                    .clone()
                    .unwrap_or_else(|| maven_path(&lib.name));
                entries.push(libs.join(rel));
            }
        }
    }
    let jar = vdir.join(format!("{}.jar", version.id));
    entries.push(jar);
    let sep = if cfg!(windows) { ";" } else { ":" };
    Ok(entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(sep))
}

fn maven_path(name: &str) -> String {
    let parts: Vec<_> = name.split(':').collect();
    if parts.len() < 3 {
        return name.replace(':', "/");
    }
    let group = parts[0].replace('.', "/");
    format!(
        "{}/{}/{}/{}-{}.jar",
        group, parts[1], parts[2], parts[1], parts[2]
    )
}

fn expand_game_args(
    version: &VersionJson,
    cfg: &FwlConfig,
    instance: &Instance,
    account: &Account,
    assets: &Path,
    vdir: &Path,
) -> Result<Vec<String>> {
    let mut map = std::collections::HashMap::new();
    map.insert("${auth_player_name}", account.username.clone());
    map.insert("${version_name}", version.id.clone());
    map.insert(
        "${game_directory}",
        instance.game_dir(&cfg.data_dir).to_string_lossy().into(),
    );
    map.insert("${assets_root}", assets.to_string_lossy().into());
    map.insert(
        "${assets_index_name}",
        version
            .assetIndex
            .as_ref()
            .map(|a| a.id.clone())
            .or_else(|| version.assets.clone())
            .unwrap_or_else(|| "legacy".into()),
    );
    map.insert("${auth_uuid}", account.uuid.clone());
    map.insert("${auth_access_token}", account.access_token.clone());
    map.insert("${clientid}", cfg.ms_client_id.clone());
    map.insert("${auth_xuid}", "0".into());
    map.insert("${user_type}", "msa".into());
    map.insert("${version_type}", "FWL".into());
    map.insert("${user_properties}", "{}".into());
    map.insert(
        "${natives_directory}",
        instance
            .game_dir(&cfg.data_dir)
            .join("natives")
            .to_string_lossy()
            .into(),
    );
    map.insert(
        "${launcher_name}",
        "FreshwaterLauncher".into(),
    );
    map.insert("${launcher_version}", "0.1.0".into());
    map.insert(
        "${classpath}",
        build_classpath(cfg, version, vdir)?,
    );

    if let Some(legacy) = &version.minecraft_arguments {
        let mut out = Vec::new();
        for token in legacy.split_whitespace() {
            out.push(replace_tokens(token, &map));
        }
        return Ok(out);
    }

    let mut out = Vec::new();
    if let Some(args) = &version.arguments {
        if let Some(game) = &args.game {
            for v in game {
                match v {
                    serde_json::Value::String(s) => out.push(replace_tokens(s, &map)),
                    serde_json::Value::Object(obj) => {
                        // skip conditional rules for simplicity unless value is plain array of strings
                        if let Some(serde_json::Value::Array(vals)) = obj.get("value") {
                            for item in vals {
                                if let Some(s) = item.as_str() {
                                    out.push(replace_tokens(s, &map));
                                }
                            }
                        } else if let Some(serde_json::Value::String(s)) = obj.get("value") {
                            out.push(replace_tokens(s, &map));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if out.is_empty() {
        // minimal fallback
        out.extend([
            account.username.clone(),
            version.id.clone(),
            "--gameDir".into(),
            instance.game_dir(&cfg.data_dir).to_string_lossy().into(),
            "--assetsDir".into(),
            assets.to_string_lossy().into(),
            "--uuid".into(),
            account.uuid.clone(),
            "--accessToken".into(),
            account.access_token.clone(),
            "--userType".into(),
            "msa".into(),
        ]);
    }
    Ok(out)
}

fn replace_tokens(s: &str, map: &std::collections::HashMap<&str, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in map {
        out = out.replace(k, v);
    }
    out
}

pub fn launch_game(plan: &LaunchPlan) -> Result<u32> {
    let log_dir = PathBuf::from(&plan.cwd).join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let mut cmd = Command::new(&plan.java);
    cmd.args(&plan.args)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd
        .spawn()
        .map_err(|e| FwlError::Launch(format!("无法启动 Java: {e}")))?;
    Ok(child.id())
}

pub fn recent_crash_summary(cfg: &FwlConfig, instance: &Instance) -> Option<String> {
    let crash_dir = instance.game_dir(&cfg.data_dir).join("crash-reports");
    let mut latest: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(crash_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("txt") {
                if latest.as_ref().map(|l| p > *l).unwrap_or(true) {
                    latest = Some(p);
                }
            }
        }
    }
    let path = latest?;
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.lines().take(40).collect::<Vec<_>>().join("\n"))
}

pub fn read_latest_log(cfg: &FwlConfig, instance: &Instance) -> Option<String> {
    let log = instance.game_dir(&cfg.data_dir).join("logs").join("latest.log");
    let fwl_logs = logs_dir(&cfg.data_dir);
    let path = if log.exists() {
        log
    } else {
        fwl_logs.join("latest.log")
    };
    std::fs::read_to_string(path).ok()
}
