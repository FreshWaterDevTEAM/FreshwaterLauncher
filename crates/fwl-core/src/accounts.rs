use crate::config::FwlConfig;
use crate::error::Result;
use crate::paths::{accounts_file, ensure_dir};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Microsoft,
    Offline,
    AuthlibInjector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub authlib_server: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AccountsStore {
    pub accounts: Vec<Account>,
}

impl AccountsStore {
    pub fn load(cfg: &FwlConfig) -> Result<Self> {
        ensure_dir(&cfg.data_dir)?;
        let path = accounts_file(&cfg.data_dir);
        if path.exists() {
            Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, cfg: &FwlConfig) -> Result<()> {
        ensure_dir(&cfg.data_dir)?;
        std::fs::write(
            accounts_file(&cfg.data_dir),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    pub fn upsert(&mut self, account: Account) {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.id == account.id) {
            *existing = account;
        } else {
            self.accounts.push(account);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
    }

    pub fn get(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }
}
