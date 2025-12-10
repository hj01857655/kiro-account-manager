# 测试指南

本文档描述如何运行 kiro2api-deno 项目的各种测试。

## 🚀 快速开始

运行所有测试：

```bash
chmod +x test.sh
./test.sh
```

## 📋 测试类型

### 1. 类型检查（TypeScript）

验证所有 TypeScript 代码的类型安全性。

```bash
deno task check
# 或
deno check main.ts
```

**通过条件**：无类型错误

### 2. Lint 检查

检查代码质量和风格问题。

```bash
deno task lint
# 或
deno lint
```

**通过条件**：无 lint 警告或错误

### 3. 代码格式化

检查代码格式是否一致。

```bash
# 检查格式
deno fmt --check

# 自动修复格式
deno fmt
```

**通过条件**：所有文件格式正确

### 4. 冒烟测试

测试核心功能和格式转换逻辑。

```bash
deno run --allow-net --allow-env --allow-read smoke_test.ts
```

**测试内容**：
- ✅ OpenAI → Anthropic 格式转换
- ✅ Anthropic → OpenAI 格式转换
- ✅ 工具调用格式转换
- ✅ 图片内容处理
- ✅ 模型映射验证

**通过条件**：所有断言通过（18/18）

### 5. 端到端（E2E）测试

测试 API 端点和服务器功能。

```bash
deno run --allow-net --allow-env --allow-read --allow-write --allow-run --unstable-kv e2e_test.ts
```

**测试内容**：
- ✅ 健康检查端点 (`/`)
- ✅ 模型列表端点 (`/v1/models`)
- ✅ 认证验证
- ✅ CORS headers
- ✅ 错误处理

**注意**：E2E 测试会启动临时服务器，可能需要几秒钟。

## 🔍 测试结果解读

### ✅ 成功标识

- `✅ PASS` - 单个测试通过
- `✓` - 测试步骤完成
- `🎉 所有测试通过！` - 所有测试成功

### ❌ 失败标识

- `❌ FAIL` - 单个测试失败
- `✗` - 测试步骤失败
- `❌ 部分测试失败` - 至少一个测试失败

### ⚠️ 警告标识

- `⚠` - 非关键问题，不影响主要功能

## 🛠️ 常见问题

### Q: 类型检查通过但 lint 失败？

A: 这通常是代码风格问题，比如未使用的变量。检查 lint 输出并修复。

### Q: E2E 测试失败？

A: E2E 测试需要启动服务器。确保：
1. 端口 18080 未被占用
2. 有足够的权限运行 `deno task start`
3. 环境变量正确设置

### Q: 如何添加新测试？

A: 
1. **单元测试**：添加到 `smoke_test.ts`
2. **集成测试**：添加到 `e2e_test.ts`
3. 更新本文档

## 📊 测试覆盖率

当前测试覆盖的主要模块：

| 模块 | 覆盖率 | 说明 |
|------|--------|------|
| 格式转换器 (`converter/`) | ✅ 高 | 冒烟测试覆盖 |
| 类型定义 (`types/`) | ✅ 高 | TypeScript 类型检查 |
| 配置 (`config/`) | ✅ 中 | 常量验证 |
| API 端点 (`server/`) | ✅ 中 | E2E 测试覆盖 |
| 认证 (`auth/`) | ⚠️ 低 | 需要真实 token |
| 流处理 (`parser/`, `server/stream_processor.ts`) | ⚠️ 低 | 需要真实 AWS API |

## 🎯 CI/CD 集成

### GitHub Actions 示例

```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: denoland/setup-deno@v1
        with:
          deno-version: v2.x
      - name: Run type check
        run: deno task check
      - name: Run lint
        run: deno task lint
      - name: Run smoke tests
        run: deno run --allow-net --allow-env --allow-read smoke_test.ts
```

## 🚦 测试状态

最近测试结果：

```
✅ 类型检查: PASS (0 errors)
✅ Lint 检查: PASS (0 warnings)
✅ 冒烟测试: PASS (18/18 tests)
⚠️  E2E 测试: 需要环境变量
```

## 📝 测试最佳实践

1. **提交前运行**：`./test.sh` 确保所有测试通过
2. **修复 lint 警告**：保持代码整洁
3. **添加测试**：新功能应该有对应测试
4. **更新文档**：测试变更要更新本文档

## 🔗 相关资源

- [Deno 测试文档](https://deno.land/manual/testing)
- [主项目 README](./README.md)
- [贡献指南](../CONTRIBUTING.md)

---

最后更新：2025-10-21
