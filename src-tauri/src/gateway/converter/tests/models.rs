//! 模型 ID 归一化与可用模型列表的回归测试。

use super::*;

#[test]
fn get_internal_model_id_normalizes_versioned_public_model_names() {
    assert_eq!(
        get_internal_model_id("claude-sonnet-4-5-20250929")
            .expect("versioned sonnet 4.5 should map"),
        "claude-sonnet-4.5"
    );
    assert_eq!(
        get_internal_model_id("claude-sonnet-4-6").expect("sonnet 4.6 alias should map"),
        "claude-sonnet-4.6"
    );
    assert_eq!(
        get_internal_model_id("claude-sonnet-4-6-20260217")
            .expect("versioned sonnet 4.6 should map"),
        "claude-sonnet-4.6"
    );
    assert_eq!(
        get_internal_model_id("claude-opus-4-6").expect("opus 4.6 alias should map"),
        "claude-opus-4.6"
    );
    assert_eq!(
        get_internal_model_id("claude-opus-4-6-20260205")
            .expect("versioned opus 4.6 should map"),
        "claude-opus-4.6"
    );
    assert_eq!(
        get_internal_model_id("claude-haiku-4-5-20251001")
            .expect("versioned haiku 4.5 should map"),
        "claude-haiku-4.5"
    );
    assert_eq!(
        get_internal_model_id("claude-sonnet-latest")
            .expect("latest sonnet alias should resolve to Sonnet 5"),
        "claude-sonnet-5"
    );
    // "sonnet" 默认指向当前最新的 Sonnet（Sonnet 4.6）
    assert_eq!(
        get_internal_model_id("sonnet").expect("plain sonnet alias should resolve"),
        "claude-sonnet-4.6"
    );
    // 5.5 系列（Kiro 2026-10 官方清单新增）
    assert_eq!(
        get_internal_model_id("claude-opus-5-5").expect("opus 5.5 alias should map"),
        "claude-opus-5.5"
    );
    assert_eq!(
        get_internal_model_id("claude-sonnet-5-5").expect("sonnet 5.5 alias should map"),
        "claude-sonnet-5.5"
    );
    // Kimi K3 别名（开源模型，无 thinking 变体，-thinking 后缀被剥离）
    assert_eq!(
        get_internal_model_id("kimi-k3").expect("kimi-k3 alias should map"),
        "kimi-k3"
    );
    assert_eq!(
        get_internal_model_id("kimi").expect("plain kimi alias should map"),
        "kimi-k3"
    );
    // Claude Fable 5.1 别名（Enterprise 预览，按并集暴露）
    assert_eq!(
        get_internal_model_id("claude-fable-5-1").expect("fable 5.1 alias should map"),
        "claude-fable-5"
    );
    assert_eq!(
        get_internal_model_id("fable").expect("plain fable alias should map"),
        "claude-fable-5"
    );
}

#[test]
fn get_available_models_includes_claude_46_official_ids() {
    let model_ids: Vec<_> = get_available_models()
        .into_iter()
        .map(|model| model.id)
        .collect();

    // Kiro ListAvailableModels API 实际返回的是带点号的 ID。
    // 注意：Claude 模型只保留 -thinking 变体，不带后缀的已下线。
    assert!(model_ids
        .iter()
        .any(|id| id == "claude-opus-4.6-thinking"));
    assert!(model_ids
        .iter()
        .any(|id| id == "claude-sonnet-4.6-thinking"));
    // 4.6 之上的新版本
    assert!(model_ids.iter().any(|id| id == "claude-opus-4.7-thinking"));
    assert!(model_ids.iter().any(|id| id == "claude-opus-4.8-thinking"));
    assert!(model_ids
        .iter()
        .any(|id| id == "claude-sonnet-5-thinking"));
    // 2026-10 官方模型清单新增：Opus 5.5 / Sonnet 5.5 / Kimi K3
    assert!(model_ids
        .iter()
        .any(|id| id == "claude-opus-5.5-thinking"));
    assert!(model_ids
        .iter()
        .any(|id| id == "claude-sonnet-5.5-thinking"));
    assert!(model_ids.iter().any(|id| id == "kimi-k3"));
    // 2026-10 官方文档按并集列出：Fable 5.1（Enterprise 预览）
    assert!(model_ids.iter().any(|id| id == "claude-fable-5"));
    // Sonnet 4.0 已于 2026-10-14 EOL 并从 API 摘除，不应再出现在清单中
    // （注意前缀匹配会误伤 4.5/4.6，必须精确匹配退役 ID）
    assert!(!model_ids.iter().any(|id| {
        id == "claude-sonnet-4"
            || id == "claude-sonnet-4-thinking"
            || id.starts_with("claude-sonnet-4.0")
    }));
}

#[test]
fn get_internal_model_id_with_fallback_caps_to_sonnet_45_when_it_is_highest_available() {
    let available_models = vec![
        "auto".to_string(),
        "claude-sonnet-4.5".to_string(),
        "claude-haiku-4.5".to_string(),
        "deepseek-3.2".to_string(),
    ];

    for requested in [
        "claude-sonnet-5",
        "claude-opus-4.8",
        "claude-opus-4.7",
        "claude-opus-4.6",
        "claude-opus-4.5",
        "claude-sonnet-4.6",
    ] {
        assert_eq!(
            get_internal_model_id_with_fallback(requested, &available_models)
                .expect("model should fallback"),
            "claude-sonnet-4.5",
            "{requested} should be capped to highest available Claude model"
        );
    }
}
