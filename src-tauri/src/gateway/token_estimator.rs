/// Token 估算模块
///
/// 由于 Kiro IDE 的响应中不包含实际的 token 统计信息，
/// 我们使用与 Kiro IDE 相同的估算算法来计算 token 数量。
///
/// 参考：Kiro IDE extension.js 中的 token-estimator 模块

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::Value;

/// 估算文本的 token 数量（Claude 模型）
///
/// 算法：字符数 / 4
/// 这是 Kiro IDE 对 Claude 模型使用的估算方法
pub fn estimate_tokens_claude(text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }

    // Claude 模型：字符数 / 4
    ((text.len() as f64) / 4.0).ceil() as i32
}

/// 估算文本的 token 数量（通用方法）
///
/// 算法：
/// - 基础 tokens = 字符数 / 4
/// - 换行符加成 = 换行数 * 0.5
/// - 代码块加成 = 代码块数量 * 2
///
/// 这是 Kiro IDE 的 fallback 估算方法
pub fn estimate_tokens_generic(text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }

    // 基础 tokens
    let base_tokens = ((text.len() as f64) / 4.0).ceil() as i32;

    // 换行符加成
    let newline_count = text.matches('\n').count();
    let newline_bonus = ((newline_count as f64) * 0.5).ceil() as i32;

    // 代码块加成（```）
    let code_block_count = text.matches("```").count();
    let code_block_bonus = (code_block_count * 2) as i32;

    base_tokens + newline_bonus + code_block_bonus
}

/// 估算文本的 token 数量（Llama 模型）
///
/// 算法：字符数 / 3.5
pub fn estimate_tokens_llama(text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }

    // Llama 模型：字符数 / 3.5
    ((text.len() as f64) / 3.5).ceil() as i32
}

/// 根据模型名称选择合适的估算方法
pub fn estimate_tokens(text: &str, model_name: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }

    let model_lower = model_name.to_lowercase();

    // 根据模型名称选择估算方法
    if model_lower.contains("claude")
        || model_lower.contains("sonnet")
        || model_lower.contains("opus")
        || model_lower.contains("haiku")
    {
        // Claude 系列模型
        estimate_tokens_claude(text)
    } else if model_lower.contains("llama") {
        // Llama 系列模型
        estimate_tokens_llama(text)
    } else if model_lower.contains("deepseek")
        || model_lower.contains("qwen")
        || model_lower.contains("glm")
        || model_lower.contains("minimax")
    {
        // 中文开源模型（DeepSeek、Qwen、GLM、MiniMax）
        // 使用 Llama 的估算方法（字符数 / 3.5）
        estimate_tokens_llama(text)
    } else {
        // 默认使用通用方法
        estimate_tokens_generic(text)
    }
}

/// 图片块的 token 估算：Anthropic 官方口径 宽×高÷750，单张上限 1600。
///
/// base64 图会解析 PNG IHDR / JPEG SOF 头部拿真实像素（一张 300×200 的图
/// 只计 80 token，而不是 base64 ÷4 的十几万）；解析不出（非 PNG/JPEG、
/// 数据异常）退回固定 1600，与前端 prompt_info 的 IMAGE_TOKENS 一致。
const IMAGE_TOKENS_FIXED: i64 = 1600;
const IMAGE_PIXELS_PER_TOKEN: i64 = 750;

pub fn estimate_image_tokens(source: Option<&serde_json::Value>) -> i64 {
    let Some(source) = source else {
        return IMAGE_TOKENS_FIXED;
    };
    if source.get("type").and_then(Value::as_str) != Some("base64") {
        return IMAGE_TOKENS_FIXED;
    }
    let data = source
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default();
    image_dimensions_from_base64(data)
        .map(|(w, h)| ((w * h) / IMAGE_PIXELS_PER_TOKEN).clamp(1, IMAGE_TOKENS_FIXED))
        .unwrap_or(IMAGE_TOKENS_FIXED)
}

