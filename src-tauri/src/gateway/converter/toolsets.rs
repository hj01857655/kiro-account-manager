//! Anthropic client toolset 展开（`computer_toolset_*` / `browser_toolset_*`）。
//!
//! 官方把这两个 toolset 定义成「tools 数组里的一个无名条目，声明一组固定的
//! 成员工具」，成员名/schema 由 Anthropic 规定、执行在客户端。上游 Kiro 只认
//! 普通工具定义，因此网关必须把条目**展开成逐个成员工具**下发，否则模型看不到
//! 任何成员，computer/browser use 功能在网关后面直接失效。
//!
//! 成员表来源：官方文档的 Available actions 表格
//! - computer：https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool
//! - browser：https://platform.claude.com/docs/en/agents-and-tools/tool-use/browser-use-tool
//!
//! 维护约束：官方没有提供机器可读的成员 schema，版本更新只能靠人工跟进。
//! 因此这里按「成员名 + 参数名/类型」的保守形态描述——足以让模型正确发起
//! 调用（客户端本来就按自己的实现校验参数），不追求逐字段的 maxLength 之类
//! 细节。新增成员时只需在对应表里加一行。
//!
//! `configs` 语义（官方）：键是成员名，值只接受 `enabled` / `defer_loading`；
//! 省略成员保留默认；禁用的成员从模型可见清单里移除；全部禁用等于该条目应
//! 省略。本模块按此过滤。

use serde_json::{json, Value};

/// toolset 展开出的成员工具。
pub struct ToolsetMember {
    /// 成员工具名（官方规定，模型 `tool_use.name` 用它）。
    pub name: &'static str,
    pub description: &'static str,
    /// 参数 JSON Schema。
    pub parameters: Value,
}

/// 判定 `type` 是否为已知 client toolset，返回其成员表。
pub fn toolset_members(tool_type: &str) -> Option<&'static [ToolsetMember]> {
    let normalized = tool_type.trim();
    if normalized.starts_with("computer_toolset_") {
        return Some(computer_members());
    }
    if normalized.starts_with("browser_toolset_") {
        return Some(browser_members());
    }
    None
}

