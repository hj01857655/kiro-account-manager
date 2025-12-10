/**
 * 日志系统使用示例
 * 展示如何在实际代码中使用优化后的日志系统
 */

import * as logger from "./logger.ts";
import { metricsCollector } from "./metrics.ts";
import { errorTracker, ErrorCategory, trackUpstreamError } from "./error_tracker.ts";

/**
 * 示例1: 简单的API请求处理
 */
async function simpleRequestHandler(req: Request): Promise<Response> {
  const requestId = crypto.randomUUID();
  
  logger.info(
    "收到请求",
    logger.String("request_id", requestId),
    logger.String("method", req.method),
    logger.String("path", new URL(req.url).pathname),
  );

  try {
    const body = await req.json();
    
    logger.debug(
      "请求体解析成功",
      logger.String("request_id", requestId),
      logger.LazyJson("body", body), // 懒加载，只在debug级别时序列化
    );

    return new Response("OK");
  } catch (error) {
    logger.error(
      "请求处理失败",
      logger.String("request_id", requestId),
      logger.Err(error), // 包含完整的错误堆栈
    );
    return new Response("Error", { status: 500 });
  }
}

/**
 * 示例2: 带性能追踪的请求处理
 */
async function requestWithMetrics(req: Request): Promise<Response> {
  const requestId = crypto.randomUUID();
  
  // 开始性能追踪
  metricsCollector.startRequest(requestId);
  
  try {
    // 阶段1: 解析请求
    metricsCollector.startPhase(requestId, "parse_request");
    const body = await req.json();
    metricsCollector.endPhase(requestId, "parse_request", {
      bodySize: JSON.stringify(body).length,
    });

    // 阶段2: 验证参数
    metricsCollector.startPhase(requestId, "validate");
    if (!body.model) {
      errorTracker.track(
        ErrorCategory.REQUEST_INVALID_PARAMS,
        "缺少model参数",
        new Error("Missing model parameter"),
        requestId,
      );
      metricsCollector.endRequest(requestId, false);
      return new Response("Bad Request", { status: 400 });
    }
    metricsCollector.endPhase(requestId, "validate");

    // 阶段3: 调用上游API
    metricsCollector.startPhase(requestId, "upstream_request");
    const startTime = Date.now();
    
    const response = await fetch("https://api.example.com/v1/chat", {
      method: "POST",
      body: JSON.stringify(body),
    });
    
    const latency = Date.now() - startTime;
    metricsCollector.endPhase(requestId, "upstream_request", {
      status: response.status,
      latency,
    });

    logger.info(
      "上游请求完成",
      logger.String("request_id", requestId),
      logger.HttpStatus(response.status),
      logger.Latency(latency),
    );

    // 成功完成
    metricsCollector.endRequest(requestId, true);
    return response;

  } catch (error) {
    // 记录错误并完成追踪
    logger.error(
      "请求处理失败",
      logger.String("request_id", requestId),
      logger.Err(error),
    );
    
    metricsCollector.endRequest(requestId, false, error as Error);
    return new Response("Internal Server Error", { status: 500 });
  }
}

/**
 * 示例3: 带重试逻辑的上游请求
 */
async function requestWithRetry(
  url: string,
  body: unknown,
  requestId: string,
  maxRetries = 3,
): Promise<Response> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      logger.debug(
        "发送上游请求",
        logger.String("request_id", requestId),
        logger.Int("attempt", attempt + 1),
        logger.Int("max_retries", maxRetries),
      );

      const startTime = Date.now();
      const response = await fetch(url, {
        method: "POST",
        body: JSON.stringify(body),
      });

      const latency = Date.now() - startTime;

      if (response.ok) {
        logger.info(
          "上游请求成功",
          logger.String("request_id", requestId),
          logger.HttpStatus(response.status),
          logger.Latency(latency),
          logger.RetryCount(attempt),
        );
        return response;
      }

      // 非2xx响应
      const errorText = await response.text();
      lastError = new Error(`HTTP ${response.status}: ${errorText}`);

      logger.warn(
        "上游请求失败，准备重试",
        logger.String("request_id", requestId),
        logger.HttpStatus(response.status),
        logger.Latency(latency),
        logger.RetryCount(attempt),
      );

      // 指数退避
      if (attempt < maxRetries - 1) {
        const delay = Math.pow(2, attempt) * 1000;
        await new Promise((resolve) => setTimeout(resolve, delay));
      }

    } catch (error) {
      lastError = error as Error;
      
      logger.warn(
        "上游请求异常",
        logger.String("request_id", requestId),
        logger.RetryCount(attempt),
        logger.Err(error),
      );

      if (attempt < maxRetries - 1) {
        const delay = Math.pow(2, attempt) * 1000;
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }

  // 所有重试都失败
  trackUpstreamError(
    `上游请求失败，已重试${maxRetries}次`,
    lastError!,
    requestId,
  );

  throw lastError;
}

