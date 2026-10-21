//! 内容块解析的回归测试。

use super::*;

#[test]
fn extract_text_content_reads_text_from_content_array_without_unwrap() {
    let content = json!([
        { "type": "input_text", "text": "第一段" },
        { "type": "output_text", "text": "第二段" },
        { "type": "input_image", "image_url": "data:image/png;base64,aGVsbG8=" }
    ]);

    assert_eq!(extract_text_content(Some(&content)), "第一段\n第二段");
}

// system prompt 里的 max_thinking_length 必须按值剥离：客户端（Claude Code）
// 可能带任意数值，早期实现只剥离固定的 200000，其他数值会残留并与 converter
// 注入的标签形成重复标签。
#[test]
fn clean_system_prompt_strips_max_thinking_length_with_any_value() {
    for value in ["200000", "20000", "1024", "999999"] {
        let prompt = format!(
            "前置内容\n<thinking_mode>enabled</thinking_mode>\n<max_thinking_length>{}</max_thinking_length>\n后置内容",
            value
        );
        let cleaned = clean_system_prompt(&prompt);
        assert!(
            !cleaned.contains("<max_thinking_length>"),
            "value {value} should be stripped, got: {cleaned}"
        );
        assert!(
            !cleaned.contains("<thinking_mode>"),
            "value {value}: thinking_mode tag should be stripped, got: {cleaned}"
        );
        assert!(cleaned.contains("前置内容"));
        assert!(cleaned.contains("后置内容"));
    }
}

// 多个标签都要剥离；标签不完整时原样保留，不能把正文吃掉。
#[test]
fn clean_system_prompt_strips_multiple_and_handles_broken_tags() {
    let prompt = "A<max_thinking_length>100</max_thinking_length>B<max_thinking_length>200</max_thinking_length>C";
    let cleaned = clean_system_prompt(prompt);
    assert!(!cleaned.contains("<max_thinking_length>"), "got: {cleaned}");
    assert!(cleaned.contains('A'));
    assert!(cleaned.contains('B'));
    assert!(cleaned.contains('C'));

    // 缺闭合标签：原样保留，不吞正文
    let broken = "正文<max_thinking_length>100";
    let cleaned = clean_system_prompt(broken);
    assert!(cleaned.contains("正文"));
    assert!(cleaned.contains("<max_thinking_length>100"));
}
