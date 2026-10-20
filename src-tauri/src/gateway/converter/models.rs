//! 模型 ID 映射：公网模型名 <-> Kiro 上游内部模型 ID，以及可用模型清单。

use super::*;

pub fn get_internal_model_id(external_model: &str) -> Result<String, String> {
    let normalized = normalize_external_model_alias(external_model);

    // 1. 特殊别名（简写 / latest / 特殊值）
    let model_id = match normalized.as_str() {
        "auto" | "default" => return Ok("auto".to_string()),
        // opus 系列：裸 "opus" 指向当前最新的 Opus（5）
        "opus" | "opus-5" | "opus-5-0" => return Ok("claude-opus-5".to_string()),
        "opus-5-5" | "opus-5.5" => return Ok("claude-opus-5.5".to_string()),
        "opus-4-7" => return Ok("claude-opus-4.7".to_string()),
        "sonnet" | "sonnet-4-6" => return Ok("claude-sonnet-4.6".to_string()),
        "sonnet-5-5" | "sonnet-5.5" => return Ok("claude-sonnet-5.5".to_string()),
        "haiku" | "haiku-4-5" => return Ok("claude-haiku-4.5".to_string()),
        "claude-sonnet-latest" => return Ok("claude-sonnet-5".to_string()),
        "claude-sonnet-5" => return Ok("claude-sonnet-5".to_string()),
        // Claude Fable 5.1（Enterprise 预览；上游内部 ID 经社区项目旁证为 claude-fable-5）
        "fable" | "fable-5" | "fable-5.1" | "claude-fable-5" | "claude-fable-5-1" => {
            return Ok("claude-fable-5".to_string())
        }
        // OpenAI GPT 兼容映射（默认映射到 claude-sonnet-4.5；用户可以在前端「模型映射」配置里覆盖。
        // 兜底目标曾指向 claude-sonnet-4，该模型已于 2026-10 从 ListAvailableModels 摘除，改指 4.5）
        "gpt-4" | "gpt-4o" | "gpt-4-turbo" | "gpt-3.5-turbo" | "gpt-4o-mini" => {
            return Ok("claude-sonnet-4.5".to_string());
        }
        // GPT-5.6 系列（Kiro 原生 GPT 模型，不做映射）
        "gpt-5-6-sol" | "gpt-5.6-sol" => return Ok("gpt-5.6-sol".to_string()),
        "gpt-5-6-terra" | "gpt-5.6-terra" => return Ok("gpt-5.6-terra".to_string()),
        "gpt-5-6-luna" | "gpt-5.6-luna" => return Ok("gpt-5.6-luna".to_string()),
        "gpt-5-6" | "gpt-5.6" | "gpt5.6" => return Ok("gpt-5.6-sol".to_string()),
        // 开源模型别名
        "deepseek-3-2" | "deepseek-3.2" | "deepseek" => return Ok("deepseek-3.2".to_string()),
        "minimax-m2-5" | "minimax-m2.5" | "minimax" => return Ok("minimax-m2.5".to_string()),
        "minimax-m2-1" | "minimax-m2.1" => return Ok("minimax-m2.1".to_string()),
        "glm-5" | "glm5" => return Ok("glm-5".to_string()),
        "kimi-k3" | "kimi-k-3" | "kimi" | "kimi-k3-thinking" => {
            return Ok("kimi-k3".to_string())
        }
        "qwen3-coder-next" | "qwen3-coder" | "qwen3" | "qwen" => {
            return Ok("qwen3-coder-next".to_string())
        }
        _ => &normalized,
    };

    // 2. 正则归一化：Anthropic 公开格式 → Kiro 内部格式
    //    claude-{family}-{major}-{minor}[-thinking][-日期] → claude-{family}-{major}.{minor}
    let normalized_model = normalize_claude_model_format(model_id);

    // 3. 兜底：如果归一化后仍然不像 Kiro 支持的格式，映射到默认 sonnet-4.5 避免直接 400
    //    向前兼容：claude-{sonnet|haiku|opus}-* 格式透传，假定 Kiro 后续新发布的版本格式不变
    if is_kiro_supported_model_format(&normalized_model) {
        Ok(normalized_model)
    } else {
        log::warn!(
            "[模型映射] 未知模型 \"{}\" → 兜底到 claude-sonnet-4.5",
            external_model
        );
        Ok("claude-sonnet-4.5".to_string())
    }
}

