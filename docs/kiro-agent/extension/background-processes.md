# background-processes 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/background-processes/**` 注释边界去重

## 结论

- `background-processes/**` 当前共识别 `3` 个唯一模块块。
- 它是 terminal 子系统的长驻命令分支。

## 模块清单

```text
background-processes/background-process-manager.ts
background-processes/background-process-terminal-manager.ts
background-processes/index.ts
```

## 分层

- `background-process-terminal-manager.ts`
  - 维护后台终端资源
- `background-process-manager.ts`
  - 管理后台进程生命周期
- `index.ts`
  - 注册入口

## 判断

这一层专门处理长期存在的后台进程，与普通 terminal 命令执行是两条生命周期。
