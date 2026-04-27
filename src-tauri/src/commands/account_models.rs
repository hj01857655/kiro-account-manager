use crate::core::account::{Account, AvailableModelsCacheEntry};
use crate::commands::machine_guid::get_machine_id;
use crate::clients::http_client::{
    build_http_client_with_user_agent, build_kiro_custom_user_agent, is_external_idp_auth_method,
    build_q_service_url, resolve_kiro_upstream_region,
};
use serde::{Deserialize, Serialize};

const AVAILABLE_MODELS_CACHE_TTL_SECONDS: i64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelTokenLimits {
    pub max_input_tokens: Option<i64>,
    pub max_output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelPromptCaching {
    pub maximum_cache_checkpoints_per_request: Option<i64>,
    pub minimum_tokens_per_cache_checkpoint: Option<i64>,
    pub supports_prompt_caching: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModel {
    pub model_id: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub description: String,
    pub provider: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub context_window: Option<i64>,
    pub is_default: Option<bool>,
    pub rate_multiplier: Option<f64>,
    pub rate_unit: Option<String>,
    pub prompt_caching: Option<AvailableModelPromptCaching>,
    #[serde(default)]
    pub supported_input_types: Vec<String>,
    pub token_limits: Option<AvailableModelTokenLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAvailableModelsResponse {
    #[serde(default)]
    pub models: Vec<AvailableModel>,
    pub next_token: Option<String>,
    pub default_model: Option<AvailableModel>,
}

fn build_list_available_models_url(
    base_url: &str,
    profile_arn: Option<&str>,
    model_provider: Option<&str>,
    next_token: Option<&str>,
) -> Result<String, String> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|error| format!("ListAvailableModels base URL 无效: {error}"))?;
    url.set_path("ListAvailableModels");

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("origin", "AI_EDITOR");
        pairs.append_pair("maxResults", "50");
        if let Some(profile_arn) = profile_arn.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("profileArn", profile_arn);
        }
        if let Some(model_provider) = model_provider.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("modelProvider", model_provider);
        }
        if let Some(next_token) = next_token.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("nextToken", next_token);
        }
    }

    Ok(url.into())
}

fn build_kiro_models_user_agent(machine_id: &str) -> String {
    build_kiro_custom_user_agent(machine_id)
}

