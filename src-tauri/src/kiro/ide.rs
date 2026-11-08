// Kiro IDE 相关功能

use serde::{Deserialize, Serialize};

// ===== 辅助函数 =====

/// 检查文件是否为符号链接（安全性检查，参考 Kiro IDE）
fn assert_not_symlink(path: &std::path::Path) -> Result<(), String> {
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        if metadata.file_type().is_symlink() {
            return Err("Token file is a symbolic link".to_string());
        }
    }
    Ok(())
}

/// 设置文件权限为 0600（仅所有者可读写，仅 Unix 系统）
#[cfg(unix)]
fn set_file_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    Ok(())
}

/// Windows 系统不需要设置权限
#[cfg(not(unix))]
fn set_file_permissions(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}

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
    // external_idp（企业 SSO / Entra ID）专用：Kiro IDE 登录企业 IdP 后写入缓存，
    // 字段名对齐 kiro.rs-admin / Account Manager 导出格式（camelCase）
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    #[serde(default)]
    pub issuer_url: Option<String>,
    #[serde(default)]
    pub scopes: Option<String>,
    // 注意：Kiro IDE 不在 kiro-auth-token.json 中存储 startUrl
    // startUrl 包含在 clientSecret JWT 的 initiateLoginUri 字段中
}

/// `IdC` 客户端注册信息 (从 {clientIdHash}.json 读取)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRegistration {
    pub client_id: String,
    pub client_secret: String,
    pub expires_at: Option<String>,
}

#[tauri::command]
pub async fn get_kiro_local_token() -> Option<KiroLocalToken> {
    tokio::task::spawn_blocking(|| {
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
    })
    .await
    .ok()
    .flatten()
}

/// 读取 `IdC` 客户端注册信息
pub async fn get_client_registration(client_id_hash: &str) -> Option<ClientRegistration> {
    // 安全检查：防止路径遍历攻击
    // 只允许字母、数字、下划线和连字符
    if !client_id_hash
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        log::warn!("[安全] 检测到非法的 client_id_hash: {}", client_id_hash);
        return None;
    }

    let hash = client_id_hash.to_string();
    tokio::task::spawn_blocking(move || {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let path = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache")
            .join(format!("{hash}.json"));

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    })
    .await
    .ok()
    .flatten()
}

// ===== 从 Kiro IDE 导入账号 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroAccountInfo {
    pub email: String,
    pub provider: String,
    pub auth_method: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    // Social 专用
    pub profile_arn: Option<String>,
    // IdC 专用
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub client_id_hash: Option<String>,
    pub region: Option<String>,
    // external_idp（企业 SSO / Entra ID）专用：微软刷新元数据
    pub token_endpoint: Option<String>,
    pub issuer_url: Option<String>,
    pub scopes: Option<String>,
    // 注意：不需要 start_url 字段
    // startUrl 包含在 clientSecret JWT 的 initiateLoginUri 字段中
    // AWS SSO OIDC API 会自动从 JWT 中解析
}

