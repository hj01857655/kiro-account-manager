# steering 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/steering/**` 注释边界去重

## 结论

- `steering/**` 当前共识别 `16` 个唯一模块块。
- 它负责 Kiro 最关键的“规则文档系统”：
  - 发现 steering 文件
  - 解析 front matter
  - 监听变更
  - 计算 progressive context
  - 暴露 tree view 与命令

## 模块清单

```text
steering/commands/create-default-steering.ts
steering/commands/create-initial-steering.ts
steering/commands/create-steering-and-skills.ts
steering/commands/delete-skill.ts
steering/commands/delete-steering.ts
steering/commands/get-skills.ts
steering/commands/get-steerings.ts
steering/commands/import-skills.ts
steering/commands/refine-steering.ts
steering/file-utils.ts
steering/index.ts
steering/progressive-context-loader.ts
steering/progressive-context-registry.ts
steering/steering-controller.ts
steering/types.ts
steering/views/steering-treeview.ts
```

## 分层

### 1. SteeringController

- `steering/steering-controller.ts`
  - 407 行，主协调器
  - 负责 steering 文档列表、读取、删除、创建、刷新
  - 也是 tree view 与 progressive context 的汇合点

### 2. 文件与类型

- `steering/types.ts`
  - 定义 steering 文档结构与 inclusion 模式
- `steering/file-utils.ts`
  - 处理 `.kiro/steering/`、`AGENTS.md`、skills 等路径细节

关键点：steering 在这里不是“任意 markdown”，而是带 front matter 语义的规则文档。

### 3. Progressive Context

- `steering/progressive-context-registry.ts`
  - registry 层
- `steering/progressive-context-loader.ts`
  - 269 行
  - 扫描 skills/steering 文件
  - 构建 progressive context 载荷
  - 监听文件变化后更新结果

这部分说明 Kiro 不是每次都把所有规则无脑塞进 prompt，而是有“按需加载的规则上下文系统”。

### 4. 命令层

- `steering/commands/create-initial-steering.ts`
  - 触发 agent 自动生成初始 steering
- `steering/commands/create-default-steering.ts`
  - 生成默认模板
- `steering/commands/create-steering-and-skills.ts`
  - 把 steering 和 skills 一起初始化
- `steering/commands/import-skills.ts`
  - 226 行，命令层里最重
  - 负责技能导入
- `steering/commands/delete-steering.ts`
- `steering/commands/delete-skill.ts`
- `steering/commands/get-steerings.ts`
- `steering/commands/get-skills.ts`
- `steering/commands/refine-steering.ts`

结论：steering 命令层不仅创建/删除规则，还直接覆盖了 skill 导入与 refine 闭环。

### 5. Tree View

- `steering/views/steering-treeview.ts`
  - 152 行
  - 提供侧边栏中的 steering explorer

## 关键机制

### 三路来源

结合 bundle 其他模块可确认，steering 文档至少来自三路：

- workspace `.kiro/steering/*.md`
- 全局 `~/.kiro/steering/*.md`
- workspace 根 `AGENTS.md`

所以 steering 不只是项目内规则，也包括用户级全局规则和仓库根指导文件。

### inclusion 模式

`steering/types.ts` 对应的文档语义至少覆盖：

- `always`
- `fileMatch`
- `manual`
- `auto`

这决定了哪些文档会在当前上下文被附加给 agent。

### skills 与 steering 深度绑定

从 `create-steering-and-skills.ts` 与 `import-skills.ts` 可以看出：

- skills 不是独立 feature
- 而是 steering 体系的一部分
- progressive context loader 也会扫描 `SKILL.md`

## 关系图

```text
.kiro/steering/*.md   ~/.kiro/steering/*.md   AGENTS.md   SKILL.md
         │                    │                  │          │
         └──────────────┬─────┴──────────────────┴──────────┘
                        ▼
                 file-utils + types
                        │
                        ▼
                SteeringController
                        │
            ┌───────────┴───────────┐
            ▼                       ▼
   steering-treeview       progressive-context-loader
            │                       │
            └───────────┬───────────┘
                        ▼
                    commands
        create / refine / import-skills / delete / list
```

## 结论

`steering/**` 是 Kiro 的规则装配层。它真正做的是：

- 把 markdown 规则文件变成可计算上下文
- 把 skills 纳入同一规则体系
- 把全局规则、工作区规则和仓库规则统一起来

这也是 Kiro “像 agent，但可被规则化约束”的基础。
