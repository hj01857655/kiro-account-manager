#!/bin/bash

# 日志系统增强测试脚本
# 测试新增的日志输出功能

set -e

echo "=== 日志系统增强测试 ==="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 测试计数
TOTAL=0
PASSED=0
FAILED=0

# 测试函数
test_log() {
    TOTAL=$((TOTAL + 1))
    local test_name="$1"
    local log_pattern="$2"
    
    echo -n "测试 $TOTAL: $test_name ... "
    
    if echo "$LOG_OUTPUT" | grep -q "$log_pattern"; then
        echo -e "${GREEN}✓ 通过${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ 失败${NC}"
        echo "  期望包含: $log_pattern"
        FAILED=$((FAILED + 1))
    fi
}

# 设置测试环境
export LOG_LEVEL=debug
export LOG_FORMAT=json
export LOG_CONSOLE=true

echo "环境配置:"
echo "  LOG_LEVEL=$LOG_LEVEL"
echo "  LOG_FORMAT=$LOG_FORMAT"
echo ""

# 运行日志测试
echo "运行日志系统测试..."
LOG_OUTPUT=$(deno run --allow-env logger/test_logging.ts 2>&1 || true)

echo ""
echo "=== 测试结果 ==="
echo ""

# 测试1: 基础字段
test_log "基础字段输出" "test.*value"

# 测试2: 新增字段
test_log "HTTP状态码字段" "http_status.*200"
test_log "错误类型字段" "error_type.*test_error"
test_log "延迟字段" "latency_ms.*150"
test_log "字节大小字段" "bytes.*1024"
test_log "阶段字段" "phase.*test_phase"
test_log "重试次数字段" "retry_count.*3"

# 测试3: 完整错误堆栈
test_log "错误消息" "message.*测试错误"
test_log "错误名称" "name.*Error"
test_log "错误堆栈" "stack"

# 测试4: 懒加载字段
test_log "懒加载JSON" "large_object"

# 测试5: 性能指标
test_log "阶段1完成" "阶段完成.*phase1"
test_log "阶段2完成" "阶段完成.*phase2"
test_log "请求完成" "请求完成"
test_log "阶段耗时" "phase_durations"

# 测试6: 错误追踪
test_log "上游错误追踪" "upstream_error"
test_log "请求超时追踪" "request_timeout"
test_log "错误统计" "upstream_error"

# 测试7: 并发请求
test_log "并发请求1" "并发请求 1"
test_log "并发请求2" "并发请求 2"
test_log "并发请求3" "并发请求 3"

echo ""
echo "=== 汇总 ==="
echo "总计: $TOTAL"
echo -e "通过: ${GREEN}$PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "失败: ${RED}$FAILED${NC}"
else
    echo -e "失败: $FAILED"
fi
echo ""

# 检查是否所有测试通过
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    echo ""
    echo "提示: 可以使用以下命令查看详细日志:"
    echo "  deno run --allow-env logger/test_logging.ts"
    echo "  LOG_FORMAT=text deno run --allow-env logger/test_logging.ts"
    exit 0
else
    echo -e "${RED}✗ 有测试失败${NC}"
    echo ""
    echo "请检查日志输出:"
    echo "  deno run --allow-env logger/test_logging.ts"
    exit 1
fi
