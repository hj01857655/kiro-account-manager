# platform 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/platform/**` 注释边界去重

## 结论

- `platform/**` 当前只有 `1` 个模块块：
  - `platform/vscode-file-system.ts`
- 它承担的是平台抽象职责：把 VS Code `workspace.fs` 包成统一文件系统接口。

## 模块清单

```text
platform/vscode-file-system.ts
```

## 职责

- 提供基于 `vscode.workspace.fs` 的文件系统读写/枚举能力
- 给上层模块一个更稳定的“平台文件系统”抽象，而不是直接散用 VS Code API

## 判断

这类单文件目录通常意味着：

- 当前只实现了 VS Code 平台适配
- 但架构上已经留好了“平台抽象层”

如果未来 Kiro 要抽离出非 VS Code 宿主，这个目录会自然扩张。
