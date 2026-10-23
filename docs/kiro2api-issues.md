# kiro2api 已知问题与待补能力清单

> 梳理日期：2026-10-18　·　锚点：`src-tauri/src/`（Rust 后端）、`src/components/features/Gateway/`（前端）
> 性质：**已知问题 + 待补能力清单**，不是已实施方案。每条给「证据 → 后果 → 改法 → 依赖」。
> 证据均来自源码静态阅读并已定位到行号；未做运行时压测。

---

## 〇、定性：它是「账号转 API」，不是网关

kiro2api 的准确描述是 **Kiro 账号池的本地协议反代 + 取号调度器**。网关的几项要件它一项都不具备：

| 网关应有 | kiro2api 现状 |
|---|---|
| 多上游、可按策略路由 | 只有 Kiro `generateAssistantResponse` 一条出口 |
| 与调用方隔离的账号 | `single`/`group`/`pool` 选的都是本机 Kiro 账号，token 在本机刷新落盘 |
| 每个调用方一个身份 | 客户端 Key 是共享允许值列表，无命名、无配额 |
| 独立常驻进程 | 与 Tauri 窗口同生命周期，关掉管理器服务即停 |

三个协议入口容易让人误判为「多上游」，但它们最后都汇进同一个 `NormalizedRequest` 和同一个上游函数——**一个上游的三种门面**。

**架构决定：内嵌于 kiro-account-manager，不拆分为独立服务**（此前 `kiro-gateway-server` 独立拆分计划作废）。下文所有改法均按内嵌形态设计。

---

## 一、④ 账号状态没有单一 owner —— P0，唯一「现在就在坏」的一条

**owner = 唯一持有者。** 账号状态（token、`failure_count`、`enabled`、`status`）应当只有一份内存副本、读写经它串行。现在同时存在两份 store，彼此不知道对方改了什么：

- **全局那一份**：`AppState.store`（`src-tauri/src/state.rs:17`，类型 `Mutex<AccountStore>`），启动时建一次，管理器界面读写的是它。
- **临时那一份**：gateway 命令里直接 `AccountStore::new()`（`src-tauri/src/commands/gateway_cmd.rs:384` 和 `:489`），从磁盘重新读一份完整快照，用完即扔。

而 `AccountStore::new()` 每次全量读盘（`src-tauri/src/core/account.rs:570` → `load_from_file`），`try_save_to_file()` 把 `self.accounts` 整个序列化后**覆盖写回**同一个 `accounts.json`（`src-tauri/src/core/account.rs:690` 起）。两份对象之间没有任何同步机制，只靠「谁后写文件谁赢」对齐。

### 后果：静默丢数据

1. 请求 A `new()` 读到旧快照 → 刷新出 tokenA → 覆盖写盘，文件 = tokenA。
2. 请求 B 在 A 写盘之前已经 `new()`，手里同样是旧快照 → 刷新出 tokenB → 覆盖写盘，文件 = tokenB。**tokenA 没了**，而 A 还以为自己的凭证有效。
3. 更日常的版本：A 把某账号 `failure_count` 累加到阈值并标 `TooManyFailures`，B 同时在刷新该账号 token，B 后写 → **失败标记被抹掉**，坏账号重新进池被当成好的用。
4. `get_banned_accounts`（`gateway_cmd.rs:384`）读的是临时快照，界面显示的「已封禁」可能与网关实际使用的那份不一致。

### 改法（内嵌形态）

- gateway 侧不再 `AccountStore::new()`，统一取 `state.store` 那把锁，写操作在锁内完成，读写串行。
- 顺一遍全仓 `AccountStore::new()` / `save_store`（`src-tauri/src/commands/common.rs:359`）调用点，确认没有第二处旁路。
- 覆盖写盘改为「锁内读-改-写」，落盘用临时文件 + rename 原子替换。

### 为什么排第一

七块里只有它「现在就在坏」，其余是「缺了没做」或「做错了但还能跑」。而且 ③ 的健康计数、① 的用量归因都要落在一份状态上——owner 不定，落点就是空的。无依赖，可独立做。

> **状态：已完成（2026-10-18）。** `AppState.store` 已改为 `Arc<Mutex<AccountStore>>`（别名 `SharedAccountStore`），经 `RouterState.store` 注入 gateway、经 `LoadBalancer::with_store` 注入负载均衡器；原 12 处 `AccountStore::new()` 旁路全部改为共享锁访问，其中取凭据路径的 guard 已收在 await 之前。`cargo check` 与 `cargo test --bins`（285 passed / 0 failed）通过。**尚未做并发压测**：丢数据路径建议补一个最小复现再长期观察。

---

## 二、① 客户端 Key 没有身份与计量 —— P0

现状：一份共享的 Key 列表。`verify_client_auth`（`src-tauri/src/gateway/proxy/auth.rs:43` 起）取 `effective_client_api_keys(config)` 得到 `Vec<String>`，随后 `expected_keys.iter().any(...)` 逐字相等即放行——**「通过的是哪个 Key」这个信息不往下传**。

### 两层缺口

- **看不见（表象）**：`GatewayRequestLogEntry` 里没有 Key 字段，也**没有客户端 User-Agent 字段**——日志里只有**上游** UA（发给 Kiro 的那份，按账号 machineId 拼出来），客户端发来的 UA 从入口就被丢弃。配三个 Key 给三个人，日志只能说「有三个请求」，说不出谁用的、烧了多少 tokens。
- **管不了（根因）**：Key 无名字、无配额、无 RPM、不能单独停用；禁用靠 `#disabled#` 字符串前缀这种内容编码，误改一个字符就生效。A 的 Key 能把整个账号池烧光，B 跟着遭殃。

