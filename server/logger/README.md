# 日志系统使用指南

## 概述

优化后的日志系统提供了完整的可观测性支持，包括：

- **结构化日志**: JSON/文本格式，支持多级别
- **请求追踪**: 统一的请求ID追踪
- **性能指标**: 自动收集各阶段耗时
- **错误分类**: 结构化的错误追踪和统计
- **完整堆栈**: 保留完整的错误堆栈信息

## 核心模块

### 1. logger.ts - 基础日志

```typescript
import * as logger from "./logger/logger.ts";

// 基础日志
logger.info("服务启动", logger.Int("port", 8080));
logger.error("请求失败", logger.Err(error), logger.String("request_id", id));

// 新增字段类型
logger.HttpStatus(200)           // HTTP状态码
logger.ErrorType("auth_failed")  // 错误类型
logger.Latency(150)              // 延迟(ms)
logger.Bytes(1024)               // 字节大小
logger.Phase("upstream_request") // 处理阶段
logger.RetryCount(3)             // 重试次数
```

### 2. context.ts - 请求上下文

```typescript
import { contextManager } from "./logger/context.ts";

// 创建请求上下文
const ctx = contextManager.create(requestId, {
  method: "POST",
  path: "/v1/messages",
  model: "claude-sonnet-4",
});

// 获取上下文字段
const fields = contextManager.toFields(requestId);
logger.info("处理请求", ...fields);

// 获取耗时
const elapsed = contextManager.getElapsed(requestId);

// 清理上下文
contextManager.delete(requestId);
```

### 3. metrics.ts - 性能指标

```typescript
import { metricsCollector } from "./logger/metrics.ts";

// 开始请求追踪
metricsCollector.startRequest(requestId);

// 追踪各个阶段
metricsCollector.startPhase(requestId, "parse_request");
// ... 处理逻辑
metricsCollector.endPhase(requestId, "parse_request", { size: 1024 });

metricsCollector.startPhase(requestId, "upstream_request");
// ... 上游请求
metricsCollector.endPhase(requestId, "upstream_request", { 
  status: 200,
  latency: 150 
});

// 结束请求（自动记录汇总日志）
metricsCollector.endRequest(requestId, true);
// 或失败时
metricsCollector.endRequest(requestId, false, error);
```

输出示例：
```json
{
  "timestamp": "2025-01-15T10:30:45.123Z",
  "level": "INFO",
  "message": "请求完成",
  "request_id": "abc-123",
  "success": true,
  "total_duration": "250ms",
  "phase_durations": {
    "parse_request": 5,
    "get_token": 10,
    "upstream_request": 200,
    "parse_response": 35
  }
}
```

### 4. error_tracker.ts - 错误追踪

```typescript
import { errorTracker, ErrorCategory } from "./logger/error_tracker.ts";

// 追踪错误
errorTracker.track(
  ErrorCategory.AUTH_TOKEN_EXPIRED,
  "Token已过期",
  error,
  requestId,
  { userId: "user123" }
);

// 便捷函数
trackAuthError(ErrorCategory.AUTH_REFRESH_FAILED, "刷新失败", error, requestId);
trackRequestError(ErrorCategory.REQUEST_TIMEOUT, "请求超时", error, requestId);
trackUpstreamError("上游服务错误", error, requestId, 503);
trackStreamError(ErrorCategory.STREAM_PARSE_ERROR, "解析失败", error, requestId);

// 获取错误统计
const stats = errorTracker.getStats();
const summary = errorTracker.getSummary();
// { "auth_token_expired": 5, "request_timeout": 2 }
```

## 错误分类

```typescript
enum ErrorCategory {
  // 认证相关
  AUTH_TOKEN_EXPIRED = "auth_token_expired",
  AUTH_TOKEN_INVALID = "auth_token_invalid",
  AUTH_REFRESH_FAILED = "auth_refresh_failed",
  AUTH_NO_AVAILABLE_TOKEN = "auth_no_available_token",

  // 请求相关
  REQUEST_INVALID_PARAMS = "request_invalid_params",
  REQUEST_TIMEOUT = "request_timeout",
  REQUEST_RATE_LIMIT = "request_rate_limit",
  REQUEST_TOO_LARGE = "request_too_large",

  // 上游服务相关
  UPSTREAM_ERROR = "upstream_error",
  UPSTREAM_TIMEOUT = "upstream_timeout",
  UPSTREAM_UNAVAILABLE = "upstream_unavailable",

  // 流处理相关
  STREAM_PARSE_ERROR = "stream_parse_error",
  STREAM_TIMEOUT = "stream_timeout",
  STREAM_INTERRUPTED = "stream_interrupted",

  // 系统相关
  SYSTEM_OUT_OF_MEMORY = "system_out_of_memory",
  SYSTEM_FILE_ERROR = "system_file_error",
  SYSTEM_NETWORK_ERROR = "system_network_error",

  // 未知错误
  UNKNOWN = "unknown",
}
```

## 完整使用示例

