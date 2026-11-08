// Providers 模块 - 认证提供者

mod base;
pub(crate) mod external_idp;
mod factory;
mod idc;
mod social;

pub use base::{AuthProvider, AuthResult, RefreshMetadata};
pub use external_idp::{validate_microsoft_token_endpoint, ExternalIdpProvider};
pub use factory::*;
pub use idc::{cancel_pending_login as cancel_pending_idc_login, IdcProvider};
pub use social::{SocialProvider, SocialTokenResponse};
