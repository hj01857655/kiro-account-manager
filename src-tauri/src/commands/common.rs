// 公共工具函数 - 提取重复逻辑

use crate::core::account::Account;
use crate::auth::providers::{
    AuthProvider, IdcProvider, RefreshMetadata, SocialProvider,
};
use crate::utils::client_id_hash::calculate_client_id_hash;
use std::sync::{Mutex, MutexGuard};

// ===== Profile ARN 常量与 provider 映射 =====

/// BuilderId / Enterprise（IdC）账号的默认 profileArn
pub const KIRO_BUILDER_ID_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX";

/// Social（Github / Google）账号的默认 profileArn
pub const KIRO_SOCIAL_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK";

/// BuilderId 的固定 startUrl
pub const KIRO_BUILDER_ID_START_URL: &str = "https://view.awsapps.com/start";

/// BuilderId 的 clientIdHash（定值，无需每次现算）
///
/// 算法：`sha1(JSON.stringify({ startUrl }))`，即
/// `SHA1('{"startUrl":"https://view.awsapps.com/start"}')`。
/// startUrl 固定，hash 自然也固定，直接当常量兜底，省去重算、也避免
/// 规范化逻辑出错时把 BuilderId 也算偏。常量正确性由 `common.rs` 单测校验。
pub const KIRO_BUILDER_ID_CLIENT_ID_HASH: &str = "e909a0580879b06ece1202964fbe9dda95ea4ce3";

/// 判断给定 startUrl 是否就是 BuilderId 的默认 startUrl（去尾斜杠后比较）。
///
/// 用于拦截 Enterprise 账号回退到 BuilderId 默认值的脏数据：Enterprise 必须用
/// 自己的 `d-xxx` 域名，绝不能落到 `https://view.awsapps.com/start`。
pub fn is_builder_id_start_url(start_url: &str) -> bool {
    start_url.trim().trim_end_matches('/')
        == KIRO_BUILDER_ID_START_URL.trim_end_matches('/')
}

/// 判断给定 clientIdHash 是否就是 BuilderId 的默认 hash（忽略大小写）。
///
/// 同样用于拦截 Enterprise 误用 BuilderId 默认 hash（`e909a058...`）。
pub fn is_builder_id_client_id_hash(hash: &str) -> bool {
    hash.trim().eq_ignore_ascii_case(KIRO_BUILDER_ID_CLIENT_ID_HASH)
}

/// 校验 Enterprise 账号解析出的 clientIdHash 合法：非空、且不是 BuilderId 默认值。
///
/// Enterprise 回退到 BuilderId 的 hash（或其 startUrl 算出的 hash）正是 issue #119
/// 的根因——文件名错位导致 IDE 找不到 client registration。这里硬性拦下。
pub fn ensure_enterprise_client_id_hash(hash: &str) -> Result<(), String> {
    if hash.trim().is_empty() {
        return Err("Enterprise 账号的 clientIdHash 为空".to_string());
    }
    if is_builder_id_client_id_hash(hash) {
        return Err(
            "Enterprise 账号不能使用 BuilderId 默认 clientIdHash（e909a058...），\
             必须由企业自己的 d-xxx startUrl 算出。请检查账号的 start_url / client_id_hash。"
                .to_string(),
        );
    }
    Ok(())
}

/// 统一解析 IdC（BuilderId / Enterprise）账号的 clientIdHash —— 切号、添加、导入共用。
///
/// 这是 issue #119 的唯一裁决点：以前 switch / add / import 各抄一份「优先用已存
/// hash → BuilderId 用常量兜底 / Enterprise 由 startUrl 现算」的逻辑，容易漂移，
/// 还漏了校验。收敛到这里，规则只有一处：
///
/// 1. 账号已存 `client_id_hash` → 直接用（真实 IDE/CLI token 通常自带）；
/// 2. 否则按 provider 兜底：
///    - BuilderId：startUrl 固定，hash 是定值，直接用常量（也兼容传入的自定义 startUrl）；
///    - Enterprise：必须有自己的 `d-xxx` startUrl，现算；缺失则报错。
/// 3. Enterprise 最终硬校验：hash 绝不能为空、也绝不能等于 BuilderId 默认值
///    （`e909a058...`）。一旦命中即说明数据被污染，写出去文件名必然错位 →
///    IDE 找不到 client registration（issue #119 根因），直接拒绝而非写坏数据。
///
/// `provider` 取 `"BuilderId"` / `"Enterprise"`，其余值视为未知 provider 报错。
pub fn resolve_idc_client_id_hash(
    provider: &str,
    client_id_hash: Option<&str>,
    start_url: Option<&str>,
) -> Result<String, String> {
    // 1. 已存 hash 优先
    let hash = if let Some(hash) = client_id_hash
        .map(str::trim)
        .filter(|h| !h.is_empty())
    {
        hash.to_string()
    } else {
        // 2. 按 provider 兜底
        match provider {
            "BuilderId" => match start_url
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(url) => calculate_client_id_hash(url),
                None => KIRO_BUILDER_ID_CLIENT_ID_HASH.to_string(),
            },
            "Enterprise" => {
                let url = start_url
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or("Enterprise 账号必须提供 start_url 或 client_id_hash")?;
                calculate_client_id_hash(url)
            }
            other => return Err(format!("未知的 IdC Provider: {other}")),
        }
    };

    // 3. Enterprise 硬校验
    if provider == "Enterprise" {
        ensure_enterprise_client_id_hash(&hash)?;
    }
    Ok(hash)
}

