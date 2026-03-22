# kiricons（Kiro 图标字体）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\kiricons`
> 分析日期：2026-03-18

本包提供 Kiro 自定义图标字体与 Codicon 兼容样式，产物以 `kiricon.ttf` + `kiricon.css` 形式被 Webview 与扩展 UI 引用。

---

## 目录结构

- `src/`：SVG 源图标（与 Codicon 命名兼容）。
- `codepoints.json`：图标名 → Unicode codepoint 映射表。
- `dist/kiricon.ttf`：字体文件。
- `dist/kiricon.css`：样式与 `@font-face`。
- `build.js` / `styles.hbs` / `svgo.config.js`：构建脚本与模板。

---

## 关键样式

- `@font-face` 声明 `font-family: "kiricon"`。
- `.codicon[class*='codicon-']` 统一使用 `kiricon, codicon` 字体栈。
- `codicon-<name>:before { content: "\uXXXX" }` 映射到 `codepoints.json`。

---

## Kiro 专用图标（示例）

`codepoints.json` 包含 Kiro 相关命名，示例：

- `kiro`
- `powers`
- `powers-list`
- `powers-mcp`
- `mcp`
- `power-steering`
- `summarization`
- `hook`

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                               kiricons                                       │
│                                                                              │
│  src/*.svg  ──► build.js + svgo.config.js + styles.hbs                        │
│             │                                                                │
│             ▼                                                                │
│  dist/kiricon.ttf + dist/kiricon.css                                         │
│             │                                                                │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────────┐      │
│             │          │ Webview / Extension UI                       │      │
│             │          │ .codicon-* class → glyph                     │      │
│             │          └──────────────────────────────────────────────┘      │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  icon name → codepoint 映射（codepoints.json）                  │
└──────────────────────────────────────────────────────────────────────────────┘
```