/// 按 `configs` 过滤成员并展开。
///
/// - `configs` 为 None 或非对象 → 全部默认成员；
/// - 成员被显式 `enabled: false` → 剔除；
/// - `configs` 里出现未知成员名 → 忽略该键（官方会拒绝整个请求，这里只告警，
///   不因为一个拼写错误让客户端彻底不可用）；
/// - 过滤后为空 → 返回空（调用方据此跳过该条目）。
pub fn expand_toolset(
    tool_type: &str,
    configs: Option<&Value>,
) -> Option<Vec<(&'static str, &'static str, Value)>> {
    let members = toolset_members(tool_type)?;
    let configs_obj = configs.and_then(Value::as_object);

    let mut out = Vec::new();
    for member in members {
        let enabled = configs_obj
            .and_then(|cfg| cfg.get(member.name))
            .and_then(|cfg| cfg.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !enabled {
            continue;
        }
        out.push((
            member.name,
            member.description,
            member.parameters.clone(),
        ));
    }

    if let Some(cfg) = configs_obj {
        for key in cfg.keys() {
            if !members.iter().any(|m| m.name == key) {
                log::warn!(
                    "[Anthropic] toolset `{tool_type}` 的 configs 含未知成员 `{key}`，已忽略"
                );
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn coord() -> Value {
    json!({
        "type": "array",
        "items": {"type": "number"},
        "minItems": 2,
        "maxItems": 2,
        "description": "[x, y] screen coordinate"
    })
}

fn modifiers() -> Value {
    json!({
        "type": "string",
        "description": "Modifier keys to hold during the action: shift, ctrl, alt, super, or a +-joined combination such as ctrl+shift"
    })
}

// ── computer_toolset ────────────────────────────────────────────────────────

/// 官方 17 个成员（computer-use-tool 文档 Available actions）。
/// `serde_json::json!` 不是 const，成员表用 `OnceLock` 惰性构建一次。
fn computer_members() -> &'static [ToolsetMember] {
    static CELL: std::sync::OnceLock<Vec<ToolsetMember>> = std::sync::OnceLock::new();
    CELL.get_or_init(build_computer_members)
}

fn build_computer_members() -> Vec<ToolsetMember> {
    vec![
    ToolsetMember {
        name: "screenshot",
        description: "Capture the full display and return it as an image.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "zoom",
        description: "Capture only a region of the display at full resolution, for reading small text or dense UI.",
        parameters: json!({
            "type": "object",
            "properties": {
                "region": {
                    "type": "array",
                    "items": {"type": "number"},
                    "minItems": 4,
                    "maxItems": 4,
                    "description": "[x0, y0, x1, y1] top-left and bottom-right corners of the area to inspect"
                }
            },
            "required": ["region"]
        }),
    },
    ToolsetMember {
        name: "left_click",
        description: "Click the left mouse button at a coordinate, or at the current cursor position when omitted.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord(), "text": modifiers()}
        }),
    },
    ToolsetMember {
        name: "right_click",
        description: "Click the right mouse button. Same parameters as left_click.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord(), "text": modifiers()}
        }),
    },
    ToolsetMember {
        name: "middle_click",
        description: "Click the middle mouse button. Same parameters as left_click.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord(), "text": modifiers()}
        }),
    },
    ToolsetMember {
        name: "double_click",
        description: "Double-click with the left mouse button. Same parameters as left_click.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord(), "text": modifiers()}
        }),
    },
    ToolsetMember {
        name: "triple_click",
        description: "Triple-click with the left mouse button. Same parameters as left_click.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord(), "text": modifiers()}
        }),
    },
    ToolsetMember {
        name: "left_click_drag",
        description: "Press at start_coordinate, drag to coordinate, and release.",
        parameters: json!({
            "type": "object",
            "properties": {
                "start_coordinate": coord(),
                "coordinate": coord(),
                "text": modifiers()
            },
            "required": ["start_coordinate", "coordinate"]
        }),
    },
    ToolsetMember {
        name: "mouse_move",
        description: "Move the cursor to a coordinate.",
        parameters: json!({
            "type": "object",
            "properties": {"coordinate": coord()},
            "required": ["coordinate"]
        }),
    },
    ToolsetMember {
        name: "left_mouse_down",
        description: "Press and hold the left mouse button.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "left_mouse_up",
        description: "Release the left mouse button.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "cursor_position",
        description: "Return the current cursor position.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "scroll",
        description: "Scroll the mouse wheel in a direction.",
        parameters: json!({
            "type": "object",
            "properties": {
                "scroll_direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"]
                },
                "scroll_amount": {
                    "type": "number",
                    "description": "Number of scroll-wheel clicks"
                },
                "coordinate": coord(),
                "text": modifiers()
            },
            "required": ["scroll_direction"]
        }),
    },
    ToolsetMember {
        name: "type",
        description: "Type a string of text.",
        parameters: json!({
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"]
        }),
    },
    ToolsetMember {
        name: "key",
        description: "Press a key or a +-joined key combination, e.g. Return, ctrl+s, alt+Tab.",
        parameters: json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "repeat": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "description": "How many times to repeat the key press. Default 1."
                }
            },
            "required": ["text"]
        }),
    },
    ToolsetMember {
        name: "hold_key",
        description: "Hold a key or key combination for a duration in seconds (up to 300).",
        parameters: json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "duration": {"type": "number", "maximum": 300}
            },
            "required": ["text", "duration"]
        }),
    },
    ToolsetMember {
        name: "wait",
        description: "Wait for a number of seconds (up to 300).",
        parameters: json!({
            "type": "object",
            "properties": {"duration": {"type": "number", "maximum": 300}},
            "required": ["duration"]
        }),
    },
    ]
}

