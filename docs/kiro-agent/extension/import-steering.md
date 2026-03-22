# import-steering 模块拆分分析

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\dist\extension.js`
> 统计日期：2026-03-18
> 统计口径：按 `// src/extension/import-steering/**` 注释边界去重

## 结论

- `import-steering/**` 当前共识别 `12` 个唯一模块块。
- 这一层负责把 Cursor / Windsurf / Amazon Q / Cline 等外部 AI assistant 的规则文件扫描、解析并转换成 Kiro steering 文档。

## 模块清单

```text
import-steering/ai-assistant-import-service.ts
import-steering/generic-steering-scanner.ts
import-steering/types.ts
import-steering/parsers/cursor-types.ts
import-steering/parsers/cursor-parser.ts
import-steering/parsers/parser-utils.ts
import-steering/ai-assistant-configs.ts
import-steering/generic-steering-parser.ts
import-steering/generic-steering-file-generator.ts
import-steering/util.ts
import-steering/commands/import-all-steering.ts
import-steering/index.ts
```

## 分组

### 1. 核心服务

- `import-steering/ai-assistant-import-service.ts`
- `import-steering/generic-steering-scanner.ts`
- `import-steering/generic-steering-parser.ts`
- `import-steering/generic-steering-file-generator.ts`

这组完成扫描、解析、转换、生成的主流程：

- `import-steering/ai-assistant-import-service.ts` 是总服务入口。
- `import-steering/generic-steering-scanner.ts` 扫描外部规则文件。
- `import-steering/generic-steering-parser.ts` 做通用规则解析。
- `import-steering/generic-steering-file-generator.ts` 输出 Kiro steering 文件。

### 2. 类型与配置

- `import-steering/types.ts`
- `import-steering/ai-assistant-configs.ts`
- `import-steering/util.ts`

这一组定义支持的 assistant 类型和转换时使用的配置/辅助方法。

### 3. Cursor 解析器

- `import-steering/parsers/cursor-types.ts`
- `import-steering/parsers/cursor-parser.ts`
- `import-steering/parsers/parser-utils.ts`

这里专门处理 Cursor 规则格式：

- `import-steering/parsers/cursor-types.ts` 用 `zod` 校验 front matter。
- `import-steering/parsers/parser-utils.ts` 负责内容清洗与 glob 解析。
- `import-steering/parsers/cursor-parser.ts` 把 Cursor 规则转成通用导入结构。

### 4. 命令与注册

- `import-steering/commands/import-all-steering.ts`
- `import-steering/index.ts`

这一组负责命令注册和对外入口。

## 判断

- `import-steering/**` 是外部规则迁移层，不是 steering 本体。
- 它的价值在于把已有 AI assistant 生态中的规则资产导入 Kiro，降低迁移成本。
