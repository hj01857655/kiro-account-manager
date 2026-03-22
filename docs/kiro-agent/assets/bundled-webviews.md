# bundled-webviews（Requirements Webview）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\bundled-webviews`
> 分析日期：2026-03-18

当前目录仅包含 `requirements-webview.js` 与 `requirements-webview.css`，用于 Spec 需求文档的 Webview UI。

---

## 目录结构

- `requirements-webview.js`：React bundle（包含 requirements-view / requirement-item / questions-modal）。
- `requirements-webview.css`：样式文件。

---

## Webview 行为摘要

- 入口：`webviews/src/requirements-webview/index.tsx`。
- 初始数据来自 `window.variables.specDocument`。
- 通过 `addListener('updateRequirementsDocument', ...)` 接收扩展侧更新。
- `requirementAnalyzerEnabled` 为真时显示 Analyze 按钮。
- `analyzeRequirements(documentUri)`：触发需求分析并更新 `analysisItems`。
- `submitAnalysisAnswers(documentUri, items, answers)`：提交问答结果。
- 双击需求项：`openInTextEditor(documentUri, lineNumber)`。
- 需求展示结构：标题 / User Story / Acceptance Criteria / Open Questions。
- Analysis Items：按 requirementId 聚合，支持查看全量 items 或单条需求 items。

---

## 主要 API 调用（Webview → Extension）

- `analyzeRequirements(documentUri)`
- `submitAnalysisAnswers(documentUri, items, answers)`
- `openInTextEditor(documentUri, lineNumber)`

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                      Requirements Webview (bundled)                          │
│                                                                              │
│  index.tsx                                                                   │
│   window.variables.specDocument                                              │
│   addListener('updateRequirementsDocument', ...)                             │
│                                                                              │
│  requirements-view.tsx                                                       │
│   Analyze → analyzeRequirements(documentUri)                                 │
│   Open Questions → submitAnalysisAnswers(documentUri, items, answers)        │
│   Double Click → openInTextEditor(documentUri, lineNumber)                   │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────┐          │
│             │          │ extension.js (Spec Webview API)          │          │
│             │          │ analyzeRequirements / submitAnalysisAnswers │        │
│             │          │ openInTextEditor                          │          │
│             │          └──────────────────────────────────────────┘          │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  updateRequirementsDocument / specDocument payload             │
└──────────────────────────────────────────────────────────────────────────────┘
```
