# Kiro CLI 2.37 凭据存储格式（真实样本解析）

> 采样环境：Windows，`kiro-cli 2.37.1`，Google 社交登录（已登录状态），2026-11-02 实测。
> 本文档是账号管理器 `kiro_cli_cmd.rs`（`KiroCliSwitchPayload` / `switch_cli_account` / `import_from_kiro_cli`）的格式对照依据。
> 所有敏感值已脱敏：`前10字符…后6字符 [len=N]`。

## 1. 存储位置与形态

- **凭据落盘**：`%LOCALAPPDATA%\Kiro-Cli\data.sqlite3`（SQLite，非散文件）
- **登录前**：该文件已存在（含空表与 migrations），`auth_kv` 无凭据键
- **登录后**：`auth_kv` 表出现凭据键；`state` 表出现 profile / telemetry 键
- **没有** `~/.aws/sso/cache`、`~/.kiro/cli/`——全部凭据都在这一个 SQLite 里

表结构（`sqlite_master` 实测）：

```
auth_kv         (key TEXT, value TEXT)   ← 凭据（JSON 字符串）
state           (key TEXT, value TEXT)   ← profile / telemetry 状态
migrations      (id, version, migration_time)
history         (id, command, shell, pid, session_id, cwd, start_time, hostname, exit_code, end_time, duration)
conversations   (key, value)
conversations_v2(key, conversation_id, value, created_at, updated_at)
extracted_kas_versions (version, last_used_at)
pinned_bin_versions    (version, last_used_at)
```

## 2. `auth_kv` 表——凭据主存储

### 2.1 `kirocli:social:token`（社交登录 token，登录后出现）

```json
{
  "access_token": "aoaAAAAAGr…pwo0dH [len=230]",
  "expires_at": "2026-11-02…02761Z [len=28]",
  "refresh_token": "aorAAAAAGt…If/m1u [len=230]",
  "telemetry_generation": "d4b41670ea…d83e4a [len=64]",
  "provider": "goog…le",
  "profile_arn": "arn:aws:co…RVQMUK [len=65]"
}
```

| 字段 | 类型/长度 | 说明 |
|---|---|---|
| `access_token` | string(230) | Bearer token，`aoa` 前缀（social 流程） |
| `expires_at` | string(28) | **RFC3339 微秒 + `Z`**，如 `2026-11-02T01:12:16.5802761Z`——与账号管理器 `build_switch_payload` 写出的格式一致（`SecondsFormat::Micros, true`） |
| `refresh_token` | string(230) | `aor` 前缀 |
| `telemetry_generation` | string(64) | hex，遥测代际标识（**账号管理器当前不写此字段，见 §4 差异**） |
| `provider` | string(6) | **小写**：`google` / `github`（`Github`→`github` 的归一在管理器侧已有） |
| `profile_arn` | string(65) | Social 默认 profile：`arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK`（与管理器 `SOCIAL_PROFILE_ARN` 常量一致） |

### 2.2 `kirocli:social:device-registration`（社交登录客户端注册）

```json
{
  "client_id": "…",
  "client_secret": "…",
  "client_secret_expires_at": "2026-10-07…22:52Z [len=20]",
  "oauth_flow": "Pkce…ce",
  "region": "us-e…-1",
  "scopes": ["codewhispe…etions [len=25]", "codewhispe…alysis [len=22]", "codewhispe…ations [len=27]"]
}
```

- 30 天注册有效期（`client_secret_expires_at`：签发后 +30 天）
- `oauth_flow: "Pkce"`、`region: "us-east-1"`
- scopes 三件套：`codewhisperer:completions` / `codewhisperer:analysis` / `codewhisperer:conversations`

### 2.3 `kirocli:odic:device-registration`（IdC/SSO 客户端注册；本机未走 IdC 登录，为共存残留）

结构与 2.2 相同（client_id(34) / client_secret(4797, JWT 形态) / client_secret_expires_at / oauth_flow: Pkce / region: us-east-1 / scopes 三件套）。

## 3. `state` 表——profile 与遥测状态

