//! Tool Search Tool（Anthropic 工具发现规范）网关侧实现。
//!
//! 对应官方规范 `tool_search_tool_regex_20251119` / `tool_search_tool_bm25_20251119`：
//! 客户端把大量工具标记 `defer_loading: true` 时，网关不全量下发，而是注入一个
//! 合成 `tool_search` 工具；模型按需检索被拆下的工具定义（拿到 schema 后才能
//! 正常发起 tool_use），从而把每请求的工具定义 token 从 O(全部工具) 压到
//! O(可见工具 + 检索结果)。
//!
//! 移植自社区 kiro2api（xu-yuanhao/KiroProxy `tool_search.py`，该文件本身未
//! 接线，本模块补齐了代理侧的完整语义）。
//!
//! 职责边界：
//! 1. 请求侧拆分：[`split_deferred_tools`] 把 defer_loading 工具从下发清单挪出，
//!    注入合成工具（见 [`synthetic_tool_search_tool`]）；
//! 2. 响应侧代答：模型发起 `tool_search` 调用时，[`execute_tool_search`] 对被
//!    拆下的工具执行检索，产出回灌上游的 tool_result 内容（完整定义 JSON，
//!    模型看到 schema 才能调用）。
//!
//! 非流式与流式均已接线（upstream.rs）：非流式为代答环直改请求；流式为
//! 缓冲代答环——缓冲整轮上游 EventStream 后跑同一套循环，最后一轮原始
//! 字节重建为上游响应交既有流式管线增量转换，客户端仍收标准 SSE，仅
//! 首包延迟包含缓冲与代答耗时。

use serde_json::{json, Value};
use std::collections::HashMap;

use super::*;
use crate::gateway::models::ToolFunction;

/// 合成检索工具的名称（模型 tool_use 用此名调用）。
pub const TOOL_SEARCH_TOOL_NAME: &str = "tool_search";
/// 单次检索默认返回条数（对齐规范与 KiroProxy DEFAULT_MAX_RESULTS）。
pub const DEFAULT_MAX_RESULTS: usize = 5;
/// select: 语法前缀（精确点名，不做模糊匹配）。
const SELECT_PREFIX: &str = "select:";

/// 官方长度上限（tool-search-tool 文档，20251119）：regex 模式 ≤200 字符，
/// BM25/关键词 query ≤500 字符。
const MAX_REGEX_PATTERN_LEN: usize = 200;
const MAX_BM25_QUERY_LEN: usize = 500;
/// 官方 limit 上限（默认 5，模型可指定 1..=10000）。
const MAX_LIMIT: usize = 10_000;

/// 判定模型的 tool_use 是否为 tool_search 检索调用。
///
/// 主要匹配注入的合成名 `tool_search`；同时兜住官方服务端工具名
/// （`tool_search_tool_regex` / `_bm25_`，含日期后缀）——这些名字在模型
/// 训练数据里，即使上下文只声明了合成名（客户端声明的官方名工具在
/// normalize 层已按 server tool 剥离，模型看不到），模型仍可能凭训练记忆
/// 幻觉调用。别名漏拦会把 tool_use 透传给客户端导致"未知工具"错误，
/// 而别名匹配零成本。
pub fn is_tool_search_call(name: &str) -> bool {
    name == TOOL_SEARCH_TOOL_NAME || name.starts_with("tool_search_tool_")
}

// ── 合成工具定义 ────────────────────────────────────────────────────────────

/// 构建注入给模型的合成 `tool_search` 工具定义。
///
/// schema 与 Anthropic 服务端工具同构：query 必填，type/limit 可选；
/// 描述里写清三种检索语法（关键词 / 正则 / select: 点名）。
pub fn synthetic_tool_search_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: TOOL_SEARCH_TOOL_NAME.to_string(),
            description: Some(
                "Search deferred tools by keyword, regex, or exact name to load their \
                 definitions before calling them. Query syntax:\n\
                 - plain text: BM25 ranked keyword search\n\
                 - `re:<pattern>`: regex match against tool names, descriptions and \
                 parameter descriptions (case-insensitive)\n\
                 - `select:Name1,Name2`: load exact tools by name\n\
                 Returns the full definitions (name/description/parameters) of the best \
                 matches; call the returned tools directly afterwards."
                    .to_string(),
            ),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    // query 与 pattern 都收：官方两种变体的入参名不同——
                    // tool_search_tool_bm25_* 用 query，tool_search_tool_regex_* 用 pattern。
                    // 模型按训练记忆可能发任一名字，schema 两个都声明以免它不敢用。
                    "query": {
                        "type": "string",
                        "description": "Search query: keywords, `re:<regex>`, or `select:Name1,Name2`. Max 500 chars (200 for `re:` patterns).",
                        "maxLength": 500
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern (official `tool_search_tool_regex` argument name; same as `re:` form of `query`). Max 200 chars.",
                        "maxLength": 200
                    },
                    "type": {
                        "type": "string",
                        "enum": ["bm25", "regex"],
                        "description": "Ranking strategy for plain-text queries. Default bm25.",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max tools to return. Default 5, max 10000."
                    }
                },
                "required": ["query"]
            })),
        },
        cache_control: None,
    }
}

