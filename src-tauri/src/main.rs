#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod auth_social;
mod auto_switch;
mod aws_sso_client;
mod browser;
mod commands;
mod deep_link_handler;
mod http_client;

mod kiro;
mod kiro_auth_client;
mod kiro_portal_client;
mod kiro_cli_db;
mod mcp;

mod process;
mod providers;
mod state;
mod steering;
mod skills;
mod hooks;
mod custom_agents;
mod powers;
mod account;
mod cmd_output;
mod tray_behavior;

use account::{AccountStore, GroupTagStore};
use auth::AuthState;
use state::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{Listener, Manager};

// 导入命令
use browser::detect_installed_browsers;
use commands::account_cmd::{
    get_accounts, delete_account, delete_accounts, delete_account_remote, update_account, sync_account,
    refresh_account_token, verify_account, add_account_by_social, add_local_kiro_account,
    add_account_by_idc, import_accounts, export_accounts, list_available_models,
    get_available_accounts, get_accounts_by_group, get_accounts_by_tag, get_account_usage
};
use commands::group_tag_cmd::{
    get_groups, add_group, update_group, delete_group, reorder_groups,
    get_tags, add_tag, update_tag, delete_tag,
    set_account_group, add_tag_to_account, remove_tag_from_account, set_account_tags, remove_account_tags
};
use commands::app_settings_cmd::{
    get_app_settings, save_app_settings, get_usage_history, save_usage_history_entry,
    bind_machine_id_to_account, unbind_machine_id_from_account, get_bound_machine_id, get_all_bound_machine_ids
};
use commands::auth_cmd::{cancel_kiro_login, get_current_user, logout, kiro_login, get_supported_providers, handle_kiro_social_callback};
use commands::kiro_settings_cmd::{
    get_kiro_settings, set_kiro_proxy, set_kiro_model, set_kiro_codebase_indexing, set_kiro_trusted_commands,
    set_kiro_agent_autonomy, set_kiro_tab_autocomplete, set_kiro_usage_summary, set_kiro_code_references,
    set_kiro_debug_logs, set_kiro_notification,
    set_kiro_trusted_tools, set_kiro_reference_tracker, set_kiro_configure_mcp, set_kiro_telemetry
};
use commands::machine_guid::{
    get_system_machine_guid, backup_machine_guid, restore_machine_guid, reset_system_machine_guid,
    get_machine_guid_backup, set_custom_machine_guid, clear_macos_override, generate_machine_guid, restart_as_admin
};
use commands::mcp_cmd::{get_mcp_config, save_mcp_server, delete_mcp_server, toggle_mcp_server, get_mcp_tool_stats};
use commands::kiro_cli_cmd::{get_kiro_cli_default_path, import_from_kiro_cli};

use commands::proxy_cmd::detect_system_proxy;
use commands::update_cmd::check_update;
use commands::steering_cmd::{get_steering_files, get_steering_file, save_steering_file, delete_steering_file, create_steering_file};
use commands::skills_cmd::{get_skills, get_skill, save_skill, delete_skill, create_skill};
use commands::hooks_cmd::{get_hooks, get_hook, save_hook, delete_hook, create_hook};
use commands::custom_agents_cmd::{get_custom_agents, get_custom_agent, save_custom_agent, delete_custom_agent, create_custom_agent};
use commands::powers_cmd::{get_powers, get_power, install_power, uninstall_power, get_power_registries, get_recommended_powers};

use kiro::{
    get_kiro_local_token, switch_kiro_account, read_kiro_accounts,
};
use process::{close_kiro_ide, is_kiro_ide_running, start_kiro_ide};

/// 配置日志插件
fn setup_log_plugin() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Debug)
        // 只显示我们自己的日志，过滤掉第三方库的日志
        .filter(|metadata| {
            let target = metadata.target();
            target.starts_with("kiro_account_manager")
        })
}

/// 配置单实例插件回调
#[allow(clippy::needless_pass_by_value)] // Tauri 框架要求回调签名为 Vec<String>
fn setup_single_instance_callback(app: &tauri::AppHandle, argv: Vec<String>, _cwd: String) {
    // 当第二个实例尝试启动时，处理传入的参数（deep-link 回调）
    let protocol_prefix = format!("{}://", deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme());
    for arg in &argv {
        if arg.starts_with(&protocol_prefix) {
            deep_link_handler::handle_deep_link(arg);
        }
    }
    
    // 聚焦主窗口
    tray_behavior::show_main_window(app);
}

/// 处理 deep link 事件
fn handle_deep_link_event(app_handle: &tauri::AppHandle, payload: &str) {
    // payload 可能是 JSON 格式 ["kiro-account-manager://..."] 或纯 URL
    let url = if payload.starts_with('[') {
        // JSON 数组格式，解析第一个元素
        serde_json::from_str::<Vec<String>>(payload)
            .ok()
            .and_then(|v| v.into_iter().next())
            .unwrap_or_else(|| payload.to_string())
    } else if payload.starts_with('"') {
        // JSON 字符串格式
        serde_json::from_str::<String>(payload)
            .unwrap_or_else(|_| payload.to_string())
    } else {
        payload.to_string()
    };
    
    // 只处理当前环境的协议
    let protocol_prefix = format!("{}://", deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme());
    if !url.starts_with(&protocol_prefix) {
        return;
    }
    
    // 处理 OAuth 回调
    deep_link_handler::handle_deep_link(&url);
    // 聚焦窗口
    tray_behavior::show_main_window(app_handle);
}

