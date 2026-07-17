// 自动换号后台任务模块
// 使用 tokio::time::interval 实现真正的后台定时检查

use crate::commands::app_settings_cmd::{get_app_settings_inner, AppSettings};
use crate::commands::common::{account_machine_id_or_new, save_store};
use crate::commands::machine_guid::set_custom_machine_guid;
use crate::core::account::Account;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Duration};

// 默认值
const DEFAULT_THRESHOLD: f64 = 1.0; // 余额阈值
const DEFAULT_INTERVAL: i32 = 5; // 检查间隔（分钟）
/// 关闭时轮询间隔：过长会导致用户打开开关后长时间不生效（旧实现 30 分钟）
const DISABLED_POLL_SECS: u64 = 15;

/// 启动自动换号后台任务
pub fn start_auto_switch_task(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        log::info!("[AutoSwitch] 后台任务已启动");

        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            // 读取配置
            let settings = match get_app_settings_inner() {
                Ok(s) => {
                    retry_count = 0; // 成功后重置重试计数
                    s
                }
                Err(e) => {
                    retry_count += 1;
                    log::error!(
                        "[AutoSwitch] 读取配置失败 ({}/{}): {}",
                        retry_count,
                        MAX_RETRIES,
                        e
                    );

                    if retry_count >= MAX_RETRIES {
                        log::error!(
                            "[AutoSwitch] 达到最大重试次数 ({}), 后台任务停止",
                            MAX_RETRIES
                        );
                        return;
                    }

                    tokio::time::sleep(Duration::from_secs(300)).await;
                    continue;
                }
            };

            // 检查是否启用自动换号
            if settings.auto_switch_enabled != Some(true) {
                log::debug!(
                    "[AutoSwitch] 自动换号已禁用，{} 秒后重新检查开关",
                    DISABLED_POLL_SECS
                );
                tokio::time::sleep(Duration::from_secs(DISABLED_POLL_SECS)).await;
                continue;
            }

            // 获取配置参数
            let threshold = settings.auto_switch_threshold.unwrap_or(DEFAULT_THRESHOLD);
            let interval_minutes = settings.auto_switch_interval.unwrap_or(DEFAULT_INTERVAL);

            // 获取自动刷新间隔
            let interval_duration = Duration::from_secs((interval_minutes as u64) * 60);

            log::info!(
                "[AutoSwitch] 自动换号已启用，间隔 {} 分钟，阈值 {}",
                interval_minutes,
                threshold
            );

            // 创建定时器
            let mut timer = interval(interval_duration);
            // 消耗第一次 tick
            timer.tick().await;

            // 立即检查一次
            check_and_auto_switch(&app_handle, threshold).await;

            // 定时检查
            loop {
                timer.tick().await;

                // 重新检查配置（用户可能修改了设置）
                let current_settings = match get_app_settings_inner() {
                    Ok(s) => s,
                    Err(_) => break, // 读取失败，退出内层循环，重新初始化
                };

                // 如果禁用了自动换号，退出内层循环
                if current_settings.auto_switch_enabled != Some(true) {
                    log::info!("[AutoSwitch] 自动换号已禁用");
                    break;
                }

                // 如果配置改变了，退出内层循环，重新初始化定时器
                let current_threshold = current_settings
                    .auto_switch_threshold
                    .unwrap_or(DEFAULT_THRESHOLD);
                let current_interval = current_settings
                    .auto_switch_interval
                    .unwrap_or(DEFAULT_INTERVAL);

                if current_threshold != threshold || current_interval != interval_minutes {
                    log::info!(
                        "[AutoSwitch] 配置已改变: 阈值 {} -> {}, 间隔 {} -> {} 分钟",
                        threshold,
                        current_threshold,
                        interval_minutes,
                        current_interval
                    );
                    break;
                }

                // 执行检查
                check_and_auto_switch(&app_handle, threshold).await;
            }
        }
    });
}

