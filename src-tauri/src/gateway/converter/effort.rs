//! Effort 档位解析：由上游 `ListAvailableModels` 返回的
//! `additionalModelRequestFieldsSchema` 推导每个模型支持的推理档位。
//!
//! 权威来源是上游自己的 schema（Kiro 官方文档 kiro.dev/docs/models/effort：
//! "Each effort-capable model defines the values it accepts for reasoning-related
//! fields in its configuration schema"），因此模型换代、档位增删都无需改代码——
//! 只要上游 schema 变了，本模块自动跟随。
//!
//! schema 里同时决定三件事：
//! 1. 该模型是否支持顶层 `additionalModelRequestFields`（有 schema 才支持）；
//! 2. effort 走哪个字段——Claude 系用 `output_config.effort`，GPT-5.6 系用
//!    `reasoning.effort`（官方表：GPT-5.6 三档全用 reasoning.effort，且多一档 none）；
//! 3. 允许的档位枚举（`max` 只在部分模型可用，4.6 代无 `xhigh`）。
//!
//! 上游清单取不到时（离线 / 接口失败）退回官方文档的静态表，行为不降级。

use super::*;

/// effort 字段所在位置（决定 AMRF 内层信封的形状）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortField {
    /// Claude 系：`output_config.effort`，需与 `thinking` 信封同层下发。
    OutputConfig,
    /// GPT-5.6 系：`reasoning.effort`，且不能带 Claude 的 thinking/output_config
    /// 信封——上游会当未知字段拒收（社区 OmniRoute 实测）。
    Reasoning,
}

/// 上游 schema 里某字段的取值枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortEnum {
    pub field: EffortField,
    pub levels: Vec<String>,
}

/// 档位强度排序（用于在模型不支持某档时取最近的有效档）。
///
/// 官方定义（Kiro docs）：low < medium < high < xhigh < max。
/// `none` 不在此列——它是"不思考"而非强度档位，不能参与强度比较。
const EFFORT_RANK: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

fn effort_rank(level: &str) -> Option<usize> {
    EFFORT_RANK.iter().position(|item| *item == level)
}

/// 把请求的档位夹到允许的枚举内。
///
/// 规则（与社区 kirocc 一致）：
/// - 档位在枚举内 → 原样保留；
/// - 档位不在枚举内但强度可比（如 4.6 代收到 xhigh，而 4.6 只有
///   low/medium/high/max）→ **向上取最近的有效档**（xhigh → max），
///   保留客户端"更强思考"的意图；kirocc 同样是 clamp 到 max；
/// - 档位无法比较强度（如 none / 自造字符串）→ None，即不下发该字段
///   （kirocc：unrecognized strings are dropped），让模型用自身默认档。
fn clamp_to_levels(requested: &str, allowed: &[String]) -> Option<String> {
    if allowed.is_empty() {
        return None;
    }

    let requested = requested.trim().to_ascii_lowercase();
    if allowed.iter().any(|level| *level == requested) {
        return Some(requested);
    }

    let requested_rank = effort_rank(&requested)?;

    // 向上取最近的有效档；全都不够强时取枚举里强度最高的档
    let mut best_higher: Option<(usize, &String)> = None;
    let mut best_lower: Option<(usize, &String)> = None;
    for level in allowed {
        let Some(rank) = effort_rank(level) else {
            continue;
        };
        if rank >= requested_rank {
            if best_higher.map(|(best, _)| rank < best).unwrap_or(true) {
                best_higher = Some((rank, level));
            }
        } else if best_lower.map(|(best, _)| rank > best).unwrap_or(true) {
            best_lower = Some((rank, level));
        }
    }

    best_higher
        .map(|(_, level)| level.clone())
        .or_else(|| best_lower.map(|(_, level)| level.clone()))
}