fn build_list_available_models_runtime_request(
    client: &reqwest::Client,
    url: &str,
    account: &Account,
    access_token: &str,
    user_agent: &str,
) -> reqwest::RequestBuilder {
    use uuid::Uuid;

    let invocation_id = Uuid::new_v4().to_string();

    // ListAvailableModels 不需要 tokentype header，手动构建
    let mut builder = client
        .get(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("accept", "application/json")
        .header("user-agent", user_agent)
        .header("x-amz-user-agent", user_agent)
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=1");

    // 只为 Internal provider 添加 redirect header
    if is_external_idp_auth_method(account.auth_method.as_deref()) {
        builder = builder.header("TokenType", "EXTERNAL_IDP");
    }

    if account.provider.as_deref() == Some("Internal") {
        builder = builder.header("redirect-for-internal", "true");
    }

    builder
}

fn now_unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn is_available_models_cache_fresh(cached_at: i64, now: i64) -> bool {
    now.saturating_sub(cached_at) <= AVAILABLE_MODELS_CACHE_TTL_SECONDS
}

pub fn read_available_models_cache(
    account: &Account,
    model_provider: Option<&str>,
    force_refresh: bool,
) -> Option<ListAvailableModelsResponse> {
    if force_refresh {
        return None;
    }
    let cache = account.available_models_cache.as_ref()?;
    if !is_available_models_cache_fresh(cache.cached_at, now_unix_timestamp()) {
        return None;
    }
    if cache.model_provider.as_deref() != model_provider {
        return None;
    }
    serde_json::from_value(cache.response.clone()).ok()
}

pub fn write_available_models_cache(
    account: &mut Account,
    model_provider: Option<&str>,
    response: &ListAvailableModelsResponse,
) -> Result<(), String> {
    let response_value =
        serde_json::to_value(response).map_err(|error| format!("序列化模型缓存失败: {error}"))?;
    account.available_models_cache = Some(AvailableModelsCacheEntry {
        response: response_value,
        cached_at: now_unix_timestamp(),
        model_provider: model_provider.map(str::to_string),
    });
    Ok(())
}

pub fn clear_available_models_cache(account: &mut Account) {
    account.available_models_cache = None;
}

async fn fetch_available_models_page(
    account: &Account,
    access_token: &str,
    model_provider: Option<&str>,
    next_token: Option<&str>,
) -> Result<ListAvailableModelsResponse, String> {
    let machine_id = account
        .machine_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(get_machine_id);
    let user_agent = build_kiro_models_user_agent(&machine_id);
    let region = resolve_kiro_upstream_region(
        account.profile_arn.as_deref(),
        account.region.as_deref(),
        "us-east-1",
    );
    let base_url = build_q_service_url(&region);
    let url = build_list_available_models_url(
        &base_url,
        account.profile_arn.as_deref(),
        model_provider,
        next_token,
    )?;
    let client = build_http_client_with_user_agent(&user_agent)?;
    let request =
        build_list_available_models_runtime_request(&client, &url, account, access_token, &user_agent);
    let response = request
        .send()
        .await
        .map_err(|error| format!("ListAvailableModels 请求失败: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();

        // 401 → token 过期，可尝试刷新
        if status.as_u16() == 401 {
            return Err(format!(
                "AUTH_ERROR: ListAvailableModels failed ({status}): {body}"
            ));
        }

        // 403 → 检查是否为封禁（AccessDeniedException + TemporarilySuspended）
        if status.as_u16() == 403 {
            if body.contains("AccessDeniedException") && body.contains("TemporarilySuspended") {
                return Err(format!("BANNED: ListAvailableModels 403 封禁: {body}"));
            }
            // 其他 403 错误（如权限问题）不视为封禁
            return Err(format!("AUTH_ERROR: ListAvailableModels 403: {body}"));
        }

        return Err(format!("ListAvailableModels failed ({status}): {body}"));
    }

    response
        .json::<ListAvailableModelsResponse>()
        .await
        .map_err(|error| format!("解析 ListAvailableModels 响应失败: {error}"))
}

fn mark_default_model(models: &mut [AvailableModel], default_model_id: Option<&str>) {
    if let Some(default_id) = default_model_id {
        for model in models {
            if model.model_id == default_id && model.is_default.is_none() {
                model.is_default = Some(true);
            }
        }
    }
}

fn ensure_default_model_present(response: &mut ListAvailableModelsResponse) {
    if let Some(default_model) = response.default_model.clone() {
        if response
            .models
            .iter()
            .all(|model| model.model_id != default_model.model_id)
        {
            response.models.insert(0, default_model);
        }
    }
}

pub async fn fetch_all_available_models(
    account: &Account,
    access_token: &str,
    model_provider: Option<&str>,
) -> Result<ListAvailableModelsResponse, String> {
    let mut aggregated = ListAvailableModelsResponse {
        models: Vec::new(),
        next_token: None,
        default_model: None,
    };
    let mut next_token: Option<String> = None;

    loop {
        let mut response = fetch_available_models_page(
            account,
            access_token,
            model_provider,
            next_token.as_deref(),
        )
        .await?;

        if aggregated.default_model.is_none() {
            aggregated.default_model = response.default_model.clone();
        }

        let default_model_id = aggregated
            .default_model
            .as_ref()
            .map(|model| model.model_id.as_str());
        mark_default_model(&mut response.models, default_model_id);

        if let Some(default_model) = aggregated.default_model.as_mut() {
            default_model.is_default = Some(true);
        }

        aggregated.models.extend(response.models);
        next_token = response.next_token;
        if next_token.is_none() {
            break;
        }
    }

    ensure_default_model_present(&mut aggregated);
    sort_available_models_for_display(&mut aggregated.models);
    aggregated.next_token = None;

    Ok(aggregated)
}

fn sort_available_models_for_display(models: &mut [AvailableModel]) {
    models.sort_by_key(|model| !model.is_default.unwrap_or(false));
}

#[cfg(test)]
mod tests {
    use super::{
        build_list_available_models_runtime_request, build_list_available_models_url,
        clear_available_models_cache, ensure_default_model_present,
        is_available_models_cache_fresh, mark_default_model, read_available_models_cache,
        sort_available_models_for_display, write_available_models_cache, AvailableModel,
        ListAvailableModelsResponse,
        AVAILABLE_MODELS_CACHE_TTL_SECONDS,
    };
    use crate::core::account::Account;

    #[test]
    fn build_list_available_models_url_keeps_expected_query_shape() {
        let url = build_list_available_models_url(
            "https://q.us-east-1.amazonaws.com",
            Some("arn:aws:codewhisperer:::profile/test"),
            Some("anthropic"),
            Some("next-token"),
        )
        .expect("url should build");
        let parsed = reqwest::Url::parse(&url).expect("url should parse");
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(parsed.path(), "/ListAvailableModels");
        assert_eq!(params.get("origin").map(String::as_str), Some("AI_EDITOR"));
        assert_eq!(params.get("maxResults").map(String::as_str), Some("50"));
        assert_eq!(
            params.get("profileArn").map(String::as_str),
            Some("arn:aws:codewhisperer:::profile/test")
        );
        assert_eq!(
            params.get("modelProvider").map(String::as_str),
            Some("anthropic")
        );
        assert_eq!(
            params.get("nextToken").map(String::as_str),
            Some("next-token")
        );
    }

    #[test]
    fn deserialize_list_available_models_response_supports_known_fields() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5",
                    "description": "The Claude Sonnet 4.5 model",
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "nextToken": "page-2"
        }))
        .expect("response should deserialize");

        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].model_id, "claude-sonnet-4.5");
        assert_eq!(response.models[0].model_name, "Claude Sonnet 4.5");
        assert_eq!(
            response.models[0].supported_input_types,
            vec!["TEXT".to_string(), "IMAGE".to_string()]
        );
        assert_eq!(
            response.models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_input_tokens),
            Some(200000)
        );
        assert_eq!(
            response.models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
        assert_eq!(response.next_token.as_deref(), Some("page-2"));
    }

    #[test]
    fn deserialize_list_available_models_response_supports_full_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4",
                    "modelName": "Claude Sonnet 4",
                    "description": "Hybrid reasoning and coding for regular use",
                    "isDefault": true,
                    "promptCaching": {
                        "maximumCacheCheckpointsPerRequest": 4,
                        "minimumTokensPerCacheCheckpoint": 1024,
                        "supportsPromptCaching": true
                    },
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "defaultModel": {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4",
                "description": "Hybrid reasoning and coding for regular use",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.3,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            }
        }))
        .expect("full response should deserialize");

        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].model_id, "claude-sonnet-4");
        assert_eq!(response.models[0].model_name, "Claude Sonnet 4");
        assert_eq!(
            response.models[0].description,
            "Hybrid reasoning and coding for regular use"
        );
        assert_eq!(response.models[0].is_default, Some(true));
        assert_eq!(
            response.models[0]
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("claude-sonnet-4")
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.prompt_caching.as_ref())
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
    }

    #[test]
    fn deserialize_list_available_models_response_supports_live_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "description": "Models chosen by task for optimal usage and consistent quality",
                "modelId": "auto",
                "modelName": "Auto",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.0,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            },
            "models": [
                {
                    "description": "Models chosen by task for optimal usage and consistent quality",
                    "modelId": "auto",
                    "modelName": "Auto"
                }
            ],
            "nextToken": null
        }))
        .expect("live response shape should deserialize");

        let default_model = response
            .default_model
            .as_ref()
            .expect("default model should exist");
        assert_eq!(default_model.model_id, "auto");
        assert_eq!(default_model.model_name, "Auto");
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.maximum_cache_checkpoints_per_request),
            Some(4)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
        assert_eq!(
            default_model
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
    }

    #[test]
    fn sort_available_models_for_display_prioritizes_default_models() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            {
                "modelId": "claude-sonnet-4.5",
                "modelName": "Claude Sonnet 4.5"
            },
            {
                "modelId": "auto",
                "modelName": "Auto",
                "isDefault": true
            },
            {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4"
            }
        ]))
        .expect("models should deserialize");

        sort_available_models_for_display(&mut models);

        let ordered_ids: Vec<_> = models.iter().map(|model| model.model_id.as_str()).collect();
        assert_eq!(
            ordered_ids,
            vec!["auto", "claude-sonnet-4.5", "claude-sonnet-4"]
        );
    }

    #[test]
    fn mark_default_model_sets_matching_entry() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            { "modelId": "claude-sonnet-4.5", "modelName": "Claude Sonnet 4.5" },
            { "modelId": "auto", "modelName": "Auto" }
        ]))
        .expect("models should deserialize");

        mark_default_model(&mut models, Some("auto"));

        assert_eq!(models[0].is_default, None);
        assert_eq!(models[1].is_default, Some(true));
    }

    #[test]
    fn ensure_default_model_present_inserts_only_once() {
        let mut response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        ensure_default_model_present(&mut response);
        ensure_default_model_present(&mut response);

        let auto_count = response
            .models
            .iter()
            .filter(|model| model.model_id == "auto")
            .count();
        assert_eq!(auto_count, 1);
        assert_eq!(
            response.models.first().map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn available_models_cache_round_trips_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "auto",
                    "modelName": "Auto"
                },
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, Some("anthropic"), &response)
            .expect("cache write should succeed");
        let cached = read_available_models_cache(&account, Some("anthropic"), false)
            .expect("cache should be readable");

        assert_eq!(cached.models.len(), 2);
        assert_eq!(
            cached
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn available_models_cache_expires_after_ttl() {
        assert!(is_available_models_cache_fresh(
            100,
            100 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
        assert!(!is_available_models_cache_fresh(
            100,
            101 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
    }

    #[test]
    fn clear_available_models_cache_removes_cached_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, None, &response)
            .expect("cache write should succeed");
        clear_available_models_cache(&mut account);

        assert!(read_available_models_cache(&account, None, false).is_none());
    }

    #[test]
    fn available_models_cache_misses_when_model_provider_differs() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, Some("anthropic"), &response)
            .expect("cache write should succeed");

        assert!(read_available_models_cache(&account, Some("openai"), false).is_none());
        assert!(read_available_models_cache(&account, None, false).is_none());
        assert!(read_available_models_cache(&account, Some("anthropic"), false).is_some());
    }

    #[test]
    fn available_models_cache_skips_when_force_refresh_enabled() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, None, &response)
            .expect("cache write should succeed");

        assert!(read_available_models_cache(&account, None, true).is_none());
    }

    #[test]
    fn build_list_available_models_runtime_request_adds_runtime_headers_for_internal_external_idp() {
        let client = reqwest::Client::new();
        let mut account = Account::new("internal@example.com".to_string(), "internal".to_string());
        account.provider = Some("Internal".to_string());
        account.auth_method = Some("external_idp".to_string());
        let request = build_list_available_models_runtime_request(
            &client,
            "https://q.us-east-1.amazonaws.com/ListAvailableModels?origin=AI_EDITOR",
            &account,
            "token-1",
            "KiroIDE 0.11.34 machine-123",
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer token-1")
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::USER_AGENT)
                .and_then(|value| value.to_str().ok()),
            Some("KiroIDE 0.11.34 machine-123")
        );
        assert_eq!(
            request
                .headers()
                .get("x-amz-user-agent")
                .and_then(|value| value.to_str().ok()),
            Some("KiroIDE 0.11.34 machine-123")
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::ACCEPT)
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert_eq!(
            request
                .headers()
                .get("TokenType")
                .and_then(|value| value.to_str().ok()),
            Some("EXTERNAL_IDP")
        );
        assert_eq!(
            request
                .headers()
                .get("redirect-for-internal")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );
        assert!(request.headers().get("x-amzn-codewhisperer-optout").is_none());
        assert!(request.headers().get("x-amzn-kiro-agent-mode").is_none());
        assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
    }

    #[test]
    fn build_list_available_models_runtime_request_omits_internal_redirect_for_non_internal_provider() {
        let client = reqwest::Client::new();
        let mut account = Account::new("builder@example.com".to_string(), "builder".to_string());
        account.provider = Some("BuilderId".to_string());
        account.auth_method = Some("external_idp".to_string());
        let request = build_list_available_models_runtime_request(
            &client,
            "https://q.us-east-1.amazonaws.com/ListAvailableModels?origin=AI_EDITOR&profileArn=arn",
            &account,
            "token-2",
            "KiroIDE 0.11.34 machine-456",
        )
        .build()
        .expect("request should build");

        assert!(request.headers().get("redirect-for-internal").is_none());
        assert!(request.headers().get("x-amzn-kiro-profile-arn").is_none());
        assert_eq!(
            request.url().query(),
            Some("origin=AI_EDITOR&profileArn=arn")
        );
    }
}
