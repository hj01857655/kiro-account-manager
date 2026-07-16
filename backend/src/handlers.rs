use crate::{
    auth::AuthUser,
    db,
    error::{AppError, AppResult},
    kiro,
    models::{AccountInput, ApiResponse, DashboardStats, GroupInput, Tag},
    state::AppState,
};
use axum::{
    extract::{Extension, Path, Query, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn health(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(ApiResponse::ok(
        json!({ "status": "ok", "database": "ok", "version": env!("CARGO_PKG_VERSION") }),
    )))
}

pub async fn dashboard(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<DashboardStats>>> {
    let (total_accounts,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
        .fetch_one(&state.pool)
        .await?;
    let (available_accounts,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE enabled=1 AND status='active'")
            .fetch_one(&state.pool)
            .await?;
    let (abnormal_accounts,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE status<>'active' OR enabled=0")
            .fetch_one(&state.pool)
            .await?;
    let (last_refresh_at,): (Option<String>,) =
        sqlx::query_as("SELECT MAX(last_refreshed_at) FROM accounts")
            .fetch_one(&state.pool)
            .await?;
    let (gateway_request_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM gateway_metrics")
        .fetch_one(&state.pool)
        .await?;
    let rows: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT usage_json FROM accounts WHERE usage_json IS NOT NULL")
            .fetch_all(&state.pool)
            .await?;
    let mut total_quota = 0.0;
    let mut used_quota = 0.0;
    for (raw,) in rows {
        if let Some(value) = raw.and_then(|raw| serde_json::from_str::<Value>(&raw).ok()) {
            accumulate_quota(&value, &mut total_quota, &mut used_quota);
        }
    }
    Ok(Json(ApiResponse::ok(DashboardStats {
        total_accounts,
        available_accounts,
        abnormal_accounts,
        total_quota,
        used_quota,
        last_refresh_at,
        gateway_request_count,
    })))
}

pub async fn list_accounts(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(db::list_accounts(&state.pool, &state.crypto).await?)
            .map_err(|_| AppError::internal())?,
    )))
}

pub async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(db::get_account(&state.pool, &state.crypto, &id).await?)
            .map_err(|_| AppError::internal())?,
    )))
}

pub async fn create_account(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<AccountInput>,
) -> AppResult<(StatusCode, Json<ApiResponse<Value>>)> {
    let account = db::create_account(&state.pool, &state.crypto, input).await?;
    db::audit(
        &state.pool,
        "info",
        "account.create",
        Some("account"),
        Some(&account.id),
        "account created",
        Some(&user.client_ip),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok(
            serde_json::to_value(account).map_err(|_| AppError::internal())?,
        )),
    ))
}

pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<AccountInput>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let account = db::update_account(&state.pool, &state.crypto, &id, input).await?;
    db::audit(
        &state.pool,
        "info",
        "account.update",
        Some("account"),
        Some(&id),
        "account updated",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(account).map_err(|_| AppError::internal())?,
    )))
}

pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Value>>> {
    db::delete_account(&state.pool, &id).await?;
    db::audit(
        &state.pool,
        "warn",
        "account.delete",
        Some("account"),
        Some(&id),
        "account deleted",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(json!({ "deleted": true }))))
}

pub async fn import_accounts(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(value): Json<Value>,
) -> AppResult<(StatusCode, Json<ApiResponse<Value>>)> {
    let raw_accounts = if let Some(items) = value.as_array() {
        items.clone()
    } else {
        value
            .get("accounts")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AppError::bad_request(
                    "import must be an account array or an object with an accounts array",
                )
            })?
    };
    if raw_accounts.is_empty() || raw_accounts.len() > 500 {
        return Err(AppError::bad_request(
            "import must contain between 1 and 500 accounts",
        ));
    }
    let mut created = Vec::with_capacity(raw_accounts.len());
    let mut errors = Vec::new();
    for (index, raw) in raw_accounts.into_iter().enumerate() {
        match serde_json::from_value::<AccountInput>(raw) {
            Ok(input) => match db::create_account(&state.pool, &state.crypto, input).await {
                Ok(account) => created.push(account.id),
                Err(error) => errors.push(json!({ "index": index, "message": error.message })),
            },
            Err(_) => {
                errors.push(json!({ "index": index, "message": "invalid account JSON structure" }))
            }
        }
    }
    db::audit(
        &state.pool,
        "info",
        "account.import",
        Some("account"),
        None,
        &format!(
            "imported {} accounts; {} failed",
            created.len(),
            errors.len()
        ),
        Some(&user.client_ip),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok(
            json!({ "createdIds": created, "errors": errors }),
        )),
    ))
}

