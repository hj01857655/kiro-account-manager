// Deep Link 回调处理
// 处理 kiro://kiro.kiroAgent/authenticate-success?code=xxx&state=xxx 格式的 OAuth 回调

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEEP_LINK_SCHEME: &str = "kiro";
const DEEP_LINK_REDIRECT_URI: &str = "kiro://kiro.kiroAgent/authenticate-success";

/// OAuth 回调结果（state 已在 `handle_deep_link` 中验证）
#[derive(Debug, Clone)]
pub struct OAuthCallbackResult {
    pub code: String,
}

/// 回调结果类型别名
type CallbackResult = Result<OAuthCallbackResult, String>;
/// 回调接收器类型别名
type CallbackReceiver = Arc<Mutex<Option<Receiver<CallbackResult>>>>;
/// 待处理发送器类型别名
type PendingSender = Mutex<Option<(String, Sender<CallbackResult>)>>;

/// Deep Link OAuth 回调等待器
pub struct DeepLinkCallbackWaiter {
    result_rx: CallbackReceiver,
    timeout: Duration,
}

impl DeepLinkCallbackWaiter {
    /// 获取 `redirect_uri` (根据环境自动选择协议)
    pub fn get_redirect_uri() -> String {
        DEEP_LINK_REDIRECT_URI.to_string()
    }

    /// 获取当前环境的协议名称
    pub fn get_protocol_scheme() -> &'static str {
        DEEP_LINK_SCHEME
    }

    /// 等待回调结果
    pub fn wait_for_callback(&self) -> Result<OAuthCallbackResult, String> {
        let rx = self
            .result_rx
            .lock()
            .expect("Failed to acquire result_rx lock")
            .take()
            .ok_or("Callback channel already consumed")?;

        match rx.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(_) => Err("OAuth callback timeout (5 minutes)".to_string()),
        }
    }
}

/// 全局回调发送器存储
static PENDING_SENDER: std::sync::OnceLock<PendingSender> = std::sync::OnceLock::new();

/// 注册一个新的回调等待器，返回接收端
pub fn register_waiter(state: &str) -> DeepLinkCallbackWaiter {
    let (tx, rx) = mpsc::channel();

    // 存储发送端
    let storage = PENDING_SENDER.get_or_init(|| Mutex::new(None));
    let mut guard = storage
        .lock()
        .expect("Failed to acquire pending sender lock");
    if let Some((_state, previous_tx)) = guard.take() {
        let _ = previous_tx.send(Err("登录已取消".to_string()));
    }
    *guard = Some((state.to_string(), tx));

    DeepLinkCallbackWaiter {
        result_rx: Arc::new(Mutex::new(Some(rx))),
        timeout: Duration::from_secs(300),
    }
}

/// 取消当前等待中的 deep link 登录
pub fn cancel_waiter() -> bool {
    let Some(storage) = PENDING_SENDER.get() else {
        return false;
    };

    let mut guard = storage
        .lock()
        .expect("Failed to acquire pending sender lock");
    let Some((_state, tx)) = guard.take() else {
        return false;
    };
    let _ = tx.send(Err("登录已取消".to_string()));
    true
}

/// 将 deep link 中的 OAuth 回调路径映射到应用内的 `/callback`
pub fn get_app_callback_route(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;

    if parsed.scheme() != DeepLinkCallbackWaiter::get_protocol_scheme() {
        return None;
    }

    if parsed.path() != "/app/callback" && parsed.path() != "/authenticate-success" {
        return None;
    }

    let mut route = "/callback".to_string();
    if let Some(query) = parsed.query() {
        route.push('?');
        route.push_str(query);
    }

    Some(route)
}

