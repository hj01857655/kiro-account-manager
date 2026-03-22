# tree-sitter / treesitter-wasm（语法解析与查询）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\tree-sitter` / `treesitter-wasm`
> 分析日期：2026-03-18

该目录提供 Tree-sitter 查询脚本与多语言 WASM 解析器，用于代码片段抽取、import 识别与根路径上下文提取。

---

## tree-sitter/ 目录

### code-snippet-queries

支持语言：

- `c`
- `c_sharp`
- `cpp`
- `elisp`
- `elixir`
- `go`
- `java`
- `javascript`
- `ocaml`
- `php`
- `python`
- `ql`
- `ruby`
- `rust`
- `typescript`

用途：抽取类 / 函数 / 方法 / 接口定义，捕获 `@comment` / `@name` / `@parameters` 等字段。

### import-queries

支持语言：

- `cpp`
- `java`
- `python`
- `typescript`

用途：抽取 import / include 相关符号，用于依赖与上下文构建。

### root-path-context-queries

支持语言与类别：

- `function_declaration/typescript`
- `function_definition/cpp` / `python`
- `method_declaration/java`
- `method_definition/typescript`

用途：从方法/函数声明中提取类型标注与签名上下文。

---

## treesitter-wasm/ 目录

提供 WASM 解析器：

- `bash`
- `c`
- `c_sharp`
- `cpp`
- `css`
- `elisp`
- `elixir`
- `elm`
- `embedded_template`
- `go`
- `html`
- `java`
- `javascript`
- `json`
- `lua`
- `ocaml`
- `php`
- `python`
- `ql`
- `rescript`
- `ruby`
- `rust`
- `solidity`
- `systemrdl`
- `toml`
- `tsx`
- `typescript`

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         Tree-sitter Runtime                                  │
│                                                                              │
│  treesitter-wasm/*.wasm                                                      │
│   (multi-language parsers)                                                   │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────┐          │
│             │          │ tree-sitter/*.scm queries                │          │
│             │          │ code-snippet / import / root-path        │          │
│             │          └──────────────────────────────────────────┘          │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  AST 片段 / import 列表 / 根路径上下文                           │
└──────────────────────────────────────────────────────────────────────────────┘
```
