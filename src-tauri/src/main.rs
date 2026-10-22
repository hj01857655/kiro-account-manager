#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 核心模块
mod core;
mod state;

// 功能模块
mod auth;
mod auto_switch;
mod clients;
mod commands;
mod gateway;
mod kiro;
mod model_lock;
mod models;
mod services;
mod tasks; // 后台任务模块
mod utils;

use auth::AuthState;
use core::account::{AccountStore, GroupTagStore};
use services::session_storage::SessionStorage;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::{Listener, Manager};

// 导入命令
use utils::browser::detect_installed_browsers;

//账号管理页面
use commands::account_cmd::{
    add_account_by_external_idp, add_account_by_idc, add_account_by_social, add_local_kiro_account,
    check_all_tokens_status,
    check_token_status, delete_account, delete_account_remote, delete_accounts, export_accounts,
    get_account_usage, get_accounts, get_accounts_by_group, get_accounts_by_tag,
    get_available_accounts, get_usage_limits, import_accounts, list_available_models,
    refresh_token, refresh_all_expiring_tokens, set_overage_status, sync_account,
    update_account, verify_account,
};
//应用设置
use commands::app_data_cmd::{get_app_data_dir, open_app_data_dir};
use commands::app_settings_cmd::{
    clear_custom_kiro_path, get_app_settings, get_custom_kiro_path, get_usage_history,
    save_app_settings, save_usage_history_entry, set_custom_kiro_path,
};
//授权相关
use commands::auth_cmd::{
    cancel_kiro_login, get_current_user, handle_kiro_social_callback,
    kiro_login, logout,
};
use crate::auth::providers::get_supported_providers;
use commands::cli_config_cmd::{
    check_claude_code_installed, check_codex_cli_installed, write_claude_code_config,
    write_codex_cli_config,
};

// 网关 kiro2api
use commands::gateway_cmd::{
    cleanup_stale_health,
    clear_gateway_request_logs,
    clear_rate_limit_account,
    get_all_account_health,
    get_available_models,
    get_banned_accounts,
    get_gateway_config,
    get_gateway_client_stats,
    get_gateway_endpoint_stats,
    get_gateway_log_dir,
    get_gateway_model_stats,
    get_gateway_request_logs,
    get_gateway_request_stats,
    get_gateway_status,
    // 负载均衡 API
    get_rate_limited_accounts,
    open_gateway_log_dir,
    reset_account_health,
    save_gateway_config,
    start_gateway,
    stop_gateway,
    test_route_config,
};
//缓存管理
use commands::cache_cmd::{
    cleanup_expired_cache, clear_all_cache, clear_session_cache, get_cache_stats,
};
//分组
use commands::group_tag_cmd::{
    add_group, add_tag, add_tag_to_account, delete_group, delete_tag, get_groups, get_tags,
    remove_account_tags, remove_tag_from_account, reorder_groups, set_account_group,
    set_account_tags, update_group, update_tag,
};
//kiro-cli
use commands::kiro_cli_cmd::{
    check_cli_installation, get_kiro_cli_default_path, import_from_kiro_cli, logout_cli_account,
    read_cli_db_snapshot, rollback_cli_switch, switch_to_cli_account,
};
//kiroshe
use commands::kiro_settings_cmd::{
    get_kiro_settings, open_kiro_settings_file, read_cloud_config_enabled,
    read_remote_sessions_env_override, set_kiro_agent_autonomy,
    set_kiro_agent_setting, set_kiro_codebase_indexing, set_kiro_configure_mcp, set_kiro_debug_logs,
    set_kiro_model, set_kiro_notification, set_kiro_proxy, set_kiro_reference_tracker,
    set_kiro_tab_autocomplete, set_kiro_telemetry, set_kiro_usage_summary,
};
use commands::machine_guid::{
    clear_macos_override, generate_machine_guid, get_system_machine_guid,
    reset_system_machine_guid, restart_as_admin, set_custom_machine_guid,
};
use commands::mcp_cmd::{
    delete_mcp_server, get_mcp_config, get_mcp_tool_stats, save_mcp_server, toggle_mcp_server,
};