/// 检查并自动切换账号
async fn check_and_auto_switch(app_handle: &AppHandle, threshold: f64) {
    log::debug!("[AutoSwitch] 开始检查是否需要切换账号");

    // 获取 AppState
    let state = app_handle.state::<AppState>();

    // 获取所有账号（从本地存储读取，不调用 API）
    let accounts = {
        match state.store.lock() {
            Ok(mut s) => {
                s.reload();
                s.get_all()
            }
            Err(poisoned) => {
                log::warn!("[AutoSwitch] 锁被污染，尝试恢复");
                let mut s = poisoned.into_inner();
                s.reload();
                s.get_all()
            }
        }
    };

    if accounts.is_empty() {
        log::debug!("[AutoSwitch] 没有账号");
        return;
    }

    // 获取当前使用的账号（从本地 Kiro 凭证；token 轮换后可按 email 回退匹配）
    let current_account = match get_current_account(app_handle, &accounts).await {
        Some(acc) => acc,
        None => {
            log::warn!(
                "[AutoSwitch] 未检测到当前账号：本地 kiro-auth-token 与账号列表 token 均不匹配，\
                 且无法通过 usage 反查 email。请先在本应用中「切换」一次目标账号，或刷新列表账号 token"
            );
            return;
        }
    };

    log::info!(
        "[AutoSwitch] 当前账号: {} (enabled={}, status={})",
        current_account.email.as_deref().unwrap_or("未知"),
        current_account.enabled,
        current_account.status
    );

    // 开着自动切号时必须实时拉当前号配额：仅靠本地缓存会「开了+有号也不切」
    // （IDE 刚用完额度，列表 usage 还是旧的 → remaining 仍大于阈值）
    let current_account =
        match refresh_account_usage_for_switch(app_handle, &current_account).await {
            Ok(acc) => acc,
            Err(e) => {
                log::warn!(
                    "[AutoSwitch] 刷新当前账号配额失败，回退本地缓存: {e}"
                );
                current_account
            }
        };

    let remaining = calculate_remaining(&current_account);
    log::info!(
        "[AutoSwitch] 当前账号剩余额度: {}, 阈值: {}",
        remaining,
        threshold
    );

    // 检查是否需要切换
    if remaining > threshold {
        log::debug!("[AutoSwitch] 剩余额度充足，无需切换");
        return;
    }

    log::info!(
        "[AutoSwitch] 剩余额度不足 ({} <= {})，查找可用账号",
        remaining,
        threshold
    );

    // 查找可用账号
    let available_account = find_available_account(&accounts, &current_account, threshold);

    let available_account = match available_account {
        Some(acc) => acc,
        None => {
            let enabled_count = accounts.iter().filter(|a| a.enabled).count();
            let above_threshold = accounts
                .iter()
                .filter(|a| {
                    a.enabled
                        && a.id != current_account.id
                        && !matches!(a.status.to_lowercase().as_str(), "banned" | "invalid")
                        && calculate_remaining(a) > threshold
                })
                .count();
            log::warn!(
                "[AutoSwitch] 没有可用账号可切换（列表共 {} 个，enabled={}，余额>阈值={}）。\
                 候选需同时满足：已启用、非 banned/invalid、剩余额度 > 阈值",
                accounts.len(),
                enabled_count,
                above_threshold
            );
            return;
        }
    };

    log::info!(
        "[AutoSwitch] 找到可用账号: {}，准备切换",
        available_account.email.as_deref().unwrap_or("未知")
    );

    // 执行切换
    if let Err(e) = switch_account(app_handle, &available_account).await {
        log::error!("[AutoSwitch] 切换账号失败: {}", e);
        return;
    }

    log::info!(
        "[AutoSwitch] 切换账号成功: {}",
        available_account.email.as_deref().unwrap_or("未知")
    );

    // 发送事件通知前端
    let _ = app_handle.emit("accounts-updated", ());
    let _ = app_handle.emit(
        "account-switched",
        serde_json::json!({
            "email": available_account.email
        }),
    );
}