/// 根据 provider 决定默认 profileArn（账号自身没设置时用）
///
/// 与参考项目 chaogei `resolveProfileArn` 行为一致：
/// - Github / Google（Social 登录）→ Social ARN
/// - 其他（包括 BuilderId、Enterprise、Internal、None）→ BuilderId ARN
pub fn resolve_default_profile_arn(provider: Option<&str>) -> &'static str {
    match provider {
        Some("Github") | Some("Google") => KIRO_SOCIAL_PROFILE_ARN,
        _ => KIRO_BUILDER_ID_PROFILE_ARN,
    }
}

// ===== Mutex 锁辅助 =====

/// 锁定 AppState 中任意 `Mutex<T>`，统一错误信息
pub fn lock_store<'a, T>(
    mutex: &'a Mutex<T>,
    ctx: &str,
) -> Result<MutexGuard<'a, T>, String> {
    mutex
        .lock()
        .map_err(|_| format!("Failed to acquire {ctx} lock"))
}

/// 按 id 从 store 查账号副本，找不到时返回友好错误
pub fn find_account_by_id(
    state: &tauri::State<'_, crate::state::AppState>,
    id: &str,
) -> Result<crate::core::account::Account, String> {
    let store = lock_store(&state.store, "store")?;
    store
        .accounts
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| format!("账号未找到 (id={id})"))
}

// ===== 调用 Kiro Q API 时的上下文解析 =====

/// 调用 Kiro Q API 需要的三件套：machine_id / region / profile_arn
///
/// 解析规则：
/// - machine_id：账号自带（非空）→ 否则系统 machine_guid
/// - profile_arn：Enterprise → None；其他账号自带 → 否则 provider 默认 ARN
/// - region：profile_arn 解析出来的 region 优先 → 账号 region → fallback
pub struct KiroCallContext {
    pub machine_id: String,
    pub region: String,
    pub profile_arn: Option<String>,
}

pub fn resolve_kiro_call_context(account: &Account, fallback_region: &str) -> KiroCallContext {
    use crate::clients::http_client::resolve_kiro_upstream_region;
    use crate::commands::machine_guid::get_machine_id;

    let machine_id = account
        .machine_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(get_machine_id);

    let profile_arn = match account.provider.as_deref() {
        Some("Enterprise") => None,
        provider => account
            .profile_arn
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| Some(resolve_default_profile_arn(provider).to_string())),
    };

    let region = resolve_kiro_upstream_region(
        profile_arn.as_deref(),
        account.region.as_deref(),
        fallback_region,
    );

    KiroCallContext { machine_id, region, profile_arn }
}

// ===== 时间常量（参考 Kiro IDE 源码）=====

/// Token 提前刷新时间（10分钟）
/// 在 token 过期前 10 分钟开始尝试刷新，避免真正过期
/// 参考 Kiro IDE: REFRESH_BEFORE_EXPIRY_SECONDS = 10 * 60
pub const AUTH_TOKEN_REFRESH_BEFORE_EXPIRY_SECONDS: i64 = 10 * 60;

/// Token 过期判断的容错时间（3分钟）
/// 判断 token 是否过期时，提前 3 分钟视为过期，防止时钟偏差
/// 参考 Kiro IDE: AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS = 3 * 60
pub const AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS: i64 = 3 * 60;