use commands::custom_agents_cmd::{
    create_custom_agent, delete_custom_agent, get_custom_agent, get_custom_agents,
    save_custom_agent,
};
use commands::hooks_cmd::{create_hook, delete_hook, get_hook, get_hooks, save_hook};
use commands::permissions_cmd::{
    get_permission_capabilities, get_permissions, list_permission_workspace_roots,
    save_permissions,
};
use commands::session_manager::{
    delete_session, delete_workspace, export_session, list_sessions, list_workspaces, load_session,
    search_sessions,
};
use commands::cli_session_cmd::{
    delete_cli_session, export_cli_session, list_cli_sessions, load_cli_session,
    search_cli_sessions,
};

//代理
use commands::proxy_cmd::{detect_system_proxy, test_account_proxy};

//Powers
use commands::powers_cmd::{
    get_power, get_power_registries, get_powers, get_recommended_powers, get_user_added_powers,
    install_power, install_power_from_local, install_power_from_url,
    uninstall_power,
};
//Steering
use commands::steering_cmd::{
    create_default_steering_file, create_initial_project_steering, create_steering_file,
    delete_steering_file, get_agents_md, get_steering_file, get_steering_files,
    refine_steering_file, save_agents_md, save_steering_file, scan_agents_md,
    list_agents_md_ignore_files,
};
use commands::specs_cmd::{
    create_spec, delete_spec, list_specs, read_spec, save_spec_file,
};
use commands::workflows_cmd::{
    create_workflow, delete_workflow, list_workflows, read_workflow, save_workflow,
};
//Skills
use commands::skills_cmd::{
    create_skill, delete_skill, get_skill, get_skills, import_skill_from_github,
    import_skill_local, save_skill,
};
//Kiro IDE
use crate::kiro::ide::{
    check_ide_installation, check_kiro_config_files, get_kiro_local_token, logout_kiro_account,
    read_kiro_accounts, switch_kiro_account,
};
//Kiro进程
use crate::kiro::process::{close_kiro_ide, is_kiro_ide_running, start_kiro_ide};

use commands::update_cmd::check_update;

/// 配置日志插件
fn setup_log_plugin() -> tauri_plugin_log::Builder {
    let log_level = gateway::load_gateway_config()
        .ok()
        .map(|config| match config.log_level.as_str() {
            "debug" => log::LevelFilter::Debug,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        })
        .unwrap_or(log::LevelFilter::Info);

    tauri_plugin_log::Builder::new()
        .level(log_level)
        // 只显示我们自己的日志，过滤掉第三方库的日志
        .filter(|metadata| {
            let target = metadata.target();
            target.starts_with("kiro_account_manager")
        })
        // 紧凑格式器：HH:MM:SS LEVEL [短模块] msg
        // 默认格式 [date][time][full_target][LEVEL] msg 在终端太长不利于阅读
        .format(|out, message, record| {
            let target = record.target();
            // 取最后一段模块名
            let short_target = target.rsplit("::").next().unwrap_or(target);
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                short_target,
                message
            ))
        })
        // 输出到日志文件 + 终端 stdout（dev 模式可见）
        // 文件名：app.log（所有模块通用应用日志，含 [Gateway]/[Account] 等前缀区分）
        // Gateway 业务请求单独写 gateway-*.log
        .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                file_name: Some("app".to_string()),
            }),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
        ])
        // 日志轮转：每个文件最大 10MB，保留最近 5 个文件（含当前激活的 app.log：4 个归档 + 1 个当前）
        // 注意：KeepAll 会无限累积归档文件（实测曾堆积 11 个、共 172MB），故用 KeepSome 显式限制。
        // remove_old_files 只清理以 app_ 开头的归档，旧命名的 KiroAccountManager*.log 不会被处理。
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
        .max_file_size(10_000_000)
}