/// 判断模型 ID 是否符合 Kiro API 接受的格式
/// - claude-{sonnet|haiku|opus|fable}-{version} （包括 4.5 / 4.6 / 4.7 / 5 + 未来新版本）
/// - gpt-5.6-{sol|terra|luna} （Kiro 原生 GPT 模型）
/// - 开源模型：deepseek-3.2 / minimax-m2.5 / minimax-m2.1 / glm-5 / kimi-k3 / qwen3-coder-next
/// - 特殊值：auto
pub fn is_kiro_supported_model_format(model: &str) -> bool {
    if model == "auto" {
        return true;
    }
    if model.starts_with("claude-sonnet-")
        || model.starts_with("claude-haiku-")
        || model.starts_with("claude-opus-")
        || model.starts_with("claude-fable-")
    {
        return true;
    }
    if model.starts_with("gpt-5.6-") || model.starts_with("gpt-5-6-") {
        return true;
    }
    matches!(
        model,
        "deepseek-3.2"
            | "minimax-m2.5"
            | "minimax-m2.1"
            | "glm-5"
            | "kimi-k3"
            | "qwen3-coder-next"
    )
}

/// 将 Anthropic 公开模型名归一化为 Kiro 内部格式
///
/// 规则：
/// - 去掉日期后缀 -20xxxxxx（8位数字）
/// - 版本号横杠转点号：claude-{family}-{major}-{minor} → claude-{family}-{major}.{minor}
/// - 保留 -thinking 后缀（Kiro 通过模型 ID 区分是否启用思考）
/// - 已经是点号格式的直接返回
pub fn normalize_claude_model_format(model: &str) -> String {
    let mut s = model.to_string();

    // 去掉 -thinking 后缀（thinking 通过系统提示注入启用，Kiro API 不接受带 -thinking 的模型 ID）
    if let Some(stripped) = s.strip_suffix("-thinking") {
        s = stripped.to_string();
    }

    // 去掉日期后缀（-20xxxxxx，8位数字）
    if s.len() > 9 {
        let tail = &s[s.len() - 9..];
        if tail.starts_with('-')
            && tail[1..].chars().all(|c| c.is_ascii_digit())
            && tail[1..].starts_with("20")
        {
            s.truncate(s.len() - 9);
        }
    }

    // 版本号横杠转点号：claude-{family}-{major}-{minor} → claude-{family}-{major}.{minor}
    // 匹配模式：末尾是 -{digit}-{digit} 的情况
    if let Some(last_dash) = s.rfind('-') {
        let after_last = &s[last_dash + 1..];
        if after_last.len() == 1 && after_last.chars().all(|c| c.is_ascii_digit()) {
            // 检查倒数第二个 dash 后面是否也是单个数字
            let prefix = &s[..last_dash];
            if let Some(second_last_dash) = prefix.rfind('-') {
                let between = &prefix[second_last_dash + 1..];
                if between.len() == 1 && between.chars().all(|c| c.is_ascii_digit()) {
                    // claude-opus-4-7 → claude-opus-4.7
                    let base = &s[..second_last_dash + 1 + between.len()];
                    return format!("{}.{}", base, after_last);
                }
            }
        }
    }

    // GPT-5.6 系列横杠转点号：gpt-5-6-sol → gpt-5.6-sol
    // 匹配模式：gpt-{major}-{minor}-{variant}
    if s.starts_with("gpt-") {
        // 去掉 gpt- 前缀
        let rest = &s[4..];
        // 找第一个横杠（major 和 minor 之间）
        if let Some(first_dash) = rest.find('-') {
            let major = &rest[..first_dash];
            if major.len() == 1 && major.chars().all(|c| c.is_ascii_digit()) {
                let after_major = &rest[first_dash + 1..];
                // 找第二个横杠（minor 和 variant 之间）
                if let Some(second_dash) = after_major.find('-') {
                    let minor = &after_major[..second_dash];
                    if minor.len() == 1 && minor.chars().all(|c| c.is_ascii_digit()) {
                        let variant = &after_major[second_dash + 1..];
                        // gpt-5-6-sol → gpt-5.6-sol
                        return format!("gpt-{}.{}-{}", major, minor, variant);
                    }
                }
            }
        }
    }

    s
}

/// 带降级的模型映射函数
///
/// 根据账号可用模型列表（来自 ListAvailableModels API），自动将不可用的模型降级
///
/// ## 降级策略
///
/// Free 用户可用模型：sonnet-4.5, sonnet-4, haiku-4.5, 开源模型
/// Free 用户不可用：所有 Opus 系列、Sonnet 4.6+
/// GPT-5.6 不可用时降级到 gpt-5.6-luna（最便宜变体），避免跨协议降级
///
/// 简单策略：所有不可用模型一律降级到 claude-sonnet-4.5（保留 -thinking 后缀）
pub fn get_internal_model_id_with_fallback(
    external_model: &str,
    available_models: &[String],
) -> Result<String, String> {
    let mapped_model = get_internal_model_id(external_model)?;

    // 检查是否在可用列表中
    if available_models.contains(&mapped_model) {
        return Ok(mapped_model);
    }

    // GPT-5.6 系列降级到最便宜的 luna 变体（避免跨协议降级到 Claude）
    if mapped_model.starts_with("gpt-5.6-") {
        if available_models.contains(&"gpt-5.6-luna".to_string()) {
            log::warn!(
                "[Gateway] 模型 {} 不在可用列表中，降级到 gpt-5.6-luna",
                mapped_model
            );
            return Ok("gpt-5.6-luna".to_string());
        }
        // 如果连 luna 都没有，再降级到 Claude
    }

    // 简单粗暴：一律降级到 claude-sonnet-4.5（Free 用户最高可用模型）。
    // 实测证据（python-prototype/kiro_probe.py，2026-10-20）：带 -thinking 后缀的
    // modelId 会被上游 400 INVALID_MODEL_ID——上游靠 system prompt 的 <thinking_mode>
    // 标签启用思考，与模型 ID 无关，所以降级结果一律用裸 ID，thinking 由 payload 层注入。
    let fallback = "claude-sonnet-4.5";

    log::warn!(
        "[Gateway] 模型 {} 不在可用列表中，降级到 {}",
        mapped_model,
        fallback
    );

    Ok(fallback.to_string())
}

