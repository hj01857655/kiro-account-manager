// 系统机器码管理模块 - 支持 Windows/macOS/Linux

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数

mod types;
mod utils;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use types::SystemMachineInfo;
pub use utils::generate_random_machine_id;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod platform {
    use super::types::SystemMachineInfo;
    const ERR: &str = "此功能仅支持 Windows、macOS 和 Linux 系统";
    pub fn get_system_machine_guid_inner() -> Result<SystemMachineInfo, String> {
        Err(ERR.into())
    }
    pub fn reset_machine_guid_inner() -> Result<String, String> {
        Err(ERR.into())
    }
    pub fn set_custom_machine_guid_inner(_: String) -> Result<String, String> {
        Err(ERR.into())
    }
    pub fn clear_override_inner() -> Result<(), String> {
        Ok(())
    }
}

async fn run<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("Task failed: {e}"))
}

#[tauri::command]
pub async fn get_system_machine_guid() -> Result<SystemMachineInfo, String> {
    run(platform::get_system_machine_guid_inner).await?
}

#[tauri::command]
pub async fn reset_system_machine_guid() -> Result<String, String> {
    run(platform::reset_machine_guid_inner).await?
}

#[tauri::command]
pub async fn set_custom_machine_guid(new_guid: String) -> Result<String, String> {
    run(move || platform::set_custom_machine_guid_inner(new_guid)).await?
}

#[tauri::command]
pub async fn clear_macos_override() -> Result<(), String> {
    run(platform::clear_override_inner).await?
}

#[tauri::command]
pub fn generate_machine_guid() -> String {
    generate_random_machine_id()
}

/// 以管理员权限重启应用（仅 Windows）
#[tauri::command]
#[allow(unused_variables)]
pub async fn restart_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let exe_path = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;

        // 使用 PowerShell 的 Start-Process -Verb RunAs 以管理员权限启动
        let _status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process -FilePath '{}' -Verb RunAs",
                    exe_path.display().to_string().replace('\'', "''")
                ),
            ])
            .spawn()
            .map_err(|e| format!("启动管理员进程失败: {e}"))?;

        // 等待一小段时间确保新进程启动
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 退出当前应用
        app.exit(0);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let exe_path = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;

        // 尝试使用 pkexec
        let result = Command::new("pkexec").arg(&exe_path).spawn();

        match result {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                app.exit(0);
                Ok(())
            }
            Err(_) => Err("请使用 sudo 或 pkexec 手动以 root 权限运行程序".to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS 不需要管理员权限（写入用户目录）
        Err("macOS 不需要管理员权限".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("不支持的操作系统".to_string())
    }
}