两层的关系：**只加日志字段不做身份，等于给一个管不了的东西装仪表盘。** 先让 Key 成为身份（配模型），日志带 Key 是自然结果。

### 要的东西

每个 Key：命名 / 独立配额 / RPM 限制 / 用量归因（日志与统计带 Key）/ 单独停用 / 超额返回 429。

### 依赖与影响

依赖 ②（每协议 tokens 怎么算）和 ⑤（日志结构加字段）。它是接入层的第一块，因为它同时决定数据模型和页面——状态条该显示「账号池 12 个」还是「活跃 Key 与配额余量」，由它定。

> **状态：已完成（2026-10-18）。** 先纠正一处原描述：前端 `ApiKeysDialog` **早已支持命名与启停**（`name:key` 编码 + `#disabled#` 前缀），真正缺的是**名字没有贯通后端**，导致日志与统计无法归因，以及配额/RPM 完全不存在。
>
> 已落地：`gateway/client_key.rs`（`ClientKey` 结构 + 文本迁移 + 保真往返，4 个单测）、`gateway/client_key_meter.rs`（RPM 滑动窗口 + 配额记账，4 个单测）、`verify_client_auth` 改为返回 `ClientIdentity` 并**提前到日志上下文构造之前**（否则认证失败之外的请求都无法归因）、`GatewayRequestLogEntry` 新增 `api_key_name` / `client_user_agent` / `client_key_id` 三字段、配额记账收口在 `write_request_log`。前端新增结构化 `clientKeys` 字段，`clientApiKeysText` 退为序列化镜像，`ApiKeysDialog` 增加配额/RPM 两列，三语文言补齐。
>
> 验证：`cargo test --bins` 293 passed / 0 failed；`npx tsc --noEmit` 零 error；三个 locale JSON 合法。
>
> **测试抓到一个真 bug**：`normalize_config_promotes_legacy_access_token_to_client_api_keys` 失败，原因是老配置靠 `access_token` 存主 Key，初版迁移只读了 `client_api_keys`，会把主 Key 丢掉。已修为两个来源都进结构化列表。
>
> 尚未验证：未起服务实测「日志能看出哪个 Key 与哪个 UA」；配额与 RPM 的实际拦截效果也未在运行中观察。

---

## 三、② 三协议字段消费矩阵缺失 —— P0

已有入口：`/v1/messages`、`/v1/chat/completions`、`/v1/responses`，两个本地 token 计数端点（`/v1/messages/count_tokens`、`/v1/responses/input_tokens`，纯本地估算不调上游），`/v1/models`，`/health`。三个入口各走一个 normalize 函数收敛成 `NormalizedRequest`，这层切分是对的，新增第四个协议只需加一个 normalize + 一个响应构造。

问题：字段是**部分实现**，但没有一份「字段 × 消费判定」表，于是「静默忽略」和「有意不支持」分不开。客户端发 `response_format`、`top_k` 被静默丢掉时，它以为生效了，行为差异会被归因成「模型不听话」。宁可 400 明确拒绝。

### 判定表要覆盖

流式帧序（Anthropic `message_start` / `content_block_delta` / `message_stop`；OpenAI `[DONE]` 与 usage 帧）、工具调用（`tool_use` / function calling / `tool_choice`）、图片输入、thinking 档位、`stop`、多轮与 `previous_response_id`、`stream_options.include_usage`（已落地）。无上游对应的 `modalities` / `audio` / `prediction` 要明确拒绝而不是收下丢掉。

### 一处现成的不一致要先定规则

`/v1/models` 取自 converter 里的静态表，而每个账号真正可用的模型在 `available_models_cache` 里。两者冲突时以谁为准，得有规则。

> **状态：已完成（2026-10-18）。** 判定表落成可执行模块 `gateway/field_support.rs`，`/v1/models` 改为按账号模式解析。
>
> **判定表**：`FieldSupport` 三态——`Consumed`（真的进 `NormalizedRequest`）、`Ignored{reason}`（有意不支持，记录在案）、`Rejected{reason}`（明确 400）。三个协议各一张表，`Protocol::support_of` 查不到按 `Ignored` 处理（协议演进中客户端会提前发新字段，不能因此拒绝请求）。原先靠 `#[allow(dead_code)]` 标记的「解析了但不用」字段全部有了明确归属：Anthropic 的 `top_k` 改 Rejected、`metadata` 改 Ignored；OpenAI Chat 的 `response_format`/`modalities`/`audio`/`prediction`/`logprobs`/`top_logprobs`/`n` 改 Rejected，`seed`/`presence_penalty`/`store`/`user` 等改 Ignored；Responses 的 `text.format` 按嵌套路径 Rejected。
>
> **拒绝层**：`reject_unsupported_fields(format, payload)` 在 `proxy_handler` 里、normalize **之前**跑，命中 `Rejected` 返回 400 + 原因。顶层与嵌套路径都查（Responses 的 `text.format` 是两层结构，只扫顶层会漏）。`text` 容器本身是 Ignored，只有 `format` 子字段被拒——否则整个 `text` 被拒会误伤合法用法。
>
> **`/v1/models`：经社区调研后决定「不做账号过滤」，已回退为静态全量表（2026-10-18）。**
>
> 曾实现过 C 方案（按账号模式取 `available_models_cache` 并集，`single`/`group` 限定范围、`pool` 取全部，全未命中退静态表），并配 4 个测试。随后调研同类项目，结论是**四家一律直接返回全部模型，没有一家按账号过滤**：
> - **kirocc**：静态别名表全量返回，不匹配的回落 `claude-sonnet-4.6`
> - **OmniRoute**（issue #2114，2026-05）：AWS Builder ID 登录只有 free scope，导致 Pro 订阅也只能调 free 模型；修法是改登录方式，**不过滤列表**
> - **LiteLLM**：全量返回，只有静态开关 `discoverable: false` 隐藏模型，与账号无关
> - **9router**：全量返回
>
> 不做过滤的三个理由：
> 1. `/v1/models` 没有「当前用哪个账号」的上下文，社区也没解决这个语义问题
> 2. 选中不可用的模型时上游会**明确报错**，比「列表里没有」更好排查；过滤列表反而会让「我的 opus 呢」这类问题更难查
> 3. 账号权限差异的根因是 **token scope**（AWS 登录 vs Kiro OAuth），过滤列表治不了这个病
>
> 已删除 `resolve_models_response` / `cached_model_ids` 及 4 个对应测试，`models_handler` 恢复调用 `build_models_response()`（静态表）。`field_support.rs` 判定表与拒绝层保留——那部分与账号无关，且社区实践支持「明确拒绝」而非静默丢弃。
>
> 验证：`cargo test --bins` 309 passed / 0 failed。
>
> **若将来要做账号过滤**，前提是先有真实账号数据验证「free 与 pro 的缓存列表确实不同」，并解决缓存按需探测（当前缓存仅由管理器侧命令填充，TTL 30 分钟）。

