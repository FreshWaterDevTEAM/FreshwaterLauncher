use fwl_android_runtime::{default_backend, AndroidLaunchRequest, LaunchBackend};
use fwl_core::accounts::AccountsStore;
use fwl_core::auth::{
    add_authlib_account, add_offline_account, poll_device_code, refresh_microsoft, start_device_code,
};
use fwl_core::config::{DownloadSource, FwlConfig};
use fwl_core::download::DownloadQueue;
use fwl_core::instances::{
    list_local_mods, list_resourcepacks, list_shaderpacks, scan_installed_versions, set_mod_enabled,
    InstanceStore,
};
use fwl_core::java::detect_java;
use fwl_core::launch::{
    build_launch_plan, launch_game, read_latest_log, recent_crash_summary,
};
use fwl_core::loaders::{
    install_fabric, install_forge_profile, install_quilt, list_forge_versions,
    list_neoforge_versions, optifine_download_page, LoaderKind,
};
use fwl_core::store::{
    export_mrpack_stub, get_modrinth_versions, import_mrpack, install_modrinth_version_to_instance,
    search_curseforge, search_modrinth,
};
use fwl_core::sync::{
    apply_sync, fetch_manifest, invite_code, local_sync_revision, parse_platform_input, plan_sync,
};
use fwl_core::versions::fetch_manifest as fetch_mc_manifest;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub queue: Arc<DownloadQueue>,
}

fn cfg() -> Result<FwlConfig, String> {
    FwlConfig::load_or_default().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config() -> Result<FwlConfig, String> {
    cfg()
}

#[tauri::command]
fn save_config(mut config: FwlConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_accounts() -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = AccountsStore::load(&c).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(store.accounts).unwrap())
}

#[tauri::command]
async fn ms_start_device_code() -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let start = start_device_code(&c).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(start).unwrap())
}

#[tauri::command]
async fn ms_poll_device_code(device_code: String, interval: u64) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let acc = poll_device_code(&c, &device_code, interval)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(acc).unwrap())
}

#[tauri::command]
fn add_offline(username: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let acc = add_offline_account(&c, &username).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(acc).unwrap())
}

#[tauri::command]
fn add_authlib(
    username: String,
    uuid: String,
    access_token: String,
    server: String,
) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let acc = add_authlib_account(&c, &username, &uuid, &access_token, &server)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(acc).unwrap())
}

#[tauri::command]
fn remove_account(id: String) -> Result<(), String> {
    let c = cfg()?;
    let mut store = AccountsStore::load(&c).map_err(|e| e.to_string())?;
    store.remove(&id);
    store.save(&c).map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh_account(id: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = AccountsStore::load(&c).map_err(|e| e.to_string())?;
    let acc = store
        .get(&id)
        .cloned()
        .ok_or_else(|| "账号不存在".to_string())?;
    let updated = refresh_microsoft(&c, &acc).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(updated).unwrap())
}

#[tauri::command]
async fn fetch_versions() -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let m = fetch_mc_manifest(&c).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(m).unwrap())
}

#[tauri::command]
async fn install_version(state: State<'_, AppState>, version_id: String) -> Result<String, String> {
    let c = cfg()?;
    let m = fetch_mc_manifest(&c).await.map_err(|e| e.to_string())?;
    let path = state
        .queue
        .install_version(&c, &version_id, &m)
        .await
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

#[tauri::command]
async fn download_tasks(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::to_value(state.queue.list().await).unwrap())
}

#[tauri::command]
fn list_instances() -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(store.instances).unwrap())
}

#[tauri::command]
fn create_instance(name: String, version_id: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let mut store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .create(&c, &name, &version_id)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(inst).unwrap())
}

