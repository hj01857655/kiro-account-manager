# kiro-shared 模块概览

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\packages\kiro-shared\dist`
> 分析日期：2026-03-17

本文件聚焦 `kiro-shared` 的基础设施与认证/遥测/MCP 公共能力。

---

## 认证与凭据

- `TokenStorage`：持久化 token 到 `~/.aws/sso/cache/kiro-auth-token.json`，`fs.watchFile` 监听变化。
- `ProfileStorage`：将 profile 写入 VS Code `globalStorageUri/profile.json`，`Sema(1)` 避免并发读写冲突。
- `ExternalIdpAuthProvider`：支持 Social（Google/Github）与 IdC（Enterprise/BuilderId/Internal）的 OAuth/OIDC 流程。
- `PortalAuthProvider`：统一认证门户，回调端口列表：`3128`、`4649`、`6588`、`8008`、`9091`、`49153`、`50153`、`51153`、`52153`、`53153`。

---

## MCP 配置与连接基础

- `mcp-manager`：加载 `mcp.json`，执行 schema 校验与深度校验。
- 环境变量展开：`${VAR}` 需在 `kiroAgent.mcpApprovedEnvVars` 白名单中才会展开。
- 支持 `stdio`、`streamable HTTP`、`SSE` 传输；不安全 URL 被拒绝（仅 `https` 或 `localhost`）。
- 详细连接与 registry 行为见 `../extension/mcp.md`。

---

## MCP Registry 解析

- Registry schema 校验：包类型 `npm`、`pypi`、`oci`。
- 命令映射：
- `npm` → `npx -y <pkg>`（支持 `NPM_CONFIG_REGISTRY`）。
- `pypi` → `uvx <pkg>`（支持 `--default-index`）。
- `oci` → `docker run <image>`。

---

## 路径与 URI

- `getHomeKiroPath()` → `~/.kiro`。
- `getActiveMcpConfigLocation()`：优先 workspace `.kiro/settings/mcp.json`，否则 user `~/.kiro/settings/mcp.json`。
- `uri`：注册/分发 VS Code URI 事件。

---

## 计时与遥测

- `MetricReporter` + `TelemetryNamespace` 统一指标上报。
- `recordProfileStorageEvent` / `recordMcpRegistryEvent` / `recordPowersEvent` 等事件记录。
- `IntervalTimer`：周期任务（start/cancel）。

---

## 架构流程图

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                          kiro-shared 基础设施层                                     │
│                                                                                     │
│  ┌─────────────────────┐   ┌──────────────────────────────────────┐                │
│  │    认证与凭据         │   │           MCP 配置与连接              │                │
│  │                     │   │                                      │                │
│  │  TokenStorage       │   │  mcp-manager                         │                │
│  │  ~/.aws/sso/cache/  │   │  加载 mcp.json → schema 校验          │                │
│  │  kiro-auth-token    │   │  环境变量展开 (白名单 mcpApprovedEnvVars)│               │
│  │  .json              │   │        │                             │                │
│  │  fs.watchFile 监听  │   │   ┌────┴──────────────────────┐      │                │
│  │         │           │   │   │  传输类型选择               │      │                │
│  │  ProfileStorage     │   │   ├─ stdio                    │      │                │
│  │  globalStorageUri/  │   │   ├─ streamable HTTP          │      │                │
│  │  profile.json       │   │   └─ SSE                      │      │                │
│  │  Sema(1) 并发保护    │   │        │                      │      │                │
│  │         │           │   │   拒绝不安全 URL               │      │                │
│  │  ExternalIdpAuth    │   │   (仅 https / localhost)      │      │                │
│  │  Social / IdC       │   │                                      │                │
│  │  OAuth / OIDC 流程  │   │  MCP Registry 解析                   │                │
│  │         │           │   │  npm  → npx -y <pkg>                 │                │
│  │  PortalAuthProvider │   │  pypi → uvx <pkg>                    │                │
│  │  回调端口:          │   │  oci  → docker run <image>           │                │
│  │  3128/4649/6588...  │   └──────────────────────────────────────┘                │
│  └─────────────────────┘            │                                                │
│                                     ├──────────────────────────────────────────────► │
│                                     │                                                │
│                                     │      ┌──────────────────────────────────────┐  │
│                                     │      │ MCP Servers                          │  │
│                                     │      │  tools / resources / prompts         │  │
│                                     │      │  stdio / HTTP / SSE                   │  │
│                                     │      └──────────────────────────────────────┘  │
│                                     │                                                │
│                                     │  ◄────────────────────────────────────────────┤
│                                     │    tools + resources + prompts                │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐                    │
│  │                   路径解析 & 遥测                              │                    │
│  │                                                              │                    │
│  │  getHomeKiroPath() → ~/.kiro                                 │                    │
│  │  getActiveMcpConfigLocation()                                │                    │
│  │    workspace .kiro/settings/mcp.json  (优先)                 │                    │
│  │    user      ~/.kiro/settings/mcp.json                       │                    │
│  │                                                              │                    │
│  │  MetricReporter + TelemetryNamespace → 指标上报              │                    │
│  │  IntervalTimer → 周期任务 start/cancel                       │                    │
│  └─────────────────────────────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────────────┘
               │                          │
               ▼                          ▼
        extension.js                kiro-client
        (消费认证状态)               (消费 MCP 连接)
```

---

## 其他工具

- `errors`：统一日志输出（Kiro Logs / MCP Logs / Powers Logs）。
- `machine-id`：机器标识生成与读取。
