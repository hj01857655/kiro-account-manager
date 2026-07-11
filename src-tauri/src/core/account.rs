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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountProxyProtocol {
    Http,
    Socks5,
}

impl Default for AccountProxyProtocol {
    fn default() -> Self {
        Self::Http
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: AccountProxyProtocol,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl AccountProxyConfig {
    pub fn to_proxy_url(&self) -> Result<String, String> {
        if !self.enabled {
            return Err("Account proxy is disabled".to_string());
        }

        let host = self.host.trim();
        if host.is_empty() {
            return Err("Proxy host is required".to_string());
        }
        if self.port == 0 {
            return Err("Proxy port is required".to_string());
        }

        let scheme = match self.protocol {
            AccountProxyProtocol::Http => "http",
            AccountProxyProtocol::Socks5 => "socks5h",
        };
        let mut url = url::Url::parse(&format!("{scheme}://{host}:{}", self.port))
            .map_err(|error| format!("Invalid proxy address: {error}"))?;

        let username = self
            .username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let password = self.password.as_deref().filter(|value| !value.is_empty());

        if let Some(username) = username {
            url.set_username(username)
                .map_err(|_| "Invalid proxy username".to_string())?;
            if let Some(password) = password {
                url.set_password(Some(password))
                    .map_err(|_| "Invalid proxy password".to_string())?;
            }
        } else if password.is_some() {
            return Err("Proxy password requires a username".to_string());
        }

        Ok(url.to_string())
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
    // external_idp（微软 / Azure AD）专用：供将来独立刷新链路使用
    #[serde(default, alias = "token_endpoint")]
    pub token_endpoint: Option<String>,
    #[serde(default, alias = "issuer_url")]
    pub issuer_url: Option<String>,
    // scopes 为单词，camelCase == snake_case，天然两吃，无需 alias
    #[serde(default)]
    pub scopes: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_config: Option<AccountProxyConfig>,
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
            token_endpoint: None,
            issuer_url: None,
            scopes: None,
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
            proxy_config: None,
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
            token_endpoint: None,
            issuer_url: None,
            scopes: None,
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
            proxy_config: None,
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
    let left_user_id = left
        .user_id
        .as_ref()
        .map(|value| value.trim())
        .unwrap_or("");
    let right_user_id = right
        .user_id
        .as_ref()
        .map(|value| value.trim())
        .unwrap_or("");
    !left_user_id.is_empty() && !right_user_id.is_empty() && left_user_id == right_user_id
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
    fill_option!(token_endpoint);
    fill_option!(issuer_url);
    fill_option!(scopes);
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

    let mut machine_id_counts = std::collections::HashMap::<String, usize>::new();
    for account in &normalized {
        if let Some(machine_id) = account.machine_id.as_deref() {
            let normalized_id = machine_id.trim().to_lowercase();
            if !normalized_id.is_empty() {
                *machine_id_counts.entry(normalized_id).or_default() += 1;
            }
        }
    }

    for account in &mut normalized {
        let is_duplicate = account
            .machine_id
            .as_deref()
            .map(|machine_id| {
                let normalized_id = machine_id.trim().to_lowercase();
                machine_id_counts
                    .get(&normalized_id)
                    .copied()
                    .unwrap_or_default()
                    > 1
            })
            .unwrap_or(false);

        if is_duplicate {
            account.machine_id = Some(Uuid::new_v4().to_string().to_lowercase());
            changed = true;
        }
    }

    (normalized, changed)
}

fn is_unavailable_status(status: &str) -> bool {
    matches!(
        status,
        "banned" | "封禁" | "已封禁" | "invalid" | "失效" | "已失效" | "Token已失效"
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

    fn backup_path_for(path: &PathBuf) -> PathBuf {
        path.with_extension("json.bak")
    }

    fn backup_candidates_for(path: &PathBuf) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        let latest_backup = Self::backup_path_for(path);
        if latest_backup.exists() {
            candidates.push(latest_backup);
        }

        if let Some(parent) = path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                let mut timestamped: Vec<PathBuf> = entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|entry_path| {
                        entry_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|name| {
                                name.starts_with("accounts.backup-") && name.ends_with(".json")
                            })
                            .unwrap_or(false)
                    })
                    .collect();
                timestamped.sort_by_key(|entry_path| {
                    std::fs::metadata(entry_path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                });
                timestamped.reverse();
                candidates.extend(timestamped);
            }
        }

        candidates
    }