#[tauri::command]
fn delete_instance(id: String, delete_files: bool) -> Result<(), String> {
    let c = cfg()?;
    let mut store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    store
        .remove(&c, &id, delete_files)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn scanned_versions() -> Result<Vec<String>, String> {
    let c = cfg()?;
    scan_installed_versions(&c).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_mods(instance_id: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    let mods = list_local_mods(&c, inst).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(mods).unwrap())
}

#[tauri::command]
fn toggle_mod(path: String, enabled: bool) -> Result<(), String> {
    set_mod_enabled(&path, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_shaderpacks(instance_id: String) -> Result<Vec<String>, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    list_shaderpacks(&c, inst).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_resourcepacks(instance_id: String) -> Result<Vec<String>, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    list_resourcepacks(&c, inst).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_java() -> Result<serde_json::Value, String> {
    let c = cfg()?;
    Ok(serde_json::to_value(detect_java(&c)).unwrap())
}

#[tauri::command]
async fn launch_instance(instance_id: String, account_id: String) -> Result<u32, String> {
    let mut c = cfg()?;
    let accounts = AccountsStore::load(&c).map_err(|e| e.to_string())?;
    let mut account = accounts
        .get(&account_id)
        .cloned()
        .ok_or_else(|| "请先选择账号".to_string())?;

    if account.kind == fwl_core::accounts::AccountKind::Microsoft {
        if let Ok(updated) = refresh_microsoft(&c, &account).await {
            account = updated;
        }
    }

    let mut store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .cloned()
        .ok_or_else(|| "实例不存在".to_string())?;

    // Sync hint: if platform bound, check revision (non-blocking warn via error string prefix)
    if let Some(platform) = &inst.sync_platform {
        let (base, channel) = parse_platform_input(platform);
        if let Ok(manifest) = fetch_manifest(&base, &channel).await {
            if let Ok(diff) = plan_sync(&c, &inst, &manifest) {
                if !diff.up_to_date {
                    return Err(format!(
                        "SYNC_REQUIRED:服务器平台有更新 (revision {})，请先在实例页一键同步",
                        diff.revision
                    ));
                }
            }
        }
    }

    let plan = build_launch_plan(&c, &inst, &account).map_err(|e| e.to_string())?;

    #[cfg(target_os = "android")]
    {
        let req = AndroidLaunchRequest {
            instance_id: inst.id.clone(),
            game_dir: plan.cwd.clone(),
            version_id: inst.version_id.clone(),
            username: account.username.clone(),
            uuid: account.uuid.clone(),
            access_token: account.access_token.clone(),
            assets_dir: fwl_core::paths::assets_dir(&c.data_dir)
                .to_string_lossy()
                .into(),
            libraries_dir: fwl_core::paths::libraries_dir(&c.data_dir)
                .to_string_lossy()
                .into(),
        };
        let result = default_backend().launch(req);
        if !result.ok {
            return Err(result.message);
        }
        return Ok(0);
    }

    #[cfg(not(target_os = "android"))]
    {
        let pid = launch_game(&plan).map_err(|e| e.to_string())?;
        if let Some(i) = store.get_mut(&instance_id) {
            i.last_played = Some(chrono_like_now());
        }
        let _ = store.save(&c);
        c.selected_instance = Some(instance_id);
        c.selected_account = Some(account_id);
        let _ = c.save();
        Ok(pid)
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}

#[tauri::command]
async fn install_loader(
    state: State<'_, AppState>,
    kind: String,
    mc_version: String,
    loader_version: Option<String>,
) -> Result<String, String> {
    let c = cfg()?;
    let k = match kind.as_str() {
        "fabric" => LoaderKind::Fabric,
        "quilt" => LoaderKind::Quilt,
        "forge" => LoaderKind::Forge,
        "neoforge" => LoaderKind::Neoforge,
        "optifine" => LoaderKind::Optifine,
        _ => return Err("未知加载器".into()),
    };
    match k {
        LoaderKind::Fabric => install_fabric(
            &c,
            &state.queue,
            &mc_version,
            loader_version.as_deref(),
        )
        .await
        .map_err(|e| e.to_string()),
        LoaderKind::Quilt => install_quilt(
            &c,
            &state.queue,
            &mc_version,
            loader_version.as_deref(),
        )
        .await
        .map_err(|e| e.to_string()),
        LoaderKind::Forge => {
            let ver = if let Some(v) = loader_version {
                v
            } else {
                list_forge_versions(&mc_version)
                    .await
                    .map_err(|e| e.to_string())?
                    .first()
                    .cloned()
                    .ok_or_else(|| "无可用 Forge 版本".to_string())?
            };
            install_forge_profile(&c, &mc_version, &ver)
                .await
                .map_err(|e| e.to_string())
        }
        LoaderKind::Neoforge => {
            let list = list_neoforge_versions(&mc_version)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "NeoForge versions for {mc_version}: {}",
                list.join(", ")
            ))
        }
        LoaderKind::Optifine => Ok(optifine_download_page(&mc_version)),
    }
}

#[tauri::command]
async fn store_search(
    query: String,
    project_type: String,
    source: String,
) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let results = match source.as_str() {
        "curseforge" => {
            let class = match project_type.as_str() {
                "shader" => 6552,
                "modpack" => 4471,
                _ => 6,
            };
            search_curseforge(&c, &query, class, 20)
                .await
                .map_err(|e| e.to_string())?
        }
        _ => search_modrinth(&query, &project_type, 20)
            .await
            .map_err(|e| e.to_string())?,
    };
    Ok(serde_json::to_value(results).unwrap())
}

#[tauri::command]
async fn store_versions(project_id: String) -> Result<serde_json::Value, String> {
    let list = get_modrinth_versions(&project_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(list).unwrap())
}

#[tauri::command]
async fn store_install(
    instance_id: String,
    version_id: String,
    dest_kind: String,
) -> Result<String, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    install_modrinth_version_to_instance(&c, inst, &version_id, &dest_kind)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_mrpack_cmd(instance_id: String, path: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    let report = import_mrpack(&c, &PathBuf::from(path), inst)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(report).unwrap())
}