---

## 四、③ 取号与健康指标语义是坏的 —— P0

已有：`single`/`group`/`pool` 三种账号来源、六种策略（轮询 / 随机 / 均衡 / 最多配额 / 加权随机 / 最少连接）、`x-account-id` 测试通道、上游重试最多 5 次并按 `x-amzn-kiro-ratelimit-retry-after` 反压、限流账号临时屏蔽 60 秒、配额超阈值自动停用、全部因失败被禁用时自愈重置。

问题不是缺功能，是**指标名不副实**：

- `active_connections` 恒为 0：`increment_connections` / `decrement_connections` 的全部调用点（`src-tauri/src/gateway/proxy/auth.rs:200` 加，`:215` / `:224` / `:282` / `:322` 减）都集中在取凭据那一段里，加完立刻减，跨不过真实请求周期。**结果 `LeastConnections` 拿到的全是 0，退化成固定选第一个。**（这也说明：不是「没有 guard」，而是「根本没有覆盖请求周期」，加 guard 之前得先把它挪对位置。）
- `record_failure` 注释写「连续失败 3 次」，实现是「累计 ≥3 且从未成功」——语义与注释不符。
- 「最近 100 次」实际是指数衰减、窗口永不截断。

### 要的东西

先**定义健康指标到底表示什么**，再让它名副其实：连接计数贯穿整个请求生命周期（进请求加、出请求减，用 guard 保证 panic / 断连也减）。重试边界写死：同号最多几次、何时换号、**首字节已发出后绝不重发**（既有硬约束）。

依赖 ④——健康状态落在哪，取决于账号状态归谁管。

> **状态：已完成（2026-10-18）。** 三项改动：
> 1. **连接计数贯穿请求周期**——新增 `ConnectionGuard`（RAII，Drop 时归零）。原先加/减全挤在 `resolve_managed_account_credentials` 取凭据那一段，函数返回即归零，`LeastConnections` 永远看到 0。现在流式路径把 guard 移进 `stream_proxy_response` 的流式任务（活到流结束），非流式路径由 `proxy_handler` 持有到返回；panic / 提前 return / 断连都由 Drop 收尾。
> 2. **failure 语义修正**——原实现 `recent_failures >= 3 && recent_successes == 0` 实际是「累计 3 次失败且从未成功」，与注释的「连续失败 3 次」不符：一个曾成功 100 次、期间失败 3 次的账号会被误判。已拆成两个字段：`consecutive_failures`（连续，一次成功即清零，用于判定不健康）与 `recent_failures`（窗口内失败，用于成功率）。**不拆会把成功率算错**——一次成功后历史失败全消失，成功率永远 100%。同时把 `recent_failures = 999` 这个魔法值改为 `consecutive_failures = usize::MAX`，它原会让封禁账号的成功率统计永久为 0。
> 3. **窗口真截断**——原 `count * 9 / 10` 是指数衰减、永不归零也不到达稳态，1024 次前的失败仍以极小权重影响成功率。改为按比例截断到 `WINDOW_SIZE = 100`，成功率不变、总量受控。
>
> 重试边界核查结论（**与预判不同**）：内外两层重试的边界其实已经写死——外层只对 429/403-token/401 换号，内层只重试 5xx 且上限 5 次、4xx 全部上交；「首字节后绝不重发」由循环结构天然保证（所有换号都在 `break` 拿到响应之前，而流式直到 break 后才开始写客户端）。唯一实际问题是最坏退避累计 31 秒（1+2+4+8+16），已加 `MAX_RETRY_BACKOFF_MS = 4000` 封顶到约 19 秒，并把这条硬约束写成注释固定在代码里。
>
> 验证：`cargo test --bins` 297 passed / 0 failed。新增 4 个测试：连续失败被成功中断、窗口真截断、guard Drop 归零、健康语义主用例改写。
>
> 尚未验证：`LeastConnections` 在真实并发下是否真的开始分流——需要起服务压测才能看出来。

---

## 五、⑤ 可观测与错误分类 —— P1