/// 读取 Kiro IDE 中的所有账号
#[tauri::command]
pub async fn read_kiro_accounts() -> Result<Vec<KiroAccountInfo>, String> {
    tokio::task::spawn_blocking(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "无法获取用户目录")?;

        let cache_dir = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");

        if !cache_dir.exists() {
            return Err("未找到 Kiro IDE 缓存目录".to_string());
        }

        let mut accounts = Vec::new();

        // 读取主 token 文件
        let token_path = cache_dir.join("kiro-auth-token.json");
        if !token_path.exists() {
            return Err(format!(
                "未找到 Kiro IDE 登录缓存（{}）。请先在 Kiro IDE 中登录；\
                 若你使用的是 Kiro CLI 登录，请改用「Kiro CLI」导入页",
                token_path.display()
            ));
        }

        let content = std::fs::read_to_string(&token_path)
            .map_err(|e| format!("读取 Kiro IDE 缓存失败（{}）: {e}", token_path.display()))?;

        let token: KiroLocalToken = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                // 解析失败 ≠ 未登录：格式漂移时给出可上报的具体原因
                return Err(format!(
                    "Kiro IDE 缓存解析失败（{}）：{e}。\
                     可能是 Kiro IDE 版本更新导致缓存格式变化，请附带此信息反馈",
                    token_path.display()
                ));
            }
        };

        {
            let auth_method = token.auth_method.as_deref().unwrap_or("social");
            let provider = token
                .provider
                .clone()
                .unwrap_or_else(|| "Google".to_string());

            let mut account = KiroAccountInfo {
                email: String::new(), // 需要通过 API 获取
                provider: provider.clone(),
                auth_method: auth_method.to_string(),
                access_token: token.access_token.clone(),
                refresh_token: token.refresh_token.clone(),
                expires_at: token.expires_at.clone(),
                profile_arn: token.profile_arn.clone(),
                client_id: None,
                client_secret: None,
                client_id_hash: token.client_id_hash.clone(),
                region: token.region.clone(),
                token_endpoint: None,
                issuer_url: None,
                scopes: None,
            };

            // 如果是 IdC 账号，读取 client registration
            if auth_method == "IdC" {
                if let Some(ref hash) = token.client_id_hash {
                    let client_path = cache_dir.join(format!("{hash}.json"));
                    if let Ok(client_content) = std::fs::read_to_string(&client_path) {
                        if let Ok(client_reg) =
                            serde_json::from_str::<ClientRegistration>(&client_content)
                        {
                            account.client_id = Some(client_reg.client_id);
                            account.client_secret = Some(client_reg.client_secret);
                        }
                    }
                }
            }

            // external_idp（企业 SSO / Entra ID）：从主 token 文件读取微软刷新元数据
            if auth_method == "external_idp" {
                account.client_id = token.client_id.clone();
                account.token_endpoint = token.token_endpoint.clone();
                account.issuer_url = token.issuer_url.clone();
                account.scopes = token.scopes.clone();
                // profileArn 为空时置 None：external_idp 号的真实 ARN
                // 由 usage 链路的 ListAvailableProfiles 懒解析回填，
                // 绝不套 IdC/social 默认占位 ARN
                if account
                    .profile_arn
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
                {
                    account.profile_arn = None;
                }
            }

            accounts.push(account);
        }

        if accounts.is_empty() {
            return Err("Kiro IDE 缓存已读取但未解析出有效账号".to_string());
        }

        Ok(accounts)
    })
    .await
    .map_err(|e| format!("读取失败: {e}"))?
}

// ===== 切换账号 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchAccountResult {
    pub success: bool,
    pub message: String,
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
    pub start_url: Option<String>, // Enterprise 必须提供，BuilderId 不需要
    #[serde(default)]
    pub client_id_hash: Option<String>, // 优先用账号已存的 hash（IDE 登录产物里直接存了它）
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    // 仅用于日志记录，标识是哪个账号
    #[serde(default)]
    pub email: Option<String>,
}

