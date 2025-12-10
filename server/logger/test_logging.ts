/**
 * 日志系统测试
 * 验证所有新功能是否正常工作
 */

import * as logger from "./logger.ts";
import { metricsCollector } from "./metrics.ts";
import { errorTracker, ErrorCategory } from "./error_tracker.ts";

console.log("=== 日志系统测试 ===\n");

// 测试1: 基础日志字段
console.log("测试1: 基础日志字段");
logger.info(
  "测试基础字段",
  logger.String("test", "value"),
  logger.Int("count", 42),
  logger.Bool("success", true),
);

// 测试2: 新增字段
console.log("\n测试2: 新增字段");
logger.info(
  "测试新增字段",
  logger.HttpStatus(200),
  logger.ErrorType("test_error"),
  logger.Latency(150),
  logger.Bytes(1024),
  logger.Phase("test_phase"),
  logger.RetryCount(3),
);

// 测试3: 完整错误堆栈
console.log("\n测试3: 完整错误堆栈");
try {
  throw new Error("测试错误");
} catch (error) {
  logger.error(
    "捕获到错误",
    logger.Err(error),
  );
}

// 测试4: 懒加载字段
console.log("\n测试4: 懒加载字段");
const largeObject = { data: "x".repeat(1000) };
logger.debug(
  "测试懒加载",
  logger.LazyJson("large_object", largeObject),
  logger.LazyString("computed", () => "expensive computation"),
);

// 测试5: 性能指标收集
console.log("\n测试5: 性能指标收集");
const requestId = crypto.randomUUID();

metricsCollector.startRequest(requestId);

metricsCollector.startPhase(requestId, "phase1");
await new Promise((resolve) => setTimeout(resolve, 10));
metricsCollector.endPhase(requestId, "phase1", { result: "success" });

metricsCollector.startPhase(requestId, "phase2");
await new Promise((resolve) => setTimeout(resolve, 20));
metricsCollector.endPhase(requestId, "phase2", { items: 5 });

metricsCollector.endRequest(requestId, true);

// 测试6: 错误追踪
console.log("\n测试6: 错误追踪");
errorTracker.track(
  ErrorCategory.UPSTREAM_ERROR,
  "测试上游错误",
  new Error("Upstream failed"),
  requestId,
  { statusCode: 503 },
);

errorTracker.track(
  ErrorCategory.REQUEST_TIMEOUT,
  "测试请求超时",
  new Error("Timeout"),
  requestId,
  { duration: 5000 },
);

// 测试7: 错误统计
console.log("\n测试7: 错误统计");
const summary = errorTracker.getSummary();
console.log("错误统计:", JSON.stringify(summary, null, 2));

// 测试8: 失败请求的性能追踪
console.log("\n测试8: 失败请求的性能追踪");
const failedRequestId = crypto.randomUUID();

metricsCollector.startRequest(failedRequestId);

metricsCollector.startPhase(failedRequestId, "attempt1");
await new Promise((resolve) => setTimeout(resolve, 5));
metricsCollector.endPhase(failedRequestId, "attempt1", { error: "failed" });

const testError = new Error("Request failed");
metricsCollector.endRequest(failedRequestId, false, testError);

// 测试9: 多个请求并发
console.log("\n测试9: 多个请求并发");
const requests = Array.from({ length: 3 }, (_, i) => {
  const id = crypto.randomUUID();
  return (async () => {
    metricsCollector.startRequest(id);
    
    logger.info(
      `并发请求 ${i + 1}`,
      logger.String("request_id", id),
    );
    
    metricsCollector.startPhase(id, "processing");
    await new Promise((resolve) => setTimeout(resolve, Math.random() * 20));
    metricsCollector.endPhase(id, "processing");
    
    metricsCollector.endRequest(id, true);
  })();
});

await Promise.all(requests);

// 测试10: 文本格式日志
console.log("\n测试10: 文本格式日志（需要设置 LOG_FORMAT=text）");
logger.info(
  "文本格式测试",
  logger.String("request_id", "test-123"),
  logger.HttpStatus(200),
  logger.Latency(100),
);

console.log("\n=== 测试完成 ===");
console.log("\n提示:");
console.log("- 查看上面的日志输出，验证格式是否正确");
console.log("- 错误日志应包含完整的堆栈信息");
console.log("- 性能指标应显示各阶段耗时");
console.log("- 错误统计应显示分类计数");
console.log("\n运行命令:");
console.log("  deno run --allow-env logger/test_logging.ts");
console.log("  LOG_FORMAT=text deno run --allow-env logger/test_logging.ts");
console.log("  LOG_LEVEL=debug deno run --allow-env logger/test_logging.ts");
