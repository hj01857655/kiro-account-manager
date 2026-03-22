# terminal 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/terminal/**` 注释边界去重

## 结论

- `terminal/**` 当前共识别 `10` 个唯一模块块。
- 它不是简单包一层 VS Code terminal API，而是专门为 agent 命令执行做的终端运行时：
  - shell 检测
  - shell integration 等待
  - 交互式提示识别
  - 输出净化
  - background process 适配

## 模块清单

```text
terminal/adapters/background-process-adapter.ts
terminal/adapters/terminal-manager-adapter.ts
terminal/basic/interactive-prompt-patterns.ts
terminal/basic/terminal-manager.ts
terminal/basic/terminal.ts
terminal/index.ts
terminal/shell-detection.ts
terminal/terminal-errors.ts
terminal/utils/output-sanitizer.ts
terminal/utils/strip-ansi.ts
```

## 分层

### 1. Terminal 实体

- `terminal/basic/terminal.ts`
  - 700 行，terminal 子系统核心
  - 监听 VS Code shell execution 生命周期
  - 跟踪 busy/closed 状态
  - 执行命令并收集输出
  - 处理 shell integration 超时、原始数据读取、输入停滞等边界情况

结论：真正复杂度都在这里，说明 Kiro 很依赖“稳定读回 terminal 输出”，而不是单纯 fire-and-forget。

### 2. TerminalManager

- `terminal/basic/terminal-manager.ts`
  - 维护 terminal 池
  - `getOrCreateTerminal()` 复用空闲 terminal
  - 提供 shell type info 与 terminal 查找能力

这意味着 Kiro 把 terminal 当可复用资源，而不是每个命令新开一个。

### 3. 适配器层

- `terminal/adapters/terminal-manager-adapter.ts`
  - 对外暴露更稳定的 manager 接口
- `terminal/adapters/background-process-adapter.ts`
  - 为 background process manager 提供桥接
  - 支持按 `command + cwd` 复用后台 terminal

### 4. 输出清洗

- `terminal/utils/strip-ansi.ts`
  - 去掉 ANSI 控制序列
- `terminal/utils/output-sanitizer.ts`
  - 去掉 shell integration 的 `633/133` 标记
  - 清掉末尾 shell prompt 残留

关键点：agent 看到的 terminal 输出不是原始字节流，而是“去壳后的内容”。否则模型会被 prompt、控制符和 shell integration 噪声污染。

### 5. 交互式提示识别

- `terminal/basic/interactive-prompt-patterns.ts`

内置识别规则包括：

- `[y/n]`、`(Y/N)` 之类确认提示
- password / passphrase / login
- sudo password
- SSH host key 验证
- Git 凭证提示
- npm / apt / yum / pacman / brew 等安装确认
- Windows / PowerShell 场景

结论：Kiro 很清楚终端命令经常卡在交互提示上，所以专门做了 prompt pattern 库。

### 6. Shell 与错误

- `terminal/shell-detection.ts`
  - 推断 shell 类型和默认平台 shell
- `terminal/terminal-errors.ts`
  - 定义终端层错误类型，给 agent 提供可读错误语义
- `terminal/index.ts`
  - 注册 terminal 相关命令，比如 focus

## 关键机制

### 终端输出的“净化链”

```text
VS Code terminal raw data
    │
    ├─ strip shell integration markers
    ├─ strip ANSI sequences
    └─ trim trailing shell prompt
    ▼
sanitized terminal output
```

这是 agent 能稳定理解命令输出的前提。

### 交互命令不是普通命令

通过 `interactive-prompt-patterns.ts` 和 terminal 主类的输入停滞/等待逻辑可以推断：

- Kiro 区分“命令仍在运行”和“命令在等用户输入”
- 这是后续 approval / ask user / supervised 流程的重要基础

### 后台进程独立适配

`background-process-adapter.ts` 说明：

- 长驻命令不是走普通 terminal 读写路径
- 而是独立接到 background process manager
- 同命令可按 `cwd` 做复用

## 关系图

```text
Agent / Tool command
        │
        ▼
TerminalManager
        │
        ├─ Terminal.create/run/read
        ├─ shell-detection
        ├─ output-sanitizer + strip-ansi
        └─ interactive-prompt-patterns
        │
        ├─ TerminalManagerAdapter
        └─ BackgroundProcessAdapter
```

## 结论

`terminal/**` 是 agent shell 执行能力的可靠性基础层。它解决的不是“怎么开 terminal”，而是：

- 如何稳定复用 terminal
- 如何得到干净输出
- 如何识别交互卡点
- 如何把后台进程和普通命令分流

这也是 Kiro 能把 shell 当一等工具来用的前提。
