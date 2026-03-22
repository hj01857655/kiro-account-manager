# node_modules（扩展运行时依赖）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\node_modules`
> 分析日期：2026-03-18

该目录仅包含 10 个依赖包，主要用于本地向量检索、Tree-sitter 解析与系统证书注入。

---

## 顶层包清单

- `@lancedb/vectordb-win32-x64-msvc`
- `@vscode/ripgrep`
- `bindings`
- `file-uri-to-path`
- `onnxruntime-common`
- `onnxruntime-node`
- `sqlite3`
- `web-tree-sitter`
- `win-ca`
- `workerpool`

---

## 关键依赖说明

- `onnxruntime-node` / `onnxruntime-common`：ONNX Runtime Node 绑定与通用 API，支持本地 embedding 推理。
- `@lancedb/vectordb-win32-x64-msvc` + `sqlite3`：本地向量库与 SQLite 依赖（Windows 二进制）。
- `web-tree-sitter`：Tree-sitter Web 绑定，配合 `treesitter-wasm/` 解析多语言代码。
- `@vscode/ripgrep`：内置 ripgrep 搜索支持。
- `win-ca`：Windows 系统根证书注入。
- `workerpool`：多线程/worker 池。
- `bindings` / `file-uri-to-path`：原生模块加载与 URI 转路径工具。

---

## 与其他目录的关系

- `web-tree-sitter` 在 `extension.js` 的 `shared-parser.ts` 被加载，使用 `treesitter-wasm/tree-sitter-<lang>.wasm` 解析代码。
- `@lancedb/vectordb-*` 与 `sqlite3` 由 Continue 检索路径使用，索引目录为 `getIndexFolderPath()/lancedb` 与 `docs.sqlite`。
- `onnxruntime-*` 与 `models/all-MiniLM-L6-v2` 配合，用于本地 embedding 推理。
- `win-ca` 通过 `bindings("win_export_cert")` 读取系统证书，服务于 `setupCa()` 证书注入流程。
- `@vscode/ripgrep` 的二进制由 `getExtensionUri().fsPath/out/node_modules/@vscode/ripgrep/bin/rg` 调用，供搜索能力使用。

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                             node_modules                                     │
│                                                                              │
│  onnxruntime-node + onnxruntime-common                                       │
│  @lancedb/vectordb-win32-x64-msvc + sqlite3                                  │
│  web-tree-sitter + treesitter-wasm                                           │
│  @vscode/ripgrep + workerpool + win-ca                                       │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────┐          │
│             │          │ kiro.kiro-agent runtime                  │          │
│             │          │ embeddings / search / parsing / TLS      │          │
│             │          └──────────────────────────────────────────┘          │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  本地推理 / 向量检索 / 语法解析 / 证书注入                        │
│             │  models/ + tree-sitter/ + treesitter-wasm/ + out/node_modules/ │
└──────────────────────────────────────────────────────────────────────────────┘
```
