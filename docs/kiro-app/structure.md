# Kiro 应用目录结构（resources/app）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app`
> 分析日期：2026-03-17

范围说明：已按要求跳过 `...\resources\app\resources`，不做分析。

---

## 顶层结构

- `LICENSE.txt` / `ThirdPartyNotices.txt`：许可证与三方声明。
- `package.json`：应用入口与构建脚本，`main` 指向 `./out/main.js`，`version` 为 `0.11.34`。
- `product.json`：产品元数据（VS Code 基线版本、应用标识、更新/下载配置、内置扩展列表）。
- `out/`：打包后的主进程/CLI 入口与本地化资源。
- `extensions/`：内置扩展目录（包含 `kiro.kiro-agent` 与大量语言/功能扩展）。
- `node_modules/` 与 `node_modules.asar`：运行时依赖与 asar 索引文件，详见 `docs/kiro-app-node-modules.md`。
- `resources/`：已按要求跳过。

---

## out/ 目录摘要

- `out/main.js`：Electron 主进程入口。
- `out/cli.js`：CLI 入口。
- `out/bootstrap-fork.js`：启动/分叉引导脚本。
- `out/nls.keys.json`、`out/nls.messages.json`：本地化键与消息表。
- `out/media/`、`out/vs/`：前端资源与 VS Code 核心前端代码。
- `out/vs/`：包含 `base`、`code`、`editor`、`platform`、`workbench` 子模块。
- `out/vs/workbench/`：包含 `api`、`browser`、`contrib`、`services`，以及主入口 `workbench.desktop.main.js` 与 `workbench.desktop.main.css`。
- `out/vs/workbench/contrib/`：常见模块目录包括 `debug`、`extensions`、`externalTerminal`、`notebook`、`output`、`terminal`、`webview`。
- `out/vs/workbench/services/`：包含 `extensions`、`languageDetection`、`search`、`textMate`。
- `out/vs/workbench/api/`：`node` 与 `worker` 子模块。
- `out/vs/workbench/browser/`：主要为 `parts` 子模块。
- `out/vs/workbench/browser/parts/`：当前可见 `editor`（含 `media` 资源）。
- `out/vscode-dts/`：VS Code 类型声明（d.ts）输出。
- `out/media/`：应用图标、字体与 UI 资源（如 `codicon.ttf`、`kiricon.ttf`）。

---

## extensions/ 目录摘要

`extensions/` 包含大量内置扩展（语言支持、调试、Git、主题等），同时包含 Kiro 自带的扩展：

- `extensions/kiro.kiro-agent`
- 以及 `ms-vscode.js-debug`、`ms-vscode.js-debug-companion`、`markdown-*` 等常见内置扩展

详细扩展清单见 `product.json` 的 `builtInExtensions`。

内置扩展按用途大致分为：

- 语言支持：`cpp`、`python`、`typescript`、`json`、`yaml`、`go`、`java`、`rust` 等。
- 调试与开发：`ms-vscode.js-debug`、`ms-vscode.js-debug-companion`、`debug-*`。
- Git 与协作：`git`、`github`、`github-authentication`。
- UI/主题：`theme-*`、`markdown-*`、`media-preview`。

---

## product.json 关键字段

- `vsCodeVersion`: `1.107.1`
- `applicationName`: `kiro`
- `dataFolderName`: `.kiro`
- `commit`: `7b506f30719296ba4f1aebfe383b426ffce0913e`
- `quality`: `stable`
- `extensionsGallery`: 使用 Open VSX（`https://open-vsx.org/vscode/gallery`）
- `builtInExtensions`: 包含 `kiro.kiro-agent`、`ms-vscode.js-debug` 等

---

## 目录层级图

```
C:\Users\...\Programs\Kiro\resources\app\
│
├── out/                        打包输出
│   ├── main.js (923KB)         Electron 主进程入口
│   ├── cli.js (209KB)          CLI 入口
│   ├── bootstrap-fork.js       fork worker 引导
│   ├── nls.messages.json       国际化消息表
│   ├── nls.keys.json           国际化键表
│   ├── media/                  字体/图标 (codicon.ttf, kiricon.ttf)
│   └── vs/
│       ├── base/               基础库 (fs/process/platform)
│       ├── editor/             Monaco 编辑器核心
│       ├── platform/           服务层 (配置/日志/遥测/存储)
│       ├── code/               Electron 主进程
│       │   ├── electron-browser/
│       │   ├── electron-utility/
│       │   └── node/
│       └── workbench/          UI 容器
│           ├── api/            node / worker
│           ├── browser/parts/  editor / ...
│           ├── contrib/        debug/extensions/terminal/webview/...
│           └── services/       extensions/search/textMate/...
│
├── extensions/                 内置扩展
│   ├── kiro.kiro-agent/        ← Kiro 全部功能入口
│   ├── ms-vscode.js-debug/
│   ├── git/ github/
│   ├── python/ typescript/ go/ rust/ ...
│   └── theme-*/ markdown-*/
│
├── node_modules/               运行时依赖
├── node_modules.asar           asar 索引
├── package.json                v0.11.34, main→out/main.js
└── product.json                vsCodeVersion:1.107.1, quality:stable
```

---

## package.json 关键字段

- `name`: `Kiro`
- `version`: `0.11.34`
- `main`: `./out/main.js`
- `type`: `module`
