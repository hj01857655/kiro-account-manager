#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传参
use tauri::{AppHandle, State};

use crate::core::account::Account;
use crate::gateway::{
    clear_gateway_request_logs as clear_gateway_request_logs_inner,
    get_gateway_config as get_gateway_config_inner,
    get_gateway_client_stats as get_gateway_client_stats_inner,
    get_gateway_endpoint_stats as get_gateway_endpoint_stats_inner,
    get_gateway_log_dir as get_gateway_log_dir_inner,
    get_gateway_model_stats as get_gateway_model_stats_inner,
    get_gateway_quota as get_gateway_quota_inner,
    get_gateway_request_log_history as get_gateway_request_log_history_inner,
    get_gateway_request_logs as get_gateway_request_logs_inner,
    get_gateway_request_stats as get_gateway_request_stats_inner,
    get_gateway_status as get_gateway_status_inner, load_balancer, log_store,
    open_gateway_log_dir as open_gateway_log_dir_inner,
    read_gateway_sse_log_inner,
    save_gateway_config as save_gateway_config_inner, start_gateway as start_gateway_inner,
    stop_gateway as stop_gateway_inner, GatewayConfig, GatewayRequestLogEntry, GatewayRequestStats,
    GatewayStatus,
};
use crate::state::AppState;
use std::collections::HashMap;

fn config_for_manual_start(config: &GatewayConfig) -> GatewayConfig {
    config.clone()
}

#[cfg(test)]
fn config_after_manual_stop(config: &GatewayConfig) -> GatewayConfig {
    config.clone()
}

#[tauri::command]
pub async fn start_gateway(
    state: State<'_, AppState>,
    config: GatewayConfig,
) -> Result<GatewayStatus, String> {
    start_gateway_inner(&state, config_for_manual_start(&config)).await
}