```typescript
import * as logger from "./logger/logger.ts";
import { metricsCollector } from "./logger/metrics.ts";
import { errorTracker, ErrorCategory } from "./logger/error_tracker.ts";

async function handleRequest(req: Request): Promise<Response> {
  const requestId = crypto.randomUUID();
  
  // 开始性能追踪
  metricsCollector.startRequest(requestId);
  
  try {
    // 解析请求
    metricsCollector.startPhase(requestId, "parse_request");
    const body = await req.json();
    metricsCollector.endPhase(requestId, "parse_request");
    
    // 验证参数
    if (!body.messages) {
      errorTracker.track(
        ErrorCategory.REQUEST_INVALID_PARAMS,
        "缺少messages参数",
        new Error("Missing messages"),
        requestId
      );
      metricsCollector.endRequest(requestId, false);
      return new Response("Bad Request", { status: 400 });
    }
    
    // 调用上游
    metricsCollector.startPhase(requestId, "upstream_request");
    const startTime = Date.now();
    
    try {
      const response = await fetch("https://api.example.com", {
        method: "POST",
        body: JSON.stringify(body),
      });
      
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
      
      metricsCollector.endRequest(requestId, true);
      return response;
      
    } catch (error) {
      const latency = Date.now() - startTime;
      
      errorTracker.track(
        ErrorCategory.UPSTREAM_ERROR,
        "上游请求失败",
        error,
        requestId,
        { latency }
      );
      
      metricsCollector.endPhase(requestId, "upstream_request", {
        error: error instanceof Error ? error.message : String(error),
        latency,
      });
      
      throw error;
    }
    
  } catch (error) {
    logger.error(
      "请求处理失败",
      logger.String("request_id", requestId),
      logger.Err(error), // 包含完整堆栈
    );
    
    metricsCollector.endRequest(requestId, false, error as Error);
    return new Response("Internal Server Error", { status: 500 });
  }
}
```

## 日志输出示例

### 成功请求
```json
{
  "timestamp": "2025-01-15T10:30:45.123Z",
  "level": "INFO",
  "message": "上游请求成功",
  "request_id": "abc-123",
  "http_status": 200,
  "latency_ms": 150
}

{
  "timestamp": "2025-01-15T10:30:45.280Z",
  "level": "INFO",
  "message": "请求完成",
  "request_id": "abc-123",
  "success": true,
  "total_duration": "250ms",
  "phase_durations": {
    "parse_request": 5,
    "upstream_request": 150,
    "parse_response": 95
  }
}
```

### 错误请求
```json
{
  "timestamp": "2025-01-15T10:31:00.456Z",
  "level": "ERROR",
  "message": "[upstream_error] 上游请求失败",
  "error_type": "upstream_error",
  "request_id": "def-456",
  "error": {
    "message": "Connection timeout",
    "name": "TimeoutError",
    "stack": "TimeoutError: Connection timeout\n    at fetch (...)"
  },
  "metadata": {
    "latency": 5000
  }
}

{
  "timestamp": "2025-01-15T10:31:00.460Z",
  "level": "ERROR",
  "message": "请求失败",
  "request_id": "def-456",
  "success": false,
  "total_duration": "5100ms",
  "phase_durations": {
    "parse_request": 3,
    "upstream_request": 5000
  },
  "error": {
    "message": "Connection timeout",
    "name": "TimeoutError",
    "stack": "..."
  }
}
```

## 环境变量配置

```bash
# 日志级别
LOG_LEVEL=debug          # debug, info, warn, error

# 日志格式
LOG_FORMAT=json          # json, text

# 日志输出
LOG_CONSOLE=true         # 控制台输出
LOG_FILE=/var/log/app.log  # 文件输出（可选）
```

## 最佳实践

1. **统一使用 requestId**: 所有日志都应包含 requestId
2. **记录关键阶段**: 使用 metricsCollector 追踪性能瓶颈
3. **错误分类**: 使用 ErrorCategory 分类错误，便于统计分析
4. **完整堆栈**: 使用 logger.Err() 记录完整错误信息
5. **敏感信息脱敏**: Token、密码等敏感信息不要记录
6. **适当的日志级别**:
   - DEBUG: 详细的调试信息
   - INFO: 重要的业务事件（请求开始/完成）
   - WARN: 可恢复的异常情况
   - ERROR: 需要关注的错误

## 故障排查

### 查找特定请求的所有日志
```bash
# JSON格式
cat app.log | jq 'select(.request_id == "abc-123")'

# 文本格式
grep "abc-123" app.log
```

### 统计错误类型
```bash
cat app.log | jq 'select(.error_type) | .error_type' | sort | uniq -c
```

### 分析性能瓶颈
```bash
# 查找耗时超过1秒的请求
cat app.log | jq 'select(.total_duration and (.total_duration | tonumber > 1000))'

# 查看各阶段耗时分布
cat app.log | jq 'select(.phase_durations) | .phase_durations'
```

### 监控错误率
```bash
# 最近100条请求的成功率
tail -100 app.log | jq 'select(.success != null) | .success' | \
  awk '{sum+=$1; count++} END {print "Success Rate:", sum/count*100"%"}'
```
