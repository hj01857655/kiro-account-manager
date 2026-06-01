use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelsCacheEntry {
    pub response: serde_json::Value,
    pub cached_at: i64,
    #[serde(default)]
    pub model_provider: Option<String>,
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
    #[serde(default)]
    pub available_models_cache: Option<AvailableModelsCacheEntry>,
    // 故障追踪（阶段一：失败计数和自动禁用）
    #[serde(default)]
    pub failure_count: u32,
    #[serde(default)]
    pub last_failure_at: Option<String>,
    #[serde(default)]
    pub disabled_reason: Option<String>,
    // 成功计数（用于 balanced 策略）
    #[serde(default)]
    pub success_count: u64,
    // 启用/禁用开关（禁用的账号网关会跳过）
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
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
            available_models_cache: None,
            password: None,
            failure_count: 0,
            last_failure_at: None,
            disabled_reason: None,
            success_count: 0,
            enabled: true,
        }
    }

    /// 创建 Enterprise 账号（没有 email，使用 `user_id`）
    pub fn new_enterprise(user_id: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email: None, // Enterprise 账号没有 email
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
            available_models_cache: None,
            password: None,
            failure_count: 0,
            last_failure_at: None,
            disabled_reason: None,
            success_count: 0,
            enabled: true,
        }
    }

    /// 判断是否是 Enterprise 账号
    pub fn is_enterprise(&self) -> bool {
        self.provider.as_deref() == Some("Enterprise")
    }

    /// 获取显示用的标识（Enterprise 用 `user_id`，其他用 email）
    pub fn get_display_id(&self) -> String {
        if self.is_enterprise() {
            self.user_id
                .clone()
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            self.email.clone().unwrap_or_else(|| "Unknown".to_string())
        }
    }

    /// 判断账号是否可用（可正常参与切换/同步）
    pub fn is_available(&self) -> bool {
        !is_unavailable_status(self.status.as_str())
            && !crate::core::usage::is_usage_capped(self.usage_data.as_ref())
            && self.disabled_reason.is_none()
    }
}

fn has_value(value: Option<&String>) -> bool {
    value.is_some_and(|item| !item.trim().is_empty())
}

fn infer_auth_method(account: &Account) -> Option<String> {
    if account
        .provider
        .as_deref()
        .is_some_and(|provider| provider == "BuilderId" || provider == "Enterprise")
        || (account.client_id.is_some() && account.client_secret.is_some())
    {
        return Some("IdC".to_string());
    }

    if account.profile_arn.is_some()
        || account
            .provider
            .as_deref()
            .is_some_and(|provider| provider == "Google" || provider == "Github")
    {
        return Some("social".to_string());
    }

    None
}

fn normalize_account(account: &mut Account) -> bool {
    let mut changed = false;

    if !has_value(account.auth_method.as_ref()) {
        if let Some(auth_method) = infer_auth_method(account) {
            account.auth_method = Some(auth_method);
            changed = true;
        }
    }

    changed
}

fn same_account_identity(left: &Account, right: &Account) -> bool {
    let left_user_id = left.user_id.as_ref().map(|value| value.trim()).unwrap_or("");
    let right_user_id = right.user_id.as_ref().map(|value| value.trim()).unwrap_or("");
    !left_user_id.is_empty()
        && !right_user_id.is_empty()
        && left_user_id == right_user_id
}

fn account_quality_score(account: &Account) -> usize {
    let important_fields = [
        account.email.as_ref(),
        account.user_id.as_ref(),
        account.auth_method.as_ref(),
        account.provider.as_ref(),
        account.access_token.as_ref(),
        account.refresh_token.as_ref(),
        account.expires_at.as_ref(),
        account.client_id.as_ref(),
        account.client_secret.as_ref(),
        account.client_id_hash.as_ref(),
        account.profile_arn.as_ref(),
        account.machine_id.as_ref(),
    ];

    important_fields
        .into_iter()
        .filter(|field| has_value(*field))
        .count()
        + usize::from(account.usage_data.is_some())
        + usize::from(account.available_models_cache.is_some())
}