/// 获取当前使用的账号
///
/// 匹配顺序：
/// 1. refreshToken 全等
/// 2. accessToken 全等 / 前缀
/// 3. IdC clientIdHash
/// 4. 用本地 accessToken 调 getUsageLimits 取 email，再按 email 匹配列表，
///    并回写本地 token 到 store（修复 IDE/本机刷新后 RT 漂移导致永远匹配不上）
async fn get_current_account(app_handle: &AppHandle, accounts: &[Account]) -> Option<Account> {
    // 读取本地 Kiro Token
    let local_token = crate::kiro::ide::get_kiro_local_token().await?;

    // 优先用 refreshToken 匹配
    if let Some(refresh_token) = local_token.refresh_token.as_ref() {
        if let Some(acc) = accounts.iter().find(|acc| {
            acc.refresh_token
                .as_ref()
                .map(|rt| rt == refresh_token)
                .unwrap_or(false)
        }) {
            return Some(acc.clone());
        }
    }

    // 降级：accessToken 全等，再前缀（token refresh 后 RT 变了）
    if let Some(access_token) = local_token.access_token.as_ref() {
        if let Some(acc) = accounts.iter().find(|acc| {
            acc.access_token
                .as_ref()
                .map(|at| at == access_token)
                .unwrap_or(false)
        }) {
            return Some(acc.clone());
        }
        let prefix = &access_token[..access_token.len().min(20)];
        if let Some(acc) = accounts.iter().find(|acc| {
            acc.access_token
                .as_ref()
                .map(|at| at.starts_with(prefix))
                .unwrap_or(false)
        }) {
            return Some(acc.clone());
        }
    }

    // 再降级：用 clientIdHash 匹配（IdC 账号；Social 通常为空）
    if let Some(hash) = local_token.client_id_hash.as_ref() {
        if !hash.trim().is_empty() {
            if let Some(acc) = accounts.iter().find(|acc| {
                acc.client_id_hash
                    .as_ref()
                    .map(|h| h == hash)
                    .unwrap_or(false)
            }) {
                return Some(acc.clone());
            }
        }
    }

    // 最终回退：usage 反查 email（并同步 token，避免下次再失败）
    if let Some(account) =
        resolve_current_account_by_usage(app_handle, accounts, &local_token).await
    {
        return Some(account);
    }

    log::warn!(
        "[AutoSwitch] 无法匹配当前账号 (refreshToken/accessToken/clientIdHash/email 均不匹配)"
    );
    None
}

/// 用本地 accessToken 请求 getUsageLimits，按 userInfo.email 匹配列表账号，并回写 token
async fn resolve_current_account_by_usage(
    app_handle: &AppHandle,
    accounts: &[Account],
    local_token: &crate::kiro::ide::KiroLocalToken,
) -> Option<Account> {
    let access_token = local_token.access_token.as_deref().filter(|s| !s.is_empty())?;
    let region = local_token
        .region
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("us-east-1");

    // machineId：优先用任一已绑定机器码的账号，否则用占位（API 多数场景可接受）
    let machine_id = accounts
        .iter()
        .find_map(|a| {
            a.machine_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "auto-switch".to_string());

    let client = match crate::clients::kiro_client::KiroClient::new() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[AutoSwitch] 创建 HTTP 客户端失败: {e}");
            return None;
        }
    };

    // Enterprise IdC 本地 token 通常不带 profileArn，需 ListAvailableProfiles；
    // Social/BuilderId 用本地或默认 ARN。
    let mut profile_arn = local_token
        .profile_arn
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let needs_profile_discovery =
        local_token.auth_method.as_deref() == Some("IdC") && profile_arn.is_none();
    let regions =
        crate::clients::kiro_client::usage_limits_region_candidates(region, needs_profile_discovery);

    if needs_profile_discovery {
        match client
            .resolve_enterprise_profile_arn(access_token, &regions)
            .await
        {
            Ok(arn) => profile_arn = arn,
            Err(e) => {
                log::warn!("[AutoSwitch] ListAvailableProfiles 失败: {e}");
            }
        }
    }
    if profile_arn.is_none() {
        profile_arn = Some(
            match local_token.auth_method.as_deref() {
                Some("social") => crate::commands::common::KIRO_SOCIAL_PROFILE_ARN,
                _ => crate::commands::common::KIRO_BUILDER_ID_PROFILE_ARN,
            }
            .to_string(),
        );
    }

    let usage = match client
        .get_usage_limits_with_region_fallback(
            access_token,
            &machine_id,
            &regions,
            profile_arn.as_deref(),
        )
        .await
    {
        Ok((_region, data)) => data,
        Err(e) => {
            log::warn!("[AutoSwitch] 通过 usage 反查当前账号失败: {e}");
            return None;
        }
    };

    let (email, user_id) = crate::commands::common::extract_user_info(&usage);
    let matched = accounts.iter().find(|acc| {
        if let (Some(uid), Some(acc_uid)) = (user_id.as_ref(), acc.user_id.as_ref()) {
            if uid == acc_uid {
                return true;
            }
        }
        match (email.as_ref(), acc.email.as_ref()) {
            (Some(e1), Some(e2)) => e1.eq_ignore_ascii_case(e2),
            _ => false,
        }
    })?;

    log::info!(
        "[AutoSwitch] token 漂移后按 email/userId 匹配到当前账号: {}",
        matched.email.as_deref().unwrap_or("未知")
    );

    // 回写本地 token，避免下一轮仍匹配失败
    if let Err(e) = sync_local_tokens_to_account(
        app_handle,
        &matched.id,
        local_token.refresh_token.as_deref(),
        local_token.access_token.as_deref(),
        Some(&usage),
    ) {
        log::warn!("[AutoSwitch] 回写本地 token 到账号失败: {e}");
    }

    // 返回更新后的账号快照（含最新 usage，便于本轮 remaining 判断）
    let mut updated = matched.clone();
    if local_token.refresh_token.is_some() {
        updated.refresh_token = local_token.refresh_token.clone();
    }
    if local_token.access_token.is_some() {
        updated.access_token = local_token.access_token.clone();
    }
    updated.usage_data = Some(usage);
    Some(updated)
}

