//! Prompt Cache 模拟器
//! 在 kiro2api 侧追踪 cache_control 断点，模拟 Anthropic 的 prompt caching 行为
//! 让 Claude Code 的 cache_control 字段产生实际效果的 usage 统计

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// 常量
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5 * 60); // 5 分钟
const ONE_HOUR_CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 小时
const DEFAULT_MIN_CACHEABLE_TOKENS: usize = 1024;
const OPUS_MIN_CACHEABLE_TOKENS: usize = 4096;
const MAX_CACHE_RATIO: f64 = 0.85; // 最新内容不可能 100% 缓存命中
const MAX_ENTRIES_PER_ACCOUNT: usize = 200;

/// 缓存使用统计
#[derive(Debug, Clone, Default)]
pub struct CacheUsage {
    pub cache_creation_input_tokens: usize,
    pub cache_read_input_tokens: usize,
}

/// 缓存断点
#[derive(Debug, Clone)]
struct CacheBreakpoint {
    fingerprint: [u8; 32],
    cumulative_tokens: usize,
    ttl: Duration,
}

/// 缓存 Profile（一次请求的缓存结构）
#[derive(Debug, Clone)]
pub struct CacheProfile {
    breakpoints: Vec<CacheBreakpoint>,
    total_input_tokens: usize,
    model: String,
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    expires_at: Instant,
    ttl: Duration,
}

/// 可缓存的内容块
struct CacheableBlock {
    value: String,
    tokens: usize,
    ttl: Duration,
    is_message_end: bool,
}

/// Prompt Cache Tracker（全局单例）
pub struct PromptCacheTracker {
    entries_by_account: Mutex<HashMap<String, HashMap<[u8; 32], CacheEntry>>>,
}

impl PromptCacheTracker {
    pub fn new() -> Self {
        Self {
            entries_by_account: Mutex::new(HashMap::new()),
        }
    }

    /// 从 Anthropic 格式请求构建缓存 profile
    pub fn build_profile(
        &self,
        system: Option<&serde_json::Value>,
        messages: &[serde_json::Value],
        tools: Option<&[serde_json::Value]>,
        total_input_tokens: usize,
        model: &str,
    ) -> Option<CacheProfile> {
        let blocks = self.flatten_cache_blocks(system, messages, tools);
        if blocks.is_empty() {
            return None;
        }

        let mut hasher = Sha256::new();
        let mut breakpoints = Vec::new();
        let mut cumulative_tokens = 0usize;
        let mut active_ttl = Duration::ZERO;

        for block in &blocks {
            self.hash_chunk(&mut hasher, &block.value);
            cumulative_tokens += block.tokens;

            let breakpoint_ttl = if block.ttl > Duration::ZERO {
                active_ttl = block.ttl;
                block.ttl
            } else if block.is_message_end && active_ttl > Duration::ZERO {
                active_ttl
            } else {
                Duration::ZERO
            };

            if breakpoint_ttl == Duration::ZERO {
                continue;
            }

            let fingerprint: [u8; 32] = hasher.clone().finalize().into();
            breakpoints.push(CacheBreakpoint {
                fingerprint,
                cumulative_tokens,
                ttl: breakpoint_ttl,
            });
        }

        if breakpoints.is_empty() {
            return None;
        }

        Some(CacheProfile {
            breakpoints,
            total_input_tokens: total_input_tokens.max(cumulative_tokens),
            model: model.to_string(),
        })
    }