fn merge_accounts(preferred: &mut Account, candidate: Account) -> bool {
    let mut changed = false;

    macro_rules! fill_option {
        ($field:ident) => {
            if preferred.$field.is_none() && candidate.$field.is_some() {
                preferred.$field = candidate.$field;
                changed = true;
            }
        };
    }

    fill_option!(email);
    fill_option!(password);
    fill_option!(access_token);
    fill_option!(refresh_token);
    fill_option!(expires_at);
    fill_option!(provider);
    fill_option!(user_id);
    fill_option!(auth_method);
    fill_option!(client_id);
    fill_option!(client_secret);
    fill_option!(region);
    fill_option!(client_id_hash);
    fill_option!(sso_session_id);
    fill_option!(id_token);
    fill_option!(start_url);
    fill_option!(profile_arn);
    fill_option!(usage_data);
    fill_option!(group_id);
    fill_option!(machine_id);
    fill_option!(available_models_cache);

    if preferred.tag_links.is_empty() && !candidate.tag_links.is_empty() {
        preferred.tag_links = candidate.tag_links;
        changed = true;
    }

    if preferred.label.trim().is_empty() && !candidate.label.trim().is_empty() {
        preferred.label = candidate.label;
        changed = true;
    }

    if preferred.status.trim().is_empty() && !candidate.status.trim().is_empty() {
        preferred.status = candidate.status;
        changed = true;
    }

    changed
}

fn normalize_accounts(accounts: Vec<Account>) -> (Vec<Account>, bool) {
    let mut changed = false;
    let mut normalized = Vec::with_capacity(accounts.len());

    for mut account in accounts {
        if normalize_account(&mut account) {
            changed = true;
        }

        if let Some(existing_index) = normalized
            .iter()
            .position(|existing| same_account_identity(existing, &account))
        {
            let existing = normalized.remove(existing_index);
            let candidate_score = account_quality_score(&account);
            let existing_score = account_quality_score(&existing);
            let (mut preferred, secondary) = if candidate_score > existing_score {
                changed = true;
                (account, existing)
            } else {
                changed = true;
                (existing, account)
            };

            if merge_accounts(&mut preferred, secondary) {
                changed = true;
            }

            normalized.insert(existing_index, preferred);
            continue;
        }

        normalized.push(account);
    }

    for account in &mut normalized {
        if !has_value(account.machine_id.as_ref()) {
            account.machine_id = Some(Uuid::new_v4().to_string().to_lowercase());
            changed = true;
        }
    }

    (normalized, changed)
}

fn is_unavailable_status(status: &str) -> bool {
    matches!(
        status,
        "banned"
            | "封禁"
            | "已封禁"
            | "invalid"
            | "失效"
            | "已失效"
            | "Token已失效"
    )
}

pub struct AccountStore {
    pub accounts: Vec<Account>,
    file_path: PathBuf,
}

impl AccountStore {
    pub fn new() -> Self {
        let file_path = Self::get_storage_path();
        let accounts = Self::load_from_file(&file_path);
        let mut store = Self {
            accounts,
            file_path,
        };

        if store.normalize_in_place() {
            if let Err(error) = store.try_save_to_file() {
                eprintln!("[AccountStore] 规范化账号文件回写失败: {error}");
            }
        }

        store
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
        self.try_save_to_file().is_ok()
    }