已有（这块是强项）：内存 `LogStore` 最近 10000 条 + 落盘 JSONL，启动时装回最近 500 条；请求 / 模型 / 端点三套统计；账号健康与限流查询；协议实测（`ProtocolTestDialog`，真实调用上游、消耗额度）；错误历史；原始报文日志 32MB 自动轮转。
>
> **路由分配测试的当前状态**：对话框 `RouteTestDialog` 已删除（与协议实测高度重叠，其指定账号能力协议实测早已具备），但后端命令 `test_route_config`（`gateway_cmd.rs`）与前端封装 `testRouteConfig`（`gatewayApi.ts`）仍在、已无调用方。它的定位与协议实测不同——**只模拟选号、不花额度**，值得作为独立入口恢复；恢复前有两个问题要先修：其一，注释写「不碰账号状态」但实际会经由 `select_account` 推进 round-robin 游标、并在全部不健康时触发 `reset_health_if_all_unhealthy` 真的写回健康表；其二，mode/group/pool 取表单未保存值而 strategy 取运行时已保存值，两者不同源。

### 要补的两样

1. **日志加归因字段**：Key + 客户端 UA（被 ① 驱动）。
2. **错误结构化**：错误码目前以 `__400__` / `__402__` 形式**内嵌在中文 message 字符串里**（如 `src-tauri/src/gateway/proxy/auth.rs:197` 的 `"__402__LoadBalancer 未能选择可用账号"`），前端只能靠解析字符串区分错误类型。要改成结构化字段，展示文案再交给 i18n。

> **状态：已完成（2026-10-18）。** 使用面比预想小：后端 3 处产生（`auth.rs`）+ 1 处消费（`upstream.rs:381`），**前端零解析**。
>
> 新增 `gateway::proxy::errors::GatewayError` 枚举，状态码与 error_type 成为类型的一部分：`PreferredAccountUnavailable`（400/invalid_request_error）、`NoAvailableAccount`（503/unavailable）、`BalancerNoSelection`（503/unavailable）。`auth.rs` 三处改由枚举产生，`upstream.rs` 消费端从 `starts_with("__402__")` 字符串分支改为 `classify_gateway_error()` 类型匹配。
>
> **状态码选 503 而不是 402**（复核后修正）：这两个错误的语义是「服务端暂时挑不出账号」，不是「客户端欠费」。用 402/`insufficient_quota` 会让客户端以为该充值，而它实际该做的是退避重试。上游真实的 402（Kiro 返回配额不足）仍由 `map_upstream_error` 原样透传，与此处无关——两者此前被混用同一个码。
>
> **迁移期保留 `__NNN__` 前缀**：`auth.rs` 的其它调用方仍有按字符串判断限流的逻辑（`message.contains("429")`），且其返回类型是 `Result<_, String>`；因此 `Display` 保留前缀、`classify_gateway_error` 负责还原，行为零变化。待调用方全部改用类型传错后，前缀与 `classify` 均可删除。
>
> 文案仍是硬编码中文（未迁 i18n）——`GatewayError::message()` 是单点，将来迁移只改这一处。
>
> 验证：`cargo test --bins` 300 passed / 0 failed；新增 3 个测试（类型携带状态码、Display→classify 往返、普通 message 不误分类）。

### 内存与磁盘上限

- `responses_sessions` 的 map 原有 1 小时 TTL 淘汰（只在插入时 `retain`），**没有条数上限**，且每条存整段 `request_messages`：一小时内的大量长对话会把内存推高。

> **状态：已修（2026-10-18）。** 新增 `SESSION_MAX_ENTRIES = 200`，`prune_sessions` 先按 TTL 清、再按 `updated_at` 从旧到新裁到上限（最近用过的会话最可能被 `previous_response_id` 续接，优先保留）。清理改为**独立周期**（`SESSION_CLEANUP_INTERVAL = 10 分钟`，`spawn_responses_session_cleanup`）——只在插入时清理意味着「一小时内没有新请求，旧会话就一直占内存」，而闲置恰是内存该被释放的时候。新增 2 个测试：TTL 淘汰、条数上限。验证 `cargo test --bins` 302 passed / 0 failed。
- 日志目录曾实测涨到 372MB，`kiro-request.log` 单文件曾涨到 221MB（32MB 轮转就是为此加的兜底）：留存与清理策略要写下来并落地。

> **状态：落盘与轮转已修（2026-10-18）。** 核查发现三个问题，均已改：
> 1. **`.jsonl` 原本不轮转**——`enforce_raw_log_caps` 只认 `*.log`，而结构化请求日志是最容易涨爆的那个（`request_body` / `response_body` 原样入库、无截断）。已把 `.jsonl` 纳入轮转（上限同为 32MB，轮转为 `.jsonl.1`），并把「管哪些扩展名」提成纯函数 `managed_log_cap` 加测试锁住。
> 2. **请求体/响应体无截断**——Claude Code 单条几百 KB 是常态。已加 `truncate_logged_body`，超 64KB 截断并附 `…[truncated, total N bytes]` 标记；`request_summary` 已记录长度与工具数，完整正文非排查必需。
> 3. **三套日志无法串联**——JSONL 的 `request_id` 是落盘时才生成的，`*-request.log` dump 里只有 `idx`。已改为 `proxy_handler` 入口生成一次，JSONL、`*-request.log`、SSE dump 共用同一个 `request_id`。
>
> 验证：`cargo test --bins` 294 passed / 0 failed。
>
> 仍未做：流式路径的响应体仍只落聚合结果与 SSE 分帧，不是逐帧原文；留存策略（保留几天/几条）尚未定义，目前只有体积上限。

依赖 ①。

---

## 六、⑥ 服务边界 —— P1

已有：默认监听 `127.0.0.1`（端口 8765）、远程访问强制要求 IP 白名单、Key 校验。`ensure_config_valid` 是集中 fail-fast：拒绝历史遗留 `local` 模式、校验日志级别枚举、端口与 region。

现存开放面：

