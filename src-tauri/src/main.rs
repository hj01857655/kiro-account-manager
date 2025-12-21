#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod auth_social;
mod aws_sso_client;
mod browser;
mod codewhisperer_client;
mod commands;
mod deep_link_handler;

mod kiro;
mod kiro_auth_client;
mod mcp;
mod portable;
mod powers;
mod process;
mod providers;
mod state;
mod steering;
mod account;

use account::AccountStore;
use auth::AuthState;
use state::AppState;
use std::sync::Mutex;
use tauri::{Listener, Manager};

// 导入命令
use browser::detect_installed_browsers;
use commands::account_cmd::{
    get_accounts, delete_account, delete_accounts, update_account, sync_account,
    refresh_account_token, verify_account, add_account_by_social, add_local_kiro_account,
    add_account_by_idc, import_accounts, export_accounts
};
use commands::app_settings_cmd::*;
use commands::auth_cmd::*;
use commands::kiro_settings_cmd::*;
use commands::machine_guid_cmd::*;
use commands::mcp_cmd::*;
use commands::portable_cmd::*;
use commands::powers_cmd::*;
use commands::proxy_cmd::*;
use commands::sso_import_cmd::*;
use commands::update_cmd::*;
use commands::web_oauth_cmd::*;
use commands::steering_cmd::*;
use kiro::{
    get_kiro_local_token, get_kiro_telemetry_info, reset_kiro_machine_id, switch_kiro_account,
};
use process::{close_kiro_ide, is_kiro_ide_running, start_kiro_ide};

fn main() {
    // 初始化便携模式：确保数据目录存在，并尝试从普通模式导入配置
    if let Err(e) = portable::ensure_data_dir() {
        eprintln!("[Portable] 创建数据目录失败: {}", e);
    }
    if portable::is_portable() {
        println!("[Portable] 便携模式已启用，数据目录: {:?}", portable::get_data_dir());
        match portable::try_import_from_normal_mode() {
            Ok(true) => println!("[Portable] 已从普通模式导入配置"),
            Ok(false) => {}
            Err(e) => eprintln!("[Portable] 导入配置失败: {}", e),
        }
    }
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // 监听 deep link 事件 (使用 kiro:// 协议)
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register("kiro");
            }
            
            // 监听 deep link URL
            let app_handle = app.handle().clone();
            app.listen("deep-link://new-url", move |event| {
                let payload = event.payload();
                println!("[DeepLink] Received: {}", payload);
                // 处理 OAuth 回调
                deep_link_handler::handle_deep_link(payload);
                // 聚焦窗口
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.set_focus();
                }
            });
            
            Ok(())
        })
        .manage(AppState {
            store: Mutex::new(AccountStore::new()),
            auth: AuthState::new(),
            pending_login: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            // 账号命令
            get_accounts,
            delete_account,
            delete_accounts,
            update_account,
            sync_account,
            refresh_account_token,
            verify_account,
            add_account_by_social,
            add_local_kiro_account,
            add_account_by_idc,
            import_accounts,
            export_accounts,
            // Auth 命令
            get_current_user,
            logout,
            kiro_login,
            get_supported_providers,
            handle_kiro_social_callback,
            add_kiro_account,
            // Kiro IDE 命令
            get_kiro_local_token,
            switch_kiro_account,
            get_kiro_telemetry_info,
            reset_kiro_machine_id,
            // 进程管理命令
            close_kiro_ide,
            start_kiro_ide,
            is_kiro_ide_running,
            // Kiro IDE 设置命令
            get_kiro_settings,
            set_kiro_proxy,
            set_kiro_model,
            // 应用设置命令
            get_app_settings,
            save_app_settings,
            // 账号绑定机器码命令
            bind_machine_id_to_account,
            unbind_machine_id_from_account,
            get_bound_machine_id,
            get_all_bound_machine_ids,
            // 系统机器码命令
            get_system_machine_guid,
            backup_machine_guid,
            restore_machine_guid,
            reset_system_machine_guid,
            get_machine_guid_backup,
            set_custom_machine_guid,
            clear_macos_override,
            generate_machine_guid,
            // Web OAuth 命令 (Cognito + CBOR)
            web_oauth_initiate,
            web_oauth_complete,
            web_oauth_refresh,
            web_oauth_login,
            web_oauth_close_window,
            // 浏览器检测
            detect_installed_browsers,
            // MCP 管理命令
            get_mcp_config,
            save_mcp_server,
            delete_mcp_server,
            toggle_mcp_server,
            // Powers 管理命令
            get_powers_registry,
            get_installed_powers,
            get_all_powers,
            install_power,
            uninstall_power,
            // 代理检测命令
            detect_system_proxy,
            // SSO Token 导入命令
            import_from_sso_token,
            // 更新检查命令
            check_update,
            // Steering 管理命令
            get_steering_files,
            get_steering_file,
            save_steering_file,
            delete_steering_file,
            create_steering_file,
            // 便携模式命令
            get_portable_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
