use crate::config::FwlConfig;
use crate::error::Result;
use crate::paths::{ensure_dir, java_dir};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaRuntime {
    pub path: String,
    pub version: String,
    pub major: u32,
}

pub fn detect_java(cfg: &FwlConfig) -> Vec<JavaRuntime> {
    let mut found = Vec::new();
    if let Some(p) = &cfg.java_path {
        if let Some(rt) = probe_java(Path::new(p)) {
            found.push(rt);
        }
    }
    let candidates = [
        std::env::var("JAVA_HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(if cfg!(windows) { "bin\\java.exe" } else { "bin/java" })),
        which("java"),
    ];
    for c in candidates.into_iter().flatten() {
        if let Some(rt) = probe_java(&c) {
            if !found.iter().any(|f| f.path == rt.path) {
                found.push(rt);
            }
        }
    }
    // bundled
    let bundled = java_dir(&cfg.data_dir);
    if bundled.exists() {
        if let Ok(entries) = std::fs::read_dir(&bundled) {
            for e in entries.flatten() {
                let java = e.path().join(if cfg!(windows) {
                    "bin\\java.exe"
                } else {
                    "bin/java"
                });
                if let Some(rt) = probe_java(&java) {
                    if !found.iter().any(|f| f.path == rt.path) {
                        found.push(rt);
                    }
                }
            }
        }
    }
    found
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(if cfg!(windows) {
            format!("{cmd}.exe")
        } else {
            cmd.to_string()
        });
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn probe_java(path: &Path) -> Option<JavaRuntime> {
    if !path.exists() {
        return None;
    }
    let out = Command::new(path).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let version = text.lines().next().unwrap_or("unknown").to_string();
    let major = parse_major(&version).unwrap_or(8);
    Some(JavaRuntime {
        path: path.to_string_lossy().into(),
        version,
        major,
    })
}

fn parse_major(version_line: &str) -> Option<u32> {
    // java version "1.8.0_391" or "17.0.1"
    let q: Vec<_> = version_line.split('"').collect();
    let v = if q.len() >= 2 { q[1] } else { version_line };
    if let Some(rest) = v.strip_prefix("1.") {
        rest.split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse().ok())
    } else {
        v.split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse().ok())
    }
}

pub fn resolve_java(cfg: &FwlConfig, required_major: u32) -> Result<JavaRuntime> {
    let runtimes = detect_java(cfg);
    if let Some(rt) = runtimes
        .iter()
        .find(|r| r.major >= required_major)
        .cloned()
    {
        return Ok(rt);
    }
    // Android uses fwl-android-runtime JRE (downloaded separately); plan build only needs a stub path.
    #[cfg(target_os = "android")]
    {
        let _ = cfg;
        return Ok(JavaRuntime {
            path: "android-runtime".into(),
            version: format!("android-placeholder-{required_major}"),
            major: required_major,
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        Err(FwlError::Launch(format!(
            "未找到 Java {required_major}+，请在设置中指定 Java 路径或安装 JDK"
        )))
    }
}

pub fn ensure_java_dir(cfg: &FwlConfig) -> Result<PathBuf> {
    let d = java_dir(&cfg.data_dir);
    ensure_dir(&d)?;
    Ok(d)
}
