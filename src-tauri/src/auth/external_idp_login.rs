// External IdP（企业 SSO / Entra ID / Azure AD）交互登录 —— portal descriptor 两段式
//
// 参考 kiro.rs-admin 的 social.rs/admin/service.rs 与 portal vendor.js 的 Sxn 构造，
// 以及 portal 前端对 Start URL 的判别逻辑（ahe/zv：kiro.dev 域 → KiroStartUrl）。
//
// 触发条件：Enterprise 登录的 Start URL 是 Kiro 门户地址（kiro.dev/start/k-xxx 等），
// 而非 AWS SSO 门户（*.awsapps.com/start）。此时用户身份在外部 IdP（Entra/Okta），
// 流程走 portal 统一入口：
//   1. 打开 app.kiro.dev/signin，用户选 "Your organization" 输入工作邮箱或该 Start URL
//   2. portal 对组织做 OIDC discovery 后向 redirect_uri 回传 descriptor（无 code）：
//      .../signin/callback?login_option=external_idp&issuer_url=...&client_id=...
//      &scopes=...&login_hint=...&state=...
//   3. 本地对 issuer_url 做 OIDC discovery，生成第二段 PKCE，打开 IdP 授权页
//   4. IdP 授权完成回调 code（/oauth/callback），POST 微软 token_endpoint
//      （form 编码，公共客户端无 clientSecret）换 token
// 产物为 external_idp 凭据：刷新走微软 token_endpoint（providers/external_idp.rs），
// 数据面带 TokenType: EXTERNAL_IDP 头。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::providers::external_idp::validate_microsoft_token_endpoint;

const KIRO_PORTAL_URL: &str = "https://app.kiro.dev";
/// 两段共享的总超时（对齐 idc.rs 的 10 分钟）
const STAGE_TIMEOUT: Duration = Duration::from_secs(600);

/// 判别 Start URL 是否为 Kiro 门户地址（对齐 portal vendor.js 的 zv 域名单：
/// kiro.dev 与 gov/scloud 形态）。AWS SSO 门户（*.awsapps.com）不在此列。
pub fn is_kiro_portal_start_url(url: &str) -> bool {
    let t = url.trim().to_lowercase();
    let t = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(&t);
    let host = t.split(['/', '?', '#']).next().unwrap_or("");
    host == "kiro.dev"
        || host == "app.kiro.dev"
        || host.ends_with(".app.kiro.dev")
        || host == "kiro.awshome.ic.gov"
        || host == "kiro.awshome.scloud"
        || host == "kiro.awshome.hci.ic.gov"
}

/// portal 第一段回传的企业 SSO 描述符
#[derive(Debug, Clone)]
struct ExternalIdpDescriptor {
    issuer_url: String,
    client_id: String,
    scopes: String,
    login_hint: Option<String>,
    state: String,
}

#[derive(Debug, Deserialize, Default)]
struct OidcDocument {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Deserialize, Default)]
struct IdpTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    id_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

fn percent_decode(value: &str) -> String {
    let with_space = value.replace('+', " ");
    urlencoding::decode(&with_space)
        .map(|s| s.into_owned())
        .unwrap_or(with_space)
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let key = it.next()?.to_string();
            let value = it.next().map(percent_decode).unwrap_or_default();
            Some((key, value))
        })
        .collect()
}

fn parse_descriptor(params: &HashMap<String, String>) -> Result<ExternalIdpDescriptor, String> {
    let issuer_url = params
        .get("issuer_url")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or("企业 SSO 回调缺少 issuer_url")?;
    let client_id = params
        .get("client_id")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or("企业 SSO 回调缺少 client_id")?;
    Ok(ExternalIdpDescriptor {
        issuer_url,
        client_id,
        scopes: params.get("scopes").cloned().unwrap_or_default(),
        login_hint: params
            .get("login_hint")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        state: params.get("state").cloned().unwrap_or_default(),
    })
}

fn respond(request: tiny_http::Request, ok: bool, title: &str, body: &str) {
    let html = format!(
        "<html><head><meta charset='utf-8'><title>{title}</title></head>\
         <body style='font-family:sans-serif;text-align:center;padding:60px'>\
         <h2>{title}</h2><p>{body}</p></body></html>"
    );
    let resp = tiny_http::Response::from_string(html)
        .with_status_code(if ok { 200 } else { 400 })
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .expect("valid content-type header"),
        );
    // tiny_http::Request::respond 消费 self，调用方按值传入
    let _ = request.respond(resp);
}

