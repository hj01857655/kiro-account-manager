use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

// 自定义反序列化：处理 tag_links 的 null 值
fn deserialize_tag_links<'de, D>(deserializer: D) -> Result<Vec<AccountTagLink>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Vec<AccountTagLink>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

// ============================================================
// 分组与标签系统
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountGroup {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub order: i32,
    pub created_at: String,
}

impl AccountGroup {
    pub fn new(name: String, color: Option<String>) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            order: 0,
            created_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTag {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl AccountTag {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            created_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
        }
    }
}

// ============================================================
// 账号标签关联（带时间戳和标签名）
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTagLink {
    pub tag_id: String,
    #[serde(default)]
    pub tag_name: Option<String>,
    pub linked_at: String,
}

impl AccountTagLink {
    pub fn new(tag_id: String, tag_name: Option<String>) -> Self {
        Self {
            tag_id,
            tag_name,
            linked_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

// ============================================================
// 账号实体
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    /// email 字段（企业账号可能没有，用 `user_id` 代替）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    // 账号密码（可选）
    #[serde(default)]
    pub password: Option<String>,
    pub label: String,
    pub status: String,
    pub added_at: String,
    // 认证信息
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    // 账号信息
    pub provider: Option<String>,
    pub user_id: Option<String>,
    // 认证方式（IdC / social）
    #[serde(default)]
    pub auth_method: Option<String>,
    // IdC 专用
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
    pub client_id_hash: Option<String>,
    pub sso_session_id: Option<String>,
    pub id_token: Option<String>,
    #[serde(default)]
    pub start_url: Option<String>, // Enterprise 的 Start URL
    // Social 专用
    #[serde(default)]
    pub profile_arn: Option<String>,
    // 原始 usage API 响应
    pub usage_data: Option<serde_json::Value>,
    // 分组
    #[serde(default)]
    pub group_id: Option<String>,
    // 标签关联（带时间戳）
    #[serde(default, deserialize_with = "deserialize_tag_links")]
    pub tag_links: Vec<AccountTagLink>,
    // 绑定的机器码
    #[serde(default)]
    pub machine_id: Option<String>,
}

impl Account {
    /// 创建普通账号（Google/GitHub/BuilderId）
    pub fn new(email: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email: Some(email),
            label,
            status: "active".to_string(),
            added_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            provider: None,
            user_id: None,
            auth_method: None,
            client_id: None,
            client_secret: None,
            region: None,
            client_id_hash: None,
            sso_session_id: None,
            id_token: None,
            start_url: None,
            profile_arn: None,
            usage_data: None,
            group_id: None,
            tag_links: Vec::new(),
            machine_id: None,
            password: None,
        }
    }

    /// 创建 Enterprise 账号（没有 email，使用 `user_id`）
    pub fn new_enterprise(user_id: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email: None,  // Enterprise 账号没有 email
            label,
            status: "active".to_string(),
            added_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            provider: Some("Enterprise".to_string()),
            user_id: Some(user_id),
            auth_method: Some("IdC".to_string()),
            client_id: None,
            client_secret: None,
            region: None,
            client_id_hash: None,
            sso_session_id: None,
            id_token: None,
            start_url: None,
            profile_arn: None,
            usage_data: None,
            group_id: None,
            tag_links: Vec::new(),
            machine_id: None,
            password: None,
        }
    }

    /// 判断是否是 Enterprise 账号
    pub fn is_enterprise(&self) -> bool {
        self.provider.as_deref() == Some("Enterprise")
    }

    /// 获取显示用的标识（Enterprise 用 `user_id`，其他用 email）
    pub fn get_display_id(&self) -> String {
        if self.is_enterprise() {
            self.user_id.clone().unwrap_or_else(|| "Unknown".to_string())
        } else {
            self.email.clone().unwrap_or_else(|| "Unknown".to_string())
        }
    }