/// Client Registration 过期容错时间（15分钟）
/// IdC 账号的 clientSecret 过期检查，提前 15 分钟视为过期
/// 参考 Kiro IDE: CLIENT_REG_INVALIDATION_OFFSET_SECONDS = 15 * 60
#[allow(dead_code)] // 预留给 IdC 账号的 client registration 过期检查
pub const CLIENT_REG_INVALIDATION_OFFSET_SECONDS: i64 = 15 * 60;

/// 后台刷新检查间隔（60秒）
/// 参考 Kiro IDE: REFRESH_LOOP_INTERVAL_SECONDS = 60
pub const REFRESH_LOOP_INTERVAL_SECONDS: u64 = 60;

// ===== Token 过期检查函数 =====

/// 检查 token 是否即将过期（需要刷新）
/// 
/// 在 token 过期前 10 分钟返回 true，用于触发提前刷新
pub fn is_token_expiring_soon(expires_at: &str) -> bool {
    is_token_expired_within_seconds(expires_at, AUTH_TOKEN_REFRESH_BEFORE_EXPIRY_SECONDS)
}

/// 检查 token 是否已过期（带容错时间）
/// 
/// 在 token 过期前 3 分钟返回 true，用于判断 token 是否真正不可用
pub fn is_token_expired(expires_at: &str) -> bool {
    is_token_expired_within_seconds(expires_at, AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS)
}

/// 检查 token 是否需要刷新（即将过期或已过期）
///
/// 等价于 `is_token_expiring_soon(expires_at)`，因为 10 分钟阈值已经包含了 3 分钟的"已过期"判定。
/// 单独提供这个函数是为了让调用点的语义更清晰。
pub fn token_needs_refresh(expires_at: &str) -> bool {
    is_token_expiring_soon(expires_at)
}

/// 检查 token 是否在指定秒数内过期
fn is_token_expired_within_seconds(expires_at: &str, seconds: i64) -> bool {
    match chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
        Ok(expires) => {
            let now = chrono::Local::now().naive_local();
            let threshold = now + chrono::Duration::seconds(seconds);
            expires < threshold
        }
        Err(_) => true, // 解析失败视为已过期
    }
}

/// 检查 client registration 是否即将过期
#[allow(dead_code)] // 预留给 IdC 账号的 client registration 过期检查
pub fn is_client_registration_expiring(expires_at: &str) -> bool {
    match chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
        Ok(expires) => {
            let now = chrono::Local::now().naive_local();
            let threshold = now + chrono::Duration::seconds(CLIENT_REG_INVALIDATION_OFFSET_SECONDS);
            expires < threshold
        }
        Err(_) => true,
    }
}

pub async fn run_blocking_task<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| e.to_string())?
}

/// Token 刷新结果
#[derive(Debug)]
pub struct RefreshResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub profile_arn: Option<String>,
    pub id_token: Option<String>,
    pub sso_session_id: Option<String>,
}

/// 保存账号 store 到文件，统一错误信息
///
/// 始终走 `try_save_to_file` 这个新版 API（带详细错误），不要再用旧 `save_to_file`（只返回 bool）
pub fn save_store(store: &crate::core::account::AccountStore) -> Result<(), String> {
    store.try_save_to_file()
}

/// 把 token refresh 结果应用到 Account 上
///
/// 字段更新策略：
/// - access_token：总是覆盖（refresh 一定返回新值）
/// - refresh_token / profile_arn / id_token / sso_session_id：仅当返回了新值才覆盖
///   （避免 social refresh 没返回某字段时把已有的清掉）
/// - expires_at：根据 expires_in 重算
pub fn apply_refreshed_account_tokens(account: &mut Account, refresh: &RefreshResult) {
    account.access_token = Some(refresh.access_token.clone());
    if let Some(refresh_token) = refresh.refresh_token.clone() {
        account.refresh_token = Some(refresh_token);
    }
    if let Some(profile_arn) = refresh.profile_arn.clone() {
        account.profile_arn = Some(profile_arn);
    }
    if let Some(id_token) = refresh.id_token.clone() {
        account.id_token = Some(id_token);
    }
    if let Some(sso_session_id) = refresh.sso_session_id.clone() {
        account.sso_session_id = Some(sso_session_id);
    }
    account.expires_at = Some(calc_expires_at(refresh.expires_in));
}

/// Usage 获取结果
pub struct UsageResult {
    pub usage_data: serde_json::Value,
    pub is_banned: bool,
    pub is_auth_error: bool,
}