/// 两阶段监听的中间表示
enum StageEvent {
    /// portal 的 IdC/social 轨回调（带 code）——descriptor 流程中属于误入
    Code { path: String },
    Descriptor(ExternalIdpDescriptor),
    AuthorizationCode { code: String, state: String },
}

/// 从 tiny_http 请求解析阶段事件
fn classify_request(url: &str) -> Result<StageEvent, String> {
    let (path, query) = match url.split_once('?') {
        Some((p, q)) => (p, q),
        None => (url, ""),
    };
    let params = parse_query(query);

    if let Some(error) = params.get("error") {
        let desc = params.get("error_description").cloned().unwrap_or_default();
        return Err(format!("{error}: {desc}"));
    }

    let has_code = params
        .get("code")
        .map(|c| !c.trim().is_empty())
        .unwrap_or(false);

    if has_code {
        return Ok(StageEvent::AuthorizationCode {
            code: params.get("code").cloned().unwrap_or_default(),
            state: params.get("state").cloned().unwrap_or_default(),
        });
    }

    // 无 code：portal descriptor（路径可能是 /signin/callback，
    // 或 Sxn 拼接出的 /oauth/callback/signin/callback——统一按路径包含判定）
    if path.contains("signin/callback")
        || params
            .get("login_option")
            .map(|v| v.eq_ignore_ascii_case("external_idp"))
            .unwrap_or(false)
        || params.contains_key("issuer_url")
    {
        let descriptor = parse_descriptor(&params)?;
        return Ok(StageEvent::Descriptor(descriptor));
    }

    Ok(StageEvent::Code {
        path: path.to_string(),
    })
}

/// 阶段等待：在 tiny_http server 上循环 try_recv，
/// descriptor 阶段等到 Descriptor（Code 视为误入并报错），
/// code 阶段等到 AuthorizationCode（Descriptor 忽略继续等）。
fn wait_for_stage(
    server: &Arc<tiny_http::Server>,
    expected_state: &str,
    want_descriptor: bool,
    cancelled: &Arc<AtomicBool>,
) -> Result<StageEvent, String> {
    let deadline = Instant::now() + STAGE_TIMEOUT;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err("登录已取消".to_string());
        }
        if Instant::now() >= deadline {
            return Err("授权超时（10 分钟）".to_string());
        }

        match server.recv_timeout(Duration::from_millis(200)) {
            Ok(Some(request)) => {
                let url = request.url().to_string();
                match classify_request(&url) {
                    Err(msg) => {
                        respond(
                            request,
                            false,
                            "登录失败",
                            &format!("{msg}。请关闭此窗口并重试。"),
                        );
                        return Err(msg);
                    }
                    Ok(StageEvent::Descriptor(d)) if want_descriptor => {
                        // state 校验（portal 回传发起时的 state）
                        if !d.state.is_empty() && d.state != expected_state {
                            respond(
                                request,
                                false,
                                "登录失败",
                                "企业 SSO 回调 state 不匹配，请重新发起登录。",
                            );
                            return Err("企业 SSO 回调 state 不匹配".to_string());
                        }
                        respond(
                            request,
                            true,
                            "继续登录",
                            "已收到企业 SSO 中间链接，即将打开 IdP 登录页。此窗口可以关闭。",
                        );
                        return Ok(StageEvent::Descriptor(d));
                    }
                    // code 阶段误入 descriptor（用户中途又走了 portal 流程）——忽略继续等
                    Ok(StageEvent::Descriptor(_)) => {
                        respond(
                            request,
                            true,
                            "继续登录",
                            "回调阶段不符，已忽略。请继续在浏览器完成 IdP 登录。",
                        );
                        continue;
                    }
                    Ok(StageEvent::AuthorizationCode { code, state }) => {
                        // 带 code 的回调只属于 code 阶段；descriptor 阶段收到即误入
                        //（用户在 portal 完成了 Google/Github/BuilderId 登录而非 Your organization）
                        if want_descriptor {
                            respond(
                                request,
                                false,
                                "登录失败",
                                "收到的是普通账号授权回调，而非企业 SSO 中间链接。\
                                 请确认在 portal 选择 \"Your organization\" 并输入工作邮箱或 \
                                 Kiro Start URL；其他登录方式请使用桌面应用内对应的按钮。",
                            );
                            return Err(
                                "收到非企业 SSO 回调（用户未走 Your organization 流程）".to_string(),
                            );
                        }
                        if state != expected_state {
                            respond(
                                request,
                                false,
                                "登录失败",
                                "IdP 回调 state 不匹配，请重新发起登录。",
                            );
                            return Err("External IdP 回调 state 不匹配".to_string());
                        }
                        respond(
                            request,
                            true,
                            "登录成功",
                            "Token 已获取，请返回 Kiro Account Manager。此窗口可以关闭。",
                        );
                        return Ok(StageEvent::AuthorizationCode { code, state });
                    }
                    // 误入：descriptor 阶段收到无 code 且非 descriptor 的回调
                    Ok(StageEvent::Code { path }) if want_descriptor => {
                        respond(
                            request,
                            false,
                            "登录失败",
                            &format!(
                                "收到的是账号授权回调（{path}），而非企业 SSO 中间链接。\
                                 请确认在 portal 选择 \"Your organization\" 并输入工作邮箱或 \
                                 Kiro Start URL；其他登录方式请使用桌面应用内对应的按钮。"
                            ),
                        );
                        return Err(
                            "收到非企业 SSO 回调（用户未走 Your organization 流程）".to_string(),
                        );
                    }
                    // code 阶段重复 code 回调（state 已错或重复提交）——忽略继续等
                    Ok(StageEvent::Code { .. }) => {
                        respond(request, true, "继续登录", "回调阶段不符，已忽略。");
                        continue;
                    }
                }
            }
            Ok(None) => continue, // 超时轮询
            Err(e) => return Err(format!("回调服务器接收失败: {e}")),
        }
    }
}

