use crate::{
    crypto::Crypto,
    error::{AppError, AppResult},
    models::{
        AccountInput, AccountRecord, AccountResponse, AccountSecrets, AuditLog, Group, GroupInput,
        Tag,
    },
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use chrono::Utc;
use serde_json::Value;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(path: &Path) -> Result<SqlitePool, String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("failed to create database directory: {error}"))?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .map_err(|error| format!("failed to connect to SQLite: {error}"))?;
    MIGRATOR
        .run(&pool)
        .await
        .map_err(|error| format!("database migration failed: {error}"))?;
    Ok(pool)
}

pub async fn seed_admin(pool: &SqlitePool, username: &str, password: &str) -> Result<(), String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| format!("failed to hash administrator password: {error}"))?
        .to_string();
    sqlx::query(
        "INSERT INTO admins (username, password_hash, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(username) DO UPDATE SET password_hash = excluded.password_hash, updated_at = excluded.updated_at",
    )
    .bind(username)
    .bind(hash)
    .bind(now())
    .execute(pool)
    .await
    .map_err(|error| format!("failed to initialize administrator: {error}"))?;
    Ok(())
}

pub async fn list_accounts(pool: &SqlitePool, crypto: &Crypto) -> AppResult<Vec<AccountResponse>> {
    let records =
        sqlx::query_as::<_, AccountRecord>("SELECT * FROM accounts ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;
    let mut output = Vec::with_capacity(records.len());
    for record in records {
        output.push(to_response(pool, crypto, record).await?);
    }
    Ok(output)
}

pub async fn get_account_record(pool: &SqlitePool, id: &str) -> AppResult<AccountRecord> {
    sqlx::query_as::<_, AccountRecord>("SELECT * FROM accounts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("account"))
}

pub async fn get_account(
    pool: &SqlitePool,
    crypto: &Crypto,
    id: &str,
) -> AppResult<AccountResponse> {
    let record = get_account_record(pool, id).await?;
    to_response(pool, crypto, record).await
}

pub fn decrypt_secrets(crypto: &Crypto, record: &AccountRecord) -> AppResult<AccountSecrets> {
    crypto.decrypt_json(&record.secret_blob).map_err(|error| {
        tracing::error!(account_id = %record.id, error = %error, "account secret decryption failed");
        AppError::internal()
    })
}

async fn to_response(
    pool: &SqlitePool,
    crypto: &Crypto,
    record: AccountRecord,
) -> AppResult<AccountResponse> {
    let secrets = decrypt_secrets(crypto, &record)?;
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM tags t INNER JOIN account_tags at ON at.tag_id = t.id WHERE at.account_id = ? ORDER BY t.name",
    )
    .bind(&record.id)
    .fetch_all(pool)
    .await?;
    Ok(AccountResponse::from_record(record, &secrets, tags))
}

pub async fn create_account(
    pool: &SqlitePool,
    crypto: &Crypto,
    input: AccountInput,
) -> AppResult<AccountResponse> {
    validate_account(&input)?;
    let id = Uuid::new_v4().to_string();
    let machine_id = input
        .machine_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string().to_lowercase());
    let mut secrets = input.secrets();
    let proxy_config = sanitize_proxy(input.proxy_config.clone(), &mut secrets);
    let secret_blob = crypto
        .encrypt_json(&secrets)
        .map_err(|_| AppError::internal())?;
    let timestamp = now();
    let proxy_json = proxy_config.as_ref().map(Value::to_string);
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO accounts (id,email,label,status,provider,user_id,auth_method,expires_at,region,profile_arn,group_id,machine_id,enabled,proxy_json,secret_blob,created_at,updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id).bind(clean(input.email)).bind(input.label.trim()).bind(input.status)
    .bind(clean(input.provider)).bind(clean(input.user_id)).bind(clean(input.auth_method))
    .bind(clean(input.expires_at)).bind(clean(input.region)).bind(clean(input.profile_arn))
    .bind(clean(input.group_id)).bind(machine_id).bind(if input.enabled { 1_i64 } else { 0_i64 }).bind(proxy_json)
    .bind(secret_blob).bind(&timestamp).bind(&timestamp)
    .execute(&mut *transaction).await?;
    replace_tags(&mut transaction, &id, &input.tag_ids).await?;
    transaction.commit().await?;
    get_account(pool, crypto, &id).await
}

pub async fn update_account(
    pool: &SqlitePool,
    crypto: &Crypto,
    id: &str,
    input: AccountInput,
) -> AppResult<AccountResponse> {
    validate_account(&input)?;
    let existing = get_account_record(pool, id).await?;
    let mut secrets = decrypt_secrets(crypto, &existing)?;
    merge_secret(&mut secrets.access_token, input.access_token);
    merge_secret(&mut secrets.refresh_token, input.refresh_token);
    merge_secret(&mut secrets.client_id, input.client_id);
    merge_secret(&mut secrets.client_secret, input.client_secret);
    merge_secret(&mut secrets.id_token, input.id_token);
    merge_secret(&mut secrets.sso_session_id, input.sso_session_id);
    merge_secret(&mut secrets.password, input.password);
    merge_secret(&mut secrets.token_endpoint, input.token_endpoint);
    merge_secret(&mut secrets.issuer_url, input.issuer_url);
    merge_secret(&mut secrets.scopes, input.scopes);
    let proxy_config = sanitize_proxy(input.proxy_config.clone(), &mut secrets);
    let secret_blob = crypto
        .encrypt_json(&secrets)
        .map_err(|_| AppError::internal())?;
    let machine_id = input
        .machine_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(existing.machine_id);
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "UPDATE accounts SET email=?,label=?,status=?,provider=?,user_id=?,auth_method=?,expires_at=?,region=?,profile_arn=?,group_id=?,machine_id=?,enabled=?,proxy_json=?,secret_blob=?,updated_at=? WHERE id=?",
    )
    .bind(clean(input.email)).bind(input.label.trim()).bind(input.status)
    .bind(clean(input.provider)).bind(clean(input.user_id)).bind(clean(input.auth_method))
    .bind(clean(input.expires_at)).bind(clean(input.region)).bind(clean(input.profile_arn))
    .bind(clean(input.group_id)).bind(machine_id).bind(if input.enabled { 1_i64 } else { 0_i64 })
    .bind(proxy_config.as_ref().map(Value::to_string)).bind(secret_blob).bind(now()).bind(id)
    .execute(&mut *transaction).await?;
    replace_tags(&mut transaction, id, &input.tag_ids).await?;
    transaction.commit().await?;
    get_account(pool, crypto, id).await
}

