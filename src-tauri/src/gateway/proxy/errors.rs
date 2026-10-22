//! 上游错误映射：状态码与错误类型归一化、错误信息提取与脱敏。

use super::*;

pub fn gateway_error_response(
    format: ResponseFormat,
    status: StatusCode,
    error_type: &str,
    message: &str,
    response_body: Option<&str>,
) -> Response {
    // 如果有原始响应体且是有效的 JSON，直接透传
    if let Some(body_str) = response_body {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body_str) {
            return (status, Json(json_value)).into_response();
        }
    }

    // 否则使用构造的错误响应
    let body = build_gateway_error_body(format, status, error_type, message);
    (status, Json(body)).into_response()
}

pub fn map_upstream_error(status: StatusCode, body: &str) -> (StatusCode, &'static str, String) {
    let sanitized = sanitize_error(&extract_error_message(body));
    let explicit_error_type = extract_error_type(body);
    let text = body.to_lowercase();

    // 检测封禁错误（403 + TEMPORARILY_SUSPENDED 或 AccessDeniedException + TemporarilySuspended）
    let is_banned = status == StatusCode::FORBIDDEN
        && (body.contains("TEMPORARILY_SUSPENDED")
            || (body.contains("AccessDeniedException") && body.contains("TemporarilySuspended")));

    // 检测token失效错误（403 + bearer token invalid/expired）
    let is_token_invalid = status == StatusCode::FORBIDDEN
        && (text.contains("bearer token") || text.contains("bearer_token"))
        && (text.contains("invalid") || text.contains("expired"));

    let mapped_status = if status == StatusCode::BAD_GATEWAY || status == StatusCode::OK {
        if explicit_error_type == Some("authentication_error") {
            StatusCode::UNAUTHORIZED
        } else if explicit_error_type == Some("permission_error") {
            StatusCode::FORBIDDEN
        } else if explicit_error_type == Some("rate_limit_error") {
            StatusCode::TOO_MANY_REQUESTS
        } else if explicit_error_type == Some("invalid_request_error") {
            StatusCode::BAD_REQUEST
        } else if text.contains("throttlingexception")
            || text.contains("servicequotaexceededexception")
        {
            StatusCode::TOO_MANY_REQUESTS
        } else if text.contains("accessdeniedexception") {
            StatusCode::FORBIDDEN
        } else if text.contains("validationexception") {
            StatusCode::BAD_REQUEST
        } else if text.contains("serviceunavailableexception") {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::BAD_GATEWAY
        }
    } else {
        status
    };
    // 根据检测结果返回特殊的error_type和message
    let (error_type, message) = if is_banned {
        // 对于封禁错误，返回以 BANNED: 开头的消息，以便前端可以识别
        ("account_banned_error", format!("BANNED: {}", sanitized))
    } else if is_token_invalid {
        ("token_expired_error", sanitized)
    } else {
        let error_type = explicit_error_type.unwrap_or(match mapped_status {
            StatusCode::UNAUTHORIZED => "authentication_error",
            StatusCode::FORBIDDEN => "permission_error",
            StatusCode::PAYMENT_REQUIRED => "insufficient_quota",
            StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
            StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND | StatusCode::CONFLICT => {
                "invalid_request_error"
            }
            _ => "api_error",
        });
        (error_type, sanitized)
    };

    (mapped_status, error_type, message)
}

pub fn extract_error_type(body: &str) -> Option<&'static str> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let raw = value
        .pointer("/error/type")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/type").and_then(Value::as_str))?;

    match raw {
        "authentication_error" => Some("authentication_error"),
        "permission_error" => Some("permission_error"),
        "insufficient_quota" => Some("insufficient_quota"),
        "rate_limit_error" => Some("rate_limit_error"),
        "invalid_request_error" => Some("invalid_request_error"),
        "api_error" => Some("api_error"),
        _ => None,
    }
}

pub fn detect_upstream_error_body(body: &str) -> Option<(StatusCode, &'static str, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    let object = value.as_object()?;
    let has_error_container = object.get("error").is_some();
    let has_error_metadata = object.get("__type").and_then(Value::as_str).is_some()
        || object.get("errorCode").and_then(Value::as_str).is_some()
        || object.get("Message").and_then(Value::as_str).is_some();
    let has_message_only_error = object.get("message").and_then(Value::as_str).is_some()
        && object.get("content").is_none()
        && object.get("output").is_none()
        && object.get("choices").is_none()
        && object.get("results").is_none();

    if has_error_container || has_error_metadata || has_message_only_error {
        Some(map_upstream_error(StatusCode::OK, trimmed))
    } else {
        None
    }
}

pub fn extract_error_message(body: &str) -> String {
    if body.trim().is_empty() {
        return "上游返回空错误响应".to_string();
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        for pointer in [
            "/message",
            "/Message",
            "/error/message",
            "/reason",
            "/__type",
            "/errorCode",
        ] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
                return text.to_string();
            }
        }
    }
    body.to_string()
}