- CORS 为 `allow_origin(Any)` + `allow_headers(Any)`（`src-tauri/src/gateway/mod.rs` `router()`）。
- 打开远程访问后的完整规则未定义（TLS？绑定校验？）。

> **关于「`/health`、`/v1/models` 无鉴权」的纠正**：这条原判断是**错的**。四个本地端点（`/health`、`/v1/models`、`/v1/messages/count_tokens`、`/v1/responses/input_tokens`）都走 `generate_local_response`，而该函数在返回响应前**统一做了客户端 Key 校验**（`handlers.rs` 的 `verify_client_auth` 调用）与 IP 白名单检查。无 Key 的请求一律 401。发现过程：我给 `models_handler` 补了一段鉴权，随后测试显示 `/health` 也返回 401，才回头读到那段既有代码——我加的是重复逻辑，已撤回落回原状。
>
> 实际存在的开放面只有 CORS：全开意味着同机上任意网页都能用 JS 读走这些端点。本机边界（`local_only` / IP 白名单）挡得住远程，挡不住本机浏览器。
>
> **状态：已修（2026-10-18）。** CORS 从 `allow_origin(Any)` 收敛为 `AllowOrigin::predicate` + `is_local_origin`：只放行 `http://localhost[:port]`、`http://127.0.0.1[:port]`、`http://[::1][:port]`；前缀后必须是端口分隔符或结束，否则 `http://localhost.evil.com` 会被 `starts_with` 误放行（有测试锁住）。`allow_headers` 同时从 `Any` 收敛为 `Authorization` / `Content-Type` / `x-api-key` 三项。
>
> **无 Origin 的请求不受影响**：`AllowOrigin::to_future` 对无 Origin 的请求直接返回 `None`（闭包根本不调用），CORS 层不设回显头但**不拒绝请求**——curl、SDK、探活工具全部照常工作，有测试验证。
>
> 验证：`cargo test --bins` 304 passed / 0 failed；新增 `local_origin_accepts_loopback_and_rejects_lookalikes` 与 `runtime_rejects_models_without_client_key`（后者同时锁定「无 Key 401」与「本地 Origin 回显、外地不回显」）。

内嵌形态下这一块是**收敛而不是扩张**：确认「只服务本机」之后，把多余的开放面收掉即可。真要多人共享才需要多租户 Key、每 Key 配额、TLS、账单。

一个容易混淆的点：它**现在已经是 HTTP 服务**（Axum）。「拆成 web」不是指改成 HTTP，而是指换个地方跑、给更多人用。

---

## 七、⑦ 命名与 i18n 收口 —— P2

- 对外统一叫 **kiro2api**，一句话定位：把 Kiro 账号池转成 Anthropic / OpenAI 兼容 API。参考项目 kiro.rs 的自称是「Anthropic Claude API 兼容代理服务」，从不叫 gateway。
- 「2API」在 `src-tauri/src/` 下共 15 处、散布 7 个文件（`gateway/proxy/upstream.rs`、`gateway/mod.rs`、`gateway/proxy/auth.rs`、`main.rs`、`gateway/proxy/handlers.rs`、`gateway/prompt_cache.rs`、`commands/gateway_cmd.rs`），其中若干是**用户可见报错**（如 `auth.rs:91` 的 `"2API不再支持 local 模式"`、取凭据路径里的 `"创建账号 {} 的2API HTTP 客户端失败"`）。统一为 kiro2api。
- **顺带一个真 bug**：`gatewayPageUtils.ts` 有一批硬编码简体中文，绕过 i18n，英文/俄文界面下会显示中文。搬进 locales 时一起修。

> **状态：已完成（2026-10-18）。** 「2API」在 `src-tauri/src/` 与 `src/` 下已全部清零（共 12 处，含注释、日志与用户可见报错），统一为 kiro2api。
>
> 硬编码中文已全部参数化：新增 `gatewaySummary.*` 三语键（zh-CN / en / ru 各 55 条），涉及 6 个函数——`createGatewayFieldErrors`、`buildGatewayRoutingSummary`、`buildGatewayActionSummary`、`buildGatewaySecuritySummary`、`buildGatewayIntegrationSummary`、`buildGatewayClientRecipes`，以及 `formatGatewayAccountOptionLabel`。做法是给这些函数加 `t` 参数（由 `index.tsx` 的 `useApp().t` 传入），文案不再有字面量。
>
> 另修两处统计聚合的中文兜底值（`未记录模型` / `未解析上游来源`）——它们是聚合 key 而非展示文案，空值聚合成空档更合适，展示层自行决定如何显示。
>
> 验证：`npx tsc --noEmit` 零 error；三个 locale JSON 合法；`cargo test --bins` 302 passed / 0 failed。
>
> **注意**：`gatewayPageUtils.test.ts` 这个测试文件**从来没被跑过**——`package.json` 里没有测试运行器（无 vitest/jest/tsx），且文件用 `./gatewayPageUtils.js` 后缀 import `.ts` 源文件，node 无法解析。这是既有状态，非本次引入；要让它可跑需先补测试基建。

---

## 八、推进顺序

| 序 | 块 | 理由 |
|---|---|---|
| 1 | ④ 账号状态单一 owner | 唯一正在丢数据的；③ ① 的落点 |
| 2 | ① Key 身份与计量 | 决定数据模型与页面；② ⑤ 等它 |
| 3 | ③ 健康与重试语义 | 依赖 ④ 的落点 |
| 4 | ② 三协议消费矩阵 | 不做则客户端问题持续归因错误；可与 ③ 并行 |
| 5 | ⑤ 错误结构化 + 上限 | 影响排障效率 |
| 6 | ⑦ 命名 + 硬编码中文 | 用户可见但不影响功能 |
| 7 | ⑥ 边界收紧 | 最后收 |