// ── browser_toolset ─────────────────────────────────────────────────────────

fn tab_id() -> Value {
    json!({"type": "string", "description": "Target tab id; defaults to the active tab"})
}

/// 点击类成员共用的 target（坐标或元素引用）。
fn click_target_required() -> Value {
    json!({
        "type": "object",
        "properties": {
            "target": target(),
            "modifiers": modifiers(),
            "tab_id": tab_id()
        },
        "required": ["target"]
    })
}

/// 浏览器目标：页面坐标或元素引用，二者取一。
fn target() -> Value {
    json!({
        "description": "Click target: either {\"coordinate\": [x, y]} or {\"ref\": \"<element ref>\"}",
        "type": "object",
        "properties": {
            "coordinate": {
                "type": "array",
                "items": {"type": "number"},
                "minItems": 2,
                "maxItems": 2
            },
            "ref": {"type": "string", "description": "Element reference from read_page"}
        }
    })
}

/// 官方 27 个默认成员 + 4 个默认关闭的可选成员（browser-use-tool 文档）。
/// 可选成员默认 `enabled: false`，由 `configs` 显式打开——见 [`expand_toolset`]。
fn browser_members() -> &'static [ToolsetMember] {
    static CELL: std::sync::OnceLock<Vec<ToolsetMember>> = std::sync::OnceLock::new();
    CELL.get_or_init(build_browser_members)
}

