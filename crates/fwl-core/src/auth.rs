use crate::accounts::{Account, AccountKind, AccountsStore};
use crate::config::FwlConfig;
use crate::error::{FwlError, Result};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration as StdDuration;

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeStart {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MsTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XblResponse {
    Token: String,
    DisplayClaims: XblDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XblDisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct McLoginResponse {
    access_token: String,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct McProfile {
    id: String,
    name: String,
}

pub async fn start_device_code(cfg: &FwlConfig) -> Result<DeviceCodeStart> {
    let client = reqwest::Client::new();
    let resp = client
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", cfg.ms_client_id.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<DeviceCodeResponse>()
        .await?;

    Ok(DeviceCodeStart {
        user_code: resp.user_code,
        device_code: resp.device_code,
        verification_uri: resp.verification_uri,
        expires_in: resp.expires_in,
        interval: resp.interval.max(1),
        message: resp.message.unwrap_or_else(|| {
            format!(
                "请打开 {} 并输入代码 {}",
                "https://www.microsoft.com/link",
                ""
            )
        }),
    })
}

pub async fn poll_device_code(cfg: &FwlConfig, device_code: &str, interval: u64) -> Result<Account> {
    let client = reqwest::Client::new();
    let started = std::time::Instant::now();
    let max_wait = StdDuration::from_secs(900);

    loop {
        if started.elapsed() > max_wait {
            return Err(FwlError::Auth("设备码登录超时".into()));
        }
        tokio::time::sleep(StdDuration::from_secs(interval)).await;

        let resp = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", cfg.ms_client_id.as_str()),
                ("device_code", device_code),
            ])
            .send()
            .await?
            .json::<MsTokenResponse>()
            .await?;

        if let Some(err) = resp.error.as_deref() {
            match err {
                "authorization_pending" => continue,
                "slow_down" => {
                    tokio::time::sleep(StdDuration::from_secs(interval + 2)).await;
                    continue;
                }
                "expired_token" => return Err(FwlError::Auth("设备码已过期，请重试".into())),
                "access_denied" => return Err(FwlError::Auth("用户拒绝授权".into())),
                other => {
                    return Err(FwlError::Auth(format!(
                        "{}: {}",
                        other,
                        resp.error_description.unwrap_or_default()
                    )));
                }
            }
        }

        let access = resp
            .access_token
            .ok_or_else(|| FwlError::Auth("未返回 access_token".into()))?;
        let refresh = resp.refresh_token.unwrap_or_default();
        return complete_microsoft_login(cfg, &access, &refresh, resp.expires_in.unwrap_or(3600))
            .await;
    }
}

pub async fn refresh_microsoft(cfg: &FwlConfig, account: &Account) -> Result<Account> {
    let refresh = account
        .refresh_token
        .as_deref()
        .ok_or_else(|| FwlError::Auth("缺少 refresh_token".into()))?;
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", cfg.ms_client_id.as_str()),
            ("refresh_token", refresh),
            ("scope", SCOPE),
        ])
        .send()
        .await?
        .json::<MsTokenResponse>()
        .await?;

    if let Some(err) = resp.error {
        return Err(FwlError::Auth(format!(
            "刷新失败: {} {}",
            err,
            resp.error_description.unwrap_or_default()
        )));
    }
    let access = resp
        .access_token
        .ok_or_else(|| FwlError::Auth("刷新未返回 access_token".into()))?;
    let new_refresh = resp.refresh_token.unwrap_or_else(|| refresh.to_string());
    complete_microsoft_login(cfg, &access, &new_refresh, resp.expires_in.unwrap_or(3600)).await
}