    /// 计算缓存命中情况
    pub fn compute(&self, account_id: &str, profile: &CacheProfile) -> CacheUsage {
        if profile.breakpoints.is_empty() || account_id.is_empty() {
            return CacheUsage::default();
        }

        let min_tokens = self.min_cacheable_tokens(&profile.model);
        let last = &profile.breakpoints[profile.breakpoints.len() - 1];
        let mut last_tokens = last.cumulative_tokens.min(profile.total_input_tokens);
        let now = Instant::now();

        let mut entries_map = self
            .entries_by_account
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.prune_expired(&mut entries_map, now);

        let entries = entries_map.get_mut(account_id);
        if entries.is_none() || entries.as_ref().unwrap().is_empty() {
            // 首次请求：全部是 creation
            let effective_creation = if last_tokens >= min_tokens {
                last_tokens
            } else {
                0
            };
            return CacheUsage {
                cache_creation_input_tokens: effective_creation,
                cache_read_input_tokens: 0,
            };
        }

        let entries = entries.unwrap();

        // 上限 85%
        let max_cacheable = (profile.total_input_tokens as f64 * MAX_CACHE_RATIO) as usize;
        if last_tokens > max_cacheable {
            last_tokens = max_cacheable;
        }

        // 从后往前匹配最长前缀
        let mut matched_tokens = 0usize;
        for bp in profile.breakpoints.iter().rev() {
            if bp.cumulative_tokens < min_tokens {
                continue;
            }
            if let Some(entry) = entries.get_mut(&bp.fingerprint) {
                if entry.expires_at > now {
                    // 命中：刷新过期时间
                    entry.expires_at = now + entry.ttl;
                    matched_tokens = bp.cumulative_tokens.min(profile.total_input_tokens);
                    if matched_tokens > last_tokens {
                        matched_tokens = last_tokens;
                    }
                    break;
                }
            }
        }

        let creation = last_tokens.saturating_sub(matched_tokens);
        CacheUsage {
            cache_creation_input_tokens: creation,
            cache_read_input_tokens: matched_tokens,
        }
    }

    /// 更新缓存条目（请求成功后调用）
    pub fn update(&self, account_id: &str, profile: &CacheProfile) {
        if profile.breakpoints.is_empty() || account_id.is_empty() {
            return;
        }

        let min_tokens = self.min_cacheable_tokens(&profile.model);
        let now = Instant::now();

        let mut entries_map = self
            .entries_by_account
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let entries = entries_map.entry(account_id.to_string()).or_default();

        for bp in &profile.breakpoints {
            if bp.cumulative_tokens < min_tokens {
                continue;
            }
            entries.insert(
                bp.fingerprint,
                CacheEntry {
                    expires_at: now + bp.ttl,
                    ttl: bp.ttl,
                },
            );
        }

        // 限制条目数
        if entries.len() > MAX_ENTRIES_PER_ACCOUNT {
            let mut sorted: Vec<_> = entries.iter().map(|(k, v)| (*k, v.expires_at)).collect();
            sorted.sort_by_key(|(_, exp)| *exp);
            let to_remove = entries.len() - MAX_ENTRIES_PER_ACCOUNT;
            for (key, _) in sorted.iter().take(to_remove) {
                entries.remove(key);
            }
        }
    }

    // ============ 内部方法 ============

