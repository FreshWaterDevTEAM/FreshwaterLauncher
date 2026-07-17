//! Android Mobile Runtime for Minecraft: Java Edition.
//!
//! Flow:
//! 1. Ensure portable Android JRE under `runtimes/android-<abi>/jre`
//! 2. Write `fwl-android-launch.json` for Kotlin GameActivity
//! 3. Kotlin starts an external `java` process (separate process; not GPL-linked)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const LAUNCH_FILE_NAME: &str = "fwl-android-launch.json";
pub const RUNTIME_MANIFEST_NAME: &str = "fwl-android-runtime.json";
pub const DEFAULT_RUNTIME_INDEX: &str =
    "https://raw.githubusercontent.com/FreshWaterDevTEAM/FreshwaterLauncher/master/android-runtime/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AndroidLaunchRequest {
    pub instance_id: String,
    pub game_dir: String,
    pub version_id: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub assets_dir: String,
    pub libraries_dir: String,
    pub client_jar: String,
    pub classpath: Vec<String>,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub natives_dir: String,
    pub assets_index: String,
    pub java_home: Option<String>,
    pub abi: String,
    /// Absolute path written for the Android activity to pick up.
    pub launch_file: Option<String>,
}

impl Default for AndroidLaunchRequest {
    fn default() -> Self {
        Self {
            instance_id: String::new(),
            game_dir: String::new(),
            version_id: String::new(),
            username: String::new(),
            uuid: String::new(),
            access_token: String::new(),
            assets_dir: String::new(),
            libraries_dir: String::new(),
            client_jar: String::new(),
            classpath: vec![],
            main_class: "net.minecraft.client.main.Main".into(),
            jvm_args: vec![],
            game_args: vec![],
            natives_dir: String::new(),
            assets_index: "legacy".into(),
            java_home: None,
            abi: detect_abi(),
            launch_file: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidLaunchResult {
    pub ok: bool,
    pub message: String,
    pub launch_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeIndex {
    pub version: String,
    pub abi: String,
    pub jre_url: String,
    pub jre_sha256: Option<String>,
    pub natives_url: Option<String>,
    pub natives_sha256: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub java_home: Option<String>,
    pub java_bin: Option<String>,
    pub abi: String,
    pub message: String,
}

pub trait LaunchBackend {
    fn prepare(&self, req: &AndroidLaunchRequest, data_dir: &Path) -> AndroidLaunchResult;
}

#[derive(Default)]
pub struct FwlAndroidBackend;

impl LaunchBackend for FwlAndroidBackend {
    fn prepare(&self, req: &AndroidLaunchRequest, data_dir: &Path) -> AndroidLaunchResult {
        let abi = if req.abi.is_empty() {
            detect_abi()
        } else {
            req.abi.clone()
        };
        let runtime_root = runtime_dir(data_dir, &abi);
        let mut req = req.clone();
        req.abi = abi.clone();
        if req.java_home.is_none() {
            let status = probe_runtime(data_dir);
            req.java_home = status.java_home;
        }
        match write_launch_file(&req, &runtime_root) {
            Ok(path) => AndroidLaunchResult {
                ok: true,
                message: format!("Android launch prepared ({})", path.display()),
                launch_file: Some(path.to_string_lossy().into()),
            },
            Err(e) => AndroidLaunchResult {
                ok: false,
                message: e,
                launch_file: None,
            },
        }
    }
}

pub fn default_backend() -> FwlAndroidBackend {
    FwlAndroidBackend
}

pub fn detect_abi() -> String {
    #[cfg(target_arch = "aarch64")]
    {
        return "arm64-v8a".into();
    }
    #[cfg(all(target_arch = "arm", not(target_arch = "aarch64")))]
    {
        return "armeabi-v7a".into();
    }
    #[cfg(target_arch = "x86_64")]
    {
        return "x86_64".into();
    }
    #[cfg(all(target_arch = "x86", not(target_arch = "x86_64")))]
    {
        return "x86".into();
    }
    #[allow(unreachable_code)]
    "arm64-v8a".into()
}

pub fn runtime_dir(data_dir: &Path, abi: &str) -> PathBuf {
    data_dir.join("runtimes").join(format!("android-{abi}"))
}

pub fn java_executable(java_home: &Path) -> PathBuf {
    let marker = java_home.join(".fwl-jre-root");
    let root = if marker.exists() {
        PathBuf::from(std::fs::read_to_string(marker).unwrap_or_default().trim())
    } else {
        java_home.to_path_buf()
    };
    for c in [
        root.join("bin/java"),
        root.join("jre/bin/java"),
        root.join("java"),
        java_home.join("bin/java"),
    ] {
        if c.exists() {
            return c;
        }
    }
    // search one level
    if let Ok(rd) = std::fs::read_dir(java_home) {
        for e in rd.flatten() {
            let p = e.path().join("bin/java");
            if p.exists() {
                return p;
            }
        }
    }
    root.join("bin/java")
}

pub fn probe_runtime(data_dir: &Path) -> RuntimeStatus {
    let abi = detect_abi();
    let root = runtime_dir(data_dir, &abi);
    let java_home = root.join("jre");
    let java = java_executable(&java_home);
    if java.exists() {
        RuntimeStatus {
            ready: true,
            java_home: Some(java_home.to_string_lossy().into()),
            java_bin: Some(java.to_string_lossy().into()),
            abi,
            message: "Android JRE ready".into(),
        }
    } else {
        RuntimeStatus {
            ready: false,
            java_home: None,
            java_bin: None,
            abi,
            message: format!("需要下载 Android Runtime（目录 {}）", root.display()),
        }
    }
}

pub fn write_launch_file(req: &AndroidLaunchRequest, runtime_root: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(runtime_root).map_err(|e| e.to_string())?;
    let mut req = req.clone();
    let path = runtime_root.join(LAUNCH_FILE_NAME);
    req.launch_file = Some(path.to_string_lossy().into());
    // also copy to game_dir for activity fallback
    if !req.game_dir.is_empty() {
        let gd = PathBuf::from(&req.game_dir);
        let _ = std::fs::create_dir_all(&gd);
        let _ = std::fs::write(
            gd.join(LAUNCH_FILE_NAME),
            serde_json::to_string_pretty(&req).unwrap_or_default(),
        );
    }
    let text = serde_json::to_string_pretty(&req).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn classpath_join(entries: &[String]) -> String {
    entries.join(":")
}

/// Split a desktop `LaunchPlan`-style arg list into JVM / classpath / game args.
/// Expects: [jvm..., -cp, classpath, mainClass, game...]
pub fn split_launch_args(
    args: &[String],
    main_class: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut jvm = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-cp" || args[i] == "-classpath" {
            i += 1;
            let cp = args.get(i).cloned().unwrap_or_default();
            i += 1;
            if args.get(i).map(|s| s.as_str()) == Some(main_class) {
                i += 1;
            }
            let sep = if cp.contains(';') && !cp.contains(':') {
                ';'
            } else {
                ':'
            };
            let classpath = cp
                .split(sep)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            let game = args[i..].to_vec();
            return (jvm, classpath, game);
        }
        jvm.push(args[i].clone());
        i += 1;
    }
    (jvm, Vec::new(), Vec::new())
}

pub fn request_from_plan(
    instance_id: &str,
    version_id: &str,
    username: &str,
    uuid: &str,
    access_token: &str,
    game_dir: &str,
    assets_dir: &str,
    libraries_dir: &str,
    natives_dir: &str,
    assets_index: &str,
    main_class: &str,
    args: &[String],
) -> AndroidLaunchRequest {
    let (jvm_args, classpath, game_args) = split_launch_args(args, main_class);
    let client_jar = classpath
        .iter()
        .find(|p| p.ends_with(".jar") && p.contains(version_id))
        .cloned()
        .or_else(|| classpath.last().cloned())
        .unwrap_or_default();
    AndroidLaunchRequest {
        instance_id: instance_id.into(),
        game_dir: game_dir.into(),
        version_id: version_id.into(),
        username: username.into(),
        uuid: uuid.into(),
        access_token: access_token.into(),
        assets_dir: assets_dir.into(),
        libraries_dir: libraries_dir.into(),
        client_jar,
        classpath,
        main_class: main_class.into(),
        jvm_args,
        game_args,
        natives_dir: natives_dir.into(),
        assets_index: assets_index.into(),
        java_home: None,
        abi: detect_abi(),
        launch_file: None,
    }
}

fn write_jre_root_marker(jre_dir: &Path) {
    let java = java_executable(jre_dir);
    if !java.exists() {
        return;
    }
    if let Some(home) = java.parent().and_then(|b| b.parent()) {
        let _ = std::fs::write(jre_dir.join(".fwl-jre-root"), home.to_string_lossy().as_bytes());
    }
}

pub mod ensure {
    use super::*;

    pub async fn ensure_android_runtime(
        data_dir: &Path,
        index_url: Option<&str>,
    ) -> Result<RuntimeStatus, String> {
        #[cfg(not(target_os = "android"))]
        {
            let _ = index_url;
            return Ok(probe_runtime(data_dir));
        }

        #[cfg(target_os = "android")]
        {
            ensure_android_runtime_inner(data_dir, index_url).await
        }
    }

    #[cfg(target_os = "android")]
    async fn ensure_android_runtime_inner(
        data_dir: &Path,
        index_url: Option<&str>,
    ) -> Result<RuntimeStatus, String> {
        let abi = detect_abi();
        let root = runtime_dir(data_dir, &abi);
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

        let status = probe_runtime(data_dir);
        if status.ready {
            return Ok(status);
        }

        let url = index_url.unwrap_or(DEFAULT_RUNTIME_INDEX);
        let client = reqwest::Client::new();
        let index: RuntimeIndex = client
            .get(url)
            .header("User-Agent", "FreshwaterLauncher/0.1.0")
            .send()
            .await
            .map_err(|e| format!("runtime index: {e}"))?
            .error_for_status()
            .map_err(|e| format!("runtime index http: {e}"))?
            .json()
            .await
            .map_err(|e| format!("runtime index json: {e}"))?;

        let archive = root.join("jre-download");
        download(
            &client,
            &index.jre_url,
            &archive,
            index.jre_sha256.as_deref(),
        )
        .await?;

        let jre_dir = root.join("jre");
        let _ = std::fs::remove_dir_all(&jre_dir);
        std::fs::create_dir_all(&jre_dir).map_err(|e| e.to_string())?;

        if index.jre_url.ends_with(".zip") {
            extract_zip(&archive, &jre_dir)?;
        } else if index.jre_url.contains(".tar.xz") || index.jre_url.ends_with(".txz") {
            extract_tar_xz(&archive, &jre_dir)?;
        } else if index.jre_url.contains(".tar.gz") || index.jre_url.ends_with(".tgz") {
            extract_tar_gz(&archive, &jre_dir)?;
        } else if extract_tar_xz(&archive, &jre_dir).is_err() {
            if extract_tar_gz(&archive, &jre_dir).is_err() {
                extract_zip(&archive, &jre_dir)?;
            }
        }

        write_jre_root_marker(&jre_dir);

        if let Some(nurl) = &index.natives_url {
            let nzip = root.join("natives-download");
            download(&client, nurl, &nzip, index.natives_sha256.as_deref()).await?;
            let ndir = root.join("natives");
            let _ = std::fs::remove_dir_all(&ndir);
            if nurl.ends_with(".zip") {
                extract_zip(&nzip, &ndir)?;
            } else {
                let _ = extract_tar_gz(&nzip, &ndir);
            }
        }

        std::fs::write(
            root.join(RUNTIME_MANIFEST_NAME),
            serde_json::to_string_pretty(&index).unwrap_or_default(),
        )
        .map_err(|e| e.to_string())?;

        let status = probe_runtime(data_dir);
        if !status.ready {
            return Err(format!(
                "Runtime downloaded but java binary not found under {}",
                jre_dir.display()
            ));
        }
        Ok(status)
    }

    #[cfg(target_os = "android")]
    async fn download(
        client: &reqwest::Client,
        url: &str,
        dest: &Path,
        sha256: Option<&str>,
    ) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = client
            .get(url)
            .header("User-Agent", "FreshwaterLauncher/0.1.0")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
        if let Some(expected) = sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let got = hex::encode(hasher.finalize());
            if !got.eq_ignore_ascii_case(expected) {
                return Err(format!("sha256 mismatch for {url}"));
            }
        }
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
        let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = file.name().to_string();
            let out_path = dest.join(Path::new(&name));
            if name.ends_with('/') {
                std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn extract_tar_gz(path: &Path, dest: &Path) -> Result<(), String> {
        use flate2::read::GzDecoder;
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let gz = GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(dest).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn extract_tar_xz(path: &Path, dest: &Path) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let xz = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(xz);
        archive.unpack(dest).map_err(|e| e.to_string())?;
        Ok(())
    }
}
