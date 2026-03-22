#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数

use crate::account::Account;
use crate::kiro_cli_db::read_kiro_cli_accounts;
use crate::kiro_portal_client::KiroPortalClient;
use crate::state::AppState;
use tauri::State;
use serde::Serialize;

/// 展开路径中的 ~ 为用户主目录
fn expand_home_dir(path: &str) -> Result<String, String> {
    if path.starts_with('~') {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "无法获取用户主目录".to_string())?;
        Ok(path.replacen('~', &home, 1))
    } else {
        Ok(path.to_string())
    }
}

/// 从 CLI 账号判断 provider
fn determine_provider(cli_account: &crate::kiro_cli_db::KiroCliAccount) -> String {
    if cli_account.auth_method == "social" {
        // Social Login，通过 profile_arn 判断
        if let Some(ref arn) = cli_account.profile_arn {
            if arn.contains("google") {
                return "Google".to_string();
            } else if arn.contains("github") {
                return "GitHub".to_string();
            }
        }
        "Unknown".to_string()
    } else {
        // OIDC，默认 BuilderId
        "BuilderId".to_string()
    }
}

/// 检查账号是否已存在
fn find_existing_account(
    accounts: &[Account],
    user_id: Option<&String>,
    email: Option<&String>,
) -> Option<usize> {
    accounts.iter().position(|a| {
        // 使用 user_id 或 email 去重
        if let (Some(uid), Some(a_uid)) = (user_id, &a.user_id) {
            return uid == a_uid;
        }
        if let (Some(e), Some(a_e)) = (email, &a.email) {
            return e == a_e;
        }
        false
    })
}

/// 创建账号标签
fn create_account_label(
    is_new: bool,
    token_key: &str,
    existing_account: Option<&Account>,
) -> String {
    if is_new {
        format!("从 kiro-cli 导入 ({token_key})")
    } else {
        existing_account
            .map_or_else(|| format!("从 kiro-cli 导入 ({token_key})"), |a| a.label.clone())
    }
}

#[derive(Serialize)]
pub struct KiroCliImportResult {
    pub success: bool,
    pub is_new: bool,
    pub account: Option<Account>,
    pub error: Option<String>,
}

/// 获取 kiro-cli 默认数据库路径
#[tauri::command]
pub fn get_kiro_cli_default_path() -> Result<String, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "无法获取用户主目录".to_string())?;

    let mut candidates = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(std::path::PathBuf::from(&home)
            .join("Library")
            .join("Application Support")
            .join("kiro-cli")
            .join("data.sqlite3"));
    } else if cfg!(target_os = "windows") {
        candidates.push(std::path::PathBuf::from(&home)
            .join(".local")
            .join("share")
            .join("kiro-cli")
            .join("data.sqlite3"));
    } else {
        if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
            candidates.push(std::path::PathBuf::from(xdg_data_home)
                .join("kiro-cli")
                .join("data.sqlite3"));
        }
        candidates.push(std::path::PathBuf::from(&home)
            .join(".local")
            .join("share")
            .join("kiro-cli")
            .join("data.sqlite3"));
    }

    for path in candidates {
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    // 文件不存在，返回空字符串（前端会显示占位符）
    Ok(String::new())
}
/// 从 kiro-cli 数据库导入账号
#[tauri::command]
pub async fn import_from_kiro_cli(
    db_path: String,
    state: State<'_, AppState>,
) -> Result<KiroCliImportResult, String> {
    eprintln!("[Kiro CLI Import] 开始导入，数据库路径: {db_path}");

    // 展开 ~ 为用户主目录
    let expanded_path = expand_home_dir(&db_path)?;
    eprintln!("[Kiro CLI Import] 展开后的路径: {expanded_path}");

    // 1. 读取 kiro-cli 数据库
    let cli_accounts = read_kiro_cli_accounts(&expanded_path)?;
    
    if cli_accounts.is_empty() {
        return Err("数据库中没有账号数据".to_string());
    }
    
    if cli_accounts.len() > 1 {
        return Err("数据库中有多个账号，请联系开发者".to_string());
    }
    
    let cli_account = &cli_accounts[0];
    let auth_method = &cli_account.auth_method;
    let token_key = &cli_account.token_key;
    eprintln!("[Kiro CLI Import] 读取到账号: auth_method={auth_method}, token_key={token_key}");

    // 2. 调用 Kiro Portal API 获取配额
    let portal_client = KiroPortalClient::new()?;
    
    // 判断 idp（根据 auth_method）
    let idp = if cli_account.auth_method == "social" {
        "social"
    } else {
        "idc"
    };
    
    let usage_result = portal_client
        .get_user_usage_and_limits(&cli_account.access_token, idp)
        .await;

    let (email, user_id, provider, usage_data) = match usage_result {
        Ok(usage) => {
            let email = usage.get("email")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);
            
            let user_id = usage.get("userId")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);

            let provider = determine_provider(cli_account);
            (email, user_id, provider, Some(usage))
        }
        Err(e) => {
            eprintln!("[Kiro CLI Import] 获取配额失败: {e}");
            return Ok(KiroCliImportResult {
                success: false,
                is_new: false,
                account: None,
                error: Some(format!("获取账号信息失败: {e}")),
            });
        }
    };

    // 3. 检查账号是否已存在
    let mut store = state.store.lock().unwrap();
    let existing_index = find_existing_account(&store.accounts, user_id.as_ref(), email.as_ref());
    let is_new = existing_index.is_none();

    // 4. 创建或更新 Account
    let existing_account = existing_index.and_then(|idx| store.accounts.get(idx));
    let label = create_account_label(is_new, &cli_account.token_key, existing_account);
    
    let mut account = if let Some(e) = email.clone() {
        Account::new(e, label)
    } else if let Some(uid) = user_id.clone() {
        Account::new_enterprise(uid, label)
    } else {
        return Ok(KiroCliImportResult {
            success: false,
            is_new: false,
            account: None,
            error: Some("无法获取账号标识（email 或 userId）".to_string()),
        });
    };

    // 5. 填充字段
    account.access_token = Some(cli_account.access_token.clone());
    account.refresh_token = Some(cli_account.refresh_token.clone());
    account.expires_at.clone_from(&cli_account.expires_at);
    account.provider = Some(provider);
    account.user_id = user_id;
    account.region = Some(cli_account.region.clone());
    account.usage_data = usage_data;

    // 6. 根据认证类型填充字段
    if cli_account.auth_method == "social" {
        account.auth_method = Some("social".to_string());
        account.profile_arn.clone_from(&cli_account.profile_arn);
    } else {
        account.auth_method = Some("IdC".to_string());
        account.client_id.clone_from(&cli_account.client_id);
        account.client_secret.clone_from(&cli_account.client_secret);
    }

    // 7. 生成或保留 machine_id
    if let Some(idx) = existing_index {
        // 更新现有账号，保留 machine_id
        account.machine_id.clone_from(&store.accounts[idx].machine_id);
        account.id.clone_from(&store.accounts[idx].id);
        store.accounts[idx] = account.clone();
    } else {
        // 新账号，生成 machine_id
        if account.machine_id.is_none() {
            account.machine_id = Some(uuid::Uuid::new_v4().to_string().to_lowercase());
        }
        store.accounts.push(account.clone());
    }

    store.save_to_file();
    drop(store);

    let email = &account.email;
    let user_id = &account.user_id;
    eprintln!("[Kiro CLI Import] 导入成功: is_new={is_new}, email={email:?}, user_id={user_id:?}");

    Ok(KiroCliImportResult {
        success: true,
        is_new,
        account: Some(account),
        error: None,
    })
}