/**
 * 示例4: 流式响应处理
 */
async function streamResponseHandler(
  upstreamResponse: Response,
  requestId: string,
): Promise<Response> {
  const stream = new ReadableStream({
    async start(controller) {
      metricsCollector.startPhase(requestId, "stream_processing");
      
      const reader = upstreamResponse.body?.getReader();
      if (!reader) {
        errorTracker.track(
          ErrorCategory.STREAM_INTERRUPTED,
          "上游响应没有body",
          new Error("No response body"),
          requestId,
        );
        controller.close();
        return;
      }

      let totalBytes = 0;
      let chunkCount = 0;

      try {
        while (true) {
          const { done, value } = await reader.read();
          
          if (done) {
            logger.debug(
              "流处理完成",
              logger.String("request_id", requestId),
              logger.Bytes(totalBytes),
              logger.Int("chunks", chunkCount),
            );
            break;
          }

          totalBytes += value.length;
          chunkCount++;
          
          controller.enqueue(value);

          // 每100个chunk记录一次进度
          if (chunkCount % 100 === 0) {
            logger.debug(
              "流处理进度",
              logger.String("request_id", requestId),
              logger.Bytes(totalBytes),
              logger.Int("chunks", chunkCount),
            );
          }
        }

        metricsCollector.endPhase(requestId, "stream_processing", {
          totalBytes,
          chunkCount,
        });

        controller.close();

      } catch (error) {
        errorTracker.track(
          ErrorCategory.STREAM_INTERRUPTED,
          "流处理中断",
          error,
          requestId,
          { totalBytes, chunkCount },
        );

        metricsCollector.endPhase(requestId, "stream_processing", {
          error: error instanceof Error ? error.message : String(error),
          totalBytes,
          chunkCount,
        });

        controller.error(error);
      }
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "X-Request-ID": requestId,
    },
  });
}

/**
 * 示例5: Token刷新逻辑
 */
async function refreshToken(
  refreshToken: string,
  requestId?: string,
): Promise<string> {
  const startTime = Date.now();

  try {
    logger.info(
      "开始刷新Token",
      ...(requestId ? [logger.String("request_id", requestId)] : []),
    );

    const response = await fetch("https://auth.example.com/refresh", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });

    const latency = Date.now() - startTime;

    if (!response.ok) {
      const error = new Error(`Token刷新失败: ${response.status}`);
      
      errorTracker.track(
        ErrorCategory.AUTH_REFRESH_FAILED,
        "Token刷新失败",
        error,
        requestId,
        { statusCode: response.status, latency },
      );

      throw error;
    }

    const data = await response.json();
    
    logger.info(
      "Token刷新成功",
      logger.Latency(latency),
      ...(requestId ? [logger.String("request_id", requestId)] : []),
    );

    return data.access_token;

  } catch (error) {
    const latency = Date.now() - startTime;
    
    errorTracker.track(
      ErrorCategory.AUTH_REFRESH_FAILED,
      "Token刷新异常",
      error,
      requestId,
      { latency },
    );

    throw error;
  }
}

/**
 * 示例6: 批量操作
 */
async function batchProcess(
  items: Array<{ id: string; data: unknown }>,
  requestId: string,
): Promise<void> {
  logger.info(
    "开始批量处理",
    logger.String("request_id", requestId),
    logger.Int("total_items", items.length),
  );

  metricsCollector.startPhase(requestId, "batch_process");

  let successCount = 0;
  let failureCount = 0;

  for (const [index, item] of items.entries()) {
    try {
      // 处理单个项目
      await processItem(item, requestId);
      successCount++;

      // 每10个记录一次进度
      if ((index + 1) % 10 === 0) {
        logger.debug(
          "批量处理进度",
          logger.String("request_id", requestId),
          logger.Int("processed", index + 1),
          logger.Int("total", items.length),
          logger.Int("success", successCount),
          logger.Int("failure", failureCount),
        );
      }

    } catch (error) {
      failureCount++;
      
      logger.warn(
        "处理项目失败",
        logger.String("request_id", requestId),
        logger.String("item_id", item.id),
        logger.Err(error),
      );
    }
  }

  metricsCollector.endPhase(requestId, "batch_process", {
    totalItems: items.length,
    successCount,
    failureCount,
  });

  logger.info(
    "批量处理完成",
    logger.String("request_id", requestId),
    logger.Int("total", items.length),
    logger.Int("success", successCount),
    logger.Int("failure", failureCount),
  );
}

async function processItem(_item: { id: string; data: unknown }, _requestId: string): Promise<void> {
  // 模拟处理
  await new Promise((resolve) => setTimeout(resolve, 10));
}

// 导出示例函数
export {
  simpleRequestHandler,
  requestWithMetrics,
  requestWithRetry,
  streamResponseHandler,
  refreshToken,
  batchProcess,
};
