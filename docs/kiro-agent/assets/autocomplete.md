# Autocomplete（kiro.kiro-agent）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\autocomplete\dist`
> 分析日期：2026-03-17

本文件聚焦自动补全（Inline Completion）实现与调用路径。

---

## 运行机制

- 基于 VS Code Inline Completion Provider。
- 通过 `CodeWhispererRuntime + GenerateCompletionsCommand` 获取补全。
- 请求注入隐私头、`x-amzn-kiro-agent-mode=autocomplete`、外部 IdP token 类型。

---

## 架构流程图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                          Autocomplete (Inline)                                │
│                                                                              │
│  VS Code Editor                                                              │
│   输入变化 → Inline Completion Provider                                     │
│             │                                                               │
│             ▼                                                               │
│  shouldTriggerFromClassifier                                                │
│   基于行号/上下文长度/OS/语言特征判定是否触发                                 │
│             │                                                               │
│             ▼                                                               │
│  CodeWhispererRuntime                                                       │
│   GenerateCompletionsCommand                                                │
│   注入隐私头 + x-amzn-kiro-agent-mode + IdP TokenType                        │
│             │                                                               │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │                    ┌──────────────────────────────────────┐    │
│             │                    │ Kiro 后端 (CodeWhisperer)            │    │
│             │                    │ q.{region}.amazonaws.com             │    │
│             │                    └──────────────────────────────────────┘    │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │   completions / errors                                         │
│             ▼                                                               │
│  Completion Result                                                          │
│   结果展示 / accept / reject                                                │
│   Telemetry: QCompletion                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 触发分类器

`shouldTriggerFromClassifier` 基于行号、上下文长度、OS、语言与字符特征打分，决定是否触发补全请求。

---

## 默认配置参数

| 参数 | 默认值 | 说明 |
|---|---|---|
| `maxPromptTokens` | 1024 | 最大 prompt token 数 |
| `debounceDelay` | 350ms | 防抖延迟 |
| `prefixPercentage` | 0.75 | prefix 占 prompt 比例 |
| `maxSuffixPercentage` | 0.25 | suffix 最大占比 |
| `multilineCompletions` | `"auto"` | 多行补全模式 |
| `slidingWindowSize` | 500 | 滑动窗口行数 |
| `slidingWindowPrefixPercentage` | 0.75 | 滑动窗口 prefix 占比 |
| `maxSnippetPercentage` | 0.6 | snippet 最大占比 |
| `recentlyEditedSimilarityThreshold` | 0.3 | 最近编辑相似度阈值 |
| `recentLinePrefixMatchMinLength` | 7 | 行前缀最小匹配长度 |
| `useCache` | true | 启用补全缓存 |
| `onlyMyCode` | true | 仅使用当前代码库 |
| `useOtherFiles` | true | 允许参考其他文件 |
| `useRecentlyEdited` | true | 参考最近编辑内容 |
| `useFileSuffix` | true | 使用文件后缀作为 suffix |

文件内容截断阈值：`MAX_FILE_CONTENT_LENGTH = 10240`，`MAX_FILE_NAME_LENGTH = 1024`。超过阈值的补全请求被拒绝计数达 `COUNT_COMPLETION_REJECTED_AFTER = 10000` 后停止。

---

## 遥测与错误处理

- 遥测命名：`TelemetryNamespace.Continue/QCompletion`。
- `AccessDenied` 会抛出 `AccessDeniedError`。
- `MONTHLY_REQUEST_COUNT` 触发用量上限提示。