若将来确实出现多人共享需求，顺序是：**先在内嵌形态补 Key 身份与计量 → 再把服务进程与窗口解绑（本机后台常驻 + 管理器作控制面）→ 最后才考虑可部署的 Web 服务**。停在中间状态（拆了一半）最差。

---

## 九、已具备的能力（用来对照上面「缺什么」）

- **协议翻译**：三入口 → `NormalizedRequest` → 单上游 → 按入口协议重构响应；不透传 Kiro 原始报文，端点不返回账号对象。
- **客户端鉴权**：认 `Authorization: Bearer` 与 `x-api-key`，共享 Key 列表逐字比较。
- **取号与调度**：三种账号来源、六种策略、`x-account-id` 强制指定账号、限流临时屏蔽 60 秒、封禁持久化 `enabled=false`、全组失败自愈重置。
- **凭证生命周期**：token 只在真正过期时刷新（不提前刷，避免与定时器/IDE 撞出 429）；每账号独立 HTTP 客户端（含账号级代理）；用量拉取、配额超阈值停用、恢复后自动启用；`failure_count` 到阈值标 `TooManyFailures`；Enterprise 账号补 `profileArn` 并落库，BuilderId 按 provider 兜底。
- **上游补偿**：`generateAssistantResponse` 单出口、重试 ≤5 次、按 `x-amzn-kiro-ratelimit-retry-after` 反压；对齐抓包的 11 个请求头；请求与错误体写 `kiro-request.log`；Claude Code 强制 `web_search` 时用账号凭证调 Kiro 远程 MCP 执行再注回结果。
- **成本与质量附件**：提示词过滤（精简 CC 系统提示 / 去边界标记 / 去环境噪音 / 自定义规则）；模型映射（replace / alias / 加权轮询）；响应缓存（默认 TTL 180s）、`prompt_cache`、`token_cache`、token 估算、`thinking_parser`、Responses 会话续接。
- **可观测与诊断**：见 ⑤ 的「已有」清单。
- **客户端接入**：三张客户端配方（Base URL / 鉴权头 / 端点 / 真实写入文本），一键写入 Claude Code、Codex 配置并备份原文件。

---

## 十、验证状态

**七块全部完成（2026-10-18）。** `cargo test --bins` 309 passed / 0 failed；`npx tsc --noEmit` 零 error；三个 locale JSON 合法。

### 已验证

- 单元测试覆盖所有改动点（新增测试累计 30+ 个）
- 测试过程中抓到 3 个真 bug：`access_token` 主 Key 迁移丢失、`/v1/models` 误用 402、旧测试锁着错误的 failure 语义
- 抓到自己 2 次判断错误并已回退：`/health` / `/v1/models` 「无鉴权」实为有（在 `generate_local_response` 统一做）、`/v1/models` 账号过滤（社区四家均不过滤）

### 未验证（全部需要跑起来或有真实数据）

| 项 | 缺什么 |
|---|---|
| 完整构建（`cargo build --release` / `npm run build`） | **从没跑过**。单测全过不等于能出包，链接、资源、Tauri 配置层面的问题只有构建能抓出来 |
| ④ 并发丢数据路径 | 并发压测 |
| ③ `LeastConnections` 真实分流 | 并发压测 |
| ① 日志归因 / 配额 / RPM 拦截 | 导入账号发真实请求 |
| ⑥ CORS 实际效果 | 浏览器或带 Origin 的 curl |
| 网关页面运行时效果 | 启动应用实际打开（当前 0 账号，网关起不来） |

### 两个技术债

1. **`gatewayPageUtils.test.ts` 从来没被跑过**：`package.json` 无测试运行器（无 vitest/jest/tsx），且文件用 `./gatewayPageUtils.js` 后缀 import `.ts` 源文件，node 报 `ERR_MODULE_NOT_FOUND`。既有状态，非本次引入；本次改过其中的断言但依然跑不了。要让它可跑需先补测试基建。
2. **迁移期 `__NNN__` 前缀**：`auth.rs` 调用方仍有 `message.contains("429")` 这类字符串判断，故 `GatewayError::Display` 保留前缀 + `classify_gateway_error` 负责还原。待调用方全部改用类型传错后可一并删除。

### 有意未做

- 流式响应体逐帧落盘（目前只落聚合结果与 SSE 分帧）
- 日志留存的时间维度策略（目前只有 32MB 体积上限）
- `/v1/models` 账号过滤（见第二节，社区调研后决定不做）

---

## 十一、macOS 适配

排查时间 2026-10-18，基于代码静态审查，**未在 macOS 实机验证**。技术栈已从 Mantine 迁到 shadcn/ui + Radix（`package.json` 依赖可证），旧记忆中「Mantine 与 Tailwind 冲突导致样式混乱」的根因已不存在。

| # | 问题 | 证据 | 严重度 |
|---|---|---|---|
| 1 | `::-webkit-scrollbar` 强制 6px 常驻滚动条 | `src/index.css:255-258` | **确定**，已修 |
| 2 | macOS 交通灯按钮（左上 ~70×28px）可能压住 Sidebar 首项 | `tauri.conf.json:23` `decorations: true` + `Layout/index.tsx` Sidebar 顶部即菜单列表 | 待实机确认 |
| 3 | `minWidth: 1200` 对 MacBook 偏大 | `tauri.conf.json:19` | 待确认目标机型 |
| 4 | 无 `data-tauri-drag-region` | 全仓 0 处 | **非问题**（用原生标题栏，无需自定义拖拽区） |

