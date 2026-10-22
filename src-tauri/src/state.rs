// 应用全局状态

use crate::auth::AuthState;
use crate::core::account::{AccountStore, GroupTagStore};
use crate::gateway::GatewayRuntime;
use std::sync::{Arc, Mutex};

/// 账号状态的唯一共享副本。
///
/// gateway（axum 任务）与 Tauri 命令都必须通过这一份读写账号状态：
/// 各自 `AccountStore::new()` 会得到互不知情的独立快照，后写覆盖前写，
/// 静默丢失 token 刷新与失败计数。
pub type SharedAccountStore = Arc<Mutex<AccountStore>>;

#[derive(Clone)]
pub struct PendingLogin {
    pub provider: String,
    pub code_verifier: String,
    pub state: String,
    pub machineid: String,
}

pub struct AppState {
    pub store: SharedAccountStore,
    pub group_tag_store: Mutex<GroupTagStore>,
    pub auth: AuthState,
    pub pending_login: Mutex<Option<PendingLogin>>,
    pub gateway: Mutex<Option<GatewayRuntime>>,
}
