# model-selection 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/model-selection/**` 注释边界去重

## 结论

- `model-selection/**` 当前共识别 `3` 个唯一模块块。
- 它负责 IDE 侧“当前选中模型”的缓存、配置和 provider 注入。

## 模块清单

```text
model-selection/index.ts
model-selection/model-cache.ts
model-selection/model-configuration.ts
```

## 分层

- `model-cache.ts`
  - 保存当前可用模型列表与默认模型
  - 刷新后触发 `kiro.updateModelsList`
- `model-configuration.ts`
  - 读取/写入 `kiroAgent.modelSelection`
  - `VSCodeModelConfigProvider`
  - 负责格式化 `provider::modelId`
  - 负责把选中模型注入 runtime
- `index.ts`
  - 注册模型相关命令

## 判断

`model-selection/**` 是宿主设置和 runtime model config 之间的桥接层。

## `ListAvailableModels` 接口记录（2026-03-21）

这部分用于记录“账号对应可用模型列表”接口的当前已知证据，方便后续继续对齐 Kiro IDE / 账号卡片展示。

### 已坐实的请求形状

当前最直接的公开实证来自 GitHub 项目：

- `chaogei/Kiro-account-manager`
- `huey1in/kiro-manager`

- `GET {baseUrl}/ListAvailableModels`
- 查询参数：
  - `origin=AI_EDITOR`
  - `maxResults=50`
  - `profileArn`：账号存在时传
  - `modelProvider`：需要按提供商过滤时传
  - `nextToken`：分页继续拉取时传
- 区域端点：
  - `eu-*` -> `https://q.eu-central-1.amazonaws.com`
  - 其他默认 -> `https://q.us-east-1.amazonaws.com`
- 关键请求头：
  - `Authorization: Bearer <accessToken>`
  - `x-amzn-codewhisperer-optout: true`
  - 同类实现通常还会带 Kiro 风格 `User-Agent` / `x-amz-user-agent`

### 已坐实的响应承载位

现在这部分证据应拆成两层来看：

- Kiro 本体打包源码证据
- 公开 GitHub 仓库证据

#### 1. Kiro 本体源码已坐实的顶层响应

`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
里的 `de_ListAvailableModelsCommand` 已直接表明顶层响应承载位是：

```ts
interface ListAvailableModelsResponse {
  defaultModel?: KiroModel
  models?: KiroModel[]
  nextToken?: string
}
```

这说明 `defaultModel` 不是本仓库或第三方项目的推测字段，而是 Kiro 本体当前 bundle 里真实反序列化接收的顶层字段。

#### 1.1 2026-03-21 真实接口实测补充

对真实 `ListAvailableModels` 接口做实测后，当前线上返回进一步确认了两点：

- `defaultModel` 不是精简引用对象，而是与 `models[]` 同形状的完整模型对象
- `promptCaching` 当前线上至少出现这些子字段：
  - `maximumCacheCheckpointsPerRequest`
  - `minimumTokensPerCacheCheckpoint`
  - `supportsPromptCaching`

本次实测样例中的 `defaultModel` 外形为：

```json
{
  "description": "Models chosen by task for optimal usage and consistent quality",
  "modelId": "auto",
  "modelName": "Auto",
  "promptCaching": {
    "maximumCacheCheckpointsPerRequest": 4,
    "minimumTokensPerCacheCheckpoint": 1024,
    "supportsPromptCaching": true
  },
  "rateMultiplier": 1.0,
  "rateUnit": "Credit",
  "supportedInputTypes": ["TEXT", "IMAGE"],
  "tokenLimits": {
    "maxInputTokens": 200000,
    "maxOutputTokens": 64000
  }
}
```

按 2026-03-21 的真实线上回包，当前可直接使用的响应格式可整理为：

```ts
interface ListAvailableModelsResponse {
  defaultModel?: KiroModel
  models?: KiroModel[]
  nextToken?: string | null
}