### 已修（问题 1、2、3）

**问题 1（滚动条）**：`::-webkit-scrollbar` 那段包进 `@media not all and (-webkit-touch-callout: none)`，只对非 macOS 生效。macOS 用 overlay 滚动条（不占布局宽、滚动时才浮现、由系统管理外观），强制 6px 会让它常驻且细到难以命中；Windows/Linux 无 overlay 滚动条，保留细化样式。

**问题 2（交通灯压内容）**：新增 `.sidebar-top-inset` 类，在 `@media (-webkit-touch-callout: none)` 下给侧栏加 36px 顶部内边距。原首个可点元素距顶仅 12px（`p-3`），完全落在交通灯的 28px 高度内——macOS 用户点侧栏首项会命中关闭按钮。36px 略大于按钮高度，留出视觉间距；按钮只覆盖左侧约 70px，侧栏宽 192px，文字与图标右半部分仍可点。

**问题 3（机型适配）**：目标用户各种机型都有，所以不能按单一人群设计。两步改：

1. `tauri.conf.json` 的 `minWidth` 1200 → 960、`minHeight` 700 → 640，让小屏 MacBook 能把窗口缩到适合的尺寸
2. `main.rs` 新增 `fit_window_to_screen`：首次显示前按 monitor 可用区收缩窗口（`show_main_window` 与 3 秒强制显示兜底都调用它）。原 `tauri.conf.json` 写死 1400×820，而 13 寸 MacBook 默认缩放的可用高度只有约 800px，窗口会被挤出屏幕——用户看到的是「打不开」。函数**只在超出屏幕时收缩、不放大**，大屏观感不变；收缩后重新 center。

栅格布局无需改动：全项目已是响应式（`grid-cols-1 lg:grid-cols-3`、`sm:grid-cols-3` 等），没有写死多列，内容区窄了会自己降列。

三处修复都用**特性查询或运行时探测，不用 UA 嗅探**：`-webkit-touch-callout: none` 在 WebKit 系（macOS 的 WKWebView / Safari）成立，在 Windows WebView2（Chromium）与 Linux WebKitGTK 上不成立。

验证：`cargo test --bins` 309 passed / 0 failed；`npm run build` 通过；产物 CSS 中 `sidebar-top-inset` 1 次、`-webkit-touch-callout` 2 次、`::-webkit-scrollbar` 5 次，规则未被构建器裁掉。

**仍未实机验证**：以上为静态审查 + 构建验证，未在 macOS 上眼看效果。

### 非问题

全仓 0 处 `data-tauri-drag-region`——因为用的是系统原生标题栏（`decorations: true`），不需要自定义拖拽区。若将来改成 `decorations: false` 自绘标题栏，才需要补拖拽区。

---

## 十二、GitHub 仓库 issue / PR 对照

来源：https://github.com/hj01857655/kiro-account-manager （2026-10-18 查阅，open 状态）

### 与本次改动直接相关

| Issue | 内容 | 与本次改动的关系 |
|---|---|---|
| **#152** mac 两个版本搞反 | M5 MacBook Pro 上 `aarch64.dmg` 损坏，`x64.dmg` 反而能装 | **已查证：版本映射没搞反，根因是未签名未公证**。详见下方专节 |
| **#142** `/compact` 报 `summarization produced empty response` | 反代启用后，Claude Code 的上下文压缩得到空结果 | **反向支持 ② 的 Rejected 决策**：客户端发新用法、网关静默吞掉 → 用户看到「摘要为空」会以为是模型问题；明确 400 至少让错误可定位。官方答复明确「本项目没有专门实现 `/compact`」 |
| **#155** 切号后对话记录丢失 | 50 积分账号池，切号后记录不见 | **对我 ⑤ 的 `SESSION_MAX_ENTRIES = 200` 提出质疑**：会话跟账号走（官方答复），账号池大 / 并发会话多时 200 可能不够；且切号后旧账号缓存被裁 = 用户感知的「记录丢失」。该上限值是拍的，无数据支撑，**待实测后定值** |

### 未解决且可能命中本次改动区域

| Issue | 内容 | 疑似关联 |
|---|---|---|
| **#154** 日志显示 `TEMPORARILY_SUSPENDED`，账号状态刷新后正常 | 封禁检测与状态刷新不一致 | **已定位并回复（2026-10-18）**：根因见下方专节 |
| #136 | 在线登录 Enterprise 账号缺 userId/email，kiro-cli 导入无法识别 | 账号导入字段，非本次范围 |
| #133 | 能自动切换账号吗 | 已有 auto_switch，可能是用户不知情或文档缺失 |

### 已有 PR / 已修

| PR | 内容 | 状态 |
|---|---|---|
| #159 | social token 写入小写 provider，修复切号后 kiro-cli 未登录 | 开 |
| #150 | Linux web 管理面板 | 开 |
| #149 | 移除 enterprise getUsageLimits 的 profileArn | 已合并 |

### 已做：回复并记录结论

2026-10-18 已通过 `gh` 回复该 issue，要点：根因是封禁检测用的接口不稳定，只有对话接口的 403 是可靠判据；后续会把网关判出的封禁回写到账号。

---

## 十四、#154 专项：封禁检测的接口选择

### 结论

**只有对话接口（`generateAssistantResponse`）的 403 能作为封禁的可靠判据。**

`ListAvailableModels` 与 `getUsageLimits` 的 403 是**偶发**的：同一个被封禁的账号，同一个时刻，有时返回 403、有时正常返回用量。因此「刷新用量」那一次没撞上 403，账号就被写成正常；而网关转发走对话接口，403 稳定必然，日志里就留下封禁记录——这就是 #154 现象分裂的直接成因。