async fn refresh_token_by_provider_inner(
    account: &Account,
    use_account_proxy: bool,
) -> Result<RefreshResult, String> {
    let provider = account.provider.as_deref().unwrap_or("Google");
    let refresh_token = account.refresh_token.as_ref().ok_or("No refresh token")?;

    if provider == "BuilderId" || provider == "Enterprise" {
        let metadata = RefreshMetadata {
            client_id: account.client_id.clone(),
            client_secret: account.client_secret.clone(),
            region: account.region.clone(),
            account: use_account_proxy.then(|| account.clone()),
            ..Default::default()
        };
        let region = metadata.region.as_deref().unwrap_or("us-east-1");
        // Enterprise 使用保存的 start_url
        let start_url = if provider == "Enterprise" {
            account.start_url.clone()
        } else {
            None
        };
        let idc_provider = IdcProvider::new(provider, region, start_url);
        let auth = idc_provider.refresh_token(refresh_token, metadata).await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: None,
            id_token: auth.id_token,
            sso_session_id: auth.sso_session_id,
        })
    } else {
        let metadata = RefreshMetadata {
            profile_arn: account.profile_arn.clone(),
            machine_id: account.machine_id.clone(),
            account: use_account_proxy.then(|| account.clone()),
            ..Default::default()
        };
        let social_provider = SocialProvider::new(provider);
        let auth = social_provider
            .refresh_token(refresh_token, metadata)
            .await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: auth.profile_arn,
            id_token: None,
            sso_session_id: None,
        })
    }
}

/// 根据 provider 刷新 token（普通账号管理路径，沿用通用代理配置）
pub async fn refresh_token_by_provider(account: &Account) -> Result<RefreshResult, String> {
    refresh_token_by_provider_inner(account, false).await
}

/// 根据 provider 刷新 token（Reverse Proxy 路径，使用账号级代理）
pub async fn refresh_token_by_provider_with_account_proxy(
    account: &Account,
) -> Result<RefreshResult, String> {
    refresh_token_by_provider_inner(account, true).await
}

/// 统一使用 getUsageLimits 接口获取 usage 数据（支持所有账号类型）
pub async fn get_usage_by_account(
    account: &crate::core::account::Account,
    access_token: &str,
) -> Result<UsageResult, String> {
    get_usage_by_account_inner(account, access_token, false).await
}

pub async fn get_usage_by_account_with_account_proxy(
    account: &crate::core::account::Account,
    access_token: &str,
) -> Result<UsageResult, String> {
    get_usage_by_account_inner(account, access_token, true).await
}

async fn get_usage_by_account_inner(
    account: &crate::core::account::Account,
    access_token: &str,
    use_account_proxy: bool,
) -> Result<UsageResult, String> {
    use crate::clients::http_client::resolve_kiro_upstream_region;
    use crate::clients::kiro_q_client::KiroQClient;
    use crate::commands::machine_guid::get_machine_id;

    let machine_id = account
        .machine_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(get_machine_id);

    let region = resolve_kiro_upstream_region(
        account.profile_arn.as_deref(),
        account.region.as_deref(),
        "us-east-1",
    );

    // 为 BuilderId 和 social 账号设置默认 profile_arn（参考 Kiro IDE 源码）
    let provider = account.provider.as_deref().unwrap_or("Google");
    let profile_arn = match provider {
        "BuilderId" => {
            // BuilderId 优先使用账号自带的，否则使用默认值
            account
                .profile_arn
                .as_deref()
                .or(Some(KIRO_BUILDER_ID_PROFILE_ARN))
        }
        "Github" | "Google" => {
            // Social 账号优先使用账号自带的，否则使用默认值
            account
                .profile_arn
                .as_deref()
                .or(Some(KIRO_SOCIAL_PROFILE_ARN))
        }
        "Enterprise" => {
            // Enterprise 账号不使用 profile_arn
            None
        }
        _ => account.profile_arn.as_deref(),
    };

    let client = if use_account_proxy {
        KiroQClient::for_account(account)?
    } else {
        KiroQClient::new()?
    };
    let usage_call = client
        .get_usage_limits(
            access_token,
            &machine_id,
            &region,
            profile_arn,
            account.auth_method.as_deref(),
            account.provider.as_deref(),
        )
        .await;

    // 如果 getUsageLimits 成功，额外调用 ListAvailableModels 检测封禁
    // 因为某些封禁状态下 getUsageLimits 会正常返回，但 ListAvailableModels 会返回 403
    let mut result = parse_usage_result(usage_call)?;

    if !result.is_banned && profile_arn.is_some() {
        match client
            .list_available_models(
                access_token,
                &machine_id,
                &region,
                profile_arn,
                None,
                None,
            )
            .await
        {
            Err(e) if e.starts_with("BANNED:") => {
                log::warn!("[get_usage_by_account] ListAvailableModels 检测到封禁: {}", e);
                result.is_banned = true;
            }
            _ => {
                // 其他情况忽略（AUTH_ERROR 或成功都不影响 usage 结果）
            }
        }
    }

    Ok(result)
}