/// 配置单实例插件回调
#[allow(clippy::needless_pass_by_value)] // Tauri 框架要求回调签名为 Vec<String>
fn setup_single_instance_callback(app: &tauri::AppHandle, argv: Vec<String>, _cwd: String) {
    // 当第二个实例尝试启动时，处理传入的参数（deep-link 回调）
    let protocol_prefix = format!(
        "{}://",
        core::deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme()
    );
    for arg in &argv {
        if arg.starts_with(&protocol_prefix) {
            auth::handle_incoming_deep_link(app, arg);
        }
    }
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
        serde_json::from_str::<String>(payload).unwrap_or_else(|_| payload.to_string())
    } else {
        payload.to_string()
    };

    // 只处理当前环境的协议
    let protocol_prefix = format!(
        "{}://",
        core::deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme()
    );
    if !url.starts_with(&protocol_prefix) {
        return;
    }

    auth::handle_incoming_deep_link(app_handle, &url);
}

/// 把主窗口带到前台并按当前屏幕可用区收一次尺寸。
///
/// 窗口在 tauri.conf.json 里就是 visible，启动卡由 index.html 的 #boot-splash 盖住，
/// 所以这里不再是「首次显示」——前端首屏 ready 后调用它，是为了在 splash 撤掉的
/// 同一时刻完成 fit + focus，避免用户看到一次尺寸跳变。
#[tauri::command]
fn show_main_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        fit_window_to_screen(&window);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 把窗口尺寸收进当前屏幕可用区。
///
/// `tauri.conf.json` 里写死 1400×820，而 13 寸 MacBook 默认缩放的可用高度
/// 只有约 800px——窗口会被挤出屏幕，用户看到的是「打不开」或只能拖出来。
/// 这里在首次显示前按 monitor 可用区收缩，保留原尺寸意图（大屏不受影响）。
///
/// 只在窗口超出屏幕时收缩，不放大：放大反而会改变大屏上的观感。
fn fit_window_to_screen(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let work = monitor.size().to_logical::<u32>(monitor.scale_factor());
    // 留出余量：dock / menu bar / 窗口阴影
    let max_w = work.width.saturating_sub(80);
    let max_h = work.height.saturating_sub(120);
    let Ok(size) = window.inner_size() else {
        return;
    };
    let logical = size.to_logical::<u32>(monitor.scale_factor());
    let target_w = logical.width.min(max_w);
    let target_h = logical.height.min(max_h);
    if target_w != logical.width || target_h != logical.height {
        log::info!(
            "主窗口尺寸 {}x{} 超出屏幕可用区 {}x{}，收缩为 {}x{}",
            logical.width,
            logical.height,
            max_w,
            max_h,
            target_w,
            target_h
        );
        let _ = window.set_size(tauri::LogicalSize::new(target_w, target_h));
        let _ = window.center();
    }
}

/// 应用 setup 回调
fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 deep link 处理器
    core::deep_link_handler::init();

    // 首次启动时检查命令行参数中的 deep link（Windows/Linux）
    let protocol_prefix = format!(
        "{}://",
        core::deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme()
    );
    for arg in std::env::args() {
        if arg.starts_with(&protocol_prefix) {
            auth::handle_incoming_deep_link(app.handle(), &arg);
        }
    }

    // 监听 deep link 事件（根据环境自动选择协议）
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        use tauri_plugin_deep_link::DeepLinkExt;
        let scheme = core::deep_link_handler::DeepLinkCallbackWaiter::get_protocol_scheme();
        app.deep_link()
            .register(scheme)
            .map_err(|e| format!("Failed to register deep link: {e}"))?;
    }

    // 监听 deep link URL
    let app_handle = app.handle().clone();
    app.listen("deep-link://new-url", move |event| {
        handle_deep_link_event(&app_handle, event.payload());
    });

    // 自动启动网关（如果配置了）
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = gateway::auto_start_if_enabled(&app_handle).await {
            log::error!("自动启动 kiro2api 失败: {err}");
        }
    });

    // 启动模型锁定后台任务
    let app_handle = app.handle().clone();
    model_lock::start_model_lock_task(app_handle);

    // 启动自动换号后台任务
    let app_handle = app.handle().clone();
    auto_switch::start_auto_switch_task(app_handle);

    // 启动 Token 自动刷新后台任务（参考 Kiro IDE）
    tasks::token_refresh::start_token_refresh_loop(app.handle().clone());

    // 创建托盘图标
    setup_system_tray(app)?;

    // 监听窗口关闭事件
    setup_window_close_handler(app)?;

    // 主窗口始终可见（启动卡由 index.html 的 #boot-splash 盖住，不藏窗口）。
    // 前端首屏 ready 后调 show_main_window 做 fit + focus；这里仍保留一次兜底：
    // 若前端始终没调用（打包损坏、JS 抛异常），3 秒后强制收一次尺寸并聚焦，
    // 配合 index.html 里 8 秒强制撤 splash，保证用户至少能拿到一个可操作的窗口。
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        if let Some(window) = app_handle.get_webview_window("main") {
            if !window.is_focused().unwrap_or(false) {
                log::warn!("主窗口 3s 内未聚焦，强制 fit + focus");
                fit_window_to_screen(&window);
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });

    Ok(())
}