/// 切换 Kiro 账号（原子写入 Token 文件，无需重启 IDE）
#[tauri::command]
pub async fn switch_kiro_account(
    params: SwitchAccountParams,
) -> Result<SwitchAccountResult, String> {
    tokio::task::spawn_blocking(move || {
        let auth_method = params.auth_method.unwrap_or_else(|| "social".to_string());
        let access_token = params.access_token;
        let refresh_token = params.refresh_token;
        let provider = params.provider;
        let profile_arn = params.profile_arn;
        let start_url = params.start_url;
        let client_id = params.client_id;
        let email = params.email;
        let client_secret = params.client_secret;
        let region = params.region;
        let client_id_hash = params.client_id_hash.filter(|h| !h.trim().is_empty());

        // 获取 token 目录
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory")?;

        let dir_path = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");

        std::fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Failed to create directory: {e}"))?;

        let file_path = dir_path.join("kiro-auth-token.json");
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        // 切换前先记下「当前登录账号」的 IdC clientIdHash（若有）。
        // 对齐 Kiro IDE 行为：IDE 切号 = 先 logout（deleteClientRegistration 删旧 {hash}.json）
        // 再 login（写新文件），同一时刻只保留一个登录态的客户端注册。KAM 旧实现只写不删，
        // 切换不同 client 的账号会让 {hash}.json 越堆越多（issue: 切完不清理旧注册）。
        // 这里读出旧 hash，待新文件写成功后，若与新 hash 不同则删除旧 {hash}.json。
        // 安全前提：只读 Kiro 自己的 kiro-auth-token.json 得到的 hash——绝不扫描目录，
        // 因此不会误删 AWS CLI v2 在同一 cache 目录下的 SSO 缓存文件。解析失败不阻断切换。
        let previous_idc_hash = std::fs::read_to_string(&file_path)
            .ok()
            .and_then(|content| serde_json::from_str::<KiroLocalToken>(&content).ok())
            .filter(|tok| tok.auth_method.as_deref() == Some("IdC"))
            .and_then(|tok| tok.client_id_hash)
            .filter(|h| !h.trim().is_empty());

        // IdC 账号统一解析 clientIdHash：规则收敛在 common::resolve_idc_client_id_hash，
        // 切号 / 添加 / 导入共用同一裁决点（优先已存 hash → BuilderId 常量兜底 /
        // Enterprise 由 startUrl 现算 → Enterprise 硬校验拦截 issue #119 根因）。
        let idc_hash = if auth_method == "IdC" {
            Some(crate::commands::common::resolve_idc_client_id_hash(
                &provider,
                client_id_hash.as_deref(),
                start_url.as_deref(),
            )?)
        } else {
            None
        };

        // 提前校验 IdC 写文件所需的 client_id / client_secret，确保切号语义是
        // 「要么整体成功、要么什么都不动」。旧实现把这对凭据的校验留到写完
        // kiro-auth-token.json 之后、写 {hash}.json 之前（见下方 else 分支的 return Err），
        // 一旦缺失就提前返回——但此刻 token 文件已 rename 成新账号，而新 {hash}.json 从未写成，
        // IDE 会拿到「指向新 IdC 账号的 token + 找不到 client registration」的破碎中间态。
        // 这里在动任何文件前先校验：失败时旧登录态原封不动，不留不一致状态。
        if auth_method == "IdC" {
            let cid_ok = client_id
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            let csec_ok = client_secret
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !cid_ok || !csec_ok {
                return Err("IdC 账号必须提供 client_id 和 client_secret".to_string());
            }
        }

        // IdC 账号 client_secret 健康度检查（只警告不阻挡切号）：
        // 部分历史账号（早期 KAM 在线登录或 Amazon Q CLI 导入）的 clientSecret JWT
        // payload 里 enabledGrants 为空，缺 REFRESH_TOKEN grant，IDE 那边 token 一过期
        // 调 SSO refresh 时会被 AWS 拒，回报 "Unable to fetch account usage data: Invalid token"。
        //
        // KAM 不应替 IDE 当法官——access_token 在 1 小时有效期内**仍然能用**，
        // 历史用户切号到这种账号过去一直工作正常，强行阻断会导致灾难性回归。
        // 这里改成日志告警，让用户/KAM UI 自行决定如何提示，切号流程继续。
        // v1.8.9+ 在线登录已修复（redirect_uri 不带端口才会拿到带 REFRESH_TOKEN grant 的 client），
        // 受影响账号重新登录一次即可获得健康 client。
        if auth_method == "IdC" {
            if let Some(secret) = client_secret.as_deref() {
                if !crate::utils::client_id_hash::client_supports_refresh_token(secret) {
                    // 使用 email 标识账号，如果没有 email 则用 provider 兜底
                    let account_identifier = email
                        .as_ref()
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.as_str())
                        .unwrap_or_else(|| provider.as_str());
                    
                    log::warn!(
                        "[switch_kiro_account] {} 账号 [{}] 的 clientSecret 缺少 REFRESH_TOKEN grant，\
                         IDE token 过期后 refresh 会被 AWS 拒。\
                         建议用户在 KAM v1.8.9+ 重新登录此账号以获取健康 client。",
                        provider,
                        account_identifier
                    );
                }
            }
        }

        // 根据 auth_method 构建 token 数据
        let token_data = if auth_method == "IdC" {
            let hash = idc_hash.clone().ok_or("IdC 账号缺少 clientIdHash")?;
            let region_value = region.ok_or("IdC 账号必须提供 region")?;

            let mut obj = serde_json::json!({
                "accessToken": access_token,
                "refreshToken": refresh_token,
                "expiresAt": expires_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                "clientIdHash": hash,
                "authMethod": "IdC",
                "provider": provider,
                "region": region_value,
            });

            // BuilderId 真实缓存带 profileArn（Enterprise 不带）。实测 IDE 写出的 BuilderId
            // token 末尾有 profileArn，缺它会导致 IDE 调 CodeWhisperer/Q API 时无 profile，
            // BuilderId 账号在 IDE 里失效——别的管理器照真实格式写了 profileArn 所以能用。
            // 账号自带优先，否则用 BuilderId 默认常量（与真实缓存里的值一致）。
            if provider == "BuilderId" {
                let arn = profile_arn
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        crate::commands::common::KIRO_BUILDER_ID_PROFILE_ARN.to_string()
                    });
                obj["profileArn"] = serde_json::Value::String(arn);
            }

            obj
        } else {
            let arn = profile_arn
                .unwrap_or_else(|| crate::commands::common::KIRO_SOCIAL_PROFILE_ARN.to_string());
            serde_json::json!({
                "accessToken": access_token,
                "refreshToken": refresh_token,
                "profileArn": arn,
                "expiresAt": expires_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                "authMethod": "social",
                "provider": provider
            })
        };

        let content = serde_json::to_string_pretty(&token_data)
            .map_err(|e| format!("Failed to serialize: {e}"))?;

        // 写入顺序对齐 Kiro IDE：IDE 把 kiro-auth-token.json 当作「登录开关」——
        // 登录时 registerClient 先写 {hash}.json（依赖），最后 writeToken 写 token 文件（触发器）；
        // 退出登录时先 clearToken 删 token 文件，最后 deleteClientRegistration 删 {hash}.json。
        // 不变式：只要 token 文件在，它依赖的 {hash}.json 必定也在。
        // 因此这里 IdC 账号必须「先写 {hash}.json，最后写 kiro-auth-token.json」——
        // 即便进程在两步之间崩溃，也只是留下一个孤儿 {hash}.json（下次切号会覆盖/清理），
        // token 文件仍指向旧账号，绝不会出现「token 指向新 IdC 账号却找不到 client registration」的破碎态。
        // social 账号无 {hash}.json，直接写 token 文件即可。

        use std::io::Write;

        // 第一步（仅 IdC）：先写 Client Registration 文件 {hash}.json。
        // 上方已在动文件之前校验过 client_id / client_secret 非空，这里解构必定成功。
        if auth_method == "IdC" {
            if let (Some(cid), Some(csec)) = (client_id.as_deref(), client_secret.as_deref()) {
                // 复用上面统一解析出的 clientIdHash（与 token 文件里的 clientIdHash 保持一致，
                // 确保 {hash}.json 文件名与 IDE 查找的路径完全匹配）
                let hash = idc_hash.clone().ok_or("IdC 账号缺少 clientIdHash")?;

                let client_reg_path = dir_path.join(format!("{hash}.json"));
                let client_reg_temp_path = dir_path.join(format!("{hash}.json.tmp"));
                let client_expires = chrono::Utc::now() + chrono::Duration::days(90);
                let client_reg_data = serde_json::json!({
                    "clientId": cid,
                    "clientSecret": csec,
                    "expiresAt": client_expires.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                });
                let client_reg_content = serde_json::to_string_pretty(&client_reg_data)
                    .map_err(|e| format!("Failed to serialize client registration: {e}"))?;

                // 安全检查：确保目标文件不是符号链接
                assert_not_symlink(&client_reg_path)?;

                // 安全检查：如果临时文件已存在，确保不是符号链接后删除
                if client_reg_temp_path.exists() {
                    assert_not_symlink(&client_reg_temp_path)?;
                    std::fs::remove_file(&client_reg_temp_path)
                        .map_err(|e| format!("Failed to remove existing client temp file: {e}"))?;
                }

                // 使用 OpenOptions 安全写入（create_new 确保不跟随符号链接）
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&client_reg_temp_path)
                    .map_err(|e| format!("Failed to create client temp file: {e}"))?;
                file.write_all(client_reg_content.as_bytes())
                    .map_err(|e| format!("Failed to write client temp file: {e}"))?;
                drop(file);

                std::fs::rename(&client_reg_temp_path, &client_reg_path)
                    .map_err(|e| format!("Failed to rename client registration: {e}"))?;

                // 设置文件权限为 0600（仅 Unix 系统）
                set_file_permissions(&client_reg_path).ok();
            } else {
                // 防御兜底：上方已提前校验，正常不可达。
                return Err("IdC 账号必须提供 client_id 和 client_secret".to_string());
            }
        }

        // 第二步：最后写 kiro-auth-token.json（触发器），让 IDE 感知到登录态变化。
        // 安全检查：确保目标文件不是符号链接
        assert_not_symlink(&file_path)?;

        // 原子写入：先写临时文件，再 rename
        let temp_file_path = dir_path.join("kiro-auth-token.json.tmp");

        // 安全检查：如果临时文件已存在，确保不是符号链接后删除
        if temp_file_path.exists() {
            assert_not_symlink(&temp_file_path)?;
            std::fs::remove_file(&temp_file_path)
                .map_err(|e| format!("Failed to remove existing temp file: {e}"))?;
        }

        // 使用 OpenOptions 安全写入（create_new 确保不跟随符号链接）
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_file_path)
            .map_err(|e| format!("Failed to create temp file: {e}"))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {e}"))?;
        drop(file);

        std::fs::rename(&temp_file_path, &file_path)
            .map_err(|e| format!("Failed to rename file: {e}"))?;

        // 强制更新 mtime，让 IDE 的 fs.watchFile（基于 mtime polling）一定能感知到变化。
        // Windows 下 rename 有时不会改变 mtime（NTFS 时间戳精度 + 短间隔写入），
        // 而 Kiro IDE 用 fs.watchFile 以 5 秒 polling 监听 token 文件，mtime 不变就不触发。
        // 后果：IDE 内存里继续缓存旧账号 token → AWS 返回 401 Invalid token。
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .open(&file_path)
            .and_then(|f| f.set_modified(std::time::SystemTime::now()));

        // 设置文件权限为 0600（仅 Unix 系统）
        set_file_permissions(&file_path).ok();

        // 对齐 IDE：清理「被切走账号」遗留的旧 {clientIdHash}.json，避免客户端注册堆积。
        // 仅当旧 hash 存在、且与本次写入的新 hash 不同才删（切到同一 client 的账号时新旧
        // 相同，上面已覆盖写入，不能删）。social 账号 idc_hash 为 None，此时旧 IdC hash 必删。
        // 安全：previous_idc_hash 来自切换前 Kiro 自己的 token 文件，不扫描目录，
        // 不碰 AWS CLI v2 的 SSO 缓存；删除前做 hash 合法性 + symlink 检查；失败仅告警。
        if let Some(old_hash) = previous_idc_hash {
            let new_hash = idc_hash.as_deref();
            if Some(old_hash.as_str()) != new_hash {
                if old_hash
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                {
                    let old_reg_path = dir_path.join(format!("{old_hash}.json"));
                    if old_reg_path.exists() {
                        match assert_not_symlink(&old_reg_path) {
                            Ok(()) => {
                                if let Err(e) = std::fs::remove_file(&old_reg_path) {
                                    log::warn!(
                                        "[switch] 清理旧客户端注册失败 {}: {e}",
                                        old_reg_path.display()
                                    );
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    "[switch] 旧客户端注册未通过 symlink 检查，跳过删除: {e}"
                                );
                            }
                        }
                    }
                } else {
                    log::warn!("[switch] 检测到非法的旧 clientIdHash，跳过清理: {old_hash}");
                }
            }
        }

        Ok(SwitchAccountResult {
            success: true,
            message: format!("Switched to {provider} ({auth_method}) account"),
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 退出当前 Kiro IDE 登录（删除本地 token 文件，账号仍保留在 KAM 列表中）
///
/// 这是 `switch_kiro_account` 的逆操作：switch 写入
/// `~/.aws/sso/cache/kiro-auth-token.json`，logout 删除它。删除后 Kiro IDE 不再处于
/// 登录态——`get_kiro_local_token` 返回 None，前端的 LIVE 标识随之消失，但账号记录依旧
/// 留在 KAM 列表里，可随时再次登录。
///
/// 对齐 Kiro IDE 源码 `AuthProviderService.logout()` 的行为：
/// 1. `storage.clearToken()` —— 删除 `kiro-auth-token.json`（会话凭证）；
/// 2. IdC（BuilderId/Enterprise）provider 的 `logout()` 会
///    `deleteClientRegistration(clientIdHash)` —— 即删除 `{clientIdHash}.json`
///    （客户端注册）。Social provider 不删本地文件（它走远程 SSO logout 吊销
///    refreshToken，本函数不涉及网络，仅做本地退出登录）。
///
/// 因此本函数：删 `kiro-auth-token.json`；若该 token 是 IdC，再删对应的
/// `{clientIdHash}.json`。这样本地缓存状态与 IDE 完整退出登录一致。
///
/// 文件不存在视为已处于退出登录态，返回成功（幂等）。
#[tauri::command]
pub async fn logout_kiro_account() -> Result<SwitchAccountResult, String> {
    tokio::task::spawn_blocking(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory")?;

        let cache_dir = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");
        let file_path = cache_dir.join("kiro-auth-token.json");

        // 文件不存在 = 本来就没登录，幂等返回成功
        if !file_path.exists() {
            return Ok(SwitchAccountResult {
                success: true,
                message: "Already logged out".to_string(),
            });
        }

        // 删除前先读出 token，拿到 IdC 的 clientIdHash 以便对齐 IDE 删除客户端注册。
        // 解析失败不阻断退出登录——会话凭证该删还是删，只是跳过 {hash}.json 清理。
        let idc_client_id_hash = std::fs::read_to_string(&file_path)
            .ok()
            .and_then(|content| serde_json::from_str::<KiroLocalToken>(&content).ok())
            .filter(|tok| tok.auth_method.as_deref() == Some("IdC"))
            .and_then(|tok| tok.client_id_hash)
            .filter(|h| !h.trim().is_empty());

        // 安全检查：确保目标不是符号链接，避免误删链接指向的真实文件
        assert_not_symlink(&file_path)?;

        std::fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to remove token file: {e}"))?;

        // IdC 账号：对齐 IDE `deleteClientRegistration`，删除 {clientIdHash}.json。
        if let Some(hash) = idc_client_id_hash {
            // 防路径遍历：仅允许字母、数字、下划线、连字符（与 get_client_registration 一致）。
            if hash
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                let client_reg_path = cache_dir.join(format!("{hash}.json"));
                if client_reg_path.exists() {
                    // symlink 检查后再删，避免误删链接目标。删除失败仅告警，不影响退出登录结果。
                    match assert_not_symlink(&client_reg_path) {
                        Ok(()) => {
                            if let Err(e) = std::fs::remove_file(&client_reg_path) {
                                log::warn!(
                                    "[logout] 删除客户端注册文件失败 {}: {e}",
                                    client_reg_path.display()
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("[logout] 客户端注册文件未通过 symlink 检查，跳过删除: {e}");
                        }
                    }
                }
            } else {
                log::warn!("[logout] 检测到非法的 clientIdHash，跳过删除客户端注册: {hash}");
            }
        }

        Ok(SwitchAccountResult {
            success: true,
            message: "Logged out from Kiro IDE".to_string(),
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// IDE 安装检测结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdeInstallationInfo {
    pub ide_installed: bool,
    pub ide_path: Option<String>,
    pub ide_executable_exists: bool,
    pub config_dir_exists: bool,
    pub error_message: Option<String>,
}

/// 检测 Kiro IDE 是否安装
#[tauri::command]
pub async fn check_ide_installation() -> IdeInstallationInfo {
    tokio::task::spawn_blocking(|| {
        let (ide_path, ide_exists) = detect_kiro_ide_executable();

        // 检查配置目录是否存在
        let config_exists = check_kiro_config_dir();

        let ide_installed = ide_exists && config_exists;

        // 生成详细的错误提示
        let error_message = if !ide_installed {
            if !ide_exists && !config_exists {
                Some("未检测到默认路径的 Kiro IDE 可执行文件和配置文件。\n\n请先安装并登录 Kiro IDE，或在「设置」→「通用」中配置「自定义 Kiro IDE 安装路径」。".to_string())
            } else if !ide_exists {
                Some("未检测到默认路径的 Kiro IDE 可执行文件。\n\n请检查 IDE 是否已安装，或在「设置」→「通用」中配置「自定义 Kiro IDE 安装路径」。".to_string())
            } else if !config_exists {
                Some("Kiro IDE 已安装，但尚未首次登录。\n\n请先在 Kiro IDE 中完成首次登录后再使用切换功能。".to_string())
            } else {
                None
            }
        } else {
            None
        };

        IdeInstallationInfo {
            ide_installed,
            ide_path,
            ide_executable_exists: ide_exists,
            config_dir_exists: config_exists,
            error_message,
        }
    })
    .await
    .unwrap_or(IdeInstallationInfo {
        ide_installed: false,
        ide_path: None,
        ide_executable_exists: false,
        config_dir_exists: false,
        error_message: Some("检测 IDE 安装状态时发生错误".to_string()),
    })
}

/// 检查 Kiro IDE 配置文件是否存在且包含有效 token
fn check_kiro_config_dir() -> bool {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"));

    if let Ok(home_dir) = home {
        let cache_dir = std::path::Path::new(&home_dir)
            .join(".aws")
            .join("sso")
            .join("cache");

        let token_file = cache_dir.join("kiro-auth-token.json");

        // 1. 检查文件是否存在
        if !token_file.exists() {
            return false;
        }

        // 2. 读取并解析文件内容
        if let Ok(content) = std::fs::read_to_string(&token_file) {
            if let Ok(token_data) = serde_json::from_str::<KiroLocalToken>(&content) {
                // 3. 验证必须有 access_token 和 refresh_token
                if token_data.access_token.is_none() || token_data.refresh_token.is_none() {
                    return false;
                }

                // 4. 验证必须有 auth_method
                let auth_method = match token_data.auth_method.as_deref() {
                    Some(method) => method,
                    None => return false,
                };

                // 5. 验证必须有 provider
                if token_data.provider.is_none() {
                    return false;
                }

                // 6. 根据 auth_method 验证特定字段
                match auth_method {
                    "social" => {
                        // Social 账号需要 profileArn
                        token_data.profile_arn.is_some()
                    }
                    "IdC" => {
                        // IdC 账号 (BuilderId/Enterprise) 需要 clientIdHash 和 region
                        token_data.client_id_hash.is_some() && token_data.region.is_some()
                    }
                    _ => false, // 未知的 auth_method
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// 检测 IDE 可执行文件
fn detect_kiro_ide_executable() -> (Option<String>, bool) {
    let candidates = get_kiro_ide_paths();
    for path in candidates {
        if path.exists() {
            return (Some(path.to_string_lossy().to_string()), true);
        }
    }
    (None, false)
}

/// 检测配置文件是否存在（用于切换账号前验证）
#[tauri::command]
pub async fn check_kiro_config_files(
    auth_method: String,
    client_id_hash: Option<String>,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "无法获取用户目录".to_string())?;

        let cache_dir = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");

        // 检查主 token 文件
        let token_file = cache_dir.join("kiro-auth-token.json");
        if !token_file.exists() {
            return Ok(false);
        }

        // 如果是 IdC 账号，还需检查 client registration 文件
        if auth_method == "idc" {
            if let Some(hash) = client_id_hash {
                let client_file = cache_dir.join(format!("{}.json", hash));
                if !client_file.exists() {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 获取 Kiro IDE 候选路径
pub fn get_kiro_ide_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // 1. 优先检查自定义路径（如果用户在设置中配置了）
    if let Ok(settings) = crate::commands::app_settings_cmd::get_app_settings_inner() {
        if let Some(custom_path) = settings.custom_kiro_path {
            let path_buf = std::path::PathBuf::from(&custom_path);
            paths.push(path_buf);
        }
    }

    // 2. 如果没有自定义路径，检查默认路径
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            paths.push(
                std::path::PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("Kiro")
                    .join("Kiro.exe"),
            );
        }
    } else if cfg!(target_os = "macos") {
        // macOS: Kiro.app 安装在 /Applications
        paths.push(std::path::PathBuf::from("/Applications/Kiro.app"));
    } else {
        // Linux: 可能在多个位置
        paths.push(std::path::PathBuf::from("/usr/bin/kiro"));

        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                std::path::PathBuf::from(&home)
                    .join(".local")
                    .join("bin")
                    .join("kiro"),
            );
        }
    }

    paths
}