/// 根据 provider 获取 usage 数据（兼容旧接口，内部调用 getUsageLimits）
pub async fn get_usage_by_provider(
    provider: &str,
    access_token: &str,
) -> Result<UsageResult, String> {
    use crate::commands::machine_guid::get_machine_id;

    // 为了兼容旧调用，创建一个临时账号对象
    let mut temp_account = crate::core::account::Account::new(
        String::new(),
        String::new(),
    );
    temp_account.provider = Some(provider.to_string());
    temp_account.machine_id = Some(get_machine_id());

    // 根据 provider 设置 auth_method（profile_arn 由 get_usage_by_account 统一处理）
    if provider == "BuilderId" || provider == "Enterprise" {
        temp_account.auth_method = Some("IdC".to_string());
    } else {
        temp_account.auth_method = Some("social".to_string());
    }

    get_usage_by_account(&temp_account, access_token).await
}

pub async fn get_usage_by_provider_for_account(
    account: &crate::core::account::Account,
    provider: &str,
    access_token: &str,
) -> Result<UsageResult, String> {
    let mut temp_account = account.clone();
    temp_account.provider = Some(provider.to_string());
    if temp_account
        .machine_id
        .as_deref()
        .map_or(true, |value| value.trim().is_empty())
    {
        temp_account.machine_id = Some(crate::commands::machine_guid::get_machine_id());
    }

    if provider == "BuilderId" || provider == "Enterprise" {
        temp_account.auth_method = Some("IdC".to_string());
    } else {
        temp_account.auth_method = Some("social".to_string());
    }

    get_usage_by_account_with_account_proxy(&temp_account, access_token).await
}

/// 为企业账号获取 usage 数据（多区域探测）
/// 返回 (UsageResult, detected_region)
pub async fn get_enterprise_usage_with_region_probe_for_account(
    account: &crate::core::account::Account,
    access_token: &str,
    machine_id: &str,
) -> Result<(UsageResult, String), String> {
    use crate::clients::kiro_q_client::KiroQClient;

    let client = KiroQClient::for_account(account)?;
    let result = client
        .get_usage_limits_with_region_probe(access_token, machine_id)
        .await;

    match result {
        Ok((usage_data, region)) => Ok((
            UsageResult {
                usage_data,
                is_banned: false,
                is_auth_error: false,
            },
            region,
        )),
        Err(e) if e.starts_with("BANNED:") => Ok((
            UsageResult {
                usage_data: serde_json::Value::Null,
                is_banned: true,
                is_auth_error: false,
            },
            String::new(),
        )),
        Err(e) if is_auth_error_message(&e) => Ok((
            UsageResult {
                usage_data: serde_json::Value::Null,
                is_banned: false,
                is_auth_error: true,
            },
            String::new(),
        )),
        Err(e) => Err(e),
    }
}

pub async fn get_enterprise_usage_with_region_probe(
    access_token: &str,
    machine_id: &str,
) -> Result<(UsageResult, String), String> {
    let mut temp_account = crate::core::account::Account::new(String::new(), String::new());
    temp_account.provider = Some("Enterprise".to_string());
    temp_account.auth_method = Some("IdC".to_string());
    temp_account.machine_id = Some(machine_id.to_string());

    get_enterprise_usage_with_region_probe_for_account(&temp_account, access_token, machine_id)
        .await
}

/// 解析 usage 结果，提取封禁状态和认证错误
fn parse_usage_result(result: Result<serde_json::Value, String>) -> Result<UsageResult, String> {
    match result {
        Ok(usage_data) => Ok(UsageResult {
            usage_data, // 直接使用 JSON Value
            is_banned: false,
            is_auth_error: false,
        }),
        Err(e) if e.starts_with("BANNED:") => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: true,
            is_auth_error: false,
        }),
        // 401 或认证相关错误（包括 403 + token invalid）
        Err(e) if is_auth_error_message(&e) => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: false,
            is_auth_error: true,
        }),
        // 其他错误直接抛出
        Err(e) => Err(e),
    }
}