/// 安全截断字符串到指定字节数，确保不会切到 UTF-8 多字节字符中间
pub fn safe_truncate(s: &str, max_bytes: usize) -> usize {
    if s.len() <= max_bytes {
        return s.len();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

pub fn sanitize_error(message: &str) -> String {
    let mut sanitized = message.to_string();
    for pattern in [
        r"Bearer\s+[A-Za-z0-9._\-]+",
        r#""accessToken"\s*:\s*"[^"]+""#,
        r#""refreshToken"\s*:\s*"[^"]+""#,
        r#""clientSecret"\s*:\s*"[^"]+""#,
        r#"sk-[A-Za-z0-9]+"#,
    ] {
        if let Ok(regex) = Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }
    sanitized
}

/// 网关自身的结构化错误。
///
/// 取代原先 `"__400__中文描述"` 这种把状态码编码进 message 前缀的做法：
/// 调用方不得不 `starts_with("__402__")` 解析字符串，新增一种错误就要多一个分支，/// 且前缀一旦写错就静默退化成 401。现在状态码与错误类型是类型的一部分。
pub enum GatewayError {
    /// 请求指定的账号不在当前路由可用集合内
    PreferredAccountUnavailable { account_id: String },
    /// 当前配置下没有任何可用账号（全部禁用/封禁/超配额）
    NoAvailableAccount,
    /// 有账号列表，但负载均衡器没能选出任何一个（如全被健康检查滤掉）
    BalancerNoSelection,
}

impl GatewayError {
    /// HTTP 状态码。
    ///
    /// 「挑不出账号」是**服务端暂时不可用**，不是客户端欠费：
    /// 用 402/`insufficient_quota` 会让客户端以为该去充值，
    /// 而它实际该做的是退避重试。上游真实 402 仍走 `map_upstream_error`
    /// 原样透传，与此处无关。
    pub fn status(&self) -> StatusCode {
        match self {
            Self::PreferredAccountUnavailable { .. } => StatusCode::BAD_REQUEST,
            Self::NoAvailableAccount => StatusCode::SERVICE_UNAVAILABLE,
            Self::BalancerNoSelection => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// 错误类型标识（写入响应体与日志，客户端可按它分支）。
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::PreferredAccountUnavailable { .. } => "invalid_request_error",
            Self::NoAvailableAccount => "unavailable",
            Self::BalancerNoSelection => "unavailable",
        }
    }

    /// 用户可见描述。中文，后续迁 i18n。
    pub fn message(&self) -> String {
        match self {
            Self::PreferredAccountUnavailable { account_id } => format!(
                "指定账号 {account_id} 不在当前路由可用账号中，请检查账号模式/分组/账号池配置"
            ),
            Self::NoAvailableAccount => "未找到符合 kiro2api 配置的可用账号".to_string(),
            Self::BalancerNoSelection => "负载均衡器未能选择可用账号".to_string(),
        }
    }
}

/// 把错误 message 还原为结构化错误。
///
/// 迁移期专用：`auth.rs` 目前仍返回 `String`（其调用方有按字符串判断限流的逻辑），
/// 这里负责把 `__NNN__` 前缀还原成类型，让消费端不再自己解析。
/// 待调用方全部改用类型传错后，此函数与 `Display` 里的前缀都可删除。
pub fn classify_gateway_error(message: &str) -> Option<GatewayError> {
    if let Some(rest) = message.strip_prefix("__400__") {
        // 形如 `__400__指定账号 <id> 不在当前路由可用账号中，...`
        let account_id = rest
            .strip_prefix("指定账号 ")
            .and_then(|s| s.split(" 不在").next())
            .unwrap_or("")
            .to_string();
        return Some(GatewayError::PreferredAccountUnavailable { account_id });
    }
    if message.starts_with("__503__") {
        let rest = &message["__503__".len()..];
        if rest.starts_with("未找到符合") {
            return Some(GatewayError::NoAvailableAccount);
        }
        return Some(GatewayError::BalancerNoSelection);
    }
    None
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 保留 `__NNN__` 前缀：调用方现有按字符串分支的逻辑
        // （如 `message.contains("429")` 判限流）仍能工作，迁移期间不改变行为。
        // 全部调用方改用类型传错后，此前缀可删。
        //
        // 402 不在此列：那是上游真实配额不足的透传码（见 `map_upstream_error`），
        // 网关自身「挑不出账号」用 503，两者不能混。
        let code = match self.status() {
            StatusCode::BAD_REQUEST => "__400__",
            StatusCode::SERVICE_UNAVAILABLE => "__503__",
            StatusCode::TOO_MANY_REQUESTS => "__429__",
            _ => "",
        };
        write!(f, "{code}{}", self.message())
    }
}

pub fn short_uuid() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}