fn sync_local_tokens_to_account(
    app_handle: &AppHandle,
    account_id: &str,
    refresh_token: Option<&str>,
    access_token: Option<&str>,
    usage_data: Option<&serde_json::Value>,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut store = match state.store.lock() {
        Ok(store) => store,
        Err(poisoned) => poisoned.into_inner(),
    };
    store.reload();
    let Some(acc) = store.accounts.iter_mut().find(|a| a.id == account_id) else {
        return Err("账号不存在".to_string());
    };
    if let Some(rt) = refresh_token.filter(|s| !s.is_empty()) {
        acc.refresh_token = Some(rt.to_string());
    }
    if let Some(at) = access_token.filter(|s| !s.is_empty()) {
        acc.access_token = Some(at.to_string());
    }
    if let Some(usage) = usage_data {
        acc.usage_data = Some(usage.clone());
        let (email, user_id) = crate::commands::common::extract_user_info(usage);
        if acc.email.as_ref().map(|e| e.trim().is_empty()).unwrap_or(true) {
            if let Some(email) = email {
                acc.email = Some(email);
            }
        }
        if acc.user_id.is_none() {
            acc.user_id = user_id;
        }
    }
    save_store(&store)
}

/// 计算剩余额度（主配额 + 试用 + 奖励 + 已开启的超额，减去全部已用）
fn calculate_remaining(account: &Account) -> f64 {
    crate::core::usage::UsageDetails::from_usage_data(account.usage_data.as_ref())
        .map(|d| d.remaining())
        .unwrap_or(0.0)
}

/// 实时刷新账号 usage 并写回 store（自动切号决策用）
async fn refresh_account_usage_for_switch(
    app_handle: &AppHandle,
    account: &Account,
) -> Result<Account, String> {
    // 优先本地 IDE token（当前登录态），否则用列表里缓存的 accessToken
    let local = crate::kiro::ide::get_kiro_local_token().await;
    let access_token = local
        .as_ref()
        .and_then(|t| t.access_token.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| account.access_token.clone())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "当前账号无 accessToken，无法刷新配额".to_string())?;

    let usage_result =
        crate::commands::common::get_usage_by_account(account, &access_token).await?;

    if usage_result.is_auth_error {
        return Err("AUTH_ERROR: 刷新配额时 token 无效".to_string());
    }

    let usage_data = usage_result.usage_data;

    // 写回 store，保持列表与切号判断一致
    let state = app_handle.state::<AppState>();
    let mut store = match state.store.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };
    store.reload();
    let mut updated = account.clone();
    if let Some(acc) = store.accounts.iter_mut().find(|a| a.id == account.id) {
        acc.usage_data = Some(usage_data.clone());
        if usage_result.is_banned {
            acc.status = "banned".to_string();
        } else if crate::core::usage::is_usage_capped(Some(&usage_data)) {
            acc.status = "capped".to_string();
        }
        updated = acc.clone();
        save_store(&store)?;
    } else {
        updated.usage_data = Some(usage_data);
    }

    let remaining = calculate_remaining(&updated);
    log::info!(
        "[AutoSwitch] 已实时刷新配额: {} remaining={}",
        updated.email.as_deref().unwrap_or("未知"),
        remaining
    );
    Ok(updated)
}