    fn parse_accounts_json(path: &PathBuf, content: &str) -> Result<Vec<Account>, String> {
        serde_json::from_str::<Vec<Account>>(content)
            .map_err(|e| format!("账号文件解析失败: {}; 错误: {e}", path.display()))
    }

    fn load_backup_or_panic(path: &PathBuf, original_error: String) -> Vec<Account> {
        let mut backup_errors = Vec::new();
        for backup_path in Self::backup_candidates_for(path) {
            match std::fs::read_to_string(&backup_path)
                .map_err(|e| format!("读取备份失败: {}; 错误: {e}", backup_path.display()))
                .and_then(|content| Self::parse_accounts_json(&backup_path, &content))
            {
                Ok(accounts) => {
                    eprintln!(
                        "[AccountStore] 主账号文件损坏，已从备份加载 {} 个账号: {}",
                        accounts.len(),
                        backup_path.display()
                    );
                    if let Err(error) = std::fs::copy(&backup_path, path) {
                        eprintln!("[AccountStore] 从备份修复主账号文件失败: {error}");
                    }
                    return accounts;
                }
                Err(error) => backup_errors.push(error),
            }
        }

        panic!(
            "账号文件已损坏且没有可用备份，已阻止继续加载以避免覆盖清空: {original_error}; 备份错误: {}",
            backup_errors.join("; ")
        );
    }

