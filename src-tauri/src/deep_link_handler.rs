// Deep Link 回调处理
// 处理 kiro-account-manager://authenticate-success?code=xxx&state=xxx 格式的 OAuth 回调

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
        // 临时：开发和生产都使用 kiro:// 协议
        "kiro://kiro.kiroAgent/authenticate-success".to_string()
    }
    
    /// 获取当前环境的协议名称
    pub fn get_protocol_scheme() -> &'static str {
        // 临时：开发和生产都使用 kiro 协议
        "kiro"
    }

    /// 等待回调结果
    pub fn wait_for_callback(&self) -> Result<OAuthCallbackResult, String> {
        let rx = self.result_rx.lock().expect("Failed to acquire result_rx lock").take()
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
    *storage.lock().expect("Failed to acquire pending sender lock") = Some((state.to_string(), tx));
    
    DeepLinkCallbackWaiter {
        result_rx: Arc::new(Mutex::new(Some(rx))),
        timeout: Duration::from_secs(300),
    }
}

/// 取消当前等待中的 deep link 登录
pub fn cancel_waiter() -> bool {
    let Some(storage) = PENDING_SENDER.get() else { return false };

    let mut guard = storage.lock().expect("Failed to acquire pending sender lock");
    let Some((_state, tx)) = guard.take() else { return false };
    let _ = tx.send(Err("登录已取消".to_string()));
    true
}

/// 处理 deep link URL（由 main.rs 调用）
pub fn handle_deep_link(url: &str) -> bool {
    let Some(storage) = PENDING_SENDER.get() else { return false };
    
    let mut guard = storage.lock().expect("Failed to acquire pending sender lock");
    let Some((expected_state, tx)) = guard.take() else { return false };
    
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
        let desc = params.get("error_description")
            .map_or_else(|| "Unknown error".to_string(), std::string::ToString::to_string);
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