    pub fn try_save_to_file(&self) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[AccountStore] 创建目录失败: {e}");
                return Err(format!("创建账号目录失败: {e}"));
            }
        }

        match serde_json::to_string_pretty(&self.accounts) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file_path, json) {
                    eprintln!("[AccountStore] 写入文件失败: {e}");
                    return Err(format!("写入账号文件失败: {e}"));
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("[AccountStore] 序列化失败: {e}");
                Err(format!("序列化账号数据失败: {e}"))
            }
        }
    }

    pub fn get_all(&self) -> Vec<Account> {
        self.accounts.clone()
    }

    pub fn reload(&mut self) {
        self.accounts = Self::load_from_file(&self.file_path);
        self.normalize_in_place();
    }

    fn normalize_in_place(&mut self) -> bool {
        let current = std::mem::take(&mut self.accounts);
        let (normalized, changed) = normalize_accounts(current);
        self.accounts = normalized;
        changed
    }

    pub fn delete(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        let deleted = self.accounts.len() < len_before;
        if deleted {
            self.try_save_to_file()?;
        }
        Ok(deleted)
    }

    pub fn delete_many(&mut self, ids: &[String]) -> Result<usize, String> {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| !ids.contains(&a.id));
        let deleted = len_before - self.accounts.len();
        if deleted > 0 {
            self.try_save_to_file()?;
        }
        Ok(deleted)
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
                        if let (Some(a_uid), Some(acc_uid)) = (&a.user_id, &account.user_id) {
                            return a_uid == acc_uid;
                        }

                        false
                    });

                    if !exists {
                        // 如果没有 machine_id，生成一个
                        if account.machine_id.is_none() {
                            account.machine_id =
                                Some(uuid::Uuid::new_v4().to_string().to_lowercase());
                        }
                        self.accounts.push(account);
                        added += 1;
                    }
                }
                self.normalize_in_place();
                self.try_save_to_file()?;
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
        self.accounts
            .iter()
            .filter(|a| a.group_id.as_deref() == Some(group_id))
            .collect()
    }

    /// 按标签筛选账号
    pub fn get_accounts_by_tag(&self, tag_id: &str) -> Vec<&Account> {
        self.accounts
            .iter()
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
        data_dir
            .join(".kiro-account-manager")
            .join("groups-tags.json")
    }

    fn load_from_file(path: &PathBuf) -> GroupTagData {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            GroupTagData::default()
        }
    }

    pub fn try_save_to_file(&self) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[GroupTagStore] 创建目录失败: {e}");
                return Err(format!("创建分组标签目录失败: {e}"));
            }
        }
        match serde_json::to_string_pretty(&self.data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file_path, json) {
                    eprintln!("[GroupTagStore] 写入文件失败: {e}");
                    return Err(format!("写入分组标签文件失败: {e}"));
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("[GroupTagStore] 序列化失败: {e}");
                Err(format!("序列化分组标签数据失败: {e}"))
            }
        }
    }

    // 分组操作
    pub fn get_groups(&self) -> Vec<AccountGroup> {
        self.data.groups.clone()
    }

    pub fn add_group(
        &mut self,
        name: String,
        color: Option<String>,
    ) -> Result<AccountGroup, String> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // 分组数量不会超过 i32 范围
        let order = self.data.groups.len() as i32;
        let mut group = AccountGroup::new(name, color);
        group.order = order;
        self.data.groups.push(group.clone());
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(group)
    }

    pub fn update_group(
        &mut self,
        id: &str,
        name: Option<String>,
        color: Option<String>,
    ) -> Result<AccountGroup, String> {
        let group = self
            .data
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or("分组不存在")?;
        if let Some(n) = name {
            group.name = n;
        }
        if let Some(c) = color {
            group.color = Some(c);
        }
        let result = group.clone();
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(result)
    }

    pub fn delete_group(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.data.groups.len();
        self.data.groups.retain(|g| g.id != id);
        let deleted = self.data.groups.len() < len_before;
        if deleted {
            self.try_save_to_file()
                .map_err(|_| "保存分组失败".to_string())?;
        }
        Ok(deleted)
    }

    pub fn reorder_groups(&mut self, ids: &[String]) -> Result<bool, String> {
        for (order, id) in ids.iter().enumerate() {
            if let Some(group) = self.data.groups.iter_mut().find(|g| &g.id == id) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                // 分组数量不会超过 i32 范围
                {
                    group.order = order as i32;
                }
            }
        }
        self.data.groups.sort_by_key(|g| g.order);
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(true)
    }

    // 标签操作
    pub fn get_tags(&self) -> Vec<AccountTag> {
        self.data.tags.clone()
    }

    pub fn add_tag(&mut self, name: String, color: String) -> Result<AccountTag, String> {
        let tag = AccountTag::new(name, color);
        self.data.tags.push(tag.clone());
        self.try_save_to_file()
            .map_err(|_| "保存标签失败".to_string())?;
        Ok(tag)
    }

    pub fn update_tag(
        &mut self,
        id: &str,
        name: Option<String>,
        color: Option<String>,
    ) -> Result<AccountTag, String> {
        let tag = self
            .data
            .tags
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("标签不存在")?;
        if let Some(n) = name {
            tag.name = n;
        }
        if let Some(c) = color {
            tag.color = c;
        }
        let result = tag.clone();
        self.try_save_to_file()
            .map_err(|_| "保存标签失败".to_string())?;
        Ok(result)
    }

    pub fn delete_tag(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.data.tags.len();
        self.data.tags.retain(|t| t.id != id);
        let deleted = self.data.tags.len() < len_before;
        if deleted {
            self.try_save_to_file()
                .map_err(|_| "保存标签失败".to_string())?;
        }
        Ok(deleted)
    }
}