async fn oidc_discover(issuer_url: &str) -> Result<OidcDocument, String> {
    validate_microsoft_token_endpoint(issuer_url)
        .map_err(|e| format!("企业 SSO issuer 被拒绝: {e}"))?;
    let doc_url = format!(
        "{}/.well-known/openid-configuration",
        issuer_url.trim_end_matches('/')
    );
    let client = crate::clients::http_client::build_http_client()?;
    let resp = client
        .get(&doc_url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("OIDC discovery 请求失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("OIDC discovery 失败: HTTP {status}"));
    }
    let doc: OidcDocument =
        serde_json::from_str(&body).map_err(|e| format!("解析 OIDC discovery 失败: {e}"))?;
    if doc.authorization_endpoint.trim().is_empty() || doc.token_endpoint.trim().is_empty() {
        return Err("OIDC discovery 缺少 authorization_endpoint 或 token_endpoint".to_string());
    }
    validate_microsoft_token_endpoint(&doc.authorization_endpoint)
        .map_err(|e| format!("discovered authorization_endpoint 被拒绝: {e}"))?;
    validate_microsoft_token_endpoint(&doc.token_endpoint)
        .map_err(|e| format!("discovered token_endpoint 被拒绝: {e}"))?;
    Ok(doc)
}

/// 从微软 JWT access token 提取邮箱（email / preferred_username / upn）
pub fn extract_email_from_jwt(access_token: &str) -> Option<String> {
    use base64::{engine::general_purpose, Engine as _};
    let payload = access_token.split('.').nth(1)?;
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    for claim in ["email", "preferred_username", "upn"] {
        if let Some(value) = claims
            .get(claim)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return Some(value.to_string());
        }
    }
    None
}