/// 应用 setup 回调
fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 首次启动时检查命令行参数中的 deep link（Windows/Linux）
    let protocol_prefix = format!("{}://", deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme());
    for arg in std::env::args() {
        if arg.starts_with(&protocol_prefix) {
            deep_link_handler::handle_deep_link(&arg);
        }
    }
    
    // 监听 deep link 事件（根据环境自动选择协议）
    #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
    {
        use tauri_plugin_deep_link::DeepLinkExt;
        let scheme = deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme();
        app.deep_link().register(scheme)
            .map_err(|e| format!("Failed to register deep link: {e}"))?;
    }
    
    // 监听 deep link URL
    let app_handle = app.handle().clone();
    app.listen("deep-link://new-url", move |event| {
        handle_deep_link_event(&app_handle, event.payload());
    });
    
    match tray_behavior::create_tray_icon(app.handle()) {
        Ok(tray_icon) => {
            let state = app.state::<AppState>();
            state
                .tray_icon
                .lock()
                .expect("tray icon mutex poisoned")
                .replace(tray_icon);
            state
                .tray_ready
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Err(err) => {
            app.state::<AppState>()
                .tray_ready
                .store(false, std::sync::atomic::Ordering::Relaxed);
            log::warn!("系统托盘初始化失败，将继续启动但不启用关闭到托盘: {err}");
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)] // Tauri 框架要求在 main 中注册所有命令，无法拆分
fn main() {
    tauri::Builder::default()
        .on_window_event(tray_behavior::handle_window_event)
        .plugin(setup_log_plugin().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_http::init())
        // 单实例插件：确保只有一个实例运行，deep-link 回调传递给已运行的实例
        .plugin(tauri_plugin_single_instance::init(setup_single_instance_callback))
        .manage(AppState {
            store: Mutex::new(AccountStore::new()),
            group_tag_store: Mutex::new(GroupTagStore::new()),
            auth: AuthState::new(),
            pending_login: Mutex::new(None),
            tray_ready: AtomicBool::new(false),
            tray_icon: Mutex::new(None),
        })
        .setup(setup_app)
        .invoke_handler(tauri::generate_handler![
            // 账号命令
            get_accounts,
            delete_account,
            delete_accounts,
            delete_account_remote,
            update_account,
            sync_account,
            refresh_account_token,
            verify_account,
            add_account_by_social,
            add_local_kiro_account,
            add_account_by_idc,
            import_accounts,
            export_accounts,
            list_available_models,
            get_available_accounts,
            get_accounts_by_group,
            get_accounts_by_tag,
            get_account_usage,
            // Kiro CLI 导入命令
            get_kiro_cli_default_path,
            import_from_kiro_cli,
            // 分组与标签命令
            get_groups,
            add_group,
            update_group,
            delete_group,
            reorder_groups,
            get_tags,
            add_tag,
            update_tag,
            delete_tag,
            set_account_group,
            add_tag_to_account,
            remove_tag_from_account,
            set_account_tags,
            remove_account_tags,
            // Auth 命令
            get_current_user,
            logout,
            cancel_kiro_login,
            kiro_login,
            get_supported_providers,
            handle_kiro_social_callback,
            // Kiro IDE 命令
            get_kiro_local_token,
            switch_kiro_account,
            read_kiro_accounts,
            // 进程管理命令
            close_kiro_ide,
            start_kiro_ide,
            is_kiro_ide_running,
            // Kiro IDE 设置命令
            get_kiro_settings,
            set_kiro_proxy,
            set_kiro_model,
            set_kiro_codebase_indexing,
            set_kiro_trusted_commands,
            set_kiro_agent_autonomy,
            set_kiro_tab_autocomplete,
            set_kiro_usage_summary,
            set_kiro_code_references,
            set_kiro_debug_logs,
            set_kiro_notification,
            set_kiro_trusted_tools,
            set_kiro_reference_tracker,
            set_kiro_configure_mcp,
            set_kiro_telemetry,
            // 应用设置命令
            get_app_settings,
            save_app_settings,
            // 使用量历史记录命令
            get_usage_history,
            save_usage_history_entry,
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
            restart_as_admin,
            // 浏览器检测
            detect_installed_browsers,
            // MCP 管理命令
            get_mcp_config,
            save_mcp_server,
            delete_mcp_server,
            toggle_mcp_server,
            get_mcp_tool_stats,

            // 代理检测命令
            detect_system_proxy,
            // 更新检查命令
            check_update,
            // Steering 管理命令
            get_steering_files,
            get_steering_file,
            save_steering_file,
            delete_steering_file,
            create_steering_file,
            // Skills 管理命令
            get_skills,
            get_skill,
            save_skill,
            delete_skill,
            create_skill,
            // Hooks 管理命令
            get_hooks,
            get_hook,
            save_hook,
            delete_hook,
            create_hook,
            // Custom Agents 管理命令
            get_custom_agents,
            get_custom_agent,
            save_custom_agent,
            delete_custom_agent,
            create_custom_agent,
            // Powers 管理命令
            get_powers,
            get_power,
            install_power,
            uninstall_power,
            get_power_registries,
            get_recommended_powers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