/// 处理 deep link URL（由 main.rs 调用）
pub fn handle_deep_link(url: &str) -> bool {
    let Some(storage) = PENDING_SENDER.get() else {
        return false;
    };

    let mut guard = storage
        .lock()
        .expect("Failed to acquire pending sender lock");
    let Some((expected_state, tx)) = guard.take() else {
        return false;
    };

    // 解析 URL
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            let _ = tx.send(Err(format!("Invalid URL: {e}")));
            return false;
        }
    };

    // 检查协议是否匹配当前环境
    let expected_scheme = DeepLinkCallbackWaiter::get_protocol_scheme();
    if parsed.scheme() != expected_scheme {
        *guard = Some((expected_state, tx)); // 放回去
        return false;
    }

    // 提取参数
    let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

    // 检查错误
    if let Some(error) = params.get("error") {
        let desc = params.get("error_description").map_or_else(
            || "Unknown error".to_string(),
            std::string::ToString::to_string,
        );
        let _ = tx.send(Err(format!("OAuth error: {error} - {desc}")));
        return true;
    }

    let Some(code) = params.get("code") else {
        let _ = tx.send(Err("Missing code parameter".to_string()));
        return true;
    };
    let code = code.to_string();

    let Some(state) = params.get("state") else {
        let _ = tx.send(Err("Missing state parameter".to_string()));
        return true;
    };
    let state = state.to_string();

    // 验证 state
    if state != expected_state {
        let _ = tx.send(Err("State mismatch - possible CSRF attack".to_string()));
        return true;
    }

    let _ = tx.send(Ok(OAuthCallbackResult { code }));
    true
}

#[cfg(test)]
mod tests {
    use super::{
        get_app_callback_route, handle_deep_link, register_waiter, DeepLinkCallbackWaiter,
    };
    use std::time::Duration;

    #[test]
    fn deep_link_scheme_matches_registered_tauri_scheme() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json"))
            .expect("tauri config should parse");
        let scheme = config["plugins"]["deep-link"]["desktop"]["schemes"][0]
            .as_str()
            .expect("deep-link scheme should exist");

        assert_eq!(DeepLinkCallbackWaiter::get_protocol_scheme(), scheme);
        assert!(
            DeepLinkCallbackWaiter::get_redirect_uri().starts_with(&format!("{scheme}://")),
            "redirect uri should use registered scheme"
        );
        assert!(
            DeepLinkCallbackWaiter::get_redirect_uri().contains("/authenticate-success"),
            "redirect uri should keep callback path for social/idc compatibility"
        );
    }

    #[test]
    fn registering_new_waiter_cancels_previous_waiter() {
        let mut first = register_waiter("first-state");
        first.timeout = Duration::from_millis(20);
        let _second = register_waiter("second-state");

        let result = first.wait_for_callback();

        assert!(matches!(result, Err(message) if message == "登录已取消"));
    }

    #[test]
    fn handle_deep_link_keeps_waiter_when_scheme_does_not_match() {
        let waiter = register_waiter("expected-state");

        assert!(!handle_deep_link(
            "wrong-scheme://callback?code=ok&state=expected-state"
        ));

        let handled =
            handle_deep_link("kiro://kiro.kiroAgent/authenticate-success?code=ok&state=expected-state");
        assert!(handled);
        assert_eq!(
            waiter
                .wait_for_callback()
                .expect("callback should succeed")
                .code,
            "ok"
        );
    }

    #[test]
    fn app_callback_deep_link_maps_to_internal_callback_route() {
        let route = get_app_callback_route(
            "kiro://kiro.kiroAgent/app/callback?code=ok&state=expected-state",
        )
        .expect("app callback route should be extracted");

        assert_eq!(route, "/callback?code=ok&state=expected-state");
    }

    #[test]
    fn authenticate_success_deep_link_maps_to_internal_callback_route() {
        let route = get_app_callback_route(
            "kiro://kiro.kiroAgent/authenticate-success?code=ok&state=expected-state",
        )
        .expect("authenticate-success callback route should be extracted");

        assert_eq!(route, "/callback?code=ok&state=expected-state");
    }
}
