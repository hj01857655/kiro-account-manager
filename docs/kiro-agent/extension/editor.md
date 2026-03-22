# editor 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/editor/**` 注释边界去重

## 结论

- `editor/**` 当前共识别 `31` 个唯一模块块。
- 这一层不是通用文本编辑器，而是 Kiro 自定义 webview/editor 的桥接层。
- 它负责把 hooks UI、requirements viewer、spec 文档、context provider、设置导入、消息提示等能力以 editor API 形式暴露给前端。

## 模块清单

```text
editor/editor-api/set-cli-alias.ts
editor/editor-api/import-settings.ts
editor/errors.ts
editor/editor-api/get-settings.ts
editor/editor-api/index.ts
editor/editor-api/apply-to-code.ts
editor/editor-api/delete-hook.ts
editor/editor-api/get-full-spec-document.ts
editor/editor-api/get-text-document-content.ts
editor/editor-api/load-context-provider-items.ts
editor/editor-api/read-hook.ts
editor/editor-api/regenerate-spec-document.ts
editor/editor-api/save-hook.ts
editor/editor-api/set-hook-enabled.ts
editor/editor-api/show-message.ts
editor/editor-api/spec-actions.ts
editor/editor-api/trigger-hook.ts
editor/editor-api/update-spec-document.ts
editor/editor-api/update-text-document.ts
editor/update-hook-page.ts
editor/sync-hook-running-state.ts
editor/hooks/open-active-hook-execution.ts
editor/editor-api/analyze-requirements.ts
editor/custom-webview-provider.ts
editor/base-custom-webview-provider.ts
editor/editor-api/save-new-hook.ts
editor/hooks/index.ts
editor/hooks/hooks-ui-provider.ts
editor/requirements-viewer/index.ts
editor/requirements-viewer/requirements-viewer-provider.ts
editor/custom-editor-provider.ts
```

## 分组

### 1. editor-api

- `editor/editor-api/set-cli-alias.ts`
- `editor/editor-api/import-settings.ts`
- `editor/editor-api/get-settings.ts`
- `editor/editor-api/index.ts`
- `editor/editor-api/apply-to-code.ts`
- `editor/editor-api/delete-hook.ts`
- `editor/editor-api/get-full-spec-document.ts`
- `editor/editor-api/get-text-document-content.ts`
- `editor/editor-api/load-context-provider-items.ts`
- `editor/editor-api/read-hook.ts`
- `editor/editor-api/regenerate-spec-document.ts`
- `editor/editor-api/save-hook.ts`
- `editor/editor-api/set-hook-enabled.ts`
- `editor/editor-api/show-message.ts`
- `editor/editor-api/spec-actions.ts`
- `editor/editor-api/trigger-hook.ts`
- `editor/editor-api/update-spec-document.ts`
- `editor/editor-api/update-text-document.ts`
- `editor/editor-api/analyze-requirements.ts`
- `editor/editor-api/save-new-hook.ts`

这是前端 webview 调 extension 能力的主要 API 面：

- `editor/editor-api/index.ts` 汇总这些 API，并把命令桥接给 custom webview/custom editor。
- `editor/editor-api/apply-to-code.ts` 直接调用 `kiroAgent.spec.editorImplementSpec`。
- `editor/editor-api/get-full-spec-document.ts`、`editor/editor-api/regenerate-spec-document.ts`、`editor/editor-api/update-spec-document.ts`、`editor/editor-api/spec-actions.ts` 服务 spec 文档读写与再生成。
- `editor/editor-api/get-text-document-content.ts`、`editor/editor-api/update-text-document.ts` 处理编辑器当前文档内容同步。
- `editor/editor-api/load-context-provider-items.ts` 通过命令加载 context provider 条目。
- `editor/editor-api/read-hook.ts`、`editor/editor-api/save-hook.ts`、`editor/editor-api/save-new-hook.ts`、`editor/editor-api/delete-hook.ts`、`editor/editor-api/set-hook-enabled.ts`、`editor/editor-api/trigger-hook.ts` 暴露 hook CRUD 和触发能力。
- `editor/editor-api/import-settings.ts`、`editor/editor-api/get-settings.ts`、`editor/editor-api/set-cli-alias.ts` 处理 IDE 设置导入和 CLI alias 配置。
- `editor/editor-api/show-message.ts` 负责 UI 提示。
- `editor/editor-api/analyze-requirements.ts` 把 requirements 视图接到分析动作。

