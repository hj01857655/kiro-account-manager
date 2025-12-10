use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use std::sync::{Arc, Mutex};
use tauri_plugin_shell::ShellExt;
use std::process::Command as StdCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::state::AppState;

// Helper to get App Data Directory
fn get_app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

// Server Status Enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

// Log Entry Struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub source: String,
    pub message: String,
}

// Auth Config for kiro2api
#[derive(Debug, Clone, Serialize)]
struct AuthConfig {
    auth: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

// Server Process Manager State
pub struct ServerProcessState {
    pub status: Arc<Mutex<ServerStatus>>,
    pub child_process: Arc<Mutex<Option<u32>>>,
}

impl ServerProcessState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(ServerStatus::Stopped)),
            child_process: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn start_local_server<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ServerProcessState>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let mut status = state.status.lock().map_err(|e| e.to_string())?;
    
    if *status == ServerStatus::Running || *status == ServerStatus::Starting {
        return Ok(());
    }

    *status = ServerStatus::Starting;
    drop(status);

    let _ = app.emit("server-status-change", ServerStatus::Starting);

    let sidecar_command = app.shell().sidecar("kiro-server").map_err(|e| e.to_string())?;

    let mut env_vars = HashMap::new();
    
    // Setup KV path and ensure directory exists
    if let Some(app_data_dir) = get_app_data_dir(&app) {
        // Create the app data directory if it doesn't exist
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        
        let kv_path = app_data_dir.join("kiro_server.kv");
        env_vars.insert("KIRO_KV_PATH".to_string(), kv_path.to_string_lossy().to_string());
    }

    // Inject PORT
    env_vars.insert("PORT".to_string(), "7860".to_string());

    // Inject KIRO_CLIENT_TOKEN for API authentication
    env_vars.insert("KIRO_CLIENT_TOKEN".to_string(), "local-sidecar-token".to_string());

    // Build KIRO_AUTH_TOKEN from accounts
    let auth_token = {
        let store = app_state.store.lock().map_err(|e| e.to_string())?;
        let accounts = store.get_all();
        
        let auth_configs: Vec<AuthConfig> = accounts
            .iter()
            .filter(|acc| acc.refresh_token.is_some() && acc.status != "已封禁")
            .map(|acc| {
                let auth_type = match acc.provider.as_deref() {
                    Some("IdC") | Some("idc") => "IdC",
                    _ => "Social",
                };
                AuthConfig {
                    auth: auth_type.to_string(),
                    refresh_token: acc.refresh_token.clone().unwrap_or_default(),
                }
            })
            .collect();
        
        if auth_configs.is_empty() {
            return Err("没有可用的账号。请先添加账号后再启动服务器。".to_string());
        }
        
        serde_json::to_string(&auth_configs).map_err(|e| e.to_string())?
    };
    
    env_vars.insert("KIRO_AUTH_TOKEN".to_string(), auth_token);

    let sidecar_command = sidecar_command.envs(env_vars);
    
    let (mut rx, child) = sidecar_command
        .spawn()
        .map_err(|e| {
            let _ = app.emit("server-status-change", ServerStatus::Error);
            if let Ok(mut status) = state.status.lock() {
                *status = ServerStatus::Error;
            }
            e.to_string()
        })?;

    let pid = child.pid();
    if let Ok(mut child_store) = state.child_process.lock() {
        *child_store = Some(pid);
    }

    if let Ok(mut status) = state.status.lock() {
        *status = ServerStatus::Running;
    }
    let _ = app.emit("server-status-change", ServerStatus::Running);

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    let msg = String::from_utf8_lossy(&line).to_string();
                    let _ = app.emit("server-log", LogEntry {
                        timestamp: chrono::Utc::now().timestamp(),
                        source: "stdout".to_string(),
                        message: msg,
                    });
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    let msg = String::from_utf8_lossy(&line).to_string();
                    let _ = app.emit("server-log", LogEntry {
                        timestamp: chrono::Utc::now().timestamp(),
                        source: "stderr".to_string(),
                        message: msg,
                    });
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(_) => {
                    let _ = app.emit("server-status-change", ServerStatus::Stopped);
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_local_server<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, ServerProcessState>,
) -> Result<(), String> {
    let mut status = state.status.lock().map_err(|e| e.to_string())?;
    let mut child_pid = state.child_process.lock().map_err(|e| e.to_string())?;

    if let Some(pid) = *child_pid {
        #[cfg(not(windows))]
        {
            let _ = StdCommand::new("kill")
                .arg(pid.to_string())
                .output()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(windows)]
        {
            let _ = StdCommand::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        }
        
        *child_pid = None;
    }

    *status = ServerStatus::Stopped;
    Ok(())
}

#[tauri::command]
pub async fn get_server_status(
    state: State<'_, ServerProcessState>,
) -> Result<ServerStatus, String> {
    let status = state.status.lock().map_err(|e| e.to_string())?;
    Ok(status.clone())
}