    /// 判断账号是否可用（未封禁）
    pub fn is_available(&self) -> bool {
        !matches!(self.status.as_str(), "banned" | "封禁" | "已封禁" | "已过期")
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
        let data_dir = dirs::data_dir().unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        });
        data_dir.join(".kiro-account-manager").join("accounts.json")
    }

    fn load_from_file(path: &PathBuf) -> Vec<Account> {
        if let Ok(content) = std::fs::read_to_string(path) {
            match serde_json::from_str::<Vec<Account>>(&content) {
                Ok(accounts) => {
                    eprintln!("[AccountStore] 成功加载 {} 个账号", accounts.len());
                    accounts
                }
                Err(e) => {
                    eprintln!("[AccountStore] JSON 反序列化失败: {e}");
                    eprintln!("[AccountStore] 文件路径: {}", path.display());
                    Vec::new()
                }
            }
        } else {
            eprintln!("[AccountStore] 无法读取文件: {}", path.display());
            Vec::new()
        }
    }

    pub fn save_to_file(&self) -> bool {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[AccountStore] 创建目录失败: {e}");
                return false;
            }
        }
        
        match serde_json::to_string_pretty(&self.accounts) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file_path, json) {
                    eprintln!("[AccountStore] 写入文件失败: {e}");
                    return false;
                }
                true
            }
            Err(e) => {
                eprintln!("[AccountStore] 序列化失败: {e}");
                false
            }
        }
    }

    pub fn get_all(&self) -> Vec<Account> {
        self.accounts.clone()
    }

    pub fn reload(&mut self) {
        self.accounts = Self::load_from_file(&self.file_path);
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        let deleted = self.accounts.len() < len_before;
        if deleted { let _ = self.save_to_file(); }
        deleted
    }

    pub fn delete_many(&mut self, ids: &[String]) -> usize {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| !ids.contains(&a.id));
        let deleted = len_before - self.accounts.len();
        if deleted > 0 { let _ = self.save_to_file(); }
        deleted
    }

    pub fn import_from_json(&mut self, json: &str) -> Result<usize, String> {
        match serde_json::from_str::<Vec<Account>>(json) {
            Ok(imported) => {
                let mut added = 0;
                for mut account in imported {
                    // 修复导入账号的 authMethod（如果为 null）
                    if account.auth_method.is_none() {
                        if account.client_id.is_some() && account.client_secret.is_some() {
                            account.auth_method = Some("IdC".to_string());
                        } else {
                            account.auth_method = Some("social".to_string());
                        }
                    }
                    
                    let exists = self.accounts.iter().any(|a| {
                        // 优先用 ID 去重
                        if a.id == account.id {
                            return true;
                        }
                        
                        // 使用 user_id 去重（最简单直接）
                        if let (Some(a_uid), Some(acc_uid)) = (&a.user_id, &account.user_id) {
                            return a_uid == acc_uid;
                        }
                        
                        // 如果没有 user_id，用 email 兜底
                        if let (Some(a_email), Some(acc_email)) = (&a.email, &account.email) {
                            return a_email == acc_email;
                        }
                        
                        false
                    });
                    
                    if !exists {
                        // 如果没有 machine_id，生成一个
                        if account.machine_id.is_none() {
                            account.machine_id = Some(uuid::Uuid::new_v4().to_string().to_lowercase());
                        }
                        self.accounts.push(account);
                        added += 1;
                    }
                }
                if !self.save_to_file() {
                    return Err("保存文件失败".to_string());
                }
                Ok(added)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[allow(dead_code)]
    pub fn export_to_json(&self) -> String {
        serde_json::to_string_pretty(&self.accounts).unwrap_or_default()
    }

    /// 获取可用账号列表（用于自动换号）
    pub fn get_available_accounts(&self) -> Vec<&Account> {
        self.accounts.iter().filter(|a| a.is_available()).collect()
    }

    /// 按分组筛选账号
    pub fn get_accounts_by_group(&self, group_id: &str) -> Vec<&Account> {
        self.accounts.iter()
            .filter(|a| a.group_id.as_deref() == Some(group_id))
            .collect()
    }

    /// 按标签筛选账号
    pub fn get_accounts_by_tag(&self, tag_id: &str) -> Vec<&Account> {
        self.accounts.iter()
            .filter(|a| a.tag_links.iter().any(|l| l.tag_id == tag_id))
            .collect()
    }
}

// ============================================================
// 分组与标签存储
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupTagData {
    pub groups: Vec<AccountGroup>,
    pub tags: Vec<AccountTag>,
}

pub struct GroupTagStore {
    data: GroupTagData,
    file_path: PathBuf,
}

impl GroupTagStore {
    pub fn new() -> Self {
        let file_path = Self::get_storage_path();
        let data = Self::load_from_file(&file_path);
        Self { data, file_path }
    }

    fn get_storage_path() -> PathBuf {
        let data_dir = dirs::data_dir().unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        });
        data_dir.join(".kiro-account-manager").join("groups-tags.json")
    }

    fn load_from_file(path: &PathBuf) -> GroupTagData {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            GroupTagData::default()
        }
    }

    pub fn save_to_file(&self) -> bool {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[GroupTagStore] 创建目录失败: {e}");
                return false;
            }
        }
        match serde_json::to_string_pretty(&self.data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file_path, json) {
                    eprintln!("[GroupTagStore] 写入文件失败: {e}");
                    return false;
                }
                true
            }
            Err(e) => {
                eprintln!("[GroupTagStore] 序列化失败: {e}");
                false
            }
        }
    }

    // 分组操作
    pub fn get_groups(&self) -> Vec<AccountGroup> {
        self.data.groups.clone()
    }

    pub fn add_group(&mut self, name: String, color: Option<String>) -> Result<AccountGroup, String> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 分组数量不会超过 i32 范围
        let order = self.data.groups.len() as i32;
        let mut group = AccountGroup::new(name, color);
        group.order = order;
        self.data.groups.push(group.clone());
        if !self.save_to_file() {
            return Err("保存分组失败".to_string());
        }
        Ok(group)
    }

    pub fn update_group(&mut self, id: &str, name: Option<String>, color: Option<String>) -> Result<AccountGroup, String> {
        let group = self.data.groups.iter_mut().find(|g| g.id == id)
            .ok_or("分组不存在")?;
        if let Some(n) = name { group.name = n; }
        if let Some(c) = color { group.color = Some(c); }
        let result = group.clone();
        if !self.save_to_file() {
            return Err("保存分组失败".to_string());
        }
        Ok(result)
    }

    pub fn delete_group(&mut self, id: &str) -> bool {
        let len_before = self.data.groups.len();
        self.data.groups.retain(|g| g.id != id);
        let deleted = self.data.groups.len() < len_before;
        if deleted { let _ = self.save_to_file(); }
        deleted
    }

    pub fn reorder_groups(&mut self, ids: &[String]) -> bool {
        for (order, id) in ids.iter().enumerate() {
            if let Some(group) = self.data.groups.iter_mut().find(|g| &g.id == id) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 分组数量不会超过 i32 范围
                {
                    group.order = order as i32;
                }
            }
        }
        self.data.groups.sort_by_key(|g| g.order);
        self.save_to_file()
    }

    // 标签操作
    pub fn get_tags(&self) -> Vec<AccountTag> {
        self.data.tags.clone()
    }

    pub fn add_tag(&mut self, name: String, color: String) -> Result<AccountTag, String> {
        let tag = AccountTag::new(name, color);
        self.data.tags.push(tag.clone());
        if !self.save_to_file() {
            return Err("保存标签失败".to_string());
        }
        Ok(tag)
    }

    pub fn update_tag(&mut self, id: &str, name: Option<String>, color: Option<String>) -> Result<AccountTag, String> {
        let tag = self.data.tags.iter_mut().find(|t| t.id == id)
            .ok_or("标签不存在")?;
        if let Some(n) = name { tag.name = n; }
        if let Some(c) = color { tag.color = c; }
        let result = tag.clone();
        if !self.save_to_file() {
            return Err("保存标签失败".to_string());
        }
        Ok(result)
    }

    pub fn delete_tag(&mut self, id: &str) -> bool {
        let len_before = self.data.tags.len();
        self.data.tags.retain(|t| t.id != id);
        let deleted = self.data.tags.len() < len_before;
        if deleted { let _ = self.save_to_file(); }
        deleted
    }
}