pub fn is_auth_error_message(error: &str) -> bool {
    let lower = error.to_lowercase();
    error.starts_with("AUTH_ERROR:")
        || error.contains("401")
        || error.contains("Unauthorized")
        || lower.contains("expired")
        || lower.contains("invalid")
}
pub fn calc_expires_at(expires_in: i64) -> String {
    let now = chrono::Local::now();
    let expires_at = now + chrono::Duration::seconds(expires_in);
    expires_at.format("%Y/%m/%d %H:%M:%S").to_string()
}

/// 根据 `usage_result` 计算账号状态
pub fn calc_status(is_banned: bool, is_auth_error: bool, usage_data: Option<&serde_json::Value>) -> String {
    if is_banned {
        "banned".to_string()
    } else if is_auth_error {
        "invalid".to_string()
    } else if crate::core::usage::is_usage_capped(usage_data) {
        "capped".to_string()
    } else if crate::core::usage::is_in_overage(usage_data) {
        "overage".to_string()
    } else {
        "active".to_string()
    }
}

/// 更新账号状态并自动设置 enabled 字段
///
/// 规则：
/// - banned（封禁）→ enabled = false
/// - invalid（失效）→ enabled = false
/// - capped（封顶）→ enabled = false
/// - overage（超额）→ enabled 保持不变（账号还能用超额配额）
/// - active（正常）→ enabled 保持不变
pub fn update_account_status(
    account: &mut crate::core::account::Account,
    is_banned: bool,
    is_auth_error: bool,
) {
    account.status = calc_status(is_banned, is_auth_error, account.usage_data.as_ref());

    // 只有封禁、失效、封顶状态才自动禁用账号
    if matches!(account.status.as_str(), "banned" | "invalid" | "capped") {
        account.enabled = false;
    }
}

fn read_non_empty_string_field(
    value: &serde_json::Value,
    primary_path: &[&str],
    fallback_key: &str,
) -> Option<String> {
    let nested = primary_path
        .iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(std::string::ToString::to_string);

    nested.or_else(|| {
        value
            .get(fallback_key)
            .and_then(|field| field.as_str())
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(std::string::ToString::to_string)
    })
}
/// 从 `usage_data` 中提取 `email` 和 `user_id`
/// 兼容 `userInfo.email/userInfo.userId` 与顶层 `email/userId`
pub fn extract_user_info(usage_data: &serde_json::Value) -> (Option<String>, Option<String>) {
    let email = read_non_empty_string_field(usage_data, &["userInfo", "email"], "email");
    let user_id = read_non_empty_string_field(usage_data, &["userInfo", "userId"], "userId");
    (email, user_id)
}

/// 查找已存在的账号索引
/// 优先用 `user_id` 去重，其次用 `refresh_token`（BuilderId 可能没有 userId）
pub fn find_existing_account_idx(
    accounts: &[Account],
    _email: Option<&String>,
    _provider: &str,
    refresh_token: &str,
    user_id: Option<&String>,
) -> Option<usize> {
    if let Some(uid) = user_id {
        if let Some(idx) = accounts.iter().position(|a| a.user_id.as_ref() == Some(uid)) {
            return Some(idx);
        }
    }
    // 如果没有 userId，用 refreshToken 去重
    accounts
        .iter()
        .position(|a| a.refresh_token.as_ref() == Some(&refresh_token.to_string()))
}


    


#[cfg(test)]
mod tests {
    use super::{extract_user_info, find_existing_account_idx, parse_usage_result};
    use super::{is_token_expiring_soon, is_token_expired, is_client_registration_expiring};
    use crate::core::account::Account;

