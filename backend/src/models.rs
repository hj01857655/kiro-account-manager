use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorBody>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSecrets {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub id_token: Option<String>,
    pub sso_session_id: Option<String>,
    pub password: Option<String>,
    pub token_endpoint: Option<String>,
    pub issuer_url: Option<String>,
    pub scopes: Option<String>,
    pub proxy_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub email: Option<String>,
    pub label: String,
    #[serde(default = "active_status")]
    pub status: String,
    pub provider: Option<String>,
    pub user_id: Option<String>,
    pub auth_method: Option<String>,
    pub expires_at: Option<String>,
    pub region: Option<String>,
    pub profile_arn: Option<String>,
    pub group_id: Option<String>,
    pub machine_id: Option<String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    pub proxy_config: Option<Value>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub id_token: Option<String>,
    pub sso_session_id: Option<String>,
    pub password: Option<String>,
    pub token_endpoint: Option<String>,
    pub issuer_url: Option<String>,
    pub scopes: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
}

fn active_status() -> String {
    "active".to_string()
}
fn enabled_default() -> bool {
    true
}

impl AccountInput {
    pub fn secrets(&self) -> AccountSecrets {
        AccountSecrets {
            access_token: self.access_token.clone(),
            refresh_token: self.refresh_token.clone(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            id_token: self.id_token.clone(),
            sso_session_id: self.sso_session_id.clone(),
            password: self.password.clone(),
            token_endpoint: self.token_endpoint.clone(),
            issuer_url: self.issuer_url.clone(),
            scopes: self.scopes.clone(),
            proxy_password: None,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct AccountRecord {
    pub id: String,
    pub email: Option<String>,
    pub label: String,
    pub status: String,
    pub provider: Option<String>,
    pub user_id: Option<String>,
    pub auth_method: Option<String>,
    pub expires_at: Option<String>,
    pub region: Option<String>,
    pub profile_arn: Option<String>,
    pub group_id: Option<String>,
    pub machine_id: String,
    pub enabled: i64,
    pub proxy_json: Option<String>,
    pub usage_json: Option<String>,
    pub models_json: Option<String>,
    pub secret_blob: String,
    pub failure_count: i64,
    pub success_count: i64,
    pub disabled_reason: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_refreshed_at: Option<String>,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponse {
    pub id: String,
    pub email: Option<String>,
    pub label: String,
    pub status: String,
    pub provider: Option<String>,
    pub user_id: Option<String>,
    pub auth_method: Option<String>,
    pub expires_at: Option<String>,
    pub region: Option<String>,
    pub profile_arn: Option<String>,
    pub group_id: Option<String>,
    pub machine_id: String,
    pub enabled: bool,
    pub proxy_config: Option<Value>,
    pub usage: Option<Value>,
    pub models: Option<Value>,
    pub has_access_token: bool,
    pub has_refresh_token: bool,
    pub failure_count: i64,
    pub success_count: i64,
    pub disabled_reason: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_refreshed_at: Option<String>,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl AccountResponse {
    pub fn from_record(record: AccountRecord, secrets: &AccountSecrets, tags: Vec<Tag>) -> Self {
        Self {
            id: record.id,
            email: record.email,
            label: record.label,
            status: record.status,
            provider: record.provider,
            user_id: record.user_id,
            auth_method: record.auth_method,
            expires_at: record.expires_at,
            region: record.region,
            profile_arn: record.profile_arn,
            group_id: record.group_id,
            machine_id: record.machine_id,
            enabled: record.enabled != 0,
            proxy_config: parse_optional_json(record.proxy_json),
            usage: parse_optional_json(record.usage_json),
            models: parse_optional_json(record.models_json),
            has_access_token: secrets.access_token.as_ref().is_some_and(|v| !v.is_empty()),
            has_refresh_token: secrets
                .refresh_token
                .as_ref()
                .is_some_and(|v| !v.is_empty()),
            failure_count: record.failure_count,
            success_count: record.success_count,
            disabled_reason: record.disabled_reason,
            last_failure_at: record.last_failure_at,
            last_refreshed_at: record.last_refreshed_at,
            last_checked_at: record.last_checked_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
            tags,
        }
    }
}

fn parse_optional_json(value: Option<String>) -> Option<Value> {
    value.and_then(|raw| serde_json::from_str(&raw).ok())
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    #[sqlx(rename = "sort_order")]
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInput {
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    pub id: i64,
    pub level: String,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub message: String,
    pub client_ip: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_accounts: i64,
    pub available_accounts: i64,
    pub abnormal_accounts: i64,
    pub total_quota: f64,
    pub used_quota: f64,
    pub last_refresh_at: Option<String>,
    pub gateway_request_count: i64,
}