    fn flatten_cache_blocks(
        &self,
        system: Option<&serde_json::Value>,
        messages: &[serde_json::Value],
        tools: Option<&[serde_json::Value]>,
    ) -> Vec<CacheableBlock> {
        let mut blocks = Vec::new();
        let default_ttl = Duration::from_secs(5 * 60); // 5 分钟默认 TTL

        // 工具定义（自动可缓存）
        if let Some(tools) = tools {
            for tool in tools {
                let value = self.canonicalize(tool);
                let tokens = estimate_tokens(&value);
                let ttl = self.extract_ttl(tool);
                blocks.push(CacheableBlock {
                    value,
                    tokens,
                    ttl: if ttl > Duration::ZERO {
                        ttl
                    } else {
                        default_ttl
                    },
                    is_message_end: false,
                });
            }
        }

        // System prompt（自动可缓存）
        if let Some(system) = system {
            match system {
                serde_json::Value::String(s) => {
                    let tokens = estimate_tokens(s);
                    blocks.push(CacheableBlock {
                        value: self.canonicalize(system),
                        tokens,
                        ttl: default_ttl,
                        is_message_end: false,
                    });
                }
                serde_json::Value::Array(arr) => {
                    for block in arr {
                        let value = self.canonicalize(block);
                        let tokens = estimate_tokens(&value);
                        let ttl = self.extract_ttl(block);
                        blocks.push(CacheableBlock {
                            value,
                            tokens,
                            ttl: if ttl > Duration::ZERO {
                                ttl
                            } else {
                                default_ttl
                            },
                            is_message_end: false,
                        });
                    }
                }
                _ => {}
            }
        }

        // Messages（只有显式标记 cache_control 的才可缓存）
        for (i, msg) in messages.iter().enumerate() {
            let content = msg.get("content");
            let _is_last_msg = i == messages.len() - 1;

            match content {
                Some(serde_json::Value::String(s)) => {
                    let value = self.canonicalize(msg);
                    let tokens = estimate_tokens(s);
                    let ttl = self.extract_ttl(msg);
                    blocks.push(CacheableBlock {
                        value,
                        tokens,
                        ttl,
                        is_message_end: true,
                    });
                }
                Some(serde_json::Value::Array(arr)) => {
                    let last_idx = arr.len().saturating_sub(1);
                    for (j, block) in arr.iter().enumerate() {
                        let value = self.canonicalize(block);
                        let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        let tokens = estimate_tokens(if text.is_empty() { &value } else { text });
                        let ttl = self.extract_ttl(block);
                        blocks.push(CacheableBlock {
                            value,
                            tokens,
                            ttl,
                            is_message_end: j == last_idx,
                        });
                    }
                }
                _ => {}
            }
        }

        blocks
    }

    fn extract_ttl(&self, value: &serde_json::Value) -> Duration {
        let cache_control = value.get("cache_control");
        let Some(cc) = cache_control else {
            return Duration::ZERO;
        };
        let Some(cc_type) = cc.get("type").and_then(|t| t.as_str()) else {
            return Duration::ZERO;
        };
        if !cc_type.eq_ignore_ascii_case("ephemeral") {
            return Duration::ZERO;
        }
        // 检查 ttl 字段
        if let Some(ttl_val) = cc.get("ttl") {
            if let Some(s) = ttl_val.as_str() {
                if s == "1h" || s == "1H" {
                    return ONE_HOUR_CACHE_TTL;
                }
            }
            if let Some(n) = ttl_val.as_u64() {
                if n > 0 {
                    return Duration::from_secs(n);
                }
            }
        }
        DEFAULT_CACHE_TTL
    }

    fn canonicalize(&self, value: &serde_json::Value) -> String {
        // 排除 cache_control 字段后序列化
        match value {
            serde_json::Value::Object(map) => {
                let mut sorted: Vec<_> = map
                    .iter()
                    .filter(|(k, _)| k.as_str() != "cache_control")
                    .collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                let obj: serde_json::Map<String, serde_json::Value> = sorted
                    .into_iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_default()
            }
            _ => serde_json::to_string(value).unwrap_or_default(),
        }
    }

    fn hash_chunk(&self, hasher: &mut Sha256, chunk: &str) {
        hasher.update(chunk.len().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(chunk.as_bytes());
        hasher.update(b"\0");
    }

    fn min_cacheable_tokens(&self, model: &str) -> usize {
        if model.to_lowercase().contains("opus") {
            OPUS_MIN_CACHEABLE_TOKENS
        } else {
            DEFAULT_MIN_CACHEABLE_TOKENS
        }
    }

    fn prune_expired(
        &self,
        entries_map: &mut HashMap<String, HashMap<[u8; 32], CacheEntry>>,
        now: Instant,
    ) {
        entries_map.retain(|_, entries| {
            entries.retain(|_, entry| entry.expires_at > now);
            !entries.is_empty()
        });
    }
}

/// 估算 token 数（字符数 / 4）
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

// 全局单例
static GLOBAL_TRACKER: std::sync::OnceLock<PromptCacheTracker> = std::sync::OnceLock::new();

pub fn global_prompt_cache_tracker() -> &'static PromptCacheTracker {
    GLOBAL_TRACKER.get_or_init(PromptCacheTracker::new)
}