pub async fn export_accounts(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Response> {
    let accounts = db::list_accounts(&state.pool, &state.crypto).await?;
    db::audit(
        &state.pool,
        "info",
        "account.export",
        Some("account"),
        None,
        &format!("exported {} redacted accounts", accounts.len()),
        Some(&user.client_ip),
    )
    .await;
    let body =
        serde_json::to_vec_pretty(&json!({ "version": 1, "redacted": true, "accounts": accounts }))
            .map_err(|_| AppError::internal())?;
    let mut response = body.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=kiro-accounts-redacted.json"),
    );
    Ok(response)
}

pub async fn refresh_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let value = kiro::refresh(&state, &id).await?;
    db::audit(
        &state.pool,
        "info",
        "account.refresh",
        Some("account"),
        Some(&id),
        "account token refreshed",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(value)))
}

pub async fn check_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let value = kiro::check(&state, &id).await?;
    db::audit(
        &state.pool,
        "info",
        "account.check",
        Some("account"),
        Some(&id),
        "account availability checked",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(value)))
}

pub async fn account_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    Ok(Json(ApiResponse::ok(kiro::usage(&state, &id).await?)))
}

pub async fn account_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    Ok(Json(ApiResponse::ok(kiro::models(&state, &id).await?)))
}

pub async fn groups(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(db::list_groups(&state.pool).await?)
            .map_err(|_| AppError::internal())?,
    )))
}

pub async fn create_group(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<GroupInput>,
) -> AppResult<(StatusCode, Json<ApiResponse<Value>>)> {
    let group = db::create_group(&state.pool, input).await?;
    db::audit(
        &state.pool,
        "info",
        "group.create",
        Some("group"),
        Some(&group.id),
        "group created",
        Some(&user.client_ip),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok(
            serde_json::to_value(group).map_err(|_| AppError::internal())?,
        )),
    ))
}

pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<GroupInput>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let group = db::update_group(&state.pool, &id, input).await?;
    db::audit(
        &state.pool,
        "info",
        "group.update",
        Some("group"),
        Some(&id),
        "group updated",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(group).map_err(|_| AppError::internal())?,
    )))
}

pub async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Value>>> {
    db::delete_group(&state.pool, &id).await?;
    db::audit(
        &state.pool,
        "warn",
        "group.delete",
        Some("group"),
        Some(&id),
        "group deleted",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(json!({ "deleted": true }))))
}

