# Kiro IDE 主程序概览

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app`
> 分析日期：2026-03-17

---

## 结论

Kiro 是标准 VSCode fork，主程序无自有业务逻辑，全部 Kiro 功能通过 `kiro.kiro-agent` 扩展实现。

---

## 产品标识（main.js）

- `nameShort`：`Kiro`，`nameLong`：`Kiro Dev`
- `applicationName`：`kiro`
- `dataFolderName`：`.kiro`（用户数据目录）
- `urlProtocol`：`kiro`（深链接协议 `kiro://`）
- `reportIssueUrl`：指向 VSCode GitHub Issues（未替换为 Kiro 自有地址）
- `licenseName`：MIT

## 主程序文件

- `out/main.js`（923KB）：Electron 主进程，标准 VSCode lifecycle 管理（BrowserWindow、before-quit、will-quit）。
- `out/cli.js`（209KB）：CLI 入口。
- `out/bootstrap-fork.js`（27KB）：fork worker 启动引导。
- `out/nls.messages.json` / `nls.keys.json`：国际化字符串（合计约 1.2MB）。

## 目录结构

- `out/vs/base`：VSCode 基础库（文件系统、进程、平台工具）。
- `out/vs/editor`：Monaco 编辑器核心。
- `out/vs/platform`：服务层（配置、日志、遥测、存储）。
- `out/vs/workbench`：UI 容器（browser/contrib/services/api）。
- `out/vs/code`：Electron 主进程（electron-browser / electron-utility / node）。
- `resources/win32/`：Windows 文件类型图标（.ico/.png），标准 VSCode 资源，未替换为 Kiro 品牌图标。

## 扩展列表关键项

`extensions/` 下包含全部内置语言扩展（与 VSCode 相同），Kiro 自有扩展只有一个：

- `kiro.kiro-agent`：全部 Kiro 功能入口，详见 `../kiro-agent/extension/overview.md`。

## 依赖（package.json）

- `@kiro/sign-in-page`：Kiro 自有登录页包（`^0.1.0`）。
- `zod`：`^3.23.8`。
- `@parcel/watcher`、`ripgrep`、`tree-sitter-wasm`：文件监听、搜索、语法解析。
- `@microsoft/1ds-core-js` / `1ds-post-js`：Microsoft 遥测 SDK（`^3.2.13`）。
- `@vscode/proxy-agent`、`@vscode/iconv-lite-umd`、`@vscode/ripgrep` 等：标准 VSCode 依赖。

---

## 整体架构图

```
┌──────────────────────────────────────────────────────────┐
│                  Kiro IDE 进程 (Electron)                  │
│                                                          │
│  Electron Main (out/main.js)                             │
│    BrowserWindow lifecycle                               │
│    before-quit / will-quit                               │
│          │                                               │
│          ▼                                               │
│  VS Code Workbench (out/vs/workbench/)                   │
│    contrib / services / api / browser                    │
│          │                                               │
│          ▼  扩展宿主进程                                   │
│  extensions/kiro.kiro-agent/                             │
│    dist/extension.js (49MB)  ← 全部 Kiro 功能             │
│          │                                               │
│    ┌─────┴──────────────────────────┐                    │
│    │       packages/ 子模块          │                    │
│    │  autocomplete                  │                    │
│    │  kiro-context-providers        │                    │
│    │  kiro-client  ────────────────────────────────────► │─── ACP ──► Kiro 后端
│    │  kiro-shared                   │                    │
│    │  hook-editor                   │                    │
│    │  kiro-ui-agent-chat            │                    │
│    │  kiro-ui-powers                │                    │
│    │  requirements-webview          │                    │
│    │  acp-type-covenant             │                    │
│    └────────────────────────────────┘                    │
│                                                          │
│  深链接: kiro://  (deep link handler)                     │
│  数据目录: ~/.kiro/                                       │
└──────────────────────────────────────┬───────────────────┘
                 文件系统共享           │
          (token / profile / mcp.json) │
                                       │
┌──────────────────────────────────────▼───────────────────┐
│              kiro-account-manager (独立 Tauri 进程)        │
│  多账号管理 / token 持久化 / 自动切换                        │
│  kiro_cli_db.rs 读写 .kiro 数据目录                        │
└──────────────────────────────────────────────────────────┘
```

---

## 与 kiro-account-manager 的关系

kiro-account-manager 是独立的 Tauri 桌面应用，负责多账号管理、token 持久化和自动切换，通过 Kiro CLI 数据库（`kiro_cli_db.rs`）与 Kiro IDE 的 `.kiro` 数据目录共享认证状态。两者不在同一进程，通信依赖文件系统（token 缓存文件）和 deep link（`kiro://` 协议）。
