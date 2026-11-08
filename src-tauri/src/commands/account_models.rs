use crate::commands::common::resolve_kiro_call_context;
use crate::core::account::{Account, AvailableModelsCacheEntry};
use serde::{Deserialize, Deserializer, Serialize};

const AVAILABLE_MODELS_CACHE_TTL_SECONDS: i64 = 30 * 60;

fn null_string_as_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn null_string_vec_as_default<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<Option<String>>>::deserialize(deserializer)?
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect())
}

fn null_models_as_default<'de, D>(deserializer: D) -> Result<Vec<AvailableModel>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<AvailableModel>>::deserialize(deserializer)?.unwrap_or_default())
}

fn extract_string_enum(schema: &serde_json::Value, path: &[&str]) -> Vec<String> {
    let mut current = schema;
    for key in path {
        current = match current.get(*key) {
            Some(value) => value,
            None => return Vec::new(),
        };
    }

    current
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

const EFFORT_SCHEMA_PATHS: &[(&str, &[&str])] = &[
    (
        "output_config",
        &[
            "properties",
            "output_config",
            "properties",
            "effort",
            "enum",
        ],
    ),
    (
        "reasoning",
        &["properties", "reasoning", "properties", "effort", "enum"],
    ),
];

fn extract_effort_metadata(schema: &Option<serde_json::Value>) -> (Vec<String>, Option<String>) {
    let Some(schema) = schema.as_ref().filter(|value| value.is_object()) else {
        return (Vec::new(), None);
    };

    for (schema_path, enum_path) in EFFORT_SCHEMA_PATHS {
        let levels = extract_string_enum(schema, enum_path);
        if !levels.is_empty() {
            return (levels, Some((*schema_path).to_string()));
        }
    }

    (Vec::new(), None)
}

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModel {
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub model_id: String,
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub model_name: String,
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub description: String,
    pub provider: Option<String>,
    #[serde(default, deserialize_with = "null_string_vec_as_default")]
    pub capabilities: Vec<String>,
    pub context_window: Option<i64>,
    pub is_default: Option<bool>,
    pub rate_multiplier: Option<f64>,
    #[serde(default)]
    pub rate_unit: Option<String>,
    pub prompt_caching: Option<AvailableModelPromptCaching>,
    #[serde(default, deserialize_with = "null_string_vec_as_default")]
    pub supported_input_types: Vec<String>,
    pub token_limits: Option<AvailableModelTokenLimits>,
    #[serde(default)]
    pub additional_model_request_fields_schema: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effort_levels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_schema_path: Option<String>,
}

impl<'de> Deserialize<'de> for AvailableModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AvailableModelWire {
            #[serde(default, deserialize_with = "null_string_as_default")]
            model_id: String,
            #[serde(default, deserialize_with = "null_string_as_default")]
            model_name: String,
            #[serde(default, deserialize_with = "null_string_as_default")]
            description: String,
            provider: Option<String>,
            #[serde(default, deserialize_with = "null_string_vec_as_default")]
            capabilities: Vec<String>,
            context_window: Option<i64>,
            is_default: Option<bool>,
            rate_multiplier: Option<f64>,
            #[serde(default)]
            rate_unit: Option<String>,
            prompt_caching: Option<AvailableModelPromptCaching>,
            #[serde(default, deserialize_with = "null_string_vec_as_default")]
            supported_input_types: Vec<String>,
            token_limits: Option<AvailableModelTokenLimits>,
            #[serde(default)]
            additional_model_request_fields_schema: Option<serde_json::Value>,
            #[serde(default)]
            effort_levels: Vec<String>,
            #[serde(default)]
            effort_schema_path: Option<String>,
        }

        let wire = AvailableModelWire::deserialize(deserializer)?;
        let (schema_effort_levels, schema_effort_path) =
            extract_effort_metadata(&wire.additional_model_request_fields_schema);

        Ok(Self {
            model_id: wire.model_id,
            model_name: wire.model_name,
            description: wire.description,
            provider: wire.provider,
            capabilities: wire.capabilities,
            context_window: wire.context_window,
            is_default: wire.is_default,
            rate_multiplier: wire.rate_multiplier,
            rate_unit: wire.rate_unit,
            prompt_caching: wire.prompt_caching,
            supported_input_types: wire.supported_input_types,
            token_limits: wire.token_limits,
            additional_model_request_fields_schema: wire.additional_model_request_fields_schema,
            effort_levels: if wire.effort_levels.is_empty() {
                schema_effort_levels
            } else {
                wire.effort_levels
            },
            effort_schema_path: wire.effort_schema_path.or(schema_effort_path),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAvailableModelsResponse {
    #[serde(default, alias = "models", deserialize_with = "null_models_as_default")]
    pub available_models: Vec<AvailableModel>,
    pub next_token: Option<String>,
    pub default_model: Option<AvailableModel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvailableProfile {
    #[serde(default)]
    arn: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListAvailableProfilesResponse {
    #[serde(default)]
    profiles: Vec<AvailableProfile>,
}

#[derive(Debug, Clone)]
pub struct FetchAvailableModelsResult {
    pub response: ListAvailableModelsResponse,
    pub resolved_profile_arn: Option<String>,
}

fn now_unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn is_available_models_cache_fresh(cached_at: i64, now: i64) -> bool {
    now.saturating_sub(cached_at) <= AVAILABLE_MODELS_CACHE_TTL_SECONDS
}

pub fn read_available_models_cache(
    account: &Account,
    force_refresh: bool,
) -> Option<ListAvailableModelsResponse> {
    if force_refresh {
        return None;
    }
    let cache = account.available_models_cache.as_ref()?;
    if !is_available_models_cache_fresh(cache.cached_at, now_unix_timestamp()) {
        return None;
    }
    serde_json::from_value(cache.response.clone()).ok()
}

pub fn write_available_models_cache(
    account: &mut Account,
    response: &ListAvailableModelsResponse,
) -> Result<(), String> {
    let response_value =
        serde_json::to_value(response).map_err(|error| format!("序列化模型缓存失败: {error}"))?;
    account.available_models_cache = Some(AvailableModelsCacheEntry {
        response: response_value,
        cached_at: now_unix_timestamp(),
    });
    Ok(())
}

pub fn clear_available_models_cache(account: &mut Account) {
    account.available_models_cache = None;
}

fn first_available_profile_arn(response: ListAvailableProfilesResponse) -> Option<String> {
    response.profiles.into_iter().find_map(|profile| {
        profile
            .arn
            .map(|arn| arn.trim().to_string())
            .filter(|arn| !arn.is_empty())
    })
}

/// 获取账号可用模型列表（直接使用 KiroClient，无需重复实现）
pub async fn fetch_all_available_models(
    account: &Account,
    access_token: &str,
) -> Result<FetchAvailableModelsResult, String> {
    use crate::clients::kiro_client::KiroClient;

    let ctx = resolve_kiro_call_context(account, "us-east-1");
    let client = KiroClient::new()?;

    let resolved_profile_arn = if ctx.profile_arn.is_some() {
        ctx.profile_arn.clone()
    } else {
        match client
            .list_available_profiles(access_token, &ctx.region, account.auth_method.as_deref())
            .await
        {
            Ok(value) => {
                let profiles: ListAvailableProfilesResponse = serde_json::from_value(value)
                    .map_err(|error| format!("解析 ListAvailableProfiles 响应失败: {error}"))?;
                first_available_profile_arn(profiles)
            }
            Err(error) => {
                log::warn!(
                    "[ListAvailableModels] ListAvailableProfiles 兜底失败，继续使用现有 profileArn 解析结果: {}",
                    error
                );
                ctx.profile_arn.clone()
            }
        }
    };

    log::info!(
        "[ListAvailableModels] Account: {} | Provider: {} | ProfileArn (Original): {} | ProfileArn (Used): {}",
        account.id,
        account.provider.as_deref().unwrap_or("None"),
        account.profile_arn.as_deref().unwrap_or("None"),
        resolved_profile_arn.as_deref().unwrap_or("None")
    );

    let response_value = client
        .list_available_models(
            access_token,
            &ctx.machine_id,
            &ctx.region,
            resolved_profile_arn.as_deref(),
            account.auth_method.as_deref(),
        )
        .await?;

    let mut response: ListAvailableModelsResponse = serde_json::from_value(response_value)
        .map_err(|error| format!("解析 ListAvailableModels 响应失败: {error}"))?;

    normalize_list_available_models_response(&mut response);

    Ok(FetchAvailableModelsResult {
        response,
        resolved_profile_arn,
    })
}

fn normalize_list_available_models_response(response: &mut ListAvailableModelsResponse) {
    response
        .available_models
        .retain(|model| !model.model_id.trim().is_empty());

    let default_model_id = response
        .default_model
        .as_ref()
        .map(|model| model.model_id.trim().to_string())
        .filter(|model_id| !model_id.is_empty());

    if let Some(default_id) = default_model_id.as_deref() {
        mark_default_model(&mut response.available_models, Some(default_id));

        if let Some(default_from_list) = response
            .available_models
            .iter()
            .find(|model| model.model_id == default_id)
            .cloned()
        {
            response.default_model = Some(default_from_list);
        }
    }

    if let Some(default_model) = response.default_model.as_mut() {
        default_model.is_default = Some(true);
    }

    ensure_default_model_present(response);
    sort_available_models_for_display(&mut response.available_models);
}

fn mark_default_model(models: &mut [AvailableModel], default_model_id: Option<&str>) {
    if let Some(default_id) = default_model_id {
        for model in models {
            if model.model_id == default_id {
                model.is_default = Some(true);
            }
        }
    }
}

fn ensure_default_model_present(response: &mut ListAvailableModelsResponse) {
    if let Some(default_model) = response.default_model.clone() {
        if !default_model.model_id.trim().is_empty()
            && response
                .available_models
                .iter()
                .all(|model| model.model_id != default_model.model_id)
        {
            response.available_models.insert(0, default_model);
        }
    }
}

fn sort_available_models_for_display(models: &mut [AvailableModel]) {
    models.sort_by_key(|model| !model.is_default.unwrap_or(false));
}

#[cfg(test)]
mod tests {
    use super::{
        clear_available_models_cache, ensure_default_model_present, first_available_profile_arn,
        is_available_models_cache_fresh, mark_default_model, normalize_list_available_models_response,
        read_available_models_cache, sort_available_models_for_display, write_available_models_cache,
        AvailableModel, ListAvailableModelsResponse, ListAvailableProfilesResponse,
        AVAILABLE_MODELS_CACHE_TTL_SECONDS,
    };
    use crate::core::account::Account;

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

        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_id, "claude-sonnet-4.5");
        assert_eq!(response.available_models[0].model_name, "Claude Sonnet 4.5");
        assert_eq!(
            response.available_models[0].supported_input_types,
            vec!["TEXT".to_string(), "IMAGE".to_string()]
        );
        assert_eq!(
            response.available_models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_input_tokens),
            Some(200000)
        );
        assert_eq!(
            response.available_models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
        assert_eq!(response.next_token.as_deref(), Some("page-2"));
    }

    #[test]
    fn deserialize_list_available_models_response_allows_null_next_token_and_nullable_display_fields(
    ) {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": null,
                    "description": null,
                    "rateUnit": null
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize null optional fields");

        assert_eq!(response.next_token, None);
        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_name, "");
        assert_eq!(response.available_models[0].description, "");
        assert_eq!(response.available_models[0].rate_unit, None);
    }

    #[test]
    fn deserialize_list_available_models_response_extracts_effort_levels_from_model_schema() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "availableModels": [
                {
                    "modelId": "reasoning-model",
                    "modelName": "Reasoning Model",
                    "additionalModelRequestFieldsSchema": {
                        "type": "object",
                        "properties": {
                            "reasoning": {
                                "type": "object",
                                "properties": {
                                    "effort": {
                                        "type": "string",
                                        "enum": ["low", "medium", "high", "xhigh", "max"]
                                    }
                                }
                            }
                        }
                    }
                },
                {
                    "modelId": "plain-model",
                    "modelName": "Plain Model"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize effort metadata");

        assert_eq!(
            response.available_models[0].effort_levels,
            vec!["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            response.available_models[0].effort_schema_path.as_deref(),
            Some("reasoning")
        );
        assert!(response.available_models[1].effort_levels.is_empty());
        assert_eq!(response.available_models[1].effort_schema_path, None);
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

        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_id, "claude-sonnet-4");
        assert_eq!(response.available_models[0].model_name, "Claude Sonnet 4");
        assert_eq!(
            response.available_models[0].description,
            "Hybrid reasoning and coding for regular use"
        );
        assert_eq!(response.available_models[0].is_default, Some(true));
        assert_eq!(
            response.available_models[0]
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
            .available_models
            .iter()
            .filter(|model| model.model_id == "auto")
            .count();
        assert_eq!(auto_count, 1);
        assert_eq!(
            response
                .available_models
                .first()
                .map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn normalize_list_available_models_response_expands_default_model_from_models_list() {
        let mut response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": { "modelId": "deepseek-3.2" },
            "models": [
                {
                    "description": "Experimental preview of DeepSeek V3.2",
                    "modelId": "deepseek-3.2",
                    "modelName": "Deepseek v3.2",
                    "promptCaching": { "supportsPromptCaching": false },
                    "rateMultiplier": 0.25,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 164000,
                        "maxOutputTokens": 64000
                    }
                },
                {
                    "description": "The MiniMax M2.5 model",
                    "modelId": "minimax-m2.5",
                    "modelName": "MiniMax M2.5",
                    "promptCaching": { "supportsPromptCaching": false },
                    "rateMultiplier": 0.25,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT"],
                    "tokenLimits": {
                        "maxInputTokens": 196000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "nextToken": null
        }))
        .expect("captured response should deserialize");

        normalize_list_available_models_response(&mut response);

        assert_eq!(response.available_models.len(), 2);
        assert_eq!(response.available_models[0].model_id, "deepseek-3.2");
        assert_eq!(response.available_models[0].model_name, "Deepseek v3.2");
        assert_eq!(response.available_models[0].is_default, Some(true));
        assert_eq!(
            response
                .default_model
                .as_ref()
                .map(|model| model.model_name.as_str()),
            Some("Deepseek v3.2")
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.token_limits.as_ref())
                .and_then(|limits| limits.max_input_tokens),
            Some(164000)
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.prompt_caching.as_ref())
                .and_then(|prompt_caching| prompt_caching.supports_prompt_caching),
            Some(false)
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

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");
        let cached =
            read_available_models_cache(&account, false).expect("cache should be readable");

        assert_eq!(cached.available_models.len(), 2);
        assert_eq!(
            cached
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn first_available_profile_arn_skips_empty_profiles() {
        let response: ListAvailableProfilesResponse = serde_json::from_value(serde_json::json!({
            "profiles": [
                { "arn": "   " },
                { "arn": null },
                { "arn": "arn:aws:codewhisperer:eu-central-1:123456789012:profile/REAL" }
            ]
        }))
        .expect("profiles response should deserialize");

        assert_eq!(
            first_available_profile_arn(response).as_deref(),
            Some("arn:aws:codewhisperer:eu-central-1:123456789012:profile/REAL")
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

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");
        clear_available_models_cache(&mut account);

        assert!(read_available_models_cache(&account, false).is_none());
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

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");

        assert!(read_available_models_cache(&account, true).is_none());
    }

    #[test]
    fn deserialize_list_available_models_response_supports_both_models_and_available_models() {
        // 测试 AWS API 格式（models）
        let response_api: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "auto",
                    "modelName": "Auto"
                }
            ],
            "nextToken": null
        }))
        .expect("API format (models) should deserialize");
        assert_eq!(response_api.available_models.len(), 1);
        assert_eq!(response_api.available_models[0].model_id, "auto");

        // 测试缓存格式（availableModels）
        let response_cache: ListAvailableModelsResponse =
            serde_json::from_value(serde_json::json!({
                "availableModels": [
                    {
                        "modelId": "claude-sonnet-4.5",
                        "modelName": "Claude Sonnet 4.5"
                    }
                ],
                "nextToken": null
            }))
            .expect("Cache format (availableModels) should deserialize");
        assert_eq!(response_cache.available_models.len(), 1);
        assert_eq!(
            response_cache.available_models[0].model_id,
            "claude-sonnet-4.5"
        );
    }
}