pub fn normalize_external_model_alias(external_model: &str) -> String {
    external_model.trim().to_ascii_lowercase()
}

/// 判断模型是否支持顶层 additionalModelRequestFields（官方 thinking 通道）。
///
/// 社区实测证据（2026-10，kiro.rs / OmniRoute / kiro2cc-proxy）：
/// - 4.6 代际起（含 5.x / Fable）接受 `thinking: { type: "adaptive" }`，上游返回
///   原生 reasoningContentEvent（带签名）；
/// - 4.5 代际与 GPT-5.6 系列不支持该字段，盲传会被上游 400 或整体跳过，
///   只能继续走 `<thinking_mode>` 标签；
/// - 开源模型与 auto 无证据支持，保守跳过（auto 会路由到任意模型，不能赌）。
pub fn supports_additional_model_request_fields(model_id: &str) -> bool {
    if model_id.starts_with("gpt-5.6-") || model_id.starts_with("gpt-5-6-") {
        return false;
    }
    // 4.5 代际（sonnet-4.5 / opus-4.5 / haiku-4.5 及更早）与全部 haiku
    if model_id.starts_with("claude-haiku-")
        || model_id.contains("4.5")
        || model_id.contains("4-5")
        || model_id.contains("4.0")
        || model_id.contains("4-0")
    {
        return false;
    }
    model_id.starts_with("claude-fable-")
        || (model_id.contains("opus")
            && (model_id.contains("4.6")
                || model_id.contains("4-6")
                || model_id.contains("opus-5")))
        || (model_id.contains("sonnet")
            && (model_id.contains("4.6")
                || model_id.contains("4-6")
                || model_id.contains("sonnet-5")))
}

pub fn get_available_models() -> Vec<ModelInfo> {
    // 数据来源：Kiro ListAvailableModels API 实际返回的裸模型 ID（经真机核实，
    // 上游返回不含 -thinking 后缀）。这里的 Claude -thinking 条目是网关合成的
    // 对外变体：发上游前由 normalize_claude_model_format 剥掉后缀，思考由
    // system prompt 的 <thinking_mode> 标签启用。
    // 注意：claude-sonnet-4 已于 2026-10-14 EOL 并从 API 摘除，不再列出。
    //       GPT-5.6 系列是 Kiro 原生 GPT 模型，没有 thinking 变体
    [
        // 自动选择
        ("auto", "anthropic"),
        // Claude 系列（仅 thinking 版本；fable 为 Enterprise 预览，上游无 thinking 变体证据，按裸 ID 列出）
        ("claude-opus-5.5-thinking", "anthropic"),
        ("claude-sonnet-5.5-thinking", "anthropic"),
        ("claude-fable-5", "anthropic"),
        ("claude-sonnet-5-thinking", "anthropic"),
        ("claude-opus-4.8-thinking", "anthropic"),
        ("claude-opus-4.7-thinking", "anthropic"),
        ("claude-opus-4.6-thinking", "anthropic"),
        ("claude-sonnet-4.6-thinking", "anthropic"),
        ("claude-opus-4.5-thinking", "anthropic"),
        ("claude-sonnet-4.5-thinking", "anthropic"),
        ("claude-haiku-4.5-thinking", "anthropic"),
        // GPT-5.6 系列（Kiro 原生 GPT 模型）
        ("gpt-5.6-sol", "openai"),
        ("gpt-5.6-terra", "openai"),
        ("gpt-5.6-luna", "openai"),
        // 开源模型
        ("deepseek-3.2", "deepseek"),
        ("minimax-m2.5", "minimax"),
        ("minimax-m2.1", "minimax"),
        ("glm-5", "zhipu"),
        ("kimi-k3", "moonshot"),
        ("qwen3-coder-next", "alibaba"),
    ]
    .into_iter()
    .map(|(id, owner)| ModelInfo {
        id: id.to_string(),
        object: "model".to_string(),
        created: 1_700_000_000,
        owned_by: owner.to_string(),
    })
    .collect()
}