/// 官方 `m4e`：thinking 关闭时不得请求的深层档位。
/// Kiro 1.4.5 `ySe(effortLevel, effortLevels)`：命中即改取枚举里**最后一个**
/// 不在本集合内的档（`Cbt` 只在 `thinkingType==="disabled"` 时走这条）。
const THINKING_DISABLED_EXCLUDED_LEVELS: [&str; 2] = ["xhigh", "max"];

/// thinking 关闭时的档位降级（官方 `ySe`）。
///
/// 语义：思考关着就不该请求最深思考。命中 `m4e` 时从枚举尾部往前找第一个
/// 不属 `m4e` 的档；枚举本就没有可降的档时返回 None（不下发该字段）。
fn downgrade_level_when_thinking_disabled(requested: &str, allowed: &[String]) -> Option<String> {
    let requested = requested.trim().to_ascii_lowercase();
    if !THINKING_DISABLED_EXCLUDED_LEVELS.contains(&requested.as_str()) {
        return None;
    }
    allowed
        .iter()
        .rev()
        .find(|level| !THINKING_DISABLED_EXCLUDED_LEVELS.contains(&level.trim().to_ascii_lowercase().as_str()))
        .cloned()
}

/// 官方 `tAu`：schema 里 `thinking.properties.type.enum` 同时含
/// `"disabled"` 与 `"adaptive"` 时，该模型才有思考开关。
/// 不含则官方 UI 根本不展示开关，我们也不应下发 thinking 信封（G-2）。
fn thinking_is_toggleable(schema: &Value) -> bool {
    let Some(levels) = schema
        .get("properties")
        .and_then(|value| value.get("thinking"))
        .and_then(|value| value.get("properties"))
        .and_then(|value| value.get("type"))
        .and_then(|value| value.get("enum"))
        .and_then(|value| value.as_array())
    else {
        return false;
    };
    let has = |target: &str| {
        levels
            .iter()
            .any(|level| level.as_str() == Some(target))
    };
    has("disabled") && has("adaptive")
}

/// 从 schema 解析 effort 枚举。
///
/// 依次尝试 `output_config.properties.effort.enum` 与
/// `reasoning.properties.effort.enum`——前者是 Claude 系的官方形态，后者是
/// GPT-5.6 系的官方形态。必须按上游声明的字段来，不能一律用 output_config。
pub fn parse_effort_enum(schema: &Value) -> Option<EffortEnum> {
    let properties = schema.get("properties")?.as_object()?;

    for (key, field) in [
        ("output_config", EffortField::OutputConfig),
        ("reasoning", EffortField::Reasoning),
    ] {
        let Some(levels) = properties
            .get(key)
            .and_then(|value| value.get("properties"))
            .and_then(|value| value.get("effort"))
            .and_then(|value| value.get("enum"))
            .and_then(|value| value.as_array())
        else {
            continue;
        };
        let levels: Vec<String> = levels
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect();
        if !levels.is_empty() {
            return Some(EffortEnum { field, levels });
        }
    }

    None
}

/// 是否是 Kiro 的 auto 路由模型（不是固定 SKU，服务模型由上游决定）。
pub fn is_auto_model(model_id: &str) -> bool {
    let model = model_id.trim().to_ascii_lowercase();
    model == "auto" || model == "default"
}