/// Enterprise + Kiro 门户 Start URL 的完整两段式登录。
/// 调用方（IdcProvider::login）已设置 pending login 与 cancelled 标志。
pub async fn login_external_idp_via_portal(
    start_url: String,
    cancelled: Arc<AtomicBool>,
) -> Result<super::providers::AuthResult, String> {
    // 本地回调服务器（动态端口；AWS 对 loopback 回调按 RFC 8252 允许端口通配）
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| format!("无法启动本地回调服务器: {e}"))?;
    let server = Arc::new(server);
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .unwrap_or_default();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");

    // ---- 第一段：portal 统一入口 ----
    let portal_state = uuid::Uuid::new_v4().to_string();
    let code_verifier = super::auth_social::generate_code_verifier_social();
    let code_challenge = super::auth_social::generate_code_challenge_social(&code_verifier);
    let portal_url = format!(
        "{KIRO_PORTAL_URL}/signin?state={}&code_challenge={}&code_challenge_method=S256&redirect_uri={}&redirect_from=KiroIDE",
        urlencoding::encode(&portal_state),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(&redirect_uri),
    );
    crate::utils::browser::open_browser(&portal_url)?;

    let descriptor = match {
        let server = server.clone();
        let cancelled = cancelled.clone();
        let expected = portal_state.clone();
        tokio::task::spawn_blocking(move || {
            wait_for_stage(&server, &expected, true, &cancelled)
        })
        .await
        .map_err(|_| "descriptor 等待任务失败".to_string())??
    } {
        StageEvent::Descriptor(d) => d,
        _ => return Err("内部错误：预期收到企业 SSO descriptor".to_string()),
    };

    let ExternalIdpDescriptor {
        issuer_url,
        client_id,
        scopes,
        login_hint,
        ..
    } = descriptor;

    // ---- OIDC discovery ----
    let doc = oidc_discover(&issuer_url).await?;

    // ---- 第二段：IdP 授权页 ----
    let idp_state = uuid::Uuid::new_v4().to_string();
    let code_verifier2 = super::auth_social::generate_code_verifier_social();
    let code_challenge2 = super::auth_social::generate_code_challenge_social(&code_verifier2);

    let mut query: Vec<(String, String)> = vec![
        ("client_id".to_string(), client_id.clone()),
        ("response_type".to_string(), "code".to_string()),
        ("redirect_uri".to_string(), redirect_uri.clone()),
        ("scope".to_string(), scopes.clone()),
        ("code_challenge".to_string(), code_challenge2),
        ("code_challenge_method".to_string(), "S256".to_string()),
        ("response_mode".to_string(), "query".to_string()),
        ("state".to_string(), idp_state.clone()),
    ];
    if let Some(hint) = login_hint.as_deref().filter(|v| !v.trim().is_empty()) {
        query.push(("login_hint".to_string(), hint.to_string()));
    }
    let authorize_url = format!(
        "{}?{}",
        doc.authorization_endpoint.trim(),
        query
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    );
    crate::utils::browser::open_browser(&authorize_url)?;

    // ---- 等 code ----
    let code = {
        let server = server.clone();
        let cancelled = cancelled.clone();
        let expected = idp_state.clone();
        tokio::task::spawn_blocking(move || {
            wait_for_stage(&server, &expected, false, &cancelled)
        })
        .await
        .map_err(|_| "code 等待任务失败".to_string())??
    };
    let StageEvent::AuthorizationCode { code, .. } = code else {
        return Err("内部错误：预期收到 authorization code".to_string());
    };

    // ---- code 换 token（form 编码，公共客户端） ----
    let http = crate::clients::http_client::build_http_client()?;
    let mut form: Vec<(&str, &str)> = vec![
        ("client_id", client_id.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code.trim()),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", code_verifier2.as_str()),
    ];
    if !scopes.trim().is_empty() {
        form.push(("scope", scopes.trim()));
    }
    let resp = http
        .post(doc.token_endpoint.trim())
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("External IdP token 交换请求失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let data: IdpTokenResponse = serde_json::from_str(&body).unwrap_or_default();
    let access_token = data.access_token.filter(|v| !v.trim().is_empty());
    let refresh_token = data.refresh_token.filter(|v| !v.trim().is_empty());
    let (access_token, refresh_token) = match (status.is_success(), access_token, refresh_token) {
        (true, Some(a), Some(r)) => (a, r),
        _ => {
            return if let Some(err) = data.error {
                Err(format!(
                    "External IdP token 交换失败 {status}: {err} {}",
                    data.error_description.unwrap_or_default()
                ))
            } else {
                Err(format!("External IdP token 交换失败 {status}: {body}"))
            };
        }
    };

    let expires_at = chrono::Local::now()
        + chrono::Duration::seconds(data.expires_in.unwrap_or(3600).max(0));

    // email 优先取 id_token（微软标准响应），兜底 access_token JWT——
    // login_idc 落库时经 extract_user_info_from_id_token 自行解析
    let id_token = data.id_token.filter(|v| !v.trim().is_empty());
    let _ = extract_email_from_jwt(id_token.as_deref().unwrap_or(&access_token));

    Ok(super::providers::AuthResult {
        access_token,
        refresh_token,
        expires_at: expires_at.format("%Y/%m/%d %H:%M:%S").to_string(),
        expires_in: data.expires_in.unwrap_or(3600).max(0),
        provider: "Enterprise".to_string(),
        auth_method: "external_idp".to_string(),
        token_type: Some("Bearer".to_string()),
        id_token,
        region: None,
        client_id: Some(client_id),
        client_secret: None,
        client_id_hash: None,
        sso_session_id: None,
        start_url: Some(start_url),
        profile_arn: None,
        machine_id: None,
        token_endpoint: Some(doc.token_endpoint),
        issuer_url: Some(issuer_url),
        idp_scopes: Some(scopes),
    })
}