/// 从 base64 图像数据解析 (宽, 高)。只解码头部（PNG IHDR 固定在前 24 字节；
/// JPEG SOF 标记一般在前几十 KB 内），解析不出返回 None。
fn image_dimensions_from_base64(encoded: &str) -> Option<(i64, i64)> {
    if encoded.is_empty() {
        return None;
    }
    // 64KB 原始数据的 base64 约 87K 字符，留余量；按 4 字符对齐避免切坏尾部
    let take = encoded.len().min(88_000);
    let take = take - (take % 4);
    let decoded = STANDARD.decode(&encoded[..take]).ok()?;

    // PNG：8 字节签名 + IHDR，宽高在 16..24（大端 u32）
    if decoded.len() >= 24
        && decoded[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        && &decoded[12..16] == b"IHDR"
    {
        let width = u32::from_be_bytes(decoded[16..20].try_into().ok()?) as i64;
        let height = u32::from_be_bytes(decoded[20..24].try_into().ok()?) as i64;
        return Some((width, height));
    }

    // JPEG：FF D8 开头，扫 SOF0/SOF2（C0/C2）等帧头段：段长(2) + 精度(1) + 高(2) + 宽(2)
    if decoded.len() >= 4 && decoded[0] == 0xFF && decoded[1] == 0xD8 {
        let mut i = 2usize;
        while i + 9 <= decoded.len() {
            if decoded[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = decoded[i + 1];
            // 填充字节 / 独立标记不计段长
            if marker == 0xFF || (0xD0..=0xD9).contains(&marker) || marker == 0x01 {
                i += 2;
                continue;
            }
            let is_sof = (0xC0..=0xCF).contains(&marker)
                && marker != 0xC4
                && marker != 0xC8
                && marker != 0xCC;
            if is_sof {
                let height = u16::from_be_bytes(decoded[i + 5..i + 7].try_into().ok()?) as i64;
                let width = u16::from_be_bytes(decoded[i + 7..i + 9].try_into().ok()?) as i64;
                return Some((width, height));
            }
            let seg_len = u16::from_be_bytes(decoded[i + 2..i + 4].try_into().ok()?) as usize;
            if seg_len < 2 {
                return None;
            }
            i += 2 + seg_len;
        }
    }

    None
}

/// 把 image 块从值里剔除，按官方像素口径单独计费后回填。
///
/// 处理四种形态：字符串、块数组（含 image 块）、对象（递归剔除其 `content`
/// 字段里的图片——tool_result 就是这个形态）、其他（原样序列化）。
pub fn split_images_from_value(value: &Value) -> (String, Vec<&Value>) {
    match value {
        Value::Array(items) => {
            let mut text = String::new();
            let mut images = Vec::new();
            for item in items {
                if item.get("type").and_then(Value::as_str) == Some("image") {
                    images.push(item);
                } else {
                    let (sub_text, sub_images) = split_images_from_value(item);
                    if !sub_text.is_empty() {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(&sub_text);
                    }
                    images.extend(sub_images);
                }
            }
            (text, images)
        }
        Value::Object(map) => {
            let mut text = String::new();
            let mut images = Vec::new();
            for (key, val) in map {
                if key == "content" {
                    let (sub_text, sub_images) = split_images_from_value(val);
                    if !sub_text.is_empty() {
                        text.push_str(&sub_text);
                    }
                    images.extend(sub_images);
                } else if key != "source" {
                    // source 里是 base64 正文，不计文本；其余小字段照计
                    text.push_str(&val.to_string());
                }
            }
            (text, images)
        }
        Value::String(s) => (s.clone(), Vec::new()),
        other => (other.to_string(), Vec::new()),
    }
}

/// 估算 Anthropic /v1/messages 请求的 input tokens（含图片修正）。
///
/// 与旧口径的差异：messages 里 image 块的 base64 不再按文本长度 ÷4 计入
/// （240KB PNG 会虚报十几万 token），改为按像素 宽×高÷750（上限 1600）。
/// tool_result 嵌套里的 image 同样剔除（Claude Code Read 工具读图就是
/// 这个形态）。输出侧不含图片，不受影响。
pub fn estimate_request_tokens_with_images(messages: &[Value], model_name: &str) -> i32 {
    let mut text = String::new();
    let mut image_tokens = 0i64;
    for msg in messages {
        // content 为 null（None 序列化产物）直接跳过，避免 "null" 计入文本
        let is_null = msg
            .get("content")
            .map(Value::is_null)
            .unwrap_or(true);
        let (msg_text, images) = if is_null {
            (String::new(), Vec::new())
        } else {
            split_images_from_value(&msg["content"])
        };
        if !msg_text.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&msg_text);
        }
        for image in images {
            image_tokens += estimate_image_tokens(image.get("source"));
        }
        if let Some(tool_calls) = msg.get("tool_calls").and_then(Value::as_array) {
            for call in tool_calls {
                if let Some(name) = call
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                {
                    text.push_str(name);
                }
                if let Some(args) = call
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(Value::as_str)
                {
                    text.push_str(args);
                }
            }
        }
    }
    estimate_tokens(&text, model_name).saturating_add(image_tokens as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_claude() {
        // 空字符串
        assert_eq!(estimate_tokens_claude(""), 0);

        // 简单文本：4 个字符 = 1 token
        assert_eq!(estimate_tokens_claude("test"), 1);

        // 8 个字符 = 2 tokens
        assert_eq!(estimate_tokens_claude("test123"), 2);

        // 中文：每个汉字约 3 字节，"你好" = 6 字节 / 4 = 2 tokens
        assert_eq!(estimate_tokens_claude("你好"), 2);
    }

    #[test]
    fn test_estimate_tokens_generic() {
        // 空字符串
        assert_eq!(estimate_tokens_generic(""), 0);

        // 简单文本：4 个字符 = 1 token
        assert_eq!(estimate_tokens_generic("test"), 1);

        // 带换行符：len("test\n") = 5 → ceil(5/4) = 2，换行加成 ceil(1 * 0.5) = 1，合计 3
        // 注意 len() 已包含换行符本身，所以 base 是 2 而非 1
        assert_eq!(estimate_tokens_generic("test\n"), 3);

        // 带代码块：len("```rust\n```") = 11 → ceil(11/4) = 3，换行加成 1，2 个 ``` * 2 = 4，合计 8
        assert_eq!(estimate_tokens_generic("```rust\n```"), 8);
    }

    #[test]
    fn test_estimate_tokens_by_model() {
        let text = "Hello, world!";

        // Claude 模型
        assert_eq!(
            estimate_tokens(text, "claude-sonnet-4.5"),
            estimate_tokens_claude(text)
        );
        assert_eq!(
            estimate_tokens(text, "claude-opus-4"),
            estimate_tokens_claude(text)
        );

        // Llama 模型
        assert_eq!(
            estimate_tokens(text, "llama-3"),
            estimate_tokens_llama(text)
        );

        // 未知模型（使用通用方法）
        assert_eq!(
            estimate_tokens(text, "unknown-model"),
            estimate_tokens_generic(text)
        );
    }

    /// 1×1 透明 PNG 的完整 base64（67 字节 → 92 字符），IHDR 宽高均为 1
    const TINY_PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAIAAAACUFjqAAAAEElEQVR4nGP8z8DAwMDAxAAAAKcAq2Pb1WwAAAAASUVORK5CYII=";

    #[test]
    fn test_estimate_image_tokens_png_by_pixels() {
        let source = serde_json::json!({
            "type": "base64",
            "media_type": "image/png",
            "data": TINY_PNG_B64
        });
        // 10×10 PNG → 100/750 → clamp 到 1
        assert_eq!(estimate_image_tokens(Some(&source)), 1);
    }

    #[test]
    fn test_estimate_image_tokens_fixed_without_base64() {
        // 非 base64（URL 引用）拿不到像素 → 固定 1600
        let source = serde_json::json!({ "type": "url", "url": "https://example.com/x.png" });
        assert_eq!(estimate_image_tokens(Some(&source)), 1600);
        assert_eq!(estimate_image_tokens(None), 1600);
    }

    #[test]
    fn test_estimate_image_tokens_large_png_capped() {
        // 构造 IHDR 声称 4096×4096 的 PNG 头：4096*4096/750 ≈ 22368 → 上限 1600
        let mut head = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        head.extend_from_slice(&[0, 0, 0, 13]); // IHDR 长度
        head.extend_from_slice(b"IHDR");
        head.extend_from_slice(&4096u32.to_be_bytes());
        head.extend_from_slice(&4096u32.to_be_bytes());
        let encoded = STANDARD.encode(&head);
        let source = serde_json::json!({
            "type": "base64",
            "media_type": "image/png",
            "data": encoded
        });
        assert_eq!(estimate_image_tokens(Some(&source)), 1600);
    }

    #[test]
    fn test_estimate_request_tokens_with_images_skips_base64() {
        // 读图形态：tool_result.content 里嵌 10×10 PNG。
        // 旧口径按 base64 长度 ÷4 计 ~25 token 且文本里含整段 base64；
        // 新口径图片按像素计 1，base64 不进文本。
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "t1",
                "content": [
                    { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": TINY_PNG_B64 } },
                    { "type": "text", "text": "read this screenshot" }
                ]
            }]
        })];
        let total = estimate_request_tokens_with_images(&messages, "claude-sonnet-4-5");
        // "read this screenshot" 20 字符 /4 = 5，+ 图片 1，+ tool_result 对象小字段
        assert!(total < 30, "image base64 must not count as text, got {total}");
    }
}