    #[test]
    fn builder_id_client_id_hash_constant_matches_algorithm() {
        // 校验：常量 KIRO_BUILDER_ID_CLIENT_ID_HASH 必须等于
        // sha1(JSON.stringify({ startUrl: BUILDER_ID_START_URL }))。
        // 防止以后改 hash 算法/常量时两者漂移。
        use sha1::{Digest, Sha1};
        let input = serde_json::json!({
            "startUrl": super::KIRO_BUILDER_ID_START_URL,
        })
        .to_string();
        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let computed = hex::encode(hasher.finalize());
        assert_eq!(computed, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
    }

    #[test]
    fn is_builder_id_start_url_matches_default_variants() {
        use super::is_builder_id_start_url;
        // 默认值本身、带尾斜杠、带空白都应识别为 BuilderId 默认值
        assert!(is_builder_id_start_url("https://view.awsapps.com/start"));
        assert!(is_builder_id_start_url("https://view.awsapps.com/start/"));
        assert!(is_builder_id_start_url("  https://view.awsapps.com/start  "));
        // 企业 d-xxx 域名不应被误判
        assert!(!is_builder_id_start_url("https://d-90660ceab3.awsapps.com/start"));
    }

    #[test]
    fn is_builder_id_client_id_hash_matches_default_hash() {
        use super::is_builder_id_client_id_hash;
        // 默认 hash 本身、大写、带空白都应识别
        assert!(is_builder_id_client_id_hash(
            "e909a0580879b06ece1202964fbe9dda95ea4ce3"
        ));
        assert!(is_builder_id_client_id_hash(
            "E909A0580879B06ECE1202964FBE9DDA95EA4CE3"
        ));
        assert!(is_builder_id_client_id_hash(
            "  e909a0580879b06ece1202964fbe9dda95ea4ce3  "
        ));
        // 企业账号真实 hash（d-90660ceab3）不应被误判
        assert!(!is_builder_id_client_id_hash(
            "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35"
        ));
    }

    #[test]
    fn ensure_enterprise_client_id_hash_rejects_empty_and_builder_default() {
        use super::ensure_enterprise_client_id_hash;
        // 空 hash 拒绝
        assert!(ensure_enterprise_client_id_hash("").is_err());
        assert!(ensure_enterprise_client_id_hash("   ").is_err());
        // BuilderId 默认 hash 拒绝（issue #119 根因）
        assert!(
            ensure_enterprise_client_id_hash("e909a0580879b06ece1202964fbe9dda95ea4ce3")
                .is_err()
        );
        // 企业自己的 d-xxx hash 通过
        assert!(
            ensure_enterprise_client_id_hash("a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35")
                .is_ok()
        );
    }

    #[test]
    fn resolve_idc_client_id_hash_prefers_stored_hash() {
        use super::resolve_idc_client_id_hash;
        // 已存 hash 优先，provider / start_url 都不影响
        let resolved = resolve_idc_client_id_hash(
            "Enterprise",
            Some("a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35"),
            Some("https://d-90660ceab3.awsapps.com/start"),
        )
        .unwrap();
        assert_eq!(resolved, "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35");
    }

    #[test]
    fn resolve_idc_client_id_hash_builder_falls_back_to_constant() {
        use super::resolve_idc_client_id_hash;
        // BuilderId 无 hash、无 start_url → 直接用定值常量，不用现算
        let resolved = resolve_idc_client_id_hash("BuilderId", None, None).unwrap();
        assert_eq!(resolved, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
        // 空白也视为缺失
        let resolved = resolve_idc_client_id_hash("BuilderId", Some("  "), Some("")).unwrap();
        assert_eq!(resolved, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
    }

    #[test]
    fn resolve_idc_client_id_hash_enterprise_computes_from_start_url() {
        use super::resolve_idc_client_id_hash;
        // Enterprise 无 hash 但有 d-xxx startUrl → 现算，对得上真实 IDE 文件名
        let resolved = resolve_idc_client_id_hash(
            "Enterprise",
            None,
            Some("https://d-90660ceab3.awsapps.com/start"),
        )
        .unwrap();
        assert_eq!(resolved, "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35");
    }

    #[test]
    fn resolve_idc_client_id_hash_enterprise_rejects_missing_and_builder_default() {
        use super::resolve_idc_client_id_hash;
        // Enterprise 既无 hash 又无 startUrl → 报错（不能瞎兜底成 BuilderId）
        assert!(resolve_idc_client_id_hash("Enterprise", None, None).is_err());
        // Enterprise 的 start_url 被污染成 BuilderId 默认值 → 现算出 e909a058 → 硬校验拦下
        assert!(resolve_idc_client_id_hash(
            "Enterprise",
            None,
            Some("https://view.awsapps.com/start"),
        )
        .is_err());
        // Enterprise 直接存了 BuilderId 默认 hash → 同样拦下
        assert!(resolve_idc_client_id_hash(
            "Enterprise",
            Some("e909a0580879b06ece1202964fbe9dda95ea4ce3"),
            None,
        )
        .is_err());
    }

    #[test]
    fn resolve_idc_client_id_hash_rejects_unknown_provider() {
        use super::resolve_idc_client_id_hash;
        assert!(resolve_idc_client_id_hash("Google", None, None).is_err());
    }

    #[test]
    fn parse_usage_result_maps_banned_and_auth_errors_without_failing() {
        let banned = parse_usage_result(Err("BANNED: blocked".to_string())).unwrap();
        assert!(banned.is_banned);
        assert!(!banned.is_auth_error);
        assert_eq!(banned.usage_data, serde_json::Value::Null);

        let auth_error = parse_usage_result(Err("AUTH_ERROR: token expired".to_string())).unwrap();
        assert!(!auth_error.is_banned);
        assert!(auth_error.is_auth_error);
        assert_eq!(auth_error.usage_data, serde_json::Value::Null);
    }

    #[test]
    fn test_is_token_expiring_soon() {
        // 测试即将过期的 token（9分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(9))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_token_expiring_soon(&expires_at), "Token expiring in 9 minutes should return true");

        // 测试还有效的 token（11分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(11))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(!is_token_expiring_soon(&expires_at), "Token expiring in 11 minutes should return false");

        // 测试已过期的 token
        let expires_at = (chrono::Local::now() - chrono::Duration::minutes(5))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_token_expiring_soon(&expires_at), "Expired token should return true");

        // 测试无效的时间格式
        assert!(is_token_expiring_soon("invalid-date"), "Invalid date should return true");
    }

    #[test]
    fn test_is_token_expired() {
        // 测试已过期的 token（2分钟前）
        let expires_at = (chrono::Local::now() - chrono::Duration::minutes(2))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_token_expired(&expires_at), "Token expired 2 minutes ago should return true");

        // 测试即将过期的 token（2分钟后，在3分钟容错范围内）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(2))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_token_expired(&expires_at), "Token expiring in 2 minutes should return true (within 3min threshold)");

        // 测试还有效的 token（5分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(5))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(!is_token_expired(&expires_at), "Token expiring in 5 minutes should return false");

        // 测试无效的时间格式
        assert!(is_token_expired("invalid-date"), "Invalid date should return true");
    }

    #[test]
    fn test_is_client_registration_expiring() {
        // 测试即将过期的 client registration（10分钟后，在15分钟容错范围内）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(10))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_client_registration_expiring(&expires_at), "Client reg expiring in 10 minutes should return true");

        // 测试还有效的 client registration（20分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(20))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(!is_client_registration_expiring(&expires_at), "Client reg expiring in 20 minutes should return false");

        // 测试已过期的 client registration
        let expires_at = (chrono::Local::now() - chrono::Duration::days(1))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(is_client_registration_expiring(&expires_at), "Expired client reg should return true");
    }

    #[test]
    fn extract_user_info_ignores_empty_email_and_reads_user_id() {
        let usage = serde_json::json!({
            "userInfo": {
                "email": "",
                "userId": "user-123"
            }
        });

        assert_eq!(
            extract_user_info(&usage),
            (None, Some("user-123".to_string()))
        );
    }

    #[test]
    fn extract_user_info_falls_back_to_top_level_fields() {
        let usage = serde_json::json!({
            "email": "top@example.com",
            "userId": "top-user-123"
        });

        assert_eq!(
            extract_user_info(&usage),
            (
                Some("top@example.com".to_string()),
                Some("top-user-123".to_string())
            )
        );
    }

    #[test]
    fn extract_user_info_prefers_nested_fields_and_trims_values() {
        let usage = serde_json::json!({
            "email": "fallback@example.com",
            "userId": "fallback-user",
            "userInfo": {
                "email": " nested@example.com ",
                "userId": " nested-user "
            }
        });

        assert_eq!(
            extract_user_info(&usage),
            (
                Some("nested@example.com".to_string()),
                Some("nested-user".to_string())
            )
        );
    }

    #[test]
    fn find_existing_account_idx_uses_user_id_only() {
        let mut first = Account::new("first@example.com".to_string(), "first".to_string());
        first.user_id = Some("user-1".to_string());

        let second = Account::new("second@example.com".to_string(), "second".to_string());
        let accounts = vec![first, second];

        let user_id = "user-1".to_string();
        let second_email = "second@example.com".to_string();

        assert_eq!(
            find_existing_account_idx(&accounts, Some(&second_email), "Google", "", Some(&user_id)),
            Some(0)
        );
        assert_eq!(
            find_existing_account_idx(&accounts, None, "Google", "", None),
            None
        );
        assert_eq!(
            find_existing_account_idx(&accounts, Some(&second_email), "Google", "", None),
            None
        );
    }
}
