// 认证相关模块

#[allow(clippy::module_inception)]
mod auth;
pub mod auth_social;
pub mod external_idp_login;
pub mod providers;
mod window_navigation;

// 重新导出常用类型
pub use auth::{
    delete_account_desktop, refresh_token_desktop, AuthState, DesktopRefreshResponse, User,
    DESKTOP_AUTH_API,
};
// external_idp 两段式登录由 idc.rs 的 Enterprise 分支直接调用
// （crate::auth::external_idp_login::login_external_idp_via_portal），无需重导出
pub use window_navigation::handle_incoming_deep_link;
