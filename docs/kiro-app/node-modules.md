# app/node_modules（主程序运行时依赖）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\node_modules`
> 分析日期：2026-03-18

该目录包含约 115 个依赖包，整体与 VS Code 主程序依赖结构一致，主要支撑终端、文件监听、系统能力与遥测。

---

## 主要依赖分组

- VS Code 基础依赖（`@vscode/*`）：`ripgrep` / `sqlite3` / `tree-sitter-wasm` / `proxy-agent` / `deviceid` / `windows-mutex` / `windows-process-tree` / `windows-registry` / `spdlog` / `sudo-prompt` / `policy-watcher` / `iconv-lite-umd` 等。
- 终端与渲染（`node-pty` / `@xterm/*`）：终端进程与前端渲染。
- 系统键盘与守护（`native-keymap` / `native-watchdog`）。
- 文件监听（`@parcel/watcher`）。
- 证书与代理（`@vscode/proxy-agent` / `windows-ca-certs`）。
- 遥测（`@microsoft/1ds-*`）。
- 兼容辅助（`bindings` / `file-uri-to-path` / `native-is-elevated` / `node-abi`）。
- 其他（`kerberos` / `katex` / `opentype.js` 等）。

---

## 与 out/ 的关系（调用点）

- `native-keymap`：`out/main.js` 的 `keyboardLayoutMainService` 动态加载，负责键盘布局与映射。
- `native-watchdog`：`out/main.js` 在 Extension Host 启动时加载，用于父进程监控。
- `node-pty`：`out/vs/workbench/workbench.desktop.main.js` 终端进程后端。
- `@xterm/*`：`out/vs/workbench/workbench.desktop.main.js` 终端 UI 渲染与插件。
- `@parcel/watcher`：`out/` 文件监听实现（File Watcher）。
- `@vscode/ripgrep`：`out/` 搜索能力（rg 二进制）。
- `@vscode/tree-sitter-wasm`：语法解析（与 `tree-sitter`/`treesitter-wasm` 配合）。
- `@microsoft/1ds-*`：遥测采集与上报。

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                            app/node_modules                                  │
│                                                                              │
│  @vscode/*  (ripgrep/sqlite3/tree-sitter-wasm/proxy-agent/...)               │
│  node-pty + @xterm/*  (终端)                                                   │
│  native-keymap / native-watchdog (系统能力)                                   │
│  @parcel/watcher (文件监听)                                                   │
│  @microsoft/1ds-* (遥测)                                                      │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────┐          │
│             │          │ out/main.js + out/vs/workbench/*          │          │
│             │          │ 主进程 / Extension Host / Workbench       │          │
│             │          └──────────────────────────────────────────┘          │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  终端 / 监听 / 搜索 / 证书 / 遥测 / 系统能力                      │
└──────────────────────────────────────────────────────────────────────────────┘
```
