use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;
use crate::portable;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: String,
    pub label: String,
    pub status: String,
    pub added_at: String,
    // 认证信息
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub csrf_token: Option<String>,
    pub session_token: Option<String>,
    pub expires_at: Option<String>,
    // 账号信息
    pub provider: Option<String>,
    pub user_id: Option<String>,
    // IdC 专用
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
    pub client_id_hash: Option<String>,
    pub sso_session_id: Option<String>,
    pub id_token: Option<String>,
    // Social 专用
    pub profile_arn: Option<String>,
    // 原始 usage API 响应
    pub usage_data: Option<serde_json::Value>,
}


impl Account {
    pub fn new(email: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            label,
            status: "正常".to_string(),
            added_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
            access_token: None,
            refresh_token: None,
            csrf_token: None,
            session_token: None,
            expires_at: None,
            provider: None,
            user_id: None,
            client_id: None,
            client_secret: None,
            region: None,
            client_id_hash: None,
            sso_session_id: None,
            id_token: None,
            profile_arn: None,
            usage_data: None,
        }
    }
}

pub struct AccountStore {
    pub accounts: Vec<Account>,
    file_path: PathBuf,
}

impl AccountStore {
    pub fn new() -> Self {
        let file_path = Self::get_storage_path();
        let accounts = Self::load_from_file(&file_path);
        Self { accounts, file_path }
    }

    fn get_storage_path() -> PathBuf {
        portable::get_app_data_dir().join("accounts.json")
    }

    fn load_from_file(path: &PathBuf) -> Vec<Account> {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn save_to_file(&self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.accounts) {
            let _ = std::fs::write(&self.file_path, json);
        }
    }

    pub fn get_all(&self) -> Vec<Account> {
        self.accounts.clone()
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        let deleted = self.accounts.len() < len_before;
        if deleted {
            self.save_to_file();
        }
        deleted
    }

    pub fn delete_many(&mut self, ids: &[String]) -> usize {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| !ids.contains(&a.id));
        let deleted = len_before - self.accounts.len();
        if deleted > 0 {
            self.save_to_file();
        }
        deleted
    }

    pub fn import_from_json(&mut self, json: &str) -> Result<usize, String> {
        match serde_json::from_str::<Vec<Account>>(json) {
            Ok(imported) => {
                let count = imported.len();
                for account in imported {
                    if !self.accounts.iter().any(|a| a.id == account.id) {
                        self.accounts.push(account);
                    }
                }
                self.save_to_file();
                Ok(count)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn export_to_json(&self) -> String {
        serde_json::to_string_pretty(&self.accounts).unwrap_or_default()
    }
}