#[tauri::command]
pub async fn stop_gateway(state: State<'_, AppState>) -> Result<(), String> {
    stop_gateway_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_status(state: State<'_, AppState>) -> Result<GatewayStatus, String> {
    get_gateway_status_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_config() -> Result<GatewayConfig, String> {
    get_gateway_config_inner()
}

#[tauri::command]
pub async fn save_gateway_config(config: GatewayConfig) -> Result<(), String> {
    save_gateway_config_inner(&config)
}

#[tauri::command]
pub async fn get_gateway_log_dir(app: AppHandle) -> Result<String, String> {
    get_gateway_log_dir_inner(&app)
}

#[tauri::command]
pub async fn get_gateway_request_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
    session_only: Option<bool>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    get_gateway_request_logs_inner(&state, limit, session_only.unwrap_or(false)).await
}

/// 读落盘的全量请求日志（跨会话历史，无条件走文件）。
/// Usage / 遮蔽审计页的 7 天趋势用它——内存 LogStore 重启即丢且上限 10000。
#[tauri::command]
pub async fn get_gateway_request_log_history(
    limit: Option<usize>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    get_gateway_request_log_history_inner(limit).await
}

#[tauri::command]
pub async fn get_gateway_quota(state: State<'_, AppState>) -> Result<Vec<crate::gateway::GatewayQuotaEntry>, String> {
    get_gateway_quota_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_request_stats(
    state: State<'_, AppState>,
) -> Result<GatewayRequestStats, String> {
    get_gateway_request_stats_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_model_stats(
    state: State<'_, AppState>,
) -> Result<Vec<log_store::ModelStat>, String> {
    get_gateway_model_stats_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_endpoint_stats(
    state: State<'_, AppState>,
) -> Result<Vec<log_store::EndpointStat>, String> {
    get_gateway_endpoint_stats_inner(&state).await
}

#[tauri::command]
pub async fn get_gateway_client_stats(
    state: State<'_, AppState>,
) -> Result<Vec<log_store::ClientStat>, String> {
    get_gateway_client_stats_inner(&state).await
}

/// 读取最近的遮蔽审计记录（只含元数据，不含真值）。
#[tauri::command]
pub async fn get_mask_audit_entries(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<crate::gateway::masking::MaskAuditEntry>, String> {
    let audit = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.as_ref().map(|rt| rt.mask_audit.clone())
    };
    match audit {
        Some(audit) => Ok(audit.recent(limit.unwrap_or(100))),
        // 网关未运行：没有审计记录可看（遮蔽只发生在请求路径上）
        None => Ok(Vec::new()),
    }
}

/// Try-it：对一段文本跑遮蔽，返回占位符版与命中条目（不落任何存储）。
///
/// 与请求路径的区别：不碰请求的映射表（避免试用污染真实请求的
/// 占位符一致性），用一个一次性引擎现场跑。
#[tauri::command]
pub async fn preview_masking(text: String) -> Result<crate::gateway::masking::MaskOutcome, String> {
    let mut engine = crate::gateway::masking::MaskEngine::new();
    Ok(engine.mask_text(&text))
}

#[tauri::command]
pub async fn open_gateway_log_dir(app: AppHandle) -> Result<String, String> {
    open_gateway_log_dir_inner(&app)
}

/// 读取某个请求的 SSE 逐事件明细（面板展开流式请求时用）。
#[tauri::command]
pub async fn read_gateway_sse_log(
    app: AppHandle,
    request_id: String,
    endpoint: Option<String>,
) -> Result<Vec<String>, String> {
    read_gateway_sse_log_inner(&app, &request_id, endpoint.as_deref())
}

#[tauri::command]
pub async fn clear_gateway_request_logs(state: State<'_, AppState>) -> Result<(), String> {
    clear_gateway_request_logs_inner(&state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_start_preserves_auto_start_preference() {
        let config = GatewayConfig {
            enabled: false,
            ..GatewayConfig::default()
        };

        let next = config_for_manual_start(&config);

        assert!(
            !next.enabled,
            "manual start should not force auto-start preference on"
        );
    }

    #[test]
    fn manual_stop_preserves_auto_start_preference() {
        let config = GatewayConfig {
            enabled: true,
            ..GatewayConfig::default()
        };

        let next = config_after_manual_stop(&config);

        assert!(
            next.enabled,
            "manual stop should not clear auto-start preference"
        );
    }
}

/// 获取所有被速率限制的账号
#[tauri::command]
pub async fn get_rate_limited_accounts(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    if let Some(lb) = balancer {
        Ok(lb.get_rate_limited_accounts().await)
    } else {
        Err("Gateway not initialized".to_string())
    }
}

/// 获取所有被封禁的账号（从数据库读取）
#[tauri::command]
pub async fn get_banned_accounts(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    // 与管理器界面共用同一份账号副本，避免各读各的快照导致判定不一致
    let store = state
        .store
        .lock()
        .map_err(|_| "获取账号状态失败".to_string())?;
    let banned_ids: Vec<String> = store
        .get_all()
        .into_iter()
        .filter(|acc| acc.status == "banned" || !acc.enabled)
        .map(|acc| acc.id)
        .collect();

    Ok(banned_ids)
}

/// 清除账号的速率限制标记
#[tauri::command]
pub async fn clear_rate_limit_account(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    if let Some(lb) = balancer {
        lb.clear_rate_limit(&account_id).await;
        Ok(())
    } else {
        Err("Gateway not initialized".to_string())
    }
}

/// 获取所有账号的健康状态
#[tauri::command]
pub async fn get_all_account_health(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    if let Some(lb) = balancer {
        let health_map = lb.get_all_health().await;
        // 转换为可序列化的格式
        let serializable: HashMap<String, load_balancer::SerializableAccountHealth> = health_map
            .iter()
            .map(|(k, v)| (k.clone(), v.into()))
            .collect();
        Ok(serde_json::to_value(serializable).unwrap_or(serde_json::Value::Null))
    } else {
        Err("Gateway not initialized".to_string())
    }
}

/// 重置账号健康状态
#[tauri::command]
pub async fn reset_account_health(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    if let Some(lb) = balancer {
        lb.reset_health(&account_id).await;
        Ok(())
    } else {
        Err("Gateway not initialized".to_string())
    }
}

/// 清理过期的健康状态数据
#[tauri::command]
pub async fn cleanup_stale_health(state: State<'_, AppState>) -> Result<(), String> {
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    if let Some(lb) = balancer {
        lb.cleanup_stale_health().await;
        Ok(())
    } else {
        Err("Gateway not initialized".to_string())
    }
}

/// 测试路由结果
#[derive(serde::Serialize)]
pub struct RouteTestResult {
    pub matched_accounts: Vec<String>,
    pub selected_account: Option<String>,
    pub error: Option<String>,
}

/// 测试路由配置
#[tauri::command]
pub async fn test_route_config(
    state: State<'_, AppState>,
    config: GatewayConfig,
) -> Result<RouteTestResult, String> {
    // 只读：路由分配测试只模拟选号，不碰账号状态；用共享副本保证与运行时同源。
    // guard 收在本块内，函数后续没有 await，但保持「读完即放」的写法一致。
    let matched_accounts = {
        let store = state
            .store
            .lock()
            .map_err(|_| "获取账号状态失败".to_string())?;

        // 根据配置筛选账号
        match config.account_mode.as_str() {
        "single" => store
            .accounts
            .iter()
            .filter(|account| config.account_id.as_deref() == Some(account.id.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
        "group" => store
            .accounts
            .iter()
            .filter(|account| {
                config.group_id.as_deref() == account.group_id.as_deref()
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        "pool" => store
            .accounts
            .iter()
            .filter(|account| {
                config.pool_account_ids.contains(&account.id)
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    }
    };

    if matched_accounts.is_empty() {
        return Ok(RouteTestResult {
            matched_accounts: Vec::new(),
            selected_account: None,
            error: Some("未找到符合 kiro2api 配置的可用账号".to_string()),
        });
    }

    // 使用负载均衡器选择账号
    let balancer = {
        let gateway = state.gateway.lock().unwrap();
        gateway.as_ref().map(|g| g.load_balancer.clone())
    };

    let selected_account: Option<Account> = if let Some(lb) = balancer {
        // 路由测试没有真实客户端 IP：不带粘性 key，按配置策略选号
        lb.select_account(&matched_accounts, None).await
    } else {
        // Gateway 未初始化时使用默认策略（轮询）
        matched_accounts.first().cloned()
    };

    Ok(RouteTestResult {
        matched_accounts: matched_accounts
            .iter()
            .map(|acc| format!("{} ({})", acc.label, acc.id))
            .collect(),
        selected_account: selected_account.map(|acc| format!("{} ({})", acc.label, acc.id)),
        error: None,
    })
}

/// 获取可用模型列表（用于前端下拉框）
#[tauri::command]
pub fn get_available_models() -> Vec<String> {
    crate::gateway::get_available_models()
}