```json
{
  "api.codewhisperer.profile": {
    "arn": "arn:aws:co…RVQMUK [len=65]",
    "profile_name": "Social_Def…rofile [len=22]",
    "device_code_disabled": false
  },
  "telemetryUserId": "d-9067c984…924ef3 [len=49]",
  "telemetry-cognito-identity-id": "us-east-1:…c864cb [len=46]",
  "telemetry-cognito-credentials": {
    "access_key_id": "ASIAWEKYMY…QOXLR3 [len=20]",
    "secret_key": "rxfQFa4zTq…GBFuCc [len=40]",
    "session_token": "IQoJb3JpZ2…eh9qni [len=1540]",
    "expiration": "2026-11-02…12:24Z [len=20]"
  },
  "telemetryClientId": "3efb24fe-3…9d3586 [len=36]",
  "telemetry.lastHeartbeatDate": "2026…09",
  "telemetry.lastHeartbeatDate.2.16.0": "2026…08",
  "telemetry.lastHeartbeatDate.2.24.1": "2026…28",
  "telemetry.lastHeartbeatDate.2.37.1": "2026…02"
}
```

- `api.codewhisperer.profile`：当前生效 profile（Social 登录名为 `Social_Default_Profile` 形态）
- `telemetry-cognito-credentials`：AWS STS 临时凭据（1 小时刷新），只服务遥测链路，**不用于模型调用**

## 4. 与账号管理器 `KiroCliSwitchPayload` 的对照

| 项 | 管理器写出（kiro_cli_cmd.rs） | 真实 2.37.1 | 结论 |
|---|---|---|---|
| 存储 | `switch_cli_account` 写 key-value | `auth_kv` 表，同 key 名 | ✅ key 名一致（`kirocli:odic:token` / `kirocli:social:token` / 两组 device-registration） |
| token 字段集 | access_token / refresh_token / expires_at / region / oauth_flow / scopes | 同左 + **`telemetry_generation`** + `provider` + `profile_arn` | ⚠️ 管理器**少写三个字段**（§4.1） |
| `expires_at` | RFC3339 微秒 + `Z` | 同格式 | ✅ |
| `provider` | 小写归一（`Github`→`github`） | `google` 小写 | ✅ |
| `profile_arn` | Social 固定 `SOCIAL_PROFILE_ARN` 常量 | 同值 | ✅ |
| IdC token | 不带 profile_arn | 本样本未走 IdC，无法对照 | 按实测注释保留 |
| device-registration | 写 odic/social 两组 | 两组均在 | ✅ |
| state 表 | 管理器不写 | profile + telemetry 键 | ✅ 可不写（CLI 登录态自愈） |

### 4.1 差异明细（管理器侧可补）

1. **`telemetry_generation`**（64 位 hex）：token 对象里管理器不写。作用是遥测代际，缺失时 CLI 大概率按空处理（不影响模型调用），但「写出即像真实 CLI」建议补；
2. **`provider` 字段**：真实 token 里有 `provider: "google"`，管理器 payload 未包含；
3. **`profile_arn`**：真实 social token 里带在 token 对象内，管理器当前只在常量层持有（确认写出路径是否已含）。

三项均为锦上添花——核心五字段（access/refresh/expires/flow/scopes）管理器已对齐。

## 5. 对「切号到 KiroCrew」的结论

- KiroCrew 无自有模型凭据，spawn `kiro-cli acp` 子进程用 CLI 自己的登录态；
- **切号 = 改 `data.sqlite3` 的 `auth_kv` 凭据键**，账号管理器已实现（`switch_cli_account`）且 key 名与格式经本样本验证一致；
- 在途会话不受切号影响（子进程持有启动时凭据），新 spawn 的会话用新号；
- **操作提示**：SQLite 单文件，切换时注意 CLI 进程未在写入（文件锁）；备份/恢复以整个 `data.sqlite3` 为粒度最稳。

## 6. 复现方法（只读，无敏感落盘）

```js
// node 22+ 内置 node:sqlite，只读打开
const { DatabaseSync } = require('node:sqlite');
const db = new DatabaseSync(process.env.LOCALAPPDATA + '/Kiro-Cli/data.sqlite3', { readOnly: true });
console.log(db.prepare('SELECT key, length(value) FROM auth_kv').all());
```