/// 查找可用账号（选择剩余额度最多的）
fn find_available_account(
    accounts: &[Account],
    current_account: &Account,
    threshold: f64,
) -> Option<Account> {
    accounts
        .iter()
        .filter(|acc| {
            // 排除当前账号
            if acc.id == current_account.id {
                return false;
            }

            // 排除禁用的账号
            if !acc.enabled {
                return false;
            }

            // 排除不可用账号（banned / invalid 是真不可用）
            let status = acc.status.to_lowercase();
            if status == "banned" || status == "invalid" {
                return false;
            }

            // 排除余额不足的账号
            let remaining = calculate_remaining(acc);
            if remaining <= threshold {
                return false;
            }

            true
        })
        .max_by(|a, b| {
            calculate_remaining(a)
                .partial_cmp(&calculate_remaining(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// 切换账号
async fn switch_account(app_handle: &AppHandle, account: &Account) -> Result<(), String> {
    // 读取应用设置
    let settings = get_app_settings_inner().map_err(|e| e.to_string())?;

    // 应用机器码（如果需要）
    let account_to_switch = apply_machine_guid(app_handle, account, &settings).await?;

    // 构建切换参数
    let params = build_switch_params(&account_to_switch);

    // 执行切换
    crate::kiro::ide::switch_kiro_account(params)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MachineGuidSwitchAction {
    UseAccountMachineId(String),
}

fn resolve_machine_guid_switch_action(
    account: &Account,
    _settings: &AppSettings,
) -> MachineGuidSwitchAction {
    MachineGuidSwitchAction::UseAccountMachineId(account_machine_id_or_new(&account.machine_id))
}

fn persist_account_machine_id_if_needed(
    app_handle: &AppHandle,
    account_id: &str,
    machine_id: &str,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut store = match state.store.lock() {
        Ok(store) => store,
        Err(poisoned) => {
            log::warn!("[AutoSwitch] 锁被污染，尝试恢复");
            poisoned.into_inner()
        }
    };

    let Some(stored_account) = store
        .accounts
        .iter_mut()
        .find(|stored_account| stored_account.id == account_id)
    else {
        return Err("账号不存在".to_string());
    };

    let current_machine_id = stored_account
        .machine_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if current_machine_id == Some(machine_id) {
        return Ok(());
    }

    stored_account.machine_id = Some(machine_id.to_string());
    save_store(&store)
}

/// 应用机器码
async fn apply_machine_guid(
    app_handle: &AppHandle,
    account: &Account,
    settings: &AppSettings,
) -> Result<Account, String> {
    let mut account = account.clone();

    match resolve_machine_guid_switch_action(&account, settings) {
        MachineGuidSwitchAction::UseAccountMachineId(machine_id) => {
            log::debug!("[AutoSwitch] 使用账号绑定的机器码: {}", machine_id);
            if let Err(error) =
                persist_account_machine_id_if_needed(app_handle, &account.id, &machine_id)
            {
                log::warn!("[AutoSwitch] 保存账号机器码失败: {}", error);
            }
            if let Err(error) = set_custom_machine_guid(machine_id.clone()).await {
                log::warn!("[AutoSwitch] 写入系统机器码失败: {}", error);
            }
            account.machine_id = Some(machine_id);
        }
    }

    Ok(account)
}

/// 构建切换参数
fn build_switch_params(account: &Account) -> crate::kiro::ide::SwitchAccountParams {
    crate::kiro::ide::SwitchAccountParams {
        access_token: account.access_token.clone().unwrap_or_default(),
        refresh_token: account.refresh_token.clone().unwrap_or_default(),
        provider: account.provider.clone().unwrap_or_default(),
        auth_method: account.auth_method.clone(),
        profile_arn: account.profile_arn.clone(),
        start_url: account.start_url.clone(),
        client_id: account.client_id.clone(),
        client_secret: account.client_secret.clone(),
        client_id_hash: account.client_id_hash.clone(),
        region: account.region.clone(),
        email: account.email.clone(),
    }
}

/// 查找下一个可用账号（轮换逻辑：从当前账号之后开始找，找到第一个可用的）
/// 用于一键换号，不检查额度阈值，但跳过额度为0的账号
fn find_next_available_account(
    accounts: &[Account],
    current_account: &Account,
) -> Option<Account> {
    let current_index = accounts.iter().position(|acc| acc.id == current_account.id)?;
    
    // 从当前账号的下一个开始循环查找
    for i in 1..accounts.len() {
        let next_index = (current_index + i) % accounts.len();
        let acc = &accounts[next_index];
        
        // 检查是否可用
        if acc.id == current_account.id {
            continue; // 跳过当前账号
        }
        
        // 跳过禁用的账号
        if !acc.enabled {
            continue;
        }
        
        // 跳过真正不可用的状态（banned/invalid）
        let status = acc.status.to_lowercase();
        if status == "banned" || status == "invalid" {
            continue;
        }
        
        // 跳过额度为0的账号（已经完全用完）
        let remaining = calculate_remaining(acc);
        if remaining <= 0.0 {
            continue;
        }
        
        return Some(acc.clone());
    }
    
    None
}

/// 一键切换到下一个可用账号（前端按钮调用，不弹确认，轮换逻辑，不检查额度阈值）
#[tauri::command]
pub async fn quick_switch_next(app_handle: AppHandle) -> Result<String, String> {
    let state = app_handle.state::<AppState>();

    // 读取所有账号
    let accounts = {
        match state.store.lock() {
            Ok(mut s) => {
                s.reload();
                s.get_all()
            }
            Err(poisoned) => {
                let mut s = poisoned.into_inner();
                s.reload();
                s.get_all()
            }
        }
    };

    if accounts.is_empty() {
        return Err("没有可用账号".to_string());
    }

    // 获取当前账号
    let current_account = get_current_account(&app_handle, &accounts).await;

    // 查找下一个可用账号（使用轮换逻辑，跳过额度为0的）
    let next_account = if let Some(ref current) = current_account {
        find_next_available_account(&accounts, current)
    } else {
        // 没有当前账号，找第一个启用、状态正常、有余额的
        accounts
            .iter()
            .find(|acc| {
                acc.enabled
                    && !["banned", "invalid"].contains(&acc.status.to_lowercase().as_str())
                    && calculate_remaining(acc) > 0.0
            })
            .cloned()
    };

    let next_account = next_account.ok_or("没有可切换的可用账号")?;
    let email = next_account.email.clone().unwrap_or_else(|| "未知账号".to_string());

    // 执行切换
    switch_account(&app_handle, &next_account).await?;

    // 通知前端
    let _ = app_handle.emit("accounts-updated", ());
    let _ = app_handle.emit(
        "account-switched",
        serde_json::json!({ "email": &email }),
    );

    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::{resolve_machine_guid_switch_action, MachineGuidSwitchAction};
    use crate::commands::app_settings_cmd::AppSettings;
    use crate::core::account::Account;

    fn account_with_machine_id(machine_id: Option<&str>) -> Account {
        let mut account = Account::new("test@example.com".to_string(), "test".to_string());
        account.machine_id = machine_id.map(str::to_string);
        account
    }

    #[test]
    fn auto_switch_machine_guid_uses_existing_account_machine_id_by_default() {
        let account = account_with_machine_id(Some(" ACCOUNT-MACHINE "));
        let settings = AppSettings::default();

        assert_eq!(
            resolve_machine_guid_switch_action(&account, &settings),
            MachineGuidSwitchAction::UseAccountMachineId("account-machine".to_string())
        );
    }

    #[test]
    fn auto_switch_machine_guid_generates_account_machine_id_when_bound_id_is_missing() {
        let account = account_with_machine_id(Some("   "));
        let settings = AppSettings::default();

        let MachineGuidSwitchAction::UseAccountMachineId(machine_id) =
            resolve_machine_guid_switch_action(&account, &settings);
        assert!(!machine_id.trim().is_empty());
        assert_ne!(
            machine_id.trim(),
            account.machine_id.as_deref().unwrap().trim()
        );
    }
}
