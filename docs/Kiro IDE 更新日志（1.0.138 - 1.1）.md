# Kiro IDE 更新日志（1.0.138 – 1.1）

> 数据来源：[kiro.dev/changelog/ide](https://kiro.dev/changelog/ide/)  
> 整理时间：2026-09-15　|　覆盖版本：`1.0.138` → `1.1`，共 10 个发布条目。
>
> 说明：各版本的**概述**与**功能小节**已译为中文；`Improvements` / `Fixes` 明细条目保留官方英文原文，便于与官网逐条比对、避免转述失真。

---

## 版本总览

| 版本 | 日期 | 主题 |
| --- | --- | --- |
| `1.1` | 2026-09-14 | Agent Artifacts and Native ARM64 Builds |
| `1.0.437` | 2026-09-01 | Cloud Configuration, Synced Powers, and Agent Focus Reliability |
| `1.0.395` | 2026-08-27 | Third-Party Extension Compatibility and More Reliable MCP Sign-In |
| `1.0.337` | 2026-08-18 | Permission Improvements and Configured Windows Shells |
| `1.0.309` | 2026-08-13 | Nested AGENTS.md and Long-Session Efficiency |
| `1.0.293` | 2026-08-11 | Cloud Sessions and Agent Focus Mode Upgrades |
| `1.0.288` | 2026-08-07 | Agent Plugin Support and Session Pinning |
| `1.0.242` | 2026-07-28 | Editor Actions, Guided Hook Creation, and Code OSS Upgrade |
| `1.0.182` | 2026-07-20 | Global Hooks, Session Search, and Reliability Fixes |
| `1.0.138` | 2026-07-13 | Faster Sessions, Compaction Fixes, and PowerShell Trust |

---

## 1.1（2026-09-14）

### Agent Artifacts and Native ARM64 Builds

IDE 1.1 让智能体产出可复查、可回溯的持久化 Artifacts，新增 Windows / Linux 原生 ARM64 构建，编辑器基座升级到 Code OSS 1.131。压缩后的会话保留正确上下文，MCP 故障提示更清晰，Hooks 更可靠，企业配置文件会将请求路由到所选区域。

> 本条还汇总了 IDE 1.0 近期亮点，包含 Cloud Configuration 与同步 Powers（1.0.437）、权限改进与 Windows 已配置 shell（1.0.337）、Cloud Sessions 与 Agent Focus 升级（1.0.293）、Agent Plugin 支持与会话置顶（1.0.288），供对照查阅。

**Review agent-created artifacts**

Kiro 可以把文档、图表、代码、图片等智能体产出转换为持久化的 Artifacts，而不是埋没在聊天记录里。可以在会话中打开紧凑预览，在 Agent Focus 中并排查看完整 Artifacts，或从 Agent Focus 上下文面板随时返回。

**Install Kiro natively on ARM64**

Windows 与 Linux 现已提供原生 ARM64 构建，不再依赖 x64 模拟。可从 Kiro 下载页选择对应平台的 ARM64 安装包。

**Improvements**

- Code OSS 1.131: Kiro moves from Code OSS 1.109.5 to 1.131, bringing 22 platform releases of editor, terminal, accessibility, and extension-host updates
- Compacted conversations: Kiro rebuilds compacted conversations from system instructions and the summary so later turns retain the right context
- Clearer MCP health and permissions: An MCP server that cannot list its tools now appears as failed instead of connected with zero tools, and MCP tool annotations now inform default permission decisions
- More reliable Hooks: Hook output is retained even when a process exits quickly, legitimate Hook output is accepted without being mistaken for prompt injection, and SessionStart Hook context is now consistently available across all turns
- Project convention awareness: Kiro follows repository `CONTRIBUTING.md` instructions when they define branch, commit, or pull request conventions
- Better Agent Focus continuity: Agent Focus reopens with the same window style and size after a restart, and checkpoint history continues paging correctly after a revert
- Enterprise profile routing: When you sign in with an enterprise identity, Kiro sends service requests to the selected profile's region even when the identity provider is registered elsewhere
- Windows enterprise policies: Kiro reads managed Windows policies from the Kiro registry path instead of the VS Code policy path

**Fixes**

- Fixed UserPromptSubmit Hooks duplicating the user message, Hook instructions, and environment context sent to the model
- Fixed the Agent Focus context panel failing to load Artifacts, Specs, and Changed Files in certain configurations
- Fixed some chat requests failing when an effort level was set while model selection remained automatic
- Fixed the requirements-analysis form becoming difficult to read or submit in narrower chat layouts
- Fixed file and text search failures appearing as empty results instead of reporting the underlying problem
- Fixed dialogs opening behind the sign-in backdrop

---

## 1.0.437（2026-09-01）

### Cloud Configuration, Synced Powers, and Agent Focus Reliability

启用 Cloud configuration 后，受支持的个人配置会出现在 IDE 与 Agent Focus 各个界面，云同步 Powers 也可从已安装列表中直接试用。本版本还保证进行中的云会话草稿在切换导航时不丢失，并在你发出第一条 prompt 之前保留欢迎视图。

**Use Cloud configuration across the IDE**

为账号启用 Cloud configuration 后，可从常规界面访问个人 Steering、自定义智能体、Skills、Powers 与 Hooks。云端标识会标出由 Kiro Web 管理的条目；Steering、Skills、Hooks 以只读预览打开并提供 Edit in web 操作，Agent Focus 中显示同样的来源与管理链接。

**Try cloud-synced Powers in the IDE**

云同步 Powers 现在会出现在已安装 Powers 列表中，可选择 Try power 并查看其捆绑的 Skills 与 MCP 配置。这些账号级 Powers 需在 Kiro Web 中管理。

**Fixes**

- Fixed an issue where Agent Focus discarded an in-progress cloud session draft when you opened Settings or another session. Your draft stays available while session creation finishes in the background, and failures appear on the draft row
- Fixed the new-session welcome view disappearing before you sent your first prompt when Cloud configuration setup appeared in the session transcript
- Fixed Agent Focus showing a Cloud configuration notice on the MCP Servers tab, even though personal Cloud configuration does not include MCP servers

---

## 1.0.395（2026-08-27）

### Third-Party Extension Compatibility and More Reliable MCP Sign-In

本版本让第三方扩展可与 Kiro 协同工作，支持最新 MCP 协议修订版并带来更可靠的登录，减少大型智能体响应期间的扩展宿主崩溃，并在网络短暂断开时保持当前回合继续。

**Improvements**

- Third-party extensions work alongside Kiro: Third-party extensions such as Vue (Volar) and the AWS Toolkit can reach built-in editor APIs like the Git API. Kiro runs in a dedicated extension host, so other extensions keep full access to those APIs.
- MCP OAuth compatibility and more reliable sign-in: Adds support for MCP protocol revision 2026-07-28 and resolves several MCP sign-in and connection failures, so servers that use OAuth or credentials delivered through an enterprise registry connect reliably, and a slow or unreachable server no longer holds up session startup. Kiro now validates that authorization-server metadata `issuer` exactly matches the authorization-server URL advertised by an OAuth-protected MCP server, as required by RFC 8414. A trailing-slash difference can prevent the server from connecting until its server or identity-provider metadata is corrected. $L3c /docs/mcp/configuration#oauth-configuration text-primary hover:underline $undefined $undefined Learn more ->
- Bring your own OAuth client identity for MCP (enterprise): Point an MCP server at a hosted client metadata document with `oauth.clientMetadataUrl` to authenticate without Dynamic Client Registration. Pair it with a pinned `oauth.redirectUri` so the authorization server accepts the callback. $L3c /docs/mcp/configuration#client-id-metadata-documents-cimd text-primary hover:underline $undefined $undefined Learn more ->

**Fixes**

- Reduced extension host crashes during large agent responses that could interrupt your session and lose in-progress work
- Agent turns now recover from brief network drops, retrying dropped TLS and keep-alive connections instead of ending the turn

---

## 1.0.337（2026-08-18）

### Permission Improvements and Configured Windows Shells

本版本让被忽略的文件不再出现在搜索结果中，改进文件系统与命令权限规则，并在 Windows 上按你配置的终端配置文件运行智能体命令。

**Improvements**

- Ignored files stay out of search: File and text search now check every result against your read permissions and ignore rules, so ignored paths and matching content do not appear. $L3c /docs/kiroignore text-primary hover:underline $undefined $undefined Learn more ->
- Filesystem permission rules: Directory patterns such as `src/` and `src/**` now cover the directory and its descendants. Absolute paths and patterns beginning with `~/` match correctly, and code actions that modify files use write permissions. $L3c /docs/permissions text-primary hover:underline $undefined $undefined Learn more ->
- Persistent choices for unparseable commands: Kiro hides Always allow and Always deny when it cannot verify that a saved rule will work, and explains why. Chained `cmd.exe` commands are also parsed more accurately, reducing unnecessary prompts and rules that fail to match.
- Configured Windows shell: Agent commands now run in your default terminal profile on Windows, including Git Bash and WSL. Kiro reuses agent terminals only when their shell matches your profile. $L3c /docs/ide/chat/terminal text-primary hover:underline $undefined $undefined Learn more ->

---

## 1.0.309（2026-08-13）

### Nested AGENTS.md and Long-Session Efficiency

可在工作区各处放置 AGENTS.md，为 Kiro 提供按目录树作用域生效的指令。本版本还降低空闲资源占用，在长会话中保留更多上下文，并改进 Cloud configuration、chat、hooks 与 MCP 连接。

**Improvements**

- Nested AGENTS.md support: Place `AGENTS.md` files in subdirectories to give Kiro guidance scoped to that part of your workspace. Discovery respects Git and Kiro ignore rules.
- Lower idle resource use: Kiro uses less idle CPU as your session history grows
- More efficient context use: Unchanged files are not repeatedly added to the conversation, leaving more context available during longer sessions
- More reliable skill monitoring: Skills with dependency directories no longer use excessive system resources that can disrupt Git, linters, and editor extensions
- More reliable Cloud configuration sync: Steering, custom agents, skills, Powers, and hooks synced to Cloud Sessions now recover from temporary failures and remain consistent. $L3c /docs/cloud-sessions text-primary hover:underline $undefined $undefined Learn more ->
- Full chat tab titles on hover: Hover over a truncated chat tab title to view the full name

**Fixes**

- Fixed chats containing certain Go code blocks freezing or failing to reopen
- Fixed workspace file links opening the wrong location in multi-root workspaces or when a link uses a `file://` URI, including in Agent Focus Mode
- Fixed global hooks in `~/.kiro/hooks/` opening with empty fields in the Agent Hooks editor. Failed loads no longer risk overwriting an existing hook.

---

## 1.0.293（2026-08-11）

### Cloud Sessions and Agent Focus Mode Upgrades

本版本以预览形式把 Cloud Sessions 带入 Agent Focus Mode，并给会话侧栏加入可折叠的纯图标视图、可就地回答的 attention cards，以及后退 / 前进导航。同时把 Windows 上智能体终端固定为 PowerShell，恢复此前被静默遗忘的 Always Allow，并修复 macOS 更新器重复下载的问题。

**Cloud Sessions in Agent Focus Mode**

Cloud Sessions 现已在 Agent Focus Mode 以预览形式提供。可启动一个与本地会话并行运行在云端的会话，选择它作用的仓库，并把这些仓库常驻显示在聊天区下方。云端会话使用与本地会话相同的智能体与模型选择器。

**Agent Focus Mode: collapsible rail, attention cards, and navigation**

会话侧栏可折叠为图标条，为聊天区腾出空间，每个会话仍一次点击可达。会话需要你输入时，聊天旁会出现 attention card，把问题与智能体选项以按钮呈现，无需切换即可回答。该面板不会自行弹出，也不会为你当前正在查看的会话主动唤起。还可以在标题栏用 Cmd/Ctrl+[ 与 Cmd/Ctrl+] 在会话与设置间前后导航。

**Improvements**

- Windows agent terminal runs PowerShell: The agent's terminal is now pinned to PowerShell on Windows, so the PowerShell-style commands the agent writes run the first time instead of failing under `cmd.exe` . Your own terminals, default profile, and terminal settings are untouched

**Fixes**

- Fixed the macOS updater downloading each update at least twice, and silently re-downloading a staged update every few minutes while it waited to install
- Fixed `Always` Allow never persisting for commands with a bare `-` or `--` operand or a `./path` argument, which re-prompted you for the same command on every run. $L3c /docs/permissions text-primary hover:underline $undefined $undefined Learn more ->
- Fixed disabling an MCP server removing it from the MCP Servers panel when a custom registry is in use, which left no way to re-enable it. $L3c /docs/mcp text-primary hover:underline $undefined $undefined Learn more ->
- Fixed `UserPromptSubmit` command hooks in the new hook format receiving an empty prompt on stdin, and their output not reaching the agent's context. $L3c /docs/hooks text-primary hover:underline $undefined $undefined Learn more ->
- Fixed file-reading tool calls being rejected when the model left an optional field empty, which could stall a session

---

## 1.0.288（2026-08-07）

### Agent Plugin Support and Session Pinning

本版本支持以开放 Agent Plugin 格式打包的 Powers，并重做 Agent Focus Mode 会话侧栏，加入置顶、移交 Kiro CLI 与应用内更新。同时将 Kiro 升级到 Code OSS v1.109.5，改进 spec 悬浮与 MCP 安装对不受信任工作区内容的处理，并新增批量会话迁移与更稳定的错误恢复。

**Agent Plugin support for Powers**

可从本地文件夹或 GitHub URL 安装符合开放 Agent Plugin 格式的 Powers。Kiro 现支持捆绑 skills 与 MCP 的插件，使 Powers 更易在兼容的智能体工具间分享。

**Pin and manage sessions in Agent Focus Mode**

用 Pin Session 置顶会话，使其固定在所属分区顶部；用 Open with Kiro CLI 在 Kiro CLI 中恢复已有会话。有新版本时侧栏底部会出现更新行，设置面板新增 Check for Updates 按钮。上下文面板中的 spec 文档与任务行现在可直接在编辑器中打开。

**Improvements**

- Safer MCP installs and spec hovers: Installing an MCP server from a `kiro://` link opens a confirmation dialog before anything is written to your configuration, showing the command, its arguments, and the names of environment variables or headers with their values hidden. Spec file content renders as literal text in hover tooltips, and spec hovers contain no clickable links in workspaces you haven't trusted. $L3c /docs/mcp text-primary hover:underline $undefined $undefined Learn more ->
- Code OSS v1.109.5 upgrade: Kiro moves from Code OSS v1.108.2 to v1.109.5. Terminal sticky scroll can skip specific commands via `terminal.integrated.stickyScroll.ignoredCommands` , you can recolor matching brackets with $L56 , and $L57 lets you opt in to terminals in untrusted workspaces

**Fixes**

- Fixed long multi-line commands being cut off partway, which left the terminal stuck at a `heredoc>` or `dquote>` continuation prompt
- Fixed orphaned shell processes lingering after their command ended
- Fixed the Windows updater installing a second copy of Kiro in another location instead of updating the copy you are running
- Fixed switching agents mid-session not taking effect, which left the previous agent driving the next turn
- Fixed stopping a turn being reported as an error instead of a cancellation
- Fixed a failed turn's error message appearing three times instead of once
- Fixed a message sent just as the agent finished its response being silently dropped
- Fixed spec tasks and sub-agent dispatch failing with `Invalid` tool use format when a custom agent profile has a long name. The agent is registered under a shortened name with a warning naming it
- Fixed a sub-agent tool call interrupted partway through failing the run instead of recovering
- Fixed `Always` Allow rules saved for a workspace not applying in future sessions
- Fixed `Always` Allow not persisting for PowerShell commands that use `--flag=value` syntax
- Fixed the agent being unable to create or edit `mcp.json` files outside Kiro's configuration directories
- Fixed custom MCP registries rejecting namespaced server names, and one malformed entry hiding every server in the registry
- Fixed clicking a skill or auto-inclusion steering document in the Steering panel not opening it on Windows
- Fixed Kiro staying signed in but unable to work after an external identity provider session expired. You now return to the sign-in page

---

## 1.0.242（2026-07-28）

### Editor Actions, Guided Hook Creation, and Code OSS Upgrade

本版本在编辑器右键菜单中加入 Kiro 子菜单，并为错误与警告提供 Ask Kiro to Fix 快速修复，同时新增创建 hooks 的引导表单。Kiro 升级到 Code OSS v1.108.2，并会按 IDE 显示语言回复。聊天记录也能在切换工作区目录后保留，另外修复了若干登录、信任与审批问题。

**Improvements**

- Editor context menu actions: Right-click a selection in a file to open the Kiro submenu. `Ask` Kiro opens a new sidebar session with the selection attached and waits for you to type. Fix Grammar / Spelling, Write Comments for this Code, Write a Docstring for this Code, Fix this Code, and Optimize this Code run in a single click. Customize any of the prompts with the `kiroAgent.editorActions.prompts` setting
- `Ask` Kiro to Fix: The lightbulb on any error or warning now offers `Ask` Kiro to Fix , with an editable prompt and the affected lines attached as context. It appears below your language server's own fixes
- Guided hook creation: The + button in the Agent Hooks view opens a Create a hook form where you can configure a hook by hand. Clicking a hook in the tree opens it in the editor, and edits made to the underlying file elsewhere refresh the open form. $L3c /docs/hooks text-primary hover:underline $undefined $undefined Learn more ->
- Replies in your display language: Kiro reads your VS Code display language and replies in it. Code, API names, file paths, and command output stay untranslated
- Code OSS v1.108.2 Upgrade: Kiro moves from Code OSS v1.107.1 to v1.108.2. Terminal IntelliSense ( `terminal.integrated.suggest.enabled` ) is now on by default, matching the upstream default

**Fixes**

- Fixed chat failing to start, with no response from the agent, when certain extensions (e.g. SARIF Viewer) are installed
- Fixed unexpected sign-outs when two IDE windows saved the stored token at the same time, which could leave it briefly unreadable and end a valid session
- Fixed the trust dialog showing `1` instead of the command for `cmd` commands containing `2>&1` , which also saved an "Always Allow" rule that never matched (Windows only)
- Fixed Tab not selecting the highlighted entry in the slash command and `#` context picker menus
- Fixed pasting a URL copied from the Chrome or Edge address bar inserting the page title instead of the URL
- Fixed a follow-up message sent while a tool approval was still pending starting a turn with no memory of the conversation, so the agent asked you to clarify what you meant
- Fixed Supervised mode losing the `Review` changes panel when a session paused on an approval was closed and reopened, leaving a "Working" card with no way to accept or reject
- Fixed clicking a file name in the Supervised mode `Review` changes bar not opening that file's diff
- Fixed spec workflow questions hanging after being answered in an Agent Focus Mode window
- Fixed answered question cards showing only the question instead of the answer you submitted. Reopening a past session still shows the question on its own
- Fixed the steering, skills, and MCP prompt pickers reporting `No` matching items in Agent Focus Mode windows
- Fixed granting `Always` Allow for a file outside the workspace failing with a `cannot` open file:///consent error
- Fixed an MCP server that requires OAuth failing with an abort error immediately after sign-in completed
- Fixed diagnostics intermittently reporting `No` diagnostics found for a file the language server had flagged

---

## 1.0.182（2026-07-20）

### Global Hooks, Session Search, and Reliability Fixes

本版本新增用户级全局 hooks、可搜索的会话历史面板，并增强企业代理环境下的可靠性。智能体现在可从中断的响应中自动恢复，并动态刷新 MCP 工具。

**Improvements**

- Global hooks: Hook files placed in `~/.kiro/hooks/` now apply across all workspaces, useful for cross-project automation like formatting or linting
- Session history search: The session list now has a search bar that filters by title or date. Use `Cmd+F` (Mac) / `Ctrl+F` (Windows) to focus it. Legacy sessions are also sorted by most recent
- MCP dynamic tool refresh: When an MCP server adds or removes tools at runtime, the IDE picks them up automatically without reconnecting
- Large command output handling: Oversized tool responses (shell, MCP, web) are now saved to a file with a head/tail preview, keeping long output accessible without cluttering the conversation.
- Custom agent hot reload: Editing files in `.kiro/agents/` now immediately updates the agent picker in running sessions without requiring a new session
- Task progress card: The `todo_list` tool now renders a dedicated card showing each task's status and a progress indicator
- Corporate proxy support: All backend connections (model streaming, MCP, feature config, usage limits) now honor `HTTP_PROXY` / `HTTPS_PROXY` environment variables. Previously only some paths respected proxy settings
- Automatic recovery from interrupted responses: Sessions are now more resilient to transient backend interruptions. If a response is cut short, the agent retries automatically instead of stopping mid-task

**Fixes**

- Fixed an issue where spec clarifying questions could hang permanently, requiring a session restart
- Fixed tool error messages sometimes showing raw technical output instead of a readable summary
- Fixed the `always` accept permission not applying correctly to shell commands containing backslashes (e.g., regex escapes in grep)
- Fixed arrow keys in the chat input always cycling prompt history instead of moving the cursor within multi-line content
- Fixed intermittent sign-outs that could occur when multiple IDE windows were open simultaneously
- Fixed markdown not rendering in pending spec workflow questions
- Fixed a file-loading issue on Windows where a system-level directory was checked before the local package
- Fixed the session title not resetting when a checkpoint revert removes the first prompt
- Fixed enterprise governance settings not appearing in the Agent Focus Mode window until the next session. Settings now display correctly on connect
- Fixed LaTeX formulas in agent responses rendering as raw code instead of proper equations
- Fixed file paths in agent responses not being clickable. Links now open the file in the editor and support line ranges

---

## 1.0.138（2026-07-13）

### Faster Sessions, Compaction Fixes, and PowerShell Trust

本版本带来更快的会话启动，修复了在大型会话上导致反复出现 Context limit exceeded 的压缩循环，并新增 Windows 上完整的 PowerShell 信任支持。MCP 工具现在也能从瞬时网络故障中恢复，而不是静默失败。

> 更新（2026 年 8 月 18 日）：IDE 自动更新已重新启用，更新会在后台自动下载。

> 注意：要获取最新的 1.0.x 版本，请直接从 kiro.dev/downloads 下载。自动更新正在逐步向用户推送。

**Improvements**

- Faster session startup and shell commands: Permission checks are now significantly faster, especially for sessions with many saved trust rules. Sessions start quicker and each shell command runs with less overhead before execution
- MCP tool resilience: MCP tools and profile lookups now recover gracefully from transient network failures. A brief connectivity interruption no longer permanently disables tools for the rest of the session
- PowerShell command trust: `Always` allow rules now work correctly for piped and compound PowerShell commands on Windows. Previously, PowerShell substitutions `($(...`)) were opaque to the trust system and re-prompted indefinitely
- Per-agent hooks: Inline `hooks` declared on custom agent profiles now register and fire correctly. Previously they were silently dropped
- Copy Request IDs — A new `Copy` Request ID(s) option in the message copy menu lets you grab backend request IDs from any turn for support tickets

**Fixes**

- Fixed context compaction loop where summarization couldn't shrink sessions containing a single large turn, causing repeated `Context` limit exceeded errors
- Fixed the first user prompt steering the agent's behavior after context compaction, causing it to forget the current task and repeat early instructions
- Fixed supervised mode silently reverting accepted same-file edits when the agent made multiple edits to one file in a single turn
- Fixed agent hang when a sub-agent asks a free-form question with no predefined options, and added a text input UI so users can answer these questions
- Fixed older sessions failing every turn with a validation error after tool names changed between versions. These sessions now self-heal on load
- Fixed shell type detection on Windows defaulting to `cmd` even when running PowerShell, which prevented the PowerShell trust parser from being used
- Fixed Agent Focus context panel auto-opening for specs the session only read rather than authored
- Fixed the feedback form not rendering above the chat webview in Agent Focus mode
- Fixed the Settings pane in Agent Focus being kicked back to chat after a few seconds
- Fixed turns appearing blank when the model declines a request due to content policy. A clear refusal message now appears instead