pub async fn tags(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(tags).map_err(|_| AppError::internal())?,
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInput {
    name: String,
    color: Option<String>,
}

pub async fn create_tag(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<TagInput>,
) -> AppResult<(StatusCode, Json<ApiResponse<Value>>)> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("tag name is required"));
    }
    let id = Uuid::new_v4().to_string();
    let color = input.color.unwrap_or_else(|| "#7c3aed".to_string());
    let created_at = db::now();
    sqlx::query("INSERT INTO tags (id,name,color,created_at) VALUES (?,?,?,?)")
        .bind(&id)
        .bind(input.name.trim())
        .bind(&color)
        .bind(&created_at)
        .execute(&state.pool)
        .await?;
    db::audit(
        &state.pool,
        "info",
        "tag.create",
        Some("tag"),
        Some(&id),
        "tag created",
        Some(&user.client_ip),
    )
    .await;
    let tag = Tag {
        id,
        name: input.name.trim().to_string(),
        color,
        created_at,
    };
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok(
            serde_json::to_value(tag).map_err(|_| AppError::internal())?,
        )),
    ))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<TagInput>,
) -> AppResult<Json<ApiResponse<Value>>> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("tag name is required"));
    }
    let result = sqlx::query("UPDATE tags SET name=?,color=? WHERE id=?")
        .bind(input.name.trim())
        .bind(input.color.unwrap_or_else(|| "#7c3aed".to_string()))
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("tag"));
    }
    db::audit(
        &state.pool,
        "info",
        "tag.update",
        Some("tag"),
        Some(&id),
        "tag updated",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(json!({ "updated": true }))))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let result = sqlx::query("DELETE FROM tags WHERE id=?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("tag"));
    }
    db::audit(
        &state.pool,
        "warn",
        "tag.delete",
        Some("tag"),
        Some(&id),
        "tag deleted",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(json!({ "deleted": true }))))
}

pub async fn settings(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let mut value = db::get_settings(&state.pool).await?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "gatewayRuntime".to_string(),
            json!({
                "enabled": state.config.gateway_enabled,
                "hasApiKey": state.config.gateway_api_key.is_some(),
                "defaultAccount": state.config.gateway_default_account,
                "autoSwitch": state.config.gateway_auto_switch
            }),
        );
    }
    Ok(Json(ApiResponse::ok(value)))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(value): Json<Value>,
) -> AppResult<Json<ApiResponse<Value>>> {
    reject_sensitive_settings(&value)?;
    let updated = db::update_settings(&state.pool, &value).await?;
    db::audit(
        &state.pool,
        "info",
        "settings.update",
        Some("settings"),
        None,
        "settings updated",
        Some(&user.client_ip),
    )
    .await;
    Ok(Json(ApiResponse::ok(updated)))
}

#[derive(Deserialize)]
pub struct LogQuery {
    limit: Option<i64>,
}

pub async fn logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let logs = db::list_logs(&state.pool, query.limit.unwrap_or(200)).await?;
    Ok(Json(ApiResponse::ok(
        serde_json::to_value(logs).map_err(|_| AppError::internal())?,
    )))
}

fn reject_sensitive_settings(value: &Value) -> AppResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::bad_request("settings must be a JSON object"))?;
    reject_sensitive_object(object)
}

fn reject_sensitive_object(object: &serde_json::Map<String, Value>) -> AppResult<()> {
    for (key, value) in object {
        let lower = key.to_ascii_lowercase();
        if lower.contains("token")
            || lower.contains("password")
            || lower.contains("secret")
            || lower.contains("apikey")
            || lower.contains("api_key")
        {
            return Err(AppError::bad_request(
                "sensitive values must be configured through environment variables",
            ));
        }
        match value {
            Value::Object(child) => reject_sensitive_object(child)?,
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(child) = item {
                        reject_sensitive_object(child)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn accumulate_quota(value: &Value, total: &mut f64, used: &mut f64) {
    match value {
        Value::Object(object) => {
            if let Some(number) = ["usageLimit", "limit", "totalQuota"]
                .iter()
                .find_map(|key| object.get(*key).and_then(Value::as_f64))
            {
                *total += number;
            }
            if let Some(number) = ["currentUsage", "used", "usedQuota"]
                .iter()
                .find_map(|key| object.get(*key).and_then(Value::as_f64))
            {
                *used += number;
            }
            for child in object.values() {
                accumulate_quota(child, total, used);
            }
        }
        Value::Array(items) => {
            for child in items {
                accumulate_quota(child, total, used);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::reject_sensitive_settings;
    use serde_json::json;

    #[test]
    fn rejects_nested_sensitive_setting() {
        assert!(reject_sensitive_settings(&json!({ "gateway": { "apiKey": "secret" } })).is_err());
        assert!(reject_sensitive_settings(&json!({ "display": { "pageSize": 50 } })).is_ok());
    }
}