#[tauri::command]
fn export_mrpack_cmd(instance_id: String, path: String) -> Result<(), String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    export_mrpack_stub(&c, inst, &PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn bind_sync_platform(instance_id: String, platform: String) -> Result<(), String> {
    let c = cfg()?;
    let mut store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get_mut(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    inst.sync_platform = Some(platform);
    store.save(&c).map_err(|e| e.to_string())
}

#[tauri::command]
async fn sync_check(instance_id: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    let platform = inst
        .sync_platform
        .as_deref()
        .ok_or_else(|| "未绑定服务器平台".to_string())?;
    let (base, channel) = parse_platform_input(platform);
    let manifest = fetch_manifest(&base, &channel)
        .await
        .map_err(|e| e.to_string())?;
    let diff = plan_sync(&c, inst, &manifest).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "manifest": manifest,
        "diff": diff,
        "localRevision": local_sync_revision(&c, inst)
    }))
}

#[tauri::command]
async fn sync_apply(instance_id: String) -> Result<serde_json::Value, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    let platform = inst
        .sync_platform
        .as_deref()
        .ok_or_else(|| "未绑定服务器平台".to_string())?;
    let (base, channel) = parse_platform_input(platform);
    let manifest = fetch_manifest(&base, &channel)
        .await
        .map_err(|e| e.to_string())?;
    let diff = plan_sync(&c, inst, &manifest).map_err(|e| e.to_string())?;
    let report = apply_sync(&c, inst, &manifest, &diff)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(report).unwrap())
}

#[tauri::command]
fn make_invite(base: String, channel: String) -> String {
    invite_code(&base, &channel)
}

#[tauri::command]
fn crash_summary(instance_id: String) -> Result<Option<String>, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    Ok(recent_crash_summary(&c, inst))
}

#[tauri::command]
fn latest_log(instance_id: String) -> Result<Option<String>, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    Ok(read_latest_log(&c, inst))
}

#[tauri::command]
fn read_servers_dat(instance_id: String) -> Result<Vec<ServerEntry>, String> {
    let c = cfg()?;
    let store = InstanceStore::load(&c).map_err(|e| e.to_string())?;
    let inst = store
        .get(&instance_id)
        .ok_or_else(|| "实例不存在".to_string())?;
    let path = inst.game_dir(&c.data_dir).join("servers.dat");
    // Minimal: also check launcher-side favorites
    let fav = c.data_dir.join("servers.json");
    if fav.exists() {
        let list: Vec<ServerEntry> =
            serde_json::from_str(&std::fs::read_to_string(fav).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        return Ok(list);
    }
    if path.exists() {
        return Ok(vec![ServerEntry {
            name: "servers.dat".into(),
            address: path.to_string_lossy().into(),
        }]);
    }
    Ok(vec![])
}

#[derive(Serialize, serde::Deserialize, Clone)]
struct ServerEntry {
    name: String,
    address: String,
}

#[tauri::command]
fn save_servers(servers: Vec<ServerEntry>) -> Result<(), String> {
    let c = cfg()?;
    std::fs::write(
        c.data_dir.join("servers.json"),
        serde_json::to_string_pretty(&servers).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn skin_preview_url(uuid: String) -> String {
    format!("https://mc-heads.net/body/{uuid}/128")
}

#[tauri::command]
fn set_download_source(source: String) -> Result<(), String> {
    let mut c = cfg()?;
    c.download_source = match source.as_str() {
        "official" => DownloadSource::Official,
        _ => DownloadSource::Bmclapi,
    };
    c.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn android_runtime_status() -> String {
    let req = AndroidLaunchRequest {
        instance_id: "probe".into(),
        game_dir: String::new(),
        version_id: String::new(),
        username: String::new(),
        uuid: String::new(),
        access_token: String::new(),
        assets_dir: String::new(),
        libraries_dir: String::new(),
    };
    default_backend().launch(req).message
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let queue = DownloadQueue::new();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { queue })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            list_accounts,
            ms_start_device_code,
            ms_poll_device_code,
            add_offline,
            add_authlib,
            remove_account,
            refresh_account,
            fetch_versions,
            install_version,
            download_tasks,
            list_instances,
            create_instance,
            delete_instance,
            scanned_versions,
            get_mods,
            toggle_mod,
            get_shaderpacks,
            get_resourcepacks,
            list_java,
            launch_instance,
            install_loader,
            store_search,
            store_versions,
            store_install,
            import_mrpack_cmd,
            export_mrpack_cmd,
            bind_sync_platform,
            sync_check,
            sync_apply,
            make_invite,
            crash_summary,
            latest_log,
            read_servers_dat,
            save_servers,
            skin_preview_url,
            set_download_source,
            android_runtime_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FreshwaterLauncher");
}
