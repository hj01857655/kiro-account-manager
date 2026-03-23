#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传参

use tauri::{AppHandle, State};

use crate::gateway::{
    clear_gateway_request_logs as clear_gateway_request_logs_inner,
    GatewayConfig,
    GatewayRequestLogEntry,
    GatewayStatus,
    get_gateway_config as get_gateway_config_inner,
    get_gateway_log_dir as get_gateway_log_dir_inner,
    get_gateway_request_logs as get_gateway_request_logs_inner,
    get_gateway_status as get_gateway_status_inner,
    open_gateway_log_dir as open_gateway_log_dir_inner,
    save_gateway_config as save_gateway_config_inner,
    start_gateway as start_gateway_inner,
    stop_gateway as stop_gateway_inner,
};
use crate::state::AppState;

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
pub async fn get_gateway_request_logs(limit: Option<usize>) -> Result<Vec<GatewayRequestLogEntry>, String> {
    get_gateway_request_logs_inner(limit)
}

#[tauri::command]
pub async fn open_gateway_log_dir(app: AppHandle) -> Result<String, String> {
    open_gateway_log_dir_inner(&app)
}

#[tauri::command]
pub async fn clear_gateway_request_logs() -> Result<(), String> {
    clear_gateway_request_logs_inner()
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

        assert!(!next.enabled, "manual start should not force auto-start preference on");
    }

    #[test]
    fn manual_stop_preserves_auto_start_preference() {
        let config = GatewayConfig {
            enabled: true,
            ..GatewayConfig::default()
        };

        let next = config_after_manual_stop(&config);

        assert!(next.enabled, "manual stop should not clear auto-start preference");
    }
}
