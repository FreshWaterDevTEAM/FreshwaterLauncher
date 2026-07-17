use crate::config::FwlConfig;
use crate::error::Result;
use crate::paths::{ensure_dir, instances_dir, versions_dir};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub game_dir_name: String,
    pub sync_platform: Option<String>,
    pub last_played: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InstanceStore {
    pub instances: Vec<Instance>,
}

impl Instance {
    pub fn game_dir(&self, data_dir: &Path) -> PathBuf {
        instances_dir(data_dir).join(&self.game_dir_name)
    }

    pub fn mods_dir(&self, data_dir: &Path) -> PathBuf {
        self.game_dir(data_dir).join("mods")
    }

    pub fn shaderpacks_dir(&self, data_dir: &Path) -> PathBuf {
        self.game_dir(data_dir).join("shaderpacks")
    }

    pub fn resourcepacks_dir(&self, data_dir: &Path) -> PathBuf {
        self.game_dir(data_dir).join("resourcepacks")
    }
}

impl InstanceStore {
    fn path(cfg: &FwlConfig) -> PathBuf {
        cfg.data_dir.join("instances.json")
    }

    pub fn load(cfg: &FwlConfig) -> Result<Self> {
        ensure_dir(&cfg.data_dir)?;
        let path = Self::path(cfg);
        if path.exists() {
            Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, cfg: &FwlConfig) -> Result<()> {
        ensure_dir(&cfg.data_dir)?;
        std::fs::write(Self::path(cfg), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn create(
        &mut self,
        cfg: &FwlConfig,
        name: &str,
        version_id: &str,
    ) -> Result<Instance> {
        let id = uuid::Uuid::new_v4().to_string();
        let game_dir_name = sanitize(name);
        let inst = Instance {
            id: id.clone(),
            name: name.to_string(),
            version_id: version_id.to_string(),
            game_dir_name: game_dir_name.clone(),
            sync_platform: None,
            last_played: None,
        };
        let dir = inst.game_dir(&cfg.data_dir);
        ensure_dir(&dir)?;
        ensure_dir(&dir.join("mods"))?;
        ensure_dir(&dir.join("resourcepacks"))?;
        ensure_dir(&dir.join("shaderpacks"))?;
        ensure_dir(&dir.join("saves"))?;
        ensure_dir(&dir.join("config"))?;
        self.instances.push(inst.clone());
        self.save(cfg)?;
        Ok(inst)
    }

    pub fn remove(&mut self, cfg: &FwlConfig, id: &str, delete_files: bool) -> Result<()> {
        if let Some(inst) = self.instances.iter().find(|i| i.id == id).cloned() {
            if delete_files {
                let dir = inst.game_dir(&cfg.data_dir);
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(dir);
                }
            }
        }
        self.instances.retain(|i| i.id != id);
        self.save(cfg)?;
        Ok(())
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|i| i.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        s
    }
}

/// Scan shared versions folder (compatible with PCL / vanilla .minecraft layout).
pub fn scan_installed_versions(cfg: &FwlConfig) -> Result<Vec<String>> {
    let dir = versions_dir(&cfg.data_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut ids = Vec::new();
    for e in std::fs::read_dir(dir)?.flatten() {
        if e.path().is_dir() {
            let id = e.file_name().to_string_lossy().to_string();
            let json = e.path().join(format!("{id}.json"));
            if json.exists() {
                ids.push(id);
            }
        }
    }
    ids.sort();
    Ok(ids)
}

pub fn list_local_mods(cfg: &FwlConfig, instance: &Instance) -> Result<Vec<LocalMod>> {
    let dir = instance.mods_dir(&cfg.data_dir);
    ensure_dir(&dir)?;
    let mut mods = Vec::new();
    for e in std::fs::read_dir(&dir)?.flatten() {
        let path = e.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.ends_with(".jar") || name.ends_with(".jar.disabled") {
            let enabled = name.ends_with(".jar") && !name.ends_with(".disabled");
            mods.push(LocalMod {
                file_name: name,
                enabled,
                path: path.to_string_lossy().into(),
            });
        }
    }
    mods.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(mods)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMod {
    pub file_name: String,
    pub enabled: bool,
    pub path: String,
}

pub fn set_mod_enabled(path: &str, enabled: bool) -> Result<()> {
    let p = PathBuf::from(path);
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    let parent = p.parent().unwrap();
    if enabled && name.ends_with(".jar.disabled") {
        let new_name = name.trim_end_matches(".disabled");
        std::fs::rename(&p, parent.join(new_name))?;
    } else if !enabled && name.ends_with(".jar") && !name.ends_with(".disabled") {
        std::fs::rename(&p, parent.join(format!("{name}.disabled")))?;
    }
    Ok(())
}

pub fn list_shaderpacks(cfg: &FwlConfig, instance: &Instance) -> Result<Vec<String>> {
    let dir = instance.shaderpacks_dir(&cfg.data_dir);
    ensure_dir(&dir)?;
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)?.flatten() {
        out.push(e.file_name().to_string_lossy().to_string());
    }
    out.sort();
    Ok(out)
}

pub fn list_resourcepacks(cfg: &FwlConfig, instance: &Instance) -> Result<Vec<String>> {
    let dir = instance.resourcepacks_dir(&cfg.data_dir);
    ensure_dir(&dir)?;
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)?.flatten() {
        out.push(e.file_name().to_string_lossy().to_string());
    }
    out.sort();
    Ok(out)
}
