//! Android Mobile Runtime for launching Minecraft Java Edition on ARM devices.
//!
//! Desktop builds compile this crate as a stub. On Android, Tauri will call into
//! `AndroidLaunchBackend` which is responsible for:
//! - preparing the instance directory (same layout as desktop `.minecraft` / instances)
//! - starting the embedded JVM / rendering bridge
//! - forwarding input and lifecycle events
//!
//! Full GLES/JVM glue is intentionally isolated here so `fwl-core` stays portable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidLaunchRequest {
    pub instance_id: String,
    pub game_dir: String,
    pub version_id: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub assets_dir: String,
    pub libraries_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidLaunchResult {
    pub ok: bool,
    pub message: String,
}

pub trait LaunchBackend {
    fn launch(&self, req: AndroidLaunchRequest) -> AndroidLaunchResult;
}

/// Stub backend used on non-Android targets and until the native bridge lands.
pub struct StubAndroidBackend;

impl LaunchBackend for StubAndroidBackend {
    fn launch(&self, req: AndroidLaunchRequest) -> AndroidLaunchResult {
        AndroidLaunchResult {
            ok: false,
            message: format!(
                "Android runtime scaffold ready for instance {} (version {}). Native JVM/GLES bridge not linked in this build.",
                req.instance_id, req.version_id
            ),
        }
    }
}

#[cfg(target_os = "android")]
pub mod jni_bridge {
    use super::*;

    /// Placeholder entry the Android activity will call via JNI.
    pub fn launch_from_jni(req: AndroidLaunchRequest) -> AndroidLaunchResult {
        // Future: load libjvm / Pojav-style glue under a compatible license,
        // or a self-built renderer bridge.
        StubAndroidBackend.launch(req)
    }
}

pub fn default_backend() -> StubAndroidBackend {
    StubAndroidBackend
}