### 现有三套判定的落点

| 入口 | 判定函数 | 写到哪里 |
|---|---|---|
| `getUsageLimits` + `ListAvailableModels` | `commands/common.rs:671` `parse_usage_result` → `is_banned` | 账号 status（**且有 `common.rs:618` 的 profileArn 前提**） |
| `generateAssistantResponse` | `clients/kiro_client.rs:153` `classify_kiro_management_error` | 只 `load_balancer.record_failure`（内存 health_map，**不落盘**） |
| 401 路径 | `update_account_status` | 账号 status = invalid |

前一套有条件跳过且判据不稳定，第二套判据稳定但不落盘。**两套都不通向账号列表。**

### 待做

把第二套接上第一套的落点：`gateway/proxy/upstream.rs` 判出 `BANNED` 时，除了 `record_failure`，同时经 `RouterState.store` 写账号 `status = banned`。④ 已提供的 `SharedAccountStore` 正是这个基础，缺的是那一次写。

### 已实现（2026-10-18）

`gateway/proxy/upstream.rs` 新增封禁分支（排在 `token_expired_error` 之后、402 之前）：命中即经 `lock_shared_store(&state.store)` 调 `update_account_status(account, true, false)` 把账号标为 `banned` 并落盘，随后 `record_failure` 切号、`continue` 不进重试轮。写盘前判 `account.status != "banned"` 保证幂等，避免每次重试都落一次盘。

**关键实现细节**：分支条件用 `error_type == "account_banned_error"`，**不自行解析 body**。`map_upstream_error`（`errors.rs:29-32`、`:68-70`）已是封禁判定的单一出处，此处复用才不会两处漂移。第一版曾用 `message.contains("suspended")` 判断，已纠正——`message` 经过 `sanitize_error` 清洗，不能当判据。

新增 2 个测试锁住判据：封禁映射为 `account_banned_error` + `BANNED:` 前缀；token 失效与封禁是两种 403 不互相掩盖。`cargo test --bins` 311 passed / 0 failed。

### 仍未验证

未起服务实测「封禁账号是否真的在账号列表显示 banned」——需要真实被封禁的账号。另有一个待定项：403 既会偶发出现，也可能偶发消失，**是否应在封禁自动恢复（`enable_account_by_id`）上也加同样的确认门槛**，避免误封与误恢复。

---

## 十三、#152 专项：macOS 未签名导致 Gatekeeper 拦截

### 查证结论：版本映射正确，包完整，非缺陷

**版本映射没有搞反。** `.github/workflows/release.yml:16-25` 的构建矩阵是正确的：

```
macos-15-intel  → x86_64-apple-darwin  → x64.dmg       （Intel Mac）
macos-latest    → aarch64-apple-darwin → aarch64.dmg   （Apple Silicon）
```

**两个包都是完整的**（GitHub Release API 实测，v1.9.2）：

| 资产 | 大小 | sha256 | 下载数 |
|---|---|---|---|
| `KiroAccountManager_1.9.2_aarch64.dmg` | 6,704,479 | 3be60477… | **1503** |
| `KiroAccountManager_1.9.2_x64.dmg` | 7,125,616 | f9dc155d… | 603 |

aarch64 下载量是 x64 的 2.5 倍——大量 Apple Silicon 用户装的就是它。若该包真有问题，不可能有 1503 次下载而仅 1 例报障。

**#152 报障者的现象解释**：他是 M5（Apple Silicon），aarch64 首次下载撞上未签名包的 Gatekeeper 拦截；x64 能装是因为他此前装过旧版、已在「系统设置 → 隐私与安全性」里信任过该开发者/包。**与版本映射无关。**

### 关于未签名

`tauri.conf.json` 与 CI 均未配置 Apple 代码签名与公证（CI 里的 minisign 是 Tauri 更新器签名，管自动更新，与 Apple 签名无关）。未签名的后果是 Gatekeeper 拦截、需用户手动点「仍要打开」，**不是文件损坏**。

**当前结论：非缺陷，无需修复。** 若将来想减少用户困惑，可配 Apple Developer 签名 + 公证（$99/年 + 需 Mac 生成 CSR），但那是体验优化，不是 bug 修复。

**处理结果（2026-10-18）**：已通过 `gh` 在 issue 下发回复（给出 `xattr -cr` 命令与「映射没反」的结论），随后以 `--reason completed` 关闭。issue 状态：CLOSED。

### 与本次改动的关系

第十节、第十一节做的 macOS 样式与窗口适配**不受此影响**——1503 次下载证明用户装得上。

### 已做的补救：README 三语（2026-10-18）

1. 下载表芯片列表 `M1/M2/M3/M4` → `M1/M2/M3/M4/M5`（M5 用户看不到自己机型，会误以为不兼容）
2. 安装说明从「首次运行在安全性与隐私中允许」改为可操作版本：明确「未做 Apple 签名」、给出 `xattr -cr /Applications/KiroAccountManager.app` 命令、提示按芯片选 aarch64 / x64

`README.md` / `README_EN.md` / `README_RU.md` 三份同步，已核对无旧表述残留。

### 给 #152 的回复（可直接贴）

> 两个包的架构没有搞反，你的 M5 用 aarch64 是对的。
>
> 应用只是没做 Apple 代码签名，macOS 对未签名的 app 会提示「已损坏」，并不是文件真的损坏。终端跑一句就行：
>
> ```
> xattr -cr /Applications/KiroAccountManager.app
> ```
>
> 然后重新打开即可。x64 能装是因为你之前装过旧版、系统已经信任过。
>
> README 已补齐说明（芯片列表补了 M5，安装说明给了具体命令）。
