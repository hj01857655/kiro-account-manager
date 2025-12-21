// Kiro IDE 相关功能

use serde::{Deserialize, Serialize};
use rusqlite::{Connection, OpenFlags};

// ===== Kiro IDE 本地 Token =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroLocalToken {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub auth_method: Option<String>,
    pub provider: Option<String>,
    // Social 专用
    pub profile_arn: Option<String>,
    // IdC 专用
    pub client_id_hash: Option<String>,
    pub region: Option<String>,
}

/// IdC 客户端注册信息 (从 {clientIdHash}.json 读取)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRegistration {
    pub client_id: String,
    pub client_secret: String,
    pub expires_at: Option<String>,
}

#[tauri::command]
pub fn get_kiro_local_token() -> Option<KiroLocalToken> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let path = std::path::Path::new(&home)
        .join(".aws")
        .join("sso")
        .join("cache")
        .join("kiro-auth-token.json");
    
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 读取 IdC 客户端注册信息
pub fn get_client_registration(client_id_hash: &str) -> Option<ClientRegistration> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let path = std::path::Path::new(&home)
        .join(".aws")
        .join("sso")
        .join("cache")
        .join(format!("{}.json", client_id_hash));
    
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ===== Kiro IDE 遥测信息 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroTelemetryInfo {
    pub machine_id: Option<String>,
    pub sqm_id: Option<String>,
    pub dev_device_id: Option<String>,
    pub service_machine_id: Option<String>,
}

