# 日志系统快速参考

## 基础日志

```typescript
import * as logger from "./logger/logger.ts";

// 日志级别
logger.debug("调试信息", ...fields);
logger.info("重要事件", ...fields);
logger.warn("警告", ...fields);
logger.error("错误", ...fields);
logger.fatal("致命错误", ...fields);  // 会退出程序
```

## 常用字段

```typescript
// 基础类型
logger.String("key", "value")
logger.Int("count", 42)
logger.Float("ratio", 0.95)
logger.Bool("success", true)
logger.Duration("elapsed", 150)  // 毫秒

// 新增类型
logger.HttpStatus(200)           // HTTP状态码
logger.ErrorType("auth_failed")  // 错误类型
logger.Latency(150)              // 延迟(ms)
logger.Bytes(1024)               // 字节大小
logger.Phase("parse")            // 处理阶段
logger.RetryCount(3)             // 重试次数

// 错误（包含完整堆栈）
logger.Err(error)

// 懒加载（性能优化）
logger.LazyJson("data", obj)     // 只在需要时序列化
logger.LazyString("key", () => expensiveComputation())
```

## 性能追踪

```typescript
import { metricsCollector } from "./logger/metrics.ts";

// 1. 开始请求
metricsCollector.startRequest(requestId);

// 2. 追踪阶段
metricsCollector.startPhase(requestId, "phase_name");
// ... 处理逻辑
metricsCollector.endPhase(requestId, "phase_name", { metadata });

// 3. 结束请求（自动记录汇总）
metricsCollector.endRequest(requestId, success, error?);
```

## 错误追踪

```typescript
import { errorTracker, ErrorCategory } from "./logger/error_tracker.ts";

// 追踪错误
errorTracker.track(
  ErrorCategory.UPSTREAM_ERROR,
  "错误描述",
  error,
  requestId,
  { metadata }
);

// 便捷函数
trackAuthError(category, message, error, requestId);
trackRequestError(category, message, error, requestId, metadata);
trackUpstreamError(message, error, requestId, statusCode);
trackStreamError(category, message, error, requestId);

// 获取统计
const summary = errorTracker.getSummary();
```

## 错误类型

```typescript
ErrorCategory.AUTH_TOKEN_EXPIRED        // Token过期
ErrorCategory.AUTH_REFRESH_FAILED       // 刷新失败
ErrorCategory.REQUEST_INVALID_PARAMS    // 参数错误
ErrorCategory.REQUEST_TIMEOUT           // 请求超时
ErrorCategory.UPSTREAM_ERROR            // 上游错误
ErrorCategory.STREAM_TIMEOUT            // 流超时
ErrorCategory.STREAM_INTERRUPTED        // 流中断
```

## 常用模式

### 请求处理
```typescript
async function handleRequest(req: Request): Promise<Response> {
  const requestId = crypto.randomUUID();
  metricsCollector.startRequest(requestId);
  
  try {
    logger.info("收到请求", logger.String("request_id", requestId));
    
    // 处理逻辑...
    
    metricsCollector.endRequest(requestId, true);
    return response;
  } catch (error) {
    errorTracker.track(ErrorCategory.UNKNOWN, "处理失败", error, requestId);
    metricsCollector.endRequest(requestId, false, error as Error);
    throw error;
  }
}
```

### 上游请求
```typescript
metricsCollector.startPhase(requestId, "upstream_request");
const startTime = Date.now();

try {
  const response = await fetch(url, options);
  const latency = Date.now() - startTime;
  
  metricsCollector.endPhase(requestId, "upstream_request", {
    status: response.status,
    latency,
  });
  
  logger.info(
    "上游请求成功",
    logger.String("request_id", requestId),
    logger.HttpStatus(response.status),
    logger.Latency(latency),
  );
  
  return response;
} catch (error) {
  trackUpstreamError("上游请求失败", error, requestId);
  throw error;
}
```

### 带重试
```typescript
for (let attempt = 0; attempt < maxRetries; attempt++) {
  try {
    logger.debug(
      "尝试请求",
      logger.String("request_id", requestId),
      logger.RetryCount(attempt),
    );
    
    const response = await fetch(url);
    if (response.ok) return response;
    
  } catch (error) {
    if (attempt === maxRetries - 1) {
      errorTracker.track(
        ErrorCategory.REQUEST_TIMEOUT,
        "重试失败",
        error,
        requestId,
        { attempts: maxRetries }
      );
      throw error;
    }
  }
}
```

## 故障排查命令

```bash
# 查找特定请求
cat app.log | jq 'select(.request_id == "abc-123")'

# 统计错误类型
cat app.log | jq 'select(.error_type) | .error_type' | sort | uniq -c

# 查找慢请求（>1秒）
cat app.log | jq 'select(.total_duration and ((.total_duration | tonumber) > 1000))'

# 实时查看错误
tail -f app.log | jq 'select(.level == "ERROR")'

# 计算成功率
tail -100 app.log | jq 'select(.success != null) | .success' | \
  awk '{sum+=$1; count++} END {print sum/count*100"%"}'
```

## 环境变量

```bash
LOG_LEVEL=debug          # debug, info, warn, error
LOG_FORMAT=json          # json, text
LOG_CONSOLE=true         # 控制台输出
LOG_FILE=./app.log       # 文件输出（可选）
```

## 最佳实践

✅ **DO**
- 始终使用 requestId
- 记录关键阶段耗时
- 错误分类追踪
- 使用懒加载（大对象）
- 适当的日志级别

❌ **DON'T**
- 记录敏感信息（密码、Token）
- 在循环中记录 info 日志
- 序列化大对象（用 LazyJson）
- 忽略错误堆栈
- 滥用 debug 级别

## 更多信息

- 完整指南: [logger/README.md](./README.md)
- 示例代码: [logger/example.ts](./example.ts)
- 优化总结: [LOGGING_OPTIMIZATION.md](../LOGGING_OPTIMIZATION.md)
