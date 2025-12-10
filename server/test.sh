#!/bin/bash
# 测试脚本 - 运行所有验证

set -e

echo "🧪 Kiro2API Deno 版本 - 测试套件"
echo "================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 错误标志
HAS_ERRORS=0

# 1. 类型检查
echo "📝 步骤 1/5: 运行类型检查..."
if deno task check; then
    echo -e "${GREEN}✓ 类型检查通过${NC}"
else
    echo -e "${RED}✗ 类型检查失败${NC}"
    HAS_ERRORS=1
fi
echo ""

# 2. Lint 检查
echo "🔍 步骤 2/5: 运行 Lint 检查..."
if deno task lint; then
    echo -e "${GREEN}✓ Lint 检查通过${NC}"
else
    echo -e "${RED}✗ Lint 检查失败${NC}"
    HAS_ERRORS=1
fi
echo ""

# 3. 格式化检查
echo "📐 步骤 3/5: 检查代码格式..."
if deno fmt --check; then
    echo -e "${GREEN}✓ 代码格式正确${NC}"
else
    echo -e "${YELLOW}⚠ 代码格式不一致（运行 'deno fmt' 修复）${NC}"
    # 不设置错误，只是警告
fi
echo ""

# 4. 冒烟测试
echo "🧪 步骤 4/5: 运行冒烟测试..."
if deno run --allow-net --allow-env --allow-read smoke_test.ts; then
    echo -e "${GREEN}✓ 冒烟测试通过${NC}"
else
    echo -e "${RED}✗ 冒烟测试失败${NC}"
    HAS_ERRORS=1
fi
echo ""

# 5. E2E 测试 (可选 - 需要环境变量)
echo "🌐 步骤 5/5: 运行端到端测试..."
echo -e "${YELLOW}注意: E2E 测试会启动临时服务器${NC}"
if deno run --allow-net --allow-env --allow-read --allow-write --allow-run --unstable-kv e2e_test.ts; then
    echo -e "${GREEN}✓ E2E 测试通过${NC}"
else
    echo -e "${YELLOW}⚠ E2E 测试失败（可能需要配置环境变量）${NC}"
    # E2E 失败不算错误，因为可能缺少 token
fi
echo ""

# 总结
echo "================================"
if [ $HAS_ERRORS -eq 0 ]; then
    echo -e "${GREEN}🎉 所有关键测试通过！${NC}"
    exit 0
else
    echo -e "${RED}❌ 部分测试失败，请检查上面的输出${NC}"
    exit 1
fi