/// 创建系统托盘
fn setup_system_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        Manager,
    };

    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Kiro Account Manager")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 监听窗口关闭事件
fn setup_window_close_handler(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::Manager;

    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 读取配置，决定是最小化到托盘还是直接退出
                let settings =
                    commands::app_settings_cmd::get_app_settings_inner().unwrap_or_default();

                if settings.close_to_tray.unwrap_or(false) {
                    // 最小化到托盘
                    api.prevent_close();
                    let _ = window_clone.hide();
                } else {
                    // 直接退出
                    // 不调用 api.prevent_close()，让窗口正常关闭
                }
            }
        });
    }

    Ok(())
}

#[allow(clippy::too_many_lines)] // Tauri 框架要求在 main 中注册所有命令，无法拆分
fn main() {
    // 解析 --data-dir=<路径>（由 restart_as_admin 在提权前注入）。
    // 必须在任何路径解析之前完成：以其他管理员账号凭据提权时，进程的 %APPDATA% 会指向
    // 那个管理员，若不沿用原目录就会读不到账号数据（表现为「账号全部消失」）。
    for arg in std::env::args() {
        if let Some(value) = arg.strip_prefix("--data-dir=") {
            core::paths::set_data_dir_override(std::path::PathBuf::from(
                value.trim_matches('"'),
            ));
            break;
        }
    }

    // 注入 bundle identifier（仅用于日志展示）。应用数据目录固定为
    // `%APPDATA%\.kiro-account-manager`（点前缀旧约定，用户确认保持不变，不随 identifier 变化）。
    let tauri_context = tauri::generate_context!();
    core::paths::set_identifier(tauri_context.config().identifier.clone());

    // 历史数据目录与应用数据目录相同（`.kiro-account-manager`），无需迁移。
    // migrate_legacy_data() 现为 no-op，保留调用以维持启动流程不变。
    match core::paths::migrate_legacy_data() {
        Ok(true) => log::info!(
            "应用数据已从 {} 迁移到 {}",
            core::paths::LEGACY_DIR_NAME,
            core::paths::identifier()
        ),
        Ok(false) => {}
        Err(err) => log::error!("应用数据目录迁移失败，将保留读取旧路径: {err}"),
    }

    tauri::Builder::default()
        .plugin(setup_log_plugin().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        // 单实例插件：确保只有一个实例运行，deep-link 回调传递给已运行的实例
        .plugin(tauri_plugin_single_instance::init(
            setup_single_instance_callback,
        ))
        .manage(AppState {
            store: Arc::new(Mutex::new(AccountStore::new())),
            group_tag_store: Mutex::new(GroupTagStore::new()),
            auth: AuthState::new(),
            pending_login: Mutex::new(None),
            gateway: Mutex::new(None),
        })
        .manage(SessionStorage::new().expect("Failed to initialize SessionStorage"))
        .manage(services::cli_session_storage::CliSessionStorage::new().expect("Failed to initialize CliSessionStorage"))
        .setup(setup_app)
        .invoke_handler(tauri::generate_handler![
            // 账号命令
            get_accounts,
            delete_account,
            delete_accounts,
            delete_account_remote,
            update_account,
            sync_account,
            get_usage_limits,
            refresh_token,
            verify_account,
            add_account_by_social,
            add_local_kiro_account,
            add_account_by_idc,
            add_account_by_external_idp,
            import_accounts,
            export_accounts,
            list_available_models,
            get_available_accounts,
            get_accounts_by_group,
            get_accounts_by_tag,
            get_account_usage,
            set_overage_status,
            // Token 状态检查命令
            check_token_status,
            check_all_tokens_status,
            refresh_all_expiring_tokens,
            // Kiro CLI 导入命令
            get_kiro_cli_default_path,
            import_from_kiro_cli,
            check_cli_installation,
            read_cli_db_snapshot,
            switch_to_cli_account,
            rollback_cli_switch,
            logout_cli_account,
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
            check_ide_installation,
            check_kiro_config_files,
            get_kiro_local_token,
            read_kiro_accounts,
            switch_kiro_account,
            logout_kiro_account,
            set_custom_kiro_path,
            get_custom_kiro_path,
            clear_custom_kiro_path,
            // 进程管理命令
            close_kiro_ide,
            start_kiro_ide,
            is_kiro_ide_running,
            // Kiro IDE 设置命令
            get_kiro_settings,
            open_kiro_settings_file,
            read_cloud_config_enabled,
            read_remote_sessions_env_override,
            get_permissions,
            save_permissions,
            get_permission_capabilities,
            list_permission_workspace_roots,
            set_kiro_proxy,
            set_kiro_model,
            set_kiro_codebase_indexing,
            set_kiro_agent_autonomy,
            set_kiro_tab_autocomplete,
            set_kiro_usage_summary,
            set_kiro_debug_logs,
            set_kiro_notification,
            set_kiro_reference_tracker,
            set_kiro_configure_mcp,
            set_kiro_telemetry,
            set_kiro_agent_setting,
            // 应用设置命令
            get_app_settings,
            save_app_settings,
            // 使用量历史记录命令
            get_usage_history,
            save_usage_history_entry,
            // 系统机器码命令
            get_system_machine_guid,
            reset_system_machine_guid,
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
            // Gateway 命令
            start_gateway,
            stop_gateway,
            get_gateway_status,
            get_gateway_config,
            save_gateway_config,
            get_gateway_log_dir,
            get_gateway_request_logs,
            get_gateway_request_stats,
            get_gateway_model_stats,
            get_gateway_endpoint_stats,
            get_gateway_client_stats,
            open_gateway_log_dir,
            clear_gateway_request_logs,
            test_account_proxy,
            // Gateway 负载均衡 API
            get_rate_limited_accounts,
            get_banned_accounts,
            clear_rate_limit_account,
            get_all_account_health,
            reset_account_health,
            cleanup_stale_health,
            test_route_config,
            get_available_models,
            // 缓存管理命令
            get_cache_stats,
            clear_all_cache,
            clear_session_cache,
            cleanup_expired_cache,
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
            create_default_steering_file,
            create_initial_project_steering,
            refine_steering_file,
            scan_agents_md,
            list_agents_md_ignore_files,
            get_agents_md,
            save_agents_md,
            // Specs 管理命令
            list_specs,
            read_spec,
            save_spec_file,
            create_spec,
            delete_spec,
            // Workflows 管理命令
            list_workflows,
            read_workflow,
            save_workflow,
            create_workflow,
            delete_workflow,
            // Skills 管理命令
            get_skills,
            get_skill,
            save_skill,
            delete_skill,
            create_skill,
            import_skill_local,
            import_skill_from_github,
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
            install_power_from_local,
            install_power_from_url,
            uninstall_power,
            get_power_registries,
            get_user_added_powers,
            get_recommended_powers,
            // Session Manager 命令
            list_workspaces,
            list_sessions,
            load_session,
            // CLI Session 命令
            list_cli_sessions,
            load_cli_session,
            delete_cli_session,
            search_cli_sessions,
            export_cli_session,
            // 一键换号
            auto_switch::quick_switch_next,
            delete_session,
            delete_workspace,
            export_session,
            search_sessions,
            // CLI 配置命令
            check_claude_code_installed,
            check_codex_cli_installed,
            write_claude_code_config,
            write_codex_cli_config,
            // 应用数据目录命令
            get_app_data_dir,
            open_app_data_dir,
            // 窗口显示命令
            show_main_window
        ])
        .run(tauri_context)
        .expect("error while running tauri application");
}