interface KiroModel {
  description?: string
  modelId: string
  modelName?: string
  promptCaching?: {
    maximumCacheCheckpointsPerRequest?: number | null
    minimumTokensPerCacheCheckpoint?: number | null
    supportsPromptCaching?: boolean | null
  }
  rateMultiplier?: number
  rateUnit?: string
  supportedInputTypes?: string[]
  tokenLimits?: {
    maxInputTokens?: number | null
    maxOutputTokens?: number | null
  }
}
```

#### 2. Kiro 本体源码已坐实的模型项字段

同一份 Kiro bundle 里的 `de_Model(...)` 目前直接接收这些字段：

```ts
interface KiroModel {
  description?: string
  modelId?: string
  modelName?: string
  promptCaching?: unknown
  rateMultiplier?: number
  rateUnit?: string
  supportedInputTypes?: unknown
  tokenLimits?: unknown
}
```

这里要明确两个边界：

- `promptCaching`、`supportedInputTypes`、`tokenLimits` 在 Kiro 本体 bundle 里当前是按 `_json` 直收。
- 这能坐实“这些字段存在且承载 JSON 值”，但**不能仅凭这份 bundle**就把其内部每个子字段都表述成完全展开。

#### 3. 公开 GitHub 仓库已坐实的更细字段

你指定的公开仓库 `huey1in/kiro-manager` 在 commit
`a9d50139bb6cc6f2c87087c231cc014c698b2da4` 的
`src-tauri/src/models.rs`
里，进一步把 `supportedInputTypes` / `tokenLimits` 的常见业务结构坐实成了：

```ts
interface KiroModel {
  modelId: string
  modelName: string
  description: string
  rateMultiplier?: number
  rateUnit?: string
  supportedInputTypes?: string[]
  tokenLimits?: {
    maxInputTokens?: number | null
    maxOutputTokens?: number | null
  }
}
```

同时，这个公开实现还说明业务层可以继续把原始结构扁平化成：

- `inputTypes`
- `maxInputTokens`
- `maxOutputTokens`

这说明当前我们把原始响应保留为：

- `supported_input_types: Vec<String>`
- `token_limits.max_input_tokens`
- `token_limits.max_output_tokens`

是合理的；如果前端后续需要更轻量的展示层 DTO，再单独做扁平映射即可，不必在底层响应结构里提前丢失层级。

### 当前仓库的解析策略

`src-tauri/src/commands/account_cmd.rs` 现已按真实响应结构直接建模：

- 顶层按 `defaultModel` / `models[]` / `nextToken` 解析
- Tauri `list_available_models` 命令对前端直接返回同形状顶层对象，而不是只返回模型数组
- `defaultModel` 直接按完整 `KiroModel` 结构反序列化
- 模型项按 `modelId` / `modelName` / `description` / `rateMultiplier` / `rateUnit` 解析
- `supportedInputTypes` 直接按 `string[]` 接收
- `tokenLimits` 直接按：
  - `maxInputTokens`
  - `maxOutputTokens`
  解析
- `promptCaching` 直接按：
  - `maximumCacheCheckpointsPerRequest`
  - `minimumTokensPerCacheCheckpoint`
  - `supportsPromptCaching`
  解析
- 请求构造已支持可选 `modelProvider`

### 当前仓库的缓存设计

当前仓库现在不再只依赖前端内存态缓存，而是做了账号级持久化缓存：

- 缓存位置：
  - `accounts.json` 内每个账号的 `availableModelsCache`
- 缓存内容：
  - 完整 `ListAvailableModelsResponse`
  - `cachedAt` Unix 时间戳
  - `modelProvider` 维度
- 命中策略：
  - `list_available_models` 默认优先返回 30 分钟内的有效缓存
  - 只有当 `modelProvider` 与当前请求一致时才命中缓存
  - 超过 TTL 后才重新请求远端
- 失效策略：
  - `refresh_account_token` 成功后清空该账号模型缓存
  - `sync_account` 发生 token 刷新时清空该账号模型缓存
  - `list_available_models` 自身发生鉴权刷新时也会清空旧缓存并回写新缓存

这套设计的目的很直接：

- 避免同一账号反复点开模型列表就反复打远端
- 在应用重启后仍保留最近一次已解析的模型列表
- 当账号凭证或 profile 变化时，自动让缓存失效，避免旧模型集污染新会话

这样做的目的不是做“兼容猜测”，而是把当前真实线上和 Kiro 本体源码已经坐实的结构直接落到类型系统里。

### 证据边界

- Kiro 本体 bundle 现已直接坐实：
  - `defaultModel`
  - `models`
  - `nextToken`
  - `description`
  - `modelId`
  - `modelName`
  - `promptCaching`
  - `rateMultiplier`
  - `rateUnit`
  - `supportedInputTypes`
  - `tokenLimits`
- 公开 GitHub 证据目前可以进一步细化坐实：
  - `supportedInputTypes` 常见为 `string[]`
  - `tokenLimits.maxInputTokens`
  - `tokenLimits.maxOutputTokens`
  - 这些字段在应用层常被映射为：
    - `inputTypes`
    - `maxInputTokens`
    - `maxOutputTokens`
- 目前仍然**没有**足够证据可直接坐实的内容包括：
  - `recommended`
  - `isDefault`
  - `tokenLimits` 除 `maxInputTokens` / `maxOutputTokens` 之外的更多子字段
  - 其他“默认模型/推荐模型”扩展语义字段

因此，当前更准确的表述应该是：

- `defaultModel` 已被 Kiro 本体源码坐实
- `defaultModel` 当前真实线上返回为完整模型对象，而不是仅 `{ id, name }`
- `supportedInputTypes` / `tokenLimits.maxInputTokens` / `tokenLimits.maxOutputTokens` 已被公开 GitHub 实现进一步细化坐实
- `promptCaching` 当前线上已观察到：
  - `maximumCacheCheckpointsPerRequest`
  - `minimumTokensPerCacheCheckpoint`
  - `supportsPromptCaching`
- 其他未展开子字段仍应保持保守表述，不要包装成已知事实

### 源码定位

#### Kiro 本体 bundle

本地文件：

- `C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`

关键定位：

- 请求序列化：
  - `se_ListAvailableModelsCommand`：`850138-850153`
  - 这里能直接看到：
    - 路径 `/ListAvailableModels`
    - 查询参数 `origin`
    - 可选参数 `maxResults`
    - 可选参数 `nextToken`
    - 可选参数 `profileArn`
    - 可选参数 `modelProvider`
- 响应反序列化：
  - `de_ListAvailableModelsCommand`：`850778-850794`
  - 这里能直接看到顶层字段：
    - `defaultModel`
    - `models`
    - `nextToken`
- 模型项结构：
  - `de_Model`：`851853-851863`
  - 这里能直接看到模型项字段：
    - `description`
    - `modelId`
    - `modelName`
    - `promptCaching`
    - `rateMultiplier`
    - `rateUnit`
    - `supportedInputTypes`
    - `tokenLimits`
- Kiro 业务层消费映射：
  - `transformOutput3`：`932280-932303`
  - 这里能直接看到：
    - `modelId -> id`
    - `modelName -> name`
    - `tokenLimits?.maxInputTokens -> maxInputTokens`
- 命令实现与分页拉取：
  - `register25 / listAvailableModels`：`932305-932339`
  - 这里能直接看到：
    - `ListAvailableModelsCommand`
    - `origin: AI_EDITOR`
    - `profileArn`
    - `nextToken`
    - `defaultModel` 首次命中保留

#### 公开 GitHub 仓库

你指定的公开文件：

- `https://github.com/huey1in/kiro-manager/blob/a9d50139bb6cc6f2c87087c231cc014c698b2da4/src-tauri/src/models.rs`

关键定位：

- `ModelInfo`：大致在 `L571-L601`
  - 这里坐实了业务层展示 DTO：
    - `id`
    - `name`
    - `description`
    - `input_types`
    - `max_input_tokens`
    - `max_output_tokens`
    - `rate_multiplier`
    - `rate_unit`
- `ListModelsResponse` / `KiroModel` / `TokenLimits`：大致在 `L602-L660`
  - 这里坐实了：
    - 顶层按 `models + next_token` 解析
    - 模型项包含 `supported_input_types`
    - 模型项包含 `token_limits`
    - `token_limits` 至少包含：
      - `max_input_tokens`
      - `max_output_tokens`