### 2. Webview / Custom Editor 基座

- `editor/errors.ts`
- `editor/custom-webview-provider.ts`
- `editor/base-custom-webview-provider.ts`
- `editor/custom-editor-provider.ts`

这一组是所有 editor/webview UI 的基类层：

- `editor/base-custom-webview-provider.ts` 负责 webview 初始化、资源路径、配置监听、消息桥接，是所有自定义 UI 的基座。
- `editor/custom-webview-provider.ts` 提供普通 webview view 的封装。
- `editor/custom-editor-provider.ts` 提供 custom text editor 封装，处理标题、安全转义、文档聚焦、`onDidChangeTextDocument` 同步等。
- `editor/errors.ts` 收口 editor 侧错误。

### 3. Hooks UI

- `editor/update-hook-page.ts`
- `editor/sync-hook-running-state.ts`
- `editor/hooks/open-active-hook-execution.ts`
- `editor/hooks/index.ts`
- `editor/hooks/hooks-ui-provider.ts`

这是 hooks rich UI 的实现层：

- `editor/hooks/hooks-ui-provider.ts` 指向 `packages/hook-editor/dist`，注册 `views.rich-hooks` webview provider。
- `editor/hooks/index.ts` 负责命令注册、panel 映射、打开 hooks UI、升级新建 hook 页面。
- `editor/update-hook-page.ts` 和 `editor/sync-hook-running-state.ts` 让 hook 页面状态与运行状态保持同步。
- `editor/hooks/open-active-hook-execution.ts` 负责从 hooks UI 跳到活跃执行。

### 4. Requirements Viewer

- `editor/requirements-viewer/index.ts`
- `editor/requirements-viewer/requirements-viewer-provider.ts`

这一组实现 requirements 自定义编辑器：

- `editor/requirements-viewer/requirements-viewer-provider.ts` 基于 `CustomEditorProvider` 构建 requirements viewer。
- `editor/requirements-viewer/index.ts` 负责注册与接线。

## 关键链路

### Webview API 出口

- `editor/base-custom-webview-provider.ts`
- `editor/custom-webview-provider.ts`
- `editor/custom-editor-provider.ts`
- `editor/editor-api/index.ts`

这条链路决定前端包如何拿到 extension 能力，是所有 editor/webview 的公共基础设施。

### Hook 编辑链路

- `editor/editor-api/read-hook.ts`
- `editor/editor-api/save-hook.ts`
- `editor/editor-api/save-new-hook.ts`
- `editor/editor-api/delete-hook.ts`
- `editor/editor-api/set-hook-enabled.ts`
- `editor/editor-api/trigger-hook.ts`
- `editor/update-hook-page.ts`
- `editor/sync-hook-running-state.ts`
- `editor/hooks/index.ts`
- `editor/hooks/hooks-ui-provider.ts`
- `editor/hooks/open-active-hook-execution.ts`

这条链路把 hook 数据、开关、执行状态、UI 面板联成完整操作面。

### Spec / Requirements 链路

- `editor/editor-api/get-full-spec-document.ts`
- `editor/editor-api/regenerate-spec-document.ts`
- `editor/editor-api/update-spec-document.ts`
- `editor/editor-api/spec-actions.ts`
- `editor/editor-api/apply-to-code.ts`
- `editor/editor-api/analyze-requirements.ts`
- `editor/requirements-viewer/index.ts`
- `editor/requirements-viewer/requirements-viewer-provider.ts`

这条链路说明 editor 层不仅展示文档，还直接发起 spec 实施与 requirements 分析。

## 关键判断

- `editor/**` 是前端包和 extension 命令/状态之间的桥接总线。
- 它把 hooks、requirements、spec、settings、context provider、消息提示统一暴露成 webview 可调用接口。
- 如果缺失这一层，Kiro 仍可能保留底层命令，但 hooks UI、requirements viewer、自定义 spec 编辑器这类体验会失去可用桥接。