/// 从上游模型条目数组中取出模型 ID 列表（用于模型降级判定）。
pub fn extract_model_ids(models: &[Value]) -> Vec<String> {
    models
        .iter()
        .filter_map(|model| {
            model
                .get("modelId")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .collect()
}

/// 上游清单派生的档位档案。
#[derive(Debug, Clone, Default)]
pub struct ModelEffortProfile {
    /// 上游清单里的全部模型 ID（用于区分「上游明确说不支持」与「清单里没有」）
    model_ids: std::collections::HashSet<String>,
    /// model_id → 该模型声明的 AMRF schema（存在即代表支持顶层 AMRF）
    schemas: std::collections::HashMap<String, Value>,
    /// model_id → effort 枚举
    profiles: std::collections::HashMap<String, EffortEnum>,
    /// model_id → 该模型是否有思考开关（官方 `tAu` 判定）
    toggleable_thinking: std::collections::HashSet<String>,
    /// model_id → 输出窗口上限（上游 tokenLimits.maxOutputTokens）
    output_limits: std::collections::HashMap<String, i64>,
}

impl ModelEffortProfile {
    pub fn from_models(models: &[Value]) -> Self {
        let mut model_ids = std::collections::HashSet::new();
        let mut schemas = std::collections::HashMap::new();
        let mut profiles = std::collections::HashMap::new();
        let mut toggleable_thinking = std::collections::HashSet::new();
        let mut output_limits = std::collections::HashMap::new();
        for model in models {
            let Some(model_id) = model.get("modelId").and_then(Value::as_str) else {
                continue;
            };
            model_ids.insert(model_id.to_string());

            // 输出窗口上限（上游 tokenLimits.maxOutputTokens）
            if let Some(limit) = model
                .get("tokenLimits")
                .and_then(|limits| limits.get("maxOutputTokens"))
                .and_then(Value::as_i64)
            {
                output_limits.insert(model_id.to_string(), limit);
            }

            let Some(schema) = model.get("additionalModelRequestFieldsSchema") else {
                continue;
            };
            if !schema.is_object() {
                continue;
            }
            if thinking_is_toggleable(schema) {
                toggleable_thinking.insert(model_id.to_string());
            }
            if let Some(effort) = parse_effort_enum(schema) {
                profiles.insert(model_id.to_string(), effort);
            }
            schemas.insert(model_id.to_string(), schema.clone());
        }
        Self {
            model_ids,
            schemas,
            profiles,
            toggleable_thinking,
            output_limits,
        }
    }

    /// 该模型是否出现在上游清单里（用于区分"上游明确声明不支持"与"清单里没有"）。
    pub fn has_model(&self, model_id: &str) -> bool {
        self.model_ids.contains(model_id)
    }

    /// 上游是否为该模型声明了 AMRF schema（声明即支持）。
    ///
    /// `auto` 例外：它是 Kiro 的路由模型，实际服务模型由上游决定（官方 effort
    /// 文档：「Auto maintains a high quality bar...」），不是固定 SKU。给 auto
    /// 下发 `output_config.effort` 存在落到不支持该档位的模型上的风险，
    /// 因此无论上游是否给它挂 schema，一律排除。
    pub fn supports_amrf(&self, model_id: &str) -> bool {
        if is_auto_model(model_id) {
            return false;
        }
        self.schemas.contains_key(model_id)
    }

    /// 该模型使用的 effort 字段。
    pub fn effort_field(&self, model_id: &str) -> Option<EffortField> {
        self.profiles.get(model_id).map(|effort| effort.field)
    }

    /// 该模型允许的档位枚举。
    pub fn allowed_levels(&self, model_id: &str) -> Option<&[String]> {
        self.profiles
            .get(model_id)
            .map(|effort| effort.levels.as_slice())
    }

    /// 该模型的输出窗口上限（上游 tokenLimits.maxOutputTokens）。
    pub fn output_limit(&self, model_id: &str) -> Option<i64> {
        self.output_limits.get(model_id).copied()
    }

    /// 该模型是否声明了思考开关（`thinking.type.enum` 含 disabled+adaptive）。
    ///
    /// 上游清单里**有**该模型却未声明开关 → 肯定没有；清单里**没有**该模型
    /// （取不到清单 / 模型不在账号权限内）→ 交给调用方决定（保持旧行为）。
    pub fn thinking_toggleable(&self, model_id: &str) -> Option<bool> {
        if !self.model_ids.contains(model_id) {
            return None;
        }
        Some(self.toggleable_thinking.contains(model_id))
    }

    /// 上游 schema 是否声明了 max_tokens 字段。
    ///
    /// 用于判断能不能把客户端的输出长度上限放进 AMRF——上游没声明的字段硬发
    /// 可能被当未知字段拒收（社区 OmniRoute 实测：GPT-5.6 系会拒收 Claude
    /// 的 output_config/thinking 信封）。
    pub fn declares_max_tokens(&self, model_id: &str) -> bool {
        self.schemas
            .get(model_id)
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object)
            .map(|properties| properties.contains_key("max_tokens"))
            .unwrap_or(false)
    }

    /// 把客户端请求的档位归一化到该模型允许的档位：thinking 关闭时走官方
    /// `ySe` 降级（G-1），开启时直接 clamp 到枚举。
    pub fn resolve_level_with_thinking_state(
        &self,
        model_id: &str,
        requested: &str,
        thinking_disabled: bool,
    ) -> Option<String> {
        let allowed = self.allowed_levels(model_id)?;
        if thinking_disabled {
            // 官方先降档（ySe）再归一到枚举；降档后的值必然在枚举内
            if let Some(downgraded) = downgrade_level_when_thinking_disabled(requested, allowed) {
                return Some(downgraded);
            }
        }
        clamp_to_levels(requested, allowed)
    }
}

/// 官方文档静态表兜底：Kiro docs/models/effort 的 "Reasoning configuration
/// examples" 一节。仅在上游清单取不到时使用。
pub fn fallback_allowed_levels(model_id: &str) -> Option<Vec<String>> {
    let model = model_id.trim().to_ascii_lowercase();

    // GPT-5.6 系：reasoning.effort，多一档 none
    if model.starts_with("gpt-5.6-") || model.starts_with("gpt-5-6-") {
        return Some(vec![
            "none".into(),
            "low".into(),
            "medium".into(),
            "high".into(),
            "xhigh".into(),
            "max".into(),
        ]);
    }

    // Claude 4.6 代：low/medium/high/max（无 xhigh）
    if model.contains("4.6") || model.contains("4-6") {
        return Some(vec!["low".into(), "medium".into(), "high".into(), "max".into()]);
    }

    // Claude 4.7+ / 5.x / Fable：5 档
    if model.contains("fable")
        || model.contains("opus")
        || model.contains("sonnet")
        || model.contains("haiku")
    {
        return Some(vec![
            "low".into(),
            "medium".into(),
            "high".into(),
            "xhigh".into(),
            "max".into(),
        ]);
    }

    None
}

/// 官方文档静态表兜底：该模型的 effort 字段位置。
pub fn fallback_effort_field(model_id: &str) -> Option<EffortField> {
    let model = model_id.trim().to_ascii_lowercase();
    if model.starts_with("gpt-5.6-") || model.starts_with("gpt-5-6-") {
        Some(EffortField::Reasoning)
    } else {
        Some(EffortField::OutputConfig)
    }
}

/// 统一的档位解析入口：上游声明优先，缺失时用官方静态表兜底。
///
/// 兜底的适用范围有讲究：
/// - 上游清单里该模型**声明了** effort 枚举 → 完全按上游来；
/// - 上游清单里该模型**存在但没声明** effort → 不下发 effort（上游没说的档位
///   它多半不认，硬发会 400）；
/// - 该模型**不在**上游清单里（清单取不到、或模型确实不在账号权限内）
///   → 用官方静态表兜底，行为不降级。
pub fn resolve_effort_level(
    profile: Option<&ModelEffortProfile>,
    model_id: &str,
    requested: &str,
    thinking_disabled: bool,
) -> Option<String> {
    if let Some(profile) = profile {
        if profile.allowed_levels(model_id).is_some() {
            return profile.resolve_level_with_thinking_state(model_id, requested, thinking_disabled);
        }
        // 上游清单里明确有该模型，却没声明 effort 枚举 → 不下发
        if profile.has_model(model_id) {
            return None;
        }
    }
    let allowed = fallback_allowed_levels(model_id)?;
    if thinking_disabled {
        if let Some(downgraded) = downgrade_level_when_thinking_disabled(requested, &allowed) {
            return Some(downgraded);
        }
    }
    clamp_to_levels(requested, &allowed)
}