async fn complete_microsoft_login(
    cfg: &FwlConfig,
    ms_access: &str,
    refresh_token: &str,
    _ms_expires: u64,
) -> Result<Account> {
    let client = reqwest::Client::new();

    let xbl: XblResponse = client
        .post(XBL_AUTH_URL)
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms_access)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let uhs = xbl
        .DisplayClaims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or_else(|| FwlError::Auth("Xbox 未返回 uhs".into()))?;

    let xsts: XblResponse = client
        .post(XSTS_URL)
        .json(&json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl.Token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| {
            FwlError::Auth(format!(
                "XSTS 失败（账号可能未关联 Xbox 或地区限制）: {e}"
            ))
        })?
        .json()
        .await?;

    let identity = format!("XBL3.0 x={};{}", uhs, xsts.Token);
    let mc: McLoginResponse = client
        .post(MC_LOGIN_URL)
        .json(&json!({ "identityToken": identity }))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| {
            FwlError::Auth(format!(
                "Minecraft 登录失败（API 审核未通过时常为 403）: {e}"
            ))
        })?
        .json()
        .await?;

    let ent = client
        .get(MC_ENTITLEMENTS_URL)
        .bearer_auth(&mc.access_token)
        .send()
        .await?;
    if !ent.status().is_success() {
        tracing::warn!("entitlements check returned {}", ent.status());
    }

    let profile: McProfile = client
        .get(MC_PROFILE_URL)
        .bearer_auth(&mc.access_token)
        .send()
        .await?
        .error_for_status()
        .map_err(|_| {
            FwlError::Auth(
                "未获取到 Minecraft 档案（可能未购买正版或 Game Pass 未初始化）".into(),
            )
        })?
        .json()
        .await?;

    let expires_at = Utc::now() + Duration::seconds(mc.expires_in.unwrap_or(86400) as i64 - 60);

    let mut account = Account {
        id: profile.id.clone(),
        kind: AccountKind::Microsoft,
        username: profile.name.clone(),
        uuid: normalize_uuid(&profile.id),
        access_token: mc.access_token,
        refresh_token: Some(refresh_token.to_string()),
        expires_at: Some(expires_at),
        authlib_server: None,
    };

    let mut store = AccountsStore::load(cfg)?;
    store.upsert(account.clone());
    store.save(cfg)?;

    // reopen to return stored copy
    account = store
        .accounts
        .into_iter()
        .find(|a| a.id == profile.id)
        .unwrap_or(account);
    Ok(account)
}

fn normalize_uuid(id: &str) -> String {
    if id.contains('-') {
        id.to_string()
    } else if id.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &id[0..8],
            &id[8..12],
            &id[12..16],
            &id[16..20],
            &id[20..32]
        )
    } else {
        id.to_string()
    }
}

pub fn add_offline_account(cfg: &FwlConfig, username: &str) -> Result<Account> {
    let name = username.trim();
    if name.is_empty() {
        return Err(FwlError::Auth("用户名不能为空".into()));
    }
    let uuid = offline_uuid(name);
    let account = Account {
        id: uuid.clone(),
        kind: AccountKind::Offline,
        username: name.to_string(),
        uuid,
        access_token: "0".into(),
        refresh_token: None,
        expires_at: None,
        authlib_server: None,
    };
    let mut store = AccountsStore::load(cfg)?;
    store.upsert(account.clone());
    store.save(cfg)?;
    Ok(account)
}

pub fn add_authlib_account(
    cfg: &FwlConfig,
    username: &str,
    uuid: &str,
    access_token: &str,
    server: &str,
) -> Result<Account> {
    let account = Account {
        id: uuid.to_string(),
        kind: AccountKind::AuthlibInjector,
        username: username.to_string(),
        uuid: normalize_uuid(uuid),
        access_token: access_token.to_string(),
        refresh_token: None,
        expires_at: None,
        authlib_server: Some(server.to_string()),
    };
    let mut store = AccountsStore::load(cfg)?;
    store.upsert(account.clone());
    store.save(cfg)?;
    Ok(account)
}

fn offline_uuid(name: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let hash = hasher.finalize();
    let hex = hex::encode(&hash[..16]);
    normalize_uuid(&hex)
}