/// 获取 Kiro 数据目录（始终使用系统路径，因为这是 Kiro IDE 的数据）
fn get_kiro_data_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|p| std::path::PathBuf::from(p).join("Kiro"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|p| {
            std::path::PathBuf::from(p)
                .join("Library")
                .join("Application Support")
                .join("Kiro")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

fn get_kiro_telemetry_info_inner() -> Option<KiroTelemetryInfo> {
    let kiro_dir = get_kiro_data_dir()?;
    
    // 从 storage.json 读取
    let storage_path = kiro_dir
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    
    let content = std::fs::read_to_string(&storage_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let mut info = KiroTelemetryInfo {
        machine_id: json.get("telemetry.machineId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        sqm_id: json.get("telemetry.sqmId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        dev_device_id: json.get("telemetry.devDeviceId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        service_machine_id: None,
    };
    
    // 从 state.vscdb 读取 serviceMachineId
    let db_path = kiro_dir
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    
    if db_path.exists() {
        // 只读模式打开，避免被 Kiro IDE 占用时出错
        if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            if let Ok(value) = conn.query_row(
                "SELECT value FROM ItemTable WHERE key = 'storage.serviceMachineId'",
                [],
                |row| row.get::<_, String>(0)
            ) {
                info.service_machine_id = Some(value);
            }
        }
    }
    
    Some(info)
}

#[tauri::command]
pub async fn get_kiro_telemetry_info() -> Option<KiroTelemetryInfo> {
    tokio::task::spawn_blocking(get_kiro_telemetry_info_inner)
        .await
        .ok()
        .flatten()
}

/// 获取当前机器 ID（供其他模块使用）
pub fn get_machine_id() -> String {
    get_kiro_telemetry_info_inner()
        .and_then(|info| info.machine_id)
        .unwrap_or_else(|| {
            // 如果获取失败，生成一个随机的
            use uuid::Uuid;
            let uuid_bytes = Uuid::new_v4();
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(uuid_bytes.as_bytes());
            hex::encode(hasher.finalize())
        })
}

// ===== 切换账号 =====

use crate::process::{check_kiro_running, kill_kiro, launch_kiro};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchAccountResult {
    pub success: bool,
    pub message: String,
    pub kiro_was_running: bool,
    pub kiro_restarted: bool,
}

/// 切换账号参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchAccountParams {
    pub access_token: String,
    pub refresh_token: String,
    pub provider: String,
    #[serde(default)]
    pub auth_method: Option<String>,
    // Social 专用
    #[serde(default)]
    pub profile_arn: Option<String>,
    // IdC 专用
    #[serde(default)]
    pub client_id_hash: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    // 选项
    #[serde(default)]
    pub reset_machine_id: Option<bool>,
    #[serde(default)]
    pub auto_restart: Option<bool>,
}

/// 切换 Kiro 账号（直接写入 Token 文件，仅重置机器ID时才关闭IDE）
#[tauri::command]
pub async fn switch_kiro_account(params: SwitchAccountParams) -> Result<SwitchAccountResult, String> {
    // 使用 spawn_blocking 避免阻塞异步运行时
    tokio::task::spawn_blocking(move || {
        let kiro_was_running = check_kiro_running();
        let should_reset = params.reset_machine_id.unwrap_or(false);
        let should_restart = params.auto_restart.unwrap_or(true);
        let auth_method = params.auth_method.unwrap_or_else(|| "social".to_string());
        let access_token = params.access_token;
        let refresh_token = params.refresh_token;
        let provider = params.provider;
        let profile_arn = params.profile_arn;
        let client_id_hash = params.client_id_hash;
        let client_id = params.client_id;
        let client_secret = params.client_secret;
        let region = params.region;
        
        // 1. 只在需要重置机器 ID 时才关闭 IDE
        if should_reset && kiro_was_running {
            kill_kiro()?;
            // 等待进程退出
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        
        // 2. 如果需要重置机器 ID
        if should_reset {
            let _ = reset_kiro_machine_id_inner();
        }
        
        // 3. 替换 Token（始终使用系统路径，因为 Kiro IDE 从这里读取）
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory")?;
        
        let dir_path = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");
        
        std::fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        
        let file_path = dir_path.join("kiro-auth-token.json");
        
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        
        // 根据 auth_method 构建不同的 token 数据
        let token_data = if auth_method == "IdC" {
            // IdC 账号: clientIdHash + region
            let hash = client_id_hash.clone().unwrap_or_default();
            let data = serde_json::json!({
                "accessToken": access_token,
                "refreshToken": refresh_token,
                "expiresAt": expires_at.to_rfc3339(),
                "authMethod": "IdC",
                "provider": provider,
                "clientIdHash": hash,
                "region": region.clone().unwrap_or_else(|| "us-east-1".to_string())
            });
            data
        } else {
            // Social 账号: profileArn
            let arn = profile_arn.unwrap_or_else(|| 
                "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK".to_string()
            );
            serde_json::json!({
                "accessToken": access_token,
                "refreshToken": refresh_token,
                "profileArn": arn,
                "expiresAt": expires_at.to_rfc3339(),
                "authMethod": "social",
                "provider": provider
            })
        };
        
        let content = serde_json::to_string_pretty(&token_data)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        
        // 原子写入：先写临时文件，再覆盖
        let temp_file_path = dir_path.join("kiro-auth-token.json.tmp");
        std::fs::write(&temp_file_path, &content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        std::fs::rename(&temp_file_path, &file_path)
            .map_err(|e| format!("Failed to rename file: {}", e))?;
        
        // IdC 账号还需要写入 Client Registration 文件
        if auth_method == "IdC" {
            if let (Some(hash), Some(cid), Some(csec)) = (client_id_hash, client_id, client_secret) {
                let client_reg_path = dir_path.join(format!("{}.json", hash));
                let client_reg_temp_path = dir_path.join(format!("{}.json.tmp", hash));
                let client_expires = chrono::Utc::now() + chrono::Duration::days(90);
                let client_reg_data = serde_json::json!({
                    "clientId": cid,
                    "clientSecret": csec,
                    "expiresAt": client_expires.to_rfc3339()
                });
                let client_reg_content = serde_json::to_string_pretty(&client_reg_data)
                    .map_err(|e| format!("Failed to serialize client registration: {}", e))?;
                // 原子写入
                std::fs::write(&client_reg_temp_path, client_reg_content)
                    .map_err(|e| format!("Failed to write client registration temp: {}", e))?;
                std::fs::rename(&client_reg_temp_path, &client_reg_path)
                    .map_err(|e| format!("Failed to rename client registration: {}", e))?;
            }
        }
        
        // 4. 切换完成
        let kiro_restarted = if kiro_was_running && should_restart {
            launch_kiro().is_ok()
        } else {
            false
        };
        
        Ok(SwitchAccountResult {
            success: true,
            message: format!("Switched to {} ({}) account", provider, auth_method),
            kiro_was_running,
            kiro_restarted,
        })
    }).await.map_err(|e| format!("Task failed: {}", e))?
}

// ===== 重置机器 ID =====

/// 生成新的机器 ID（64位十六进制字符串）
fn generate_machine_id() -> String {
    use sha2::{Sha256, Digest};
    let random_bytes: [u8; 32] = rand::random();
    let mut hasher = Sha256::new();
    hasher.update(&random_bytes);
    hasher.update(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
    hex::encode(hasher.finalize())
}

/// 生成新的 SQM ID（GUID 格式）
fn generate_sqm_id() -> String {
    format!("{{{}}}", uuid::Uuid::new_v4().to_string().to_uppercase())
}

/// 生成新的 Dev Device ID（UUID 格式）
fn generate_dev_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 重置机器 ID（内部函数）
fn reset_kiro_machine_id_inner() -> Result<KiroTelemetryInfo, String> {
    let kiro_dir = get_kiro_data_dir()
        .ok_or("Cannot find Kiro data directory")?;
    
    let new_machine_id = generate_machine_id();
    let new_sqm_id = generate_sqm_id();
    let new_dev_device_id = generate_dev_device_id();
    
    let storage_path = kiro_dir
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    
    let content = std::fs::read_to_string(&storage_path)
        .map_err(|e| format!("Failed to read storage.json: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse storage.json: {}", e))?;
    
    if let Some(obj) = json.as_object_mut() {
        obj.insert("telemetry.machineId".to_string(), serde_json::json!(new_machine_id));
        obj.insert("telemetry.sqmId".to_string(), serde_json::json!(new_sqm_id));
        obj.insert("telemetry.devDeviceId".to_string(), serde_json::json!(new_dev_device_id));
    }
    
    let new_content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&storage_path, new_content)
        .map_err(|e| format!("Failed to write storage.json: {}", e))?;
    
    let db_path = kiro_dir
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    
    let mut new_service_machine_id = None;
    if db_path.exists() {
        if let Ok(conn) = Connection::open(&db_path) {
            let service_id = generate_machine_id();
            if conn.execute(
                "UPDATE ItemTable SET value = ? WHERE key = 'storage.serviceMachineId'",
                [&service_id]
            ).is_ok() {
                new_service_machine_id = Some(service_id);
            }
        }
    }
    
    Ok(KiroTelemetryInfo {
        machine_id: Some(new_machine_id),
        sqm_id: Some(new_sqm_id),
        dev_device_id: Some(new_dev_device_id),
        service_machine_id: new_service_machine_id,
    })
}

#[tauri::command]
pub async fn reset_kiro_machine_id() -> Result<KiroTelemetryInfo, String> {
    tokio::task::spawn_blocking(reset_kiro_machine_id_inner)
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}