#[cfg(test)]
mod tests {
    use super::{normalize_accounts, Account};
    use crate::core::usage::is_usage_capped;

    #[test]
    fn account_is_not_available_when_monthly_usage_is_capped() {
        let mut account = Account::new("capped@example.com".to_string(), "capped".to_string());
        account.usage_data = Some(serde_json::json!({
            "overageConfiguration": {
                "overageStatus": "DISABLED"
            },
            "usageBreakdownList": [
                {
                    "currentUsage": 50,
                    "usageLimit": 50
                }
            ]
        }));

        assert!(is_usage_capped(account.usage_data.as_ref()));
        assert!(!account.is_available());
    }

    #[test]
    fn normalize_accounts_fills_missing_auth_method_from_provider() {
        let mut builder = Account::new("builder@example.com".to_string(), "builder".to_string());
        builder.provider = Some("BuilderId".to_string());
        builder.client_id = Some("client-id".to_string());
        builder.client_secret = Some("client-secret".to_string());
        builder.auth_method = None;

        let (normalized, changed) = normalize_accounts(vec![builder]);

        assert!(changed);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].auth_method.as_deref(), Some("IdC"));
    }

    #[test]
    fn normalize_accounts_merges_when_user_id_matches() {
        let mut legacy = Account::new("dup@example.com".to_string(), "legacy".to_string());
        legacy.provider = Some("BuilderId".to_string());
        legacy.user_id = Some("dup-user".to_string());
        legacy.auth_method = Some("IdC".to_string());

        let mut fresh = Account::new("other@example.com".to_string(), "fresh".to_string());
        fresh.provider = Some("Google".to_string());
        fresh.user_id = Some("dup-user".to_string());
        fresh.auth_method = Some("social".to_string());
        fresh.machine_id = Some("machine-1".to_string());

        let (normalized, changed) = normalize_accounts(vec![legacy, fresh]);

        assert!(changed);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].auth_method.as_deref(), Some("social"));
        assert_eq!(normalized[0].machine_id.as_deref(), Some("machine-1"));
        assert_eq!(normalized[0].email.as_deref(), Some("other@example.com"));
    }

    #[test]
    fn normalize_accounts_does_not_merge_when_user_id_differs_even_if_email_matches() {
        let mut social = Account::new("dup@example.com".to_string(), "social".to_string());
        social.provider = Some("Google".to_string());
        social.user_id = Some("user-1".to_string());
        social.auth_method = Some("social".to_string());

        let mut idc = Account::new("dup@example.com".to_string(), "idc".to_string());
        idc.provider = Some("Google".to_string());
        idc.user_id = Some("user-2".to_string());
        idc.auth_method = Some("IdC".to_string());

        let (normalized, changed) = normalize_accounts(vec![social, idc]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert!(normalized
            .iter()
            .all(|account| account.machine_id.as_ref().is_some_and(|id| !id.trim().is_empty())));
    }
}
