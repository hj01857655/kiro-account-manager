// 应用全局状态

use crate::account::{AccountStore, GroupTagStore};
use crate::auth::AuthState;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::tray::TrayIcon;
use tauri::Wry;

#[derive(Clone)]
pub struct PendingLogin {
    pub provider: String,
    pub code_verifier: String,
    pub state: String,
    pub machineid: String,
}

pub struct AppState {
    pub store: Mutex<AccountStore>,
    pub group_tag_store: Mutex<GroupTagStore>,
    pub auth: AuthState,
    pub pending_login: Mutex<Option<PendingLogin>>,
    pub tray_ready: AtomicBool,
    pub tray_icon: Mutex<Option<TrayIcon<Wry>>>,
}