async fn replace_tags(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    account_id: &str,
    tag_ids: &[String],
) -> AppResult<()> {
    sqlx::query("DELETE FROM account_tags WHERE account_id = ?")
        .bind(account_id)
        .execute(&mut **transaction)
        .await?;
    for tag_id in tag_ids.iter().filter(|id| !id.trim().is_empty()) {
        sqlx::query(
            "INSERT OR IGNORE INTO account_tags (account_id, tag_id, linked_at) VALUES (?, ?, ?)",
        )
        .bind(account_id)
        .bind(tag_id)
        .bind(now())
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

pub async fn delete_account(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("account"));
    }
    Ok(())
}

pub async fn list_groups(pool: &SqlitePool) -> AppResult<Vec<Group>> {
    Ok(
        sqlx::query_as::<_, Group>("SELECT * FROM groups ORDER BY sort_order, name")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn create_group(pool: &SqlitePool, input: GroupInput) -> AppResult<Group> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("group name is required"));
    }
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    sqlx::query(
        "INSERT INTO groups (id,name,color,sort_order,created_at,updated_at) VALUES (?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(input.name.trim())
    .bind(input.color)
    .bind(input.sort_order)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(pool)
    .await?;
    get_group(pool, &id).await
}

pub async fn update_group(pool: &SqlitePool, id: &str, input: GroupInput) -> AppResult<Group> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("group name is required"));
    }
    let result =
        sqlx::query("UPDATE groups SET name=?,color=?,sort_order=?,updated_at=? WHERE id=?")
            .bind(input.name.trim())
            .bind(input.color)
            .bind(input.sort_order)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("group"));
    }
    get_group(pool, id).await
}

async fn get_group(pool: &SqlitePool, id: &str) -> AppResult<Group> {
    sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("group"))
}

pub async fn delete_group(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("group"));
    }
    Ok(())
}

pub async fn audit(
    pool: &SqlitePool,
    level: &str,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    message: &str,
    client_ip: Option<&str>,
) {
    let _ = sqlx::query("INSERT INTO audit_logs (level,action,target_type,target_id,message,client_ip,created_at) VALUES (?,?,?,?,?,?,?)")
        .bind(level).bind(action).bind(target_type).bind(target_id).bind(message).bind(client_ip).bind(now()).execute(pool).await;
}

pub async fn list_logs(pool: &SqlitePool, limit: i64) -> AppResult<Vec<AuditLog>> {
    Ok(
        sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs ORDER BY id DESC LIMIT ?")
            .bind(limit.clamp(1, 1000))
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get_settings(pool: &SqlitePool) -> AppResult<Value> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value_json FROM settings")
        .fetch_all(pool)
        .await?;
    let mut object = serde_json::Map::new();
    for (key, raw) in rows {
        object.insert(key, serde_json::from_str(&raw).unwrap_or(Value::Null));
    }
    Ok(Value::Object(object))
}

pub async fn update_settings(pool: &SqlitePool, value: &Value) -> AppResult<Value> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::bad_request("settings must be a JSON object"))?;
    let mut transaction = pool.begin().await?;
    for (key, setting) in object {
        if key.len() > 100 {
            return Err(AppError::bad_request("setting key is too long"));
        }
        sqlx::query("INSERT INTO settings (key,value_json,updated_at) VALUES (?,?,?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at")
            .bind(key).bind(setting.to_string()).bind(now()).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    get_settings(pool).await
}

fn validate_account(input: &AccountInput) -> AppResult<()> {
    if input.label.trim().is_empty() {
        return Err(AppError::bad_request("account label is required"));
    }
    if input.label.len() > 200 {
        return Err(AppError::bad_request("account label is too long"));
    }
    if input.email.as_ref().is_some_and(|value| value.len() > 320) {
        return Err(AppError::bad_request("email is too long"));
    }
    if let Some(proxy) = &input.proxy_config {
        if !proxy.is_object() {
            return Err(AppError::bad_request("proxyConfig must be an object"));
        }
    }
    Ok(())
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn merge_secret(target: &mut Option<String>, update: Option<String>) {
    if let Some(value) = update.filter(|value| !value.trim().is_empty()) {
        *target = Some(value);
    }
}

fn sanitize_proxy(proxy: Option<Value>, secrets: &mut AccountSecrets) -> Option<Value> {
    let mut proxy = proxy?;
    if let Some(object) = proxy.as_object_mut() {
        if let Some(password) = object
            .remove("password")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
        {
            merge_secret(&mut secrets.proxy_password, Some(password));
        }
    }
    Some(proxy)
}

pub fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::merge_secret;

    #[test]
    fn blank_secret_update_preserves_existing_value() {
        let mut value = Some("existing".to_string());
        merge_secret(&mut value, Some(String::new()));
        assert_eq!(value.as_deref(), Some("existing"));
        merge_secret(&mut value, Some("replacement".to_string()));
        assert_eq!(value.as_deref(), Some("replacement"));
    }
}