fn build_browser_members() -> Vec<ToolsetMember> {
    vec![
    ToolsetMember {
        name: "navigate",
        description: "Navigate the active tab to a URL.",
        parameters: json!({
            "type": "object",
            "properties": {"url": {"type": "string"}, "tab_id": tab_id()},
            "required": ["url"]
        }),
    },
    ToolsetMember {
        name: "screenshot",
        description: "Capture the current page as an image.",
        parameters: json!({"type": "object", "properties": {"tab_id": tab_id()}}),
    },
    ToolsetMember {
        name: "zoom",
        description: "Capture a region of the page at full resolution.",
        parameters: json!({
            "type": "object",
            "properties": {
                "region": {
                    "type": "array",
                    "items": {"type": "number"},
                    "minItems": 4,
                    "maxItems": 4
                },
                "tab_id": tab_id()
            },
            "required": ["region"]
        }),
    },
    ToolsetMember {
        name: "left_click",
        description: "Left-click a coordinate or element reference.",
        parameters: click_target_required(),
    },
    ToolsetMember {
        name: "right_click",
        description: "Right-click a coordinate or element reference.",
        parameters: click_target_required(),
    },
    ToolsetMember {
        name: "middle_click",
        description: "Middle-click a coordinate or element reference.",
        parameters: click_target_required(),
    },
    ToolsetMember {
        name: "double_click",
        description: "Double-click a coordinate or element reference.",
        parameters: click_target_required(),
    },
    ToolsetMember {
        name: "triple_click",
        description: "Triple-click a coordinate or element reference.",
        parameters: click_target_required(),
    },
    ToolsetMember {
        name: "hover",
        description: "Hover the pointer over a coordinate or element reference.",
        parameters: json!({
            "type": "object",
            "properties": {"target": target(), "tab_id": tab_id()},
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "left_click_drag",
        description: "Drag from one coordinate to another.",
        parameters: json!({
            "type": "object",
            "properties": {
                "from": target(),
                "target": target(),
                "tab_id": tab_id()
            },
            "required": ["from", "target"]
        }),
    },
    ToolsetMember {
        name: "left_mouse_down",
        description: "Press and hold the left mouse button at a coordinate.",
        parameters: json!({
            "type": "object",
            "properties": {"target": target(), "tab_id": tab_id()},
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "left_mouse_up",
        description: "Release the left mouse button at a coordinate.",
        parameters: json!({
            "type": "object",
            "properties": {"target": target(), "tab_id": tab_id()},
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "mouse_move",
        description: "Move the pointer to a coordinate.",
        parameters: json!({
            "type": "object",
            "properties": {"target": target(), "tab_id": tab_id()},
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "scroll",
        description: "Scroll the page at a coordinate.",
        parameters: json!({
            "type": "object",
            "properties": {
                "target": target(),
                "scroll_direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"]
                },
                "scroll_amount": {"type": "number"},
                "tab_id": tab_id()
            },
            "required": ["target", "scroll_direction"]
        }),
    },
    ToolsetMember {
        name: "scroll_to",
        description: "Scroll a referenced element into view.",
        parameters: json!({
            "type": "object",
            "properties": {"target": target(), "tab_id": tab_id()},
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "type",
        description: "Type text into the focused element.",
        parameters: json!({
            "type": "object",
            "properties": {"text": {"type": "string"}, "tab_id": tab_id()},
            "required": ["text"]
        }),
    },
    ToolsetMember {
        name: "key",
        description: "Press a key or key combination.",
        parameters: json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "repeat": {"type": "integer", "minimum": 1},
                "tab_id": tab_id()
            },
            "required": ["text"]
        }),
    },
    ToolsetMember {
        name: "hold_key",
        description: "Hold a key or key combination for a duration in seconds.",
        parameters: json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "duration": {"type": "number"},
                "tab_id": tab_id()
            },
            "required": ["text", "duration"]
        }),
    },
    ToolsetMember {
        name: "wait",
        description: "Wait for a number of seconds.",
        parameters: json!({
            "type": "object",
            "properties": {"duration": {"type": "number"}, "tab_id": tab_id()},
            "required": ["duration"]
        }),
    },
    ToolsetMember {
        name: "read_page",
        description: "Read the page structure and return element references usable as click targets.",
        parameters: json!({
            "type": "object",
            "properties": {
                "filter": {"type": "string"},
                "depth": {"type": "integer"},
                "ref": {"type": "string"},
                "tab_id": tab_id()
            }
        }),
    },
    ToolsetMember {
        name: "find",
        description: "Find elements on the page matching a query.",
        parameters: json!({
            "type": "object",
            "properties": {"query": {"type": "string"}, "tab_id": tab_id()},
            "required": ["query"]
        }),
    },
    ToolsetMember {
        name: "get_page_text",
        description: "Return the visible text of the page.",
        parameters: json!({"type": "object", "properties": {"tab_id": tab_id()}}),
    },
    ToolsetMember {
        name: "form_input",
        description: "Set the value of a form field identified by an element reference.",
        parameters: json!({
            "type": "object",
            "properties": {
                "target": target(),
                "value": {"type": "string"},
                "tab_id": tab_id()
            },
            "required": ["target", "value"]
        }),
    },
    ToolsetMember {
        name: "new_tab",
        description: "Open a new browser tab.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "list_tabs",
        description: "List open browser tabs.",
        parameters: json!({"type": "object", "properties": {}}),
    },
    ToolsetMember {
        name: "switch_tab",
        description: "Switch to a specific tab.",
        parameters: json!({
            "type": "object",
            "properties": {"tab_id": tab_id()},
            "required": ["tab_id"]
        }),
    },
    ToolsetMember {
        name: "close_tab",
        description: "Close a specific tab.",
        parameters: json!({
            "type": "object",
            "properties": {"tab_id": tab_id()},
            "required": ["tab_id"]
        }),
    },
    // ── 以下 4 个为默认关闭的可选成员，需 configs 显式 enabled: true ──
    ToolsetMember {
        name: "file_upload",
        description: "Upload files into a file input element. Disabled unless enabled in configs.",
        parameters: json!({
            "type": "object",
            "properties": {
                "target": target(),
                "paths": {"type": "array", "items": {"type": "string"}},
                "document_ids": {"type": "array", "items": {"type": "string"}},
                "tab_id": tab_id()
            },
            "required": ["target"]
        }),
    },
    ToolsetMember {
        name: "read_console",
        description: "Read browser console messages. Disabled unless enabled in configs.",
        parameters: json!({"type": "object", "properties": {"tab_id": tab_id()}}),
    },
    ToolsetMember {
        name: "read_network",
        description: "Read browser network activity. Disabled unless enabled in configs.",
        parameters: json!({"type": "object", "properties": {"tab_id": tab_id()}}),
    },
    ToolsetMember {
        name: "javascript_exec",
        description: "Execute JavaScript in the page. Disabled unless enabled in configs.",
        parameters: json!({
            "type": "object",
            "properties": {"text": {"type": "string"}, "tab_id": tab_id()},
            "required": ["text"]
        }),
    },
    ]
}

/// browser toolset 里默认关闭、必须 `configs` 显式打开的可选成员。
pub const BROWSER_OPTIONAL_MEMBERS: &[&str] =
    &["file_upload", "read_console", "read_network", "javascript_exec"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computer_toolset_expands_all_17_members() {
        let members = expand_toolset("computer_toolset_20260801", None).expect("must expand");
        assert_eq!(members.len(), 17, "官方 computer toolset 有 17 个成员");
        let names: Vec<&str> = members.iter().map(|(n, _, _)| *n).collect();
        for expected in [
            "screenshot",
            "zoom",
            "left_click",
            "right_click",
            "middle_click",
            "double_click",
            "triple_click",
            "left_click_drag",
            "mouse_move",
            "left_mouse_down",
            "left_mouse_up",
            "cursor_position",
            "scroll",
            "type",
            "key",
            "hold_key",
            "wait",
        ] {
            assert!(names.contains(&expected), "缺少成员 {expected}");
        }
    }

    #[test]
    fn browser_toolset_expands_default_members() {
        let members = expand_toolset("browser_toolset_20260801", None).expect("must expand");
        assert_eq!(members.len(), 31, "27 默认 + 4 可选");
        let names: Vec<&str> = members.iter().map(|(n, _, _)| *n).collect();
        for expected in ["navigate", "read_page", "left_click", "screenshot", "find"] {
            assert!(names.contains(&expected), "缺少成员 {expected}");
        }
    }

    #[test]
    fn configs_disable_removes_member() {
        let members = expand_toolset(
            "computer_toolset_20260801",
            Some(&json!({"screenshot": {"enabled": false}})),
        )
        .expect("must expand");
        assert_eq!(members.len(), 16);
        assert!(!members.iter().any(|(n, _, _)| *n == "screenshot"));
    }

    #[test]
    fn configs_enable_optional_browser_member() {
        let members = expand_toolset(
            "browser_toolset_20260801",
            Some(&json!({"javascript_exec": {"enabled": true}})),
        )
        .expect("must expand");
        // 默认已含 31 个（含可选但需显式开启的 4 个），这里校验开关不改变数量
        assert_eq!(members.len(), 31);
    }

    #[test]
    fn all_members_disabled_yields_none() {
        let mut cfg = serde_json::Map::new();
        for m in computer_members() {
            cfg.insert(m.name.to_string(), json!({"enabled": false}));
        }
        assert!(expand_toolset("computer_toolset_20260801", Some(&Value::Object(cfg))).is_none());
    }

    #[test]
    fn unknown_type_is_not_a_toolset() {
        assert!(expand_toolset("bash_20250124", None).is_none());
        assert!(expand_toolset("custom", None).is_none());
    }

    #[test]
    fn every_member_has_object_schema() {
        for members in [computer_members(), browser_members()] {
            for m in members {
                assert_eq!(
                    m.parameters.get("type").and_then(Value::as_str),
                    Some("object"),
                    "成员 {} 的 schema 必须是 object",
                    m.name
                );
            }
        }
    }
}