// ── 检索引擎 ────────────────────────────────────────────────────────────────

/// BM25 参数（对齐 KiroProxy 与常规实现）。
const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;
/// 名字命中权重（正则打分用，名字命中远比描述命中可信）。
const REGEX_NAME_WEIGHT: usize = 10;

/// 拼接工具的可检索文本：名字 + 描述 + 参数名/参数描述。
fn build_tool_text(tool: &Tool) -> String {
    let mut parts = vec![tool.function.name.clone()];
    if let Some(desc) = &tool.function.description {
        parts.push(desc.clone());
    }
    if let Some(Value::Object(props)) = tool
        .function
        .parameters
        .as_ref()
        .and_then(|p| p.get("properties"))
    {
        for (prop_name, prop_def) in props {
            parts.push(prop_name.clone());
            if let Some(desc) = prop_def.get("description").and_then(Value::as_str) {
                parts.push(desc.to_string());
            }
        }
    }
    parts.join(" ")
}

/// BM25 分词：小写 + 非字母数字切分。
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && !c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// 正则检索：匹配工具文本，名字命中 ×10 加权，按分数降序取前 N。
fn search_regex(
    query: &str,
    tools: &HashMap<String, &Tool>,
    max_results: usize,
) -> Result<Vec<String>, String> {
    // 官方 regex 模式上限 200 字符
    if query.len() > MAX_REGEX_PATTERN_LEN {
        return Err("pattern_too_long".to_string());
    }
    // 对齐规范与合成工具描述：正则匹配大小写不敏感
    let pattern = regex::RegexBuilder::new(query)
        .case_insensitive(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut scored: Vec<(String, usize)> = Vec::new();
    for (name, tool) in tools {
        let text = build_tool_text(tool);
        let text_hits = pattern.find_iter(&text).count();
        if text_hits == 0 {
            continue;
        }
        let name_hits = pattern.find_iter(name).count();
        scored.push((name.clone(), text_hits + name_hits * REGEX_NAME_WEIGHT));
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(scored.into_iter().take(max_results).map(|(n, _)| n).collect())
}

/// BM25 检索：按 query 词项的 IDF×TF 计分排序，取前 N。
fn search_bm25(query: &str, tools: &HashMap<String, &Tool>, max_results: usize) -> Vec<String> {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() || tools.is_empty() {
        return Vec::new();
    }

    // 文档集：name -> 分词
    let mut docs: Vec<(String, Vec<String>)> = Vec::with_capacity(tools.len());
    for (name, tool) in tools {
        docs.push((name.clone(), tokenize(&build_tool_text(tool))));
    }
    let n = docs.len() as f64;
    let avg_dl = docs.iter().map(|(_, t)| t.len()).sum::<usize>() as f64 / n;

    // 词项文档频率
    let mut df: HashMap<&str, usize> = HashMap::new();
    for qt in &query_tokens {
        for (_, tokens) in &docs {
            if tokens.iter().any(|t| t == qt) {
                *df.entry(qt.as_str()).or_default() += 1;
            }
        }
    }

    let mut scored: Vec<(String, f64)> = Vec::new();
    for (name, tokens) in &docs {
        let dl = tokens.len() as f64;
        let tf: HashMap<&str, usize> = tokens.iter().fold(HashMap::new(), |mut acc, t| {
            *acc.entry(t.as_str()).or_default() += 1;
            acc
        });
        let mut score = 0.0f64;
        for qt in &query_tokens {
            let Some(&term_tf) = tf.get(qt.as_str()) else {
                continue;
            };
            let term_df = *df.get(qt.as_str()).unwrap_or(&0) as f64;
            let idf = ((n - term_df + 0.5) / (term_df + 0.5) + 1.0).ln();
            let tf_norm = (term_tf as f64 * (BM25_K1 + 1.0))
                / (term_tf as f64 + BM25_K1 * (1.0 - BM25_B + BM25_B * dl / avg_dl));
            score += idf * tf_norm;
        }
        if score > 0.0 {
            scored.push((name.clone(), score));
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(max_results).map(|(n, _)| n).collect()
}

/// 统一检索入口：`select:` 精确点名优先，否则按 type 分发（默认 BM25）。
///
/// 返回匹配的工具名（保持 deferred 清单里的原始名）。
pub fn search(
    query: &str,
    deferred_tools: &[Tool],
    search_type: &str,
    max_results: usize,
) -> Vec<String> {
    let max_results = if max_results == 0 {
        DEFAULT_MAX_RESULTS
    } else {
        max_results.min(MAX_LIMIT)
    };

    // 查询语法在分发器统一处理（单一事实源，execute_tool_search 与直调方共用）：
    // select: 精确点名优先于 re: 正则前缀。
    let (query, search_type) = match query.strip_prefix("re:") {
        Some(pattern) => (pattern.trim(), "regex"),
        None => (query, search_type),
    };

    let by_name: HashMap<String, &Tool> = deferred_tools
        .iter()
        .map(|t| (t.function.name.clone(), t))
        .collect();

    if let Some(selected) = query.strip_prefix(SELECT_PREFIX) {
        return selected
            .split(',')
            .map(str::trim)
            .filter(|name| by_name.contains_key(*name))
            .map(str::to_string)
            .take(max_results)
            .collect();
    }

    match search_type {
        "regex" => search_regex(query, &by_name, max_results).unwrap_or_default(),
        _ => search_bm25(query, &by_name, max_results),
    }
}

// ── 响应侧代答 ──────────────────────────────────────────────────────────────

/// 执行模型的 `tool_search` 调用，产出回灌上游的 tool_result 文本。
///
/// `arguments` 为模型 tool_use.input（{query, type?, limit?}）；返回值是
/// 匹配工具的完整定义 JSON 数组的文本形式——模型读到 schema 即可正常发起
/// 对应工具的 tool_use，客户端侧无感知（它本来就持有这些工具的执行权）。
///
/// 无匹配时返回提示文本（不是错误——空结果是合法检索结论）。
pub fn execute_tool_search(arguments: &Value, deferred_tools: &[Tool]) -> String {
    // 参数名双读：官方 regex 变体用 pattern、BM25 用 query——别名调用
    // （见 is_tool_search_call）可能带官方参数名
    let query = arguments
        .get("query")
        .or_else(|| arguments.get("pattern"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if query.is_empty() {
        return "tool_search: query is required".to_string();
    }
    let search_type = arguments
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("bm25");
    // 官方长度上限按变体区分：regex 模式 200，BM25/关键词 500。
    // 长度只算 pattern 本体——`re:` 是网关自定的语法前缀，不属于官方模式，
    // 计入会把恰好 200 字符的合法正则误拒（差 3 个字符）。
    let pattern_body = query.strip_prefix("re:").map(str::trim).unwrap_or(query);
    let is_regex = pattern_body.len() != query.len() || search_type == "regex";
    let max_len = if is_regex {
        MAX_REGEX_PATTERN_LEN
    } else {
        MAX_BM25_QUERY_LEN
    };
    if pattern_body.len() > max_len {
        return format!("tool_search: query too long (max {max_len} chars for this variant)");
    }
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_MAX_RESULTS as u64)
        .min(MAX_LIMIT as u64) as usize;

    // re: 前缀的强制正则语义由 search() 分发器统一处理
    let matched = search(query, deferred_tools, search_type, limit);
    if matched.is_empty() {
        return format!("tool_search: no deferred tool matches `{query}`");
    }

    let defs: Vec<Value> = deferred_tools
        .iter()
        .filter(|t| matched.contains(&t.function.name))
        .map(|t| {
            json!({
                "name": t.function.name,
                "description": t.function.description,
                "input_schema": t.function.parameters,
            })
        })
        .collect();
    format!(
        "Loaded {} deferred tool(s). Their full definitions follow; call them directly:\n{}",
        defs.len(),
        serde_json::to_string_pretty(&defs).unwrap_or_else(|_| "[]".to_string())
    )
}

// ── 请求侧拆分 ──────────────────────────────────────────────────────────────

/// 按名字把 defer_loading 工具从下发清单挪出，并注入合成 `tool_search` 工具。
///
/// `deferred_names` 由 normalize 层收集（Anthropic 工具定义的顶层
/// `defer_loading: true`）。返回被拆下的工具；调用方负责把它存到请求的
/// `deferred_tools` 上，供响应侧代答检索。
///
/// 语义保证：
/// - `deferred_names` 为空或全部未命中 → 不动 tools，返回空（不注入合成工具）；
/// - 合成工具始终追加在**末尾**，避免打乱客户端工具的缓存顺序；
/// - `tool_choice` 强制指定某工具时不做任何拆分（强制调用语义优先，
///   被指定的工具若被拆下模型将无法调用）。
pub fn split_deferred_tools(
    tools: &mut Vec<Tool>,
    deferred_names: &[String],
    tool_choice: Option<&Value>,
) -> Vec<Tool> {
    if deferred_names.is_empty() || tools.is_empty() {
        return Vec::new();
    }
    // tool_choice 强制具体工具时跳过拆分：此时模型被要求立即调用某个工具，
    // 若目标工具恰好被 defer 会直接破坏调用链。
    if let Some(choice) = tool_choice {
        let forced = choice.get("name").and_then(Value::as_str).or_else(|| {
            choice
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
        });
        if forced.is_some() {
            return Vec::new();
        }
    }

    let deferred_set: std::collections::HashSet<&str> =
        deferred_names.iter().map(String::as_str).collect();
    let mut split_off = Vec::new();
    tools.retain(|tool| {
        if deferred_set.contains(tool.function.name.as_str()) {
            split_off.push(tool.clone());
            false
        } else {
            true
        }
    });
    if split_off.is_empty() {
        return Vec::new();
    }
    tools.push(synthetic_tool_search_tool());
    split_off
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str, desc: &str, props: Value) -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                description: Some(desc.to_string()),
                parameters: Some(props),
            },
            cache_control: None,
        }
    }

    fn sample_tools() -> Vec<Tool> {
        vec![
            tool(
                "Read",
                "Read a file from the local filesystem",
                json!({"type":"object","properties":{"file_path":{"type":"string","description":"path to the file"}}}),
            ),
            tool(
                "WebFetch",
                "Fetch content from a URL over HTTP",
                json!({"type":"object","properties":{"url":{"type":"string","description":"target url"}}}),
            ),
            tool(
                "Grep",
                "Search file contents with a regular expression pattern",
                json!({"type":"object","properties":{"pattern":{"type":"string","description":"regex pattern"}}}),
            ),
        ]
    }

    #[test]
    fn bm25_ranks_by_relevance() {
        let tools = sample_tools();
        let hits = search("read file", &tools, "bm25", 5);
        assert_eq!(hits.first().map(String::as_str), Some("Read"));
        // WebFetch 与 query 无词项重叠，不应进前二
        assert!(!hits.iter().take(1).any(|n| n == "WebFetch"));
    }

    #[test]
    fn regex_weights_name_over_description() {
        let tools = sample_tools();
        // "read" 同时命中 Read 的名字与 Grep 的描述——名字加权应让 Read 居首
        let hits = search("re:read", &tools, "bm25", 5).join(",");
        assert!(hits.starts_with("Read"), "actual: {hits}");
    }

    #[test]
    fn regex_invalid_pattern_is_empty_not_panic() {
        let tools = sample_tools();
        assert!(search("re:[invalid", &tools, "regex", 5).is_empty());
    }

    #[test]
    fn select_syntax_loads_exact_names() {
        let tools = sample_tools();
        let hits = search("select: Grep, WebFetch, Missing", &tools, "bm25", 5);
        assert_eq!(hits, vec!["Grep".to_string(), "WebFetch".to_string()]);
    }

    #[test]
    fn split_moves_deferred_and_injects_synthetic() {
        let mut tools = sample_tools();
        tools.push(tool("HugeMcp", "some mcp tool", json!({})));
        let deferred = vec!["HugeMcp".to_string()];

        let split = split_deferred_tools(&mut tools, &deferred, None);

        assert_eq!(split.len(), 1);
        assert_eq!(split[0].function.name, "HugeMcp");
        // 合成工具追加在末尾
        assert_eq!(
            tools.last().unwrap().function.name,
            TOOL_SEARCH_TOOL_NAME
        );
        // 其余工具原位保留
        assert!(tools.iter().any(|t| t.function.name == "Read"));
    }

    #[test]
    fn split_skips_when_tool_choice_forced() {
        let mut tools = sample_tools();
        let deferred = vec!["Read".to_string()];
        let choice = Some(json!({"type": "tool", "name": "Read"}));

        let split = split_deferred_tools(&mut tools, &deferred, choice.as_ref());

        assert!(split.is_empty());
        assert!(tools.iter().any(|t| t.function.name == "Read"));
        assert!(!tools.iter().any(|t| t.function.name == TOOL_SEARCH_TOOL_NAME));
    }

    #[test]
    fn split_noop_without_deferred_names() {
        let mut tools = sample_tools();
        let before = tools.len();
        assert!(split_deferred_tools(&mut tools, &[], None).is_empty());
        assert_eq!(tools.len(), before);
    }

    #[test]
    fn execute_returns_full_definitions() {
        let tools = sample_tools();
        let out = execute_tool_search(&json!({"query": "read file"}), &tools);
        assert!(out.contains("Read"));
        assert!(out.contains("input_schema"), "应携带完整 schema: {out}");
    }

    #[test]
    fn execute_handles_empty_and_bad_input() {
        let tools = sample_tools();
        assert!(execute_tool_search(&json!({}), &tools).contains("query is required"));
        assert!(execute_tool_search(&json!({"query": "zzz_no_match"}), &tools)
            .contains("no deferred tool"));
    }

    #[test]
    fn synthetic_tool_schema_has_required_query() {
        let t = synthetic_tool_search_tool();
        let schema = t.function.parameters.unwrap();
        let required = schema.get("required").unwrap().as_array().unwrap();
        assert_eq!(required[0], "query");
    }

    /// 官方两种变体的入参名不同：bm25 用 `query`，regex 用 `pattern`。
    /// schema 必须两个都声明，且两个名字都能被 execute_tool_search 消费。
    #[test]
    fn schema_accepts_both_official_argument_names() {
        let schema = synthetic_tool_search_tool().function.parameters.unwrap();
        let props = schema.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("query"));
        assert!(props.contains_key("pattern"));
        assert_eq!(
            props["pattern"].get("maxLength").and_then(Value::as_u64),
            Some(MAX_REGEX_PATTERN_LEN as u64)
        );

        // 执行侧：只给 pattern 也应命中
        let tools = sample_tools();
        let by_pattern = execute_tool_search(&json!({"pattern": "file"}), &tools);
        let by_query = execute_tool_search(&json!({"query": "file"}), &tools);
        assert!(
            by_pattern.contains("Read"),
            "pattern argument must be consumed: {by_pattern}"
        );
        assert_eq!(by_pattern, by_query);
    }

    #[test]
    fn official_length_limits_enforced() {
        let tools = sample_tools();
        // 官方上限：regex 模式 200，BM25 query 500
        let long_pattern = format!("re:{}", "a".repeat(201));
        assert!(execute_tool_search(&json!({ "query": long_pattern }), &tools)
            .contains("query too long"));
        let long_query = "a".repeat(501);
        assert!(execute_tool_search(&json!({ "query": long_query }), &tools)
            .contains("query too long"));
        // 边界值内不报过长
        assert!(!execute_tool_search(&json!({ "query": "a".repeat(500) }), &tools)
            .contains("query too long"));
        assert!(!execute_tool_search(&json!({ "query": format!("re:{}", "a".repeat(200)) }), &tools)
            .contains("query too long"));
    }

    #[test]
    fn alias_names_match_interception() {
        // 官方服务端工具名（含日期后缀）与合成名都必须被拦截环识别
        assert!(is_tool_search_call("tool_search"));
        assert!(is_tool_search_call("tool_search_tool_regex"));
        assert!(is_tool_search_call("tool_search_tool_regex_20251119"));
        assert!(is_tool_search_call("tool_search_tool_bm25_20251119"));
        assert!(!is_tool_search_call("tool_search_results"));
        assert!(!is_tool_search_call("read"));
    }

    #[test]
    fn official_regex_alias_uses_pattern_param() {
        // 模型幻觉按官方名调用时可能带官方参数名 pattern（regex 变体），
        // 双读兼容后应正常检索而不是报 query is required
        let tools = sample_tools();
        let out = execute_tool_search(&json!({ "pattern": "read" }), &tools);
        assert!(out.contains("Read"), "actual: {out}");
        assert!(!out.contains("query is required"));
    }
}
