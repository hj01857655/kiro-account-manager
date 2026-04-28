// 认证相关模块

#[allow(clippy::module_inception)]
mod auth;
pub mod auth_social;
pub mod providers;

// 重新导出常用类型
pub use auth::{AuthState, DesktopRefreshResponse, User, refresh_token_desktop, delete_account_desktop, DESKTOP_AUTH_API};