    fn load_from_file(path: &PathBuf) -> Vec<Account> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                let original_error = format!("无法读取账号文件: {}; 错误: {error}", path.display());
                if !Self::backup_candidates_for(path).is_empty() {
                    eprintln!("[AccountStore] {original_error}，尝试从备份恢复");
                    return Self::load_backup_or_panic(path, original_error);
                }
                eprintln!("[AccountStore] 无法读取文件: {}", path.display());
                return Vec::new();
            }
        };

        match Self::parse_accounts_json(path, &content) {
            Ok(accounts) => {
                eprintln!("[AccountStore] 成功加载 {} 个账号", accounts.len());
                accounts
            }
            Err(error) => {
                eprintln!("[AccountStore] {error}");
                Self::load_backup_or_panic(path, error)
            }
        }
    }

    pub fn save_to_file(&self) -> bool {
        self.try_save_to_file().is_ok()
    }

    fn backup_path(&self) -> PathBuf {
        Self::backup_path_for(&self.file_path)
    }

    fn validate_existing_file_before_save(&self) -> Result<(), String> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| format!("读取现有账号文件失败: {e}"))?;
        serde_json::from_str::<Vec<Account>>(&content).map_err(|e| {
            format!(
                "现有账号文件已损坏，已拒绝覆盖保存以避免数据丢失: {}; 错误: {e}",
                self.file_path.display()
            )
        })?;

        std::fs::copy(&self.file_path, self.backup_path())
            .map_err(|e| format!("备份账号文件失败: {e}"))?;
        Ok(())
    }

    pub fn try_save_to_file(&self) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[AccountStore] 创建目录失败: {e}");
                return Err(format!("创建账号目录失败: {e}"));
            }
        }

        let json = serde_json::to_string_pretty(&self.accounts).map_err(|e| {
            eprintln!("[AccountStore] 序列化失败: {e}");
            format!("序列化账号数据失败: {e}")
        })?;

        self.validate_existing_file_before_save()?;

        let temp_path = self.file_path.with_extension("json.tmp");
        std::fs::write(&temp_path, json).map_err(|e| {
            eprintln!("[AccountStore] 写入临时账号文件失败: {e}");
            format!("写入临时账号文件失败: {e}")
        })?;

        let backup_path = self.backup_path();
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)
                .map_err(|e| format!("替换账号文件前删除旧文件失败: {e}"))?;
        }
        std::fs::rename(&temp_path, &self.file_path).map_err(|e| {
            if !self.file_path.exists() && backup_path.exists() {
                if let Err(restore_error) = std::fs::copy(&backup_path, &self.file_path) {
                    eprintln!("[AccountStore] 替换失败后恢复账号文件失败: {restore_error}");
                }
            }
            let _ = std::fs::remove_file(&temp_path);
            eprintln!("[AccountStore] 替换账号文件失败: {e}");
            format!("替换账号文件失败: {e}")
        })?;

        Ok(())
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
                    // 修复导入账号的 provider（如果为 null）
                    if account.provider.is_none() && account.auth_method.as_deref() == Some("IdC") {
                        // IdC 账号：根据 start_url 或 client_secret（JWT）判断是否 Enterprise
                        // BuilderId: https://view.awsapps.com/start
                        // Enterprise: d-xxx.awsapps.com
                        let is_enterprise = if let Some(ref start_url) = account.start_url {
                            // 有 start_url：检查是否非 BuilderId 的 awsapps.com 域名
                            !crate::commands::common::is_builder_id_start_url(start_url)
                                && start_url.contains("awsapps.com")
                        } else if let Some(ref client_secret) = account.client_secret {
                            // 无 start_url：从 client_secret（JWT）提取 initiateLoginUri 判断
                            use crate::utils::client_id_hash::extract_start_url_from_client_secret;
                            extract_start_url_from_client_secret(client_secret)
                                .map(|url| {
                                    !crate::commands::common::is_builder_id_start_url(&url)
                                        && url.contains("awsapps.com")
                                })
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        account.provider = Some(if is_enterprise {
                            "Enterprise".to_string()
                        } else {
                            "BuilderId".to_string()
                        });
                    } else if account.provider.is_none() && account.auth_method.as_deref() == Some("social") {
                        // Social 账号但 provider 为 null，根据邮箱判断
                        if let Some(ref email) = account.email {
                            if email.contains("gmail") {
                                account.provider = Some("Google".to_string());
                            } else if email.contains("github") {
                                account.provider = Some("Github".to_string());
                            } else {
                                account.provider = Some("Google".to_string());
                            }
                        } else {
                            account.provider = Some("Google".to_string());
                        }
                    }

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
    use super::{
        normalize_accounts, Account, AccountProxyConfig, AccountProxyProtocol, AccountStore,
    };
    use crate::core::usage::is_usage_capped;
    use std::path::PathBuf;

    fn unique_test_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kiro-account-store-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("accounts.json")
    }

    fn cleanup_test_path(path: PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn load_from_file_recovers_from_backup_when_primary_json_is_missing() {
        let path = unique_test_path("missing");
        let backup_path = path.with_extension("json.bak");
        let backup_account = Account::new("backup@example.com".to_string(), "backup".to_string());
        std::fs::write(
            &backup_path,
            serde_json::to_string_pretty(&vec![backup_account]).unwrap(),
        )
        .unwrap();

        let accounts = AccountStore::load_from_file(&path);

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].email.as_deref(), Some("backup@example.com"));
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert!(repaired.contains("backup@example.com"));

        cleanup_test_path(path);
    }
    #[test]
    fn load_from_file_recovers_from_backup_when_primary_json_is_corrupt() {
        let path = unique_test_path("corrupt");
        let backup_path = path.with_extension("json.bak");
        std::fs::write(&path, "[").unwrap();
        let backup_account = Account::new("backup@example.com".to_string(), "backup".to_string());
        std::fs::write(
            &backup_path,
            serde_json::to_string_pretty(&vec![backup_account]).unwrap(),
        )
        .unwrap();

        let accounts = AccountStore::load_from_file(&path);

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].email.as_deref(), Some("backup@example.com"));
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert!(repaired.contains("backup@example.com"));

        cleanup_test_path(path);
    }

    #[test]
    fn load_from_file_panics_on_corrupt_account_json_without_backup() {
        let path = unique_test_path("corrupt-no-backup");
        std::fs::write(&path, "[").unwrap();

        let result = std::panic::catch_unwind(|| {
            let _ = AccountStore::load_from_file(&path);
        });

        assert!(result.is_err());
        let message = result
            .err()
            .and_then(|panic| {
                panic.downcast_ref::<String>().cloned().or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                })
            })
            .unwrap_or_default();
        assert!(message.contains("没有可用备份"));

        cleanup_test_path(path);
    }

    #[test]
    fn try_save_to_file_preserves_previous_valid_file_as_latest_backup_only() {
        let path = unique_test_path("backup");
        let mut existing = Account::new("old@example.com".to_string(), "old".to_string());
        existing.user_id = Some("old-user".to_string());
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&vec![existing]).unwrap(),
        )
        .unwrap();

        let mut fresh = Account::new("new@example.com".to_string(), "new".to_string());
        fresh.user_id = Some("new-user".to_string());
        let store = AccountStore {
            accounts: vec![fresh],
            file_path: path.clone(),
        };

        store.try_save_to_file().unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("new@example.com"));
        let backup_path = path.with_extension("json.bak");
        let backup = std::fs::read_to_string(&backup_path).unwrap();
        assert!(backup.contains("old@example.com"));

        let history_backups: Vec<PathBuf> = AccountStore::backup_candidates_for(&path)
            .into_iter()
            .filter(|candidate| {
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("accounts.backup-") && name.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            history_backups.is_empty(),
            "regular account saves must not create timestamped backups"
        );

        cleanup_test_path(path);
    }

    #[test]
    fn account_proxy_config_uses_remote_dns_for_socks5() {
        let proxy = AccountProxyConfig {
            enabled: true,
            protocol: AccountProxyProtocol::Socks5,
            host: "127.0.0.1".to_string(),
            port: 1080,
            username: None,
            password: None,
        };

        assert_eq!(proxy.to_proxy_url().unwrap(), "socks5h://127.0.0.1:1080");
    }

    #[test]
    fn account_proxy_config_includes_auth_when_configured() {
        let proxy = AccountProxyConfig {
            enabled: true,
            protocol: AccountProxyProtocol::Http,
            host: "proxy.example.test".to_string(),
            port: 8080,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };

        assert_eq!(
            proxy.to_proxy_url().unwrap(),
            "http://user:pass@proxy.example.test:8080/"
        );
    }

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
        assert!(normalized.iter().all(|account| account
            .machine_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty())));
    }

    #[test]
    fn normalize_accounts_fills_missing_machine_ids() {
        let mut missing = Account::new("missing@example.com".to_string(), "missing".to_string());
        missing.user_id = Some("missing-user".to_string());
        missing.machine_id = None;

        let mut blank = Account::new("blank@example.com".to_string(), "blank".to_string());
        blank.user_id = Some("blank-user".to_string());
        blank.machine_id = Some("   ".to_string());

        let (normalized, changed) = normalize_accounts(vec![missing, blank]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.iter().all(|account| account
            .machine_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty())));
        assert_ne!(normalized[0].machine_id, normalized[1].machine_id);
    }

    #[test]
    fn normalize_accounts_rotates_duplicate_machine_ids() {
        let mut first = Account::new("first@example.com".to_string(), "first".to_string());
        first.user_id = Some("user-1".to_string());
        first.machine_id = Some("duplicate-machine".to_string());

        let mut second = Account::new("second@example.com".to_string(), "second".to_string());
        second.user_id = Some("user-2".to_string());
        second.machine_id = Some(" DUPLICATE-MACHINE ".to_string());

        let (normalized, changed) = normalize_accounts(vec![first, second]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert_ne!(normalized[0].machine_id, normalized[1].machine_id);
        assert_ne!(
            normalized[0].machine_id.as_deref(),
            Some("duplicate-machine")
        );
        assert_ne!(
            normalized[1].machine_id.as_deref().map(|id| id.trim()),
            Some("duplicate-machine")
        );
    }
}
