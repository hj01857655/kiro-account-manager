/**
 * 错误追踪和分类
 * 提供结构化的错误记录和统计
 */

import * as logger from "./logger.ts";

/**
 * 错误类型分类
 */
export enum ErrorCategory {
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

/**
 * 错误记录
 */
export interface ErrorRecord {
  category: ErrorCategory;
  message: string;
  error: Error | unknown;
  requestId?: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

/**
 * 错误统计
 */
interface ErrorStats {
  category: ErrorCategory;
  count: number;
  lastOccurrence: number;
  samples: ErrorRecord[];
}

/**
 * 错误追踪器
 */
class ErrorTracker {
  private stats = new Map<ErrorCategory, ErrorStats>();
  private readonly maxSamples = 10; // 每种错误保留最多10个样本

  /**
   * 记录错误
   */
  track(
    category: ErrorCategory,
    message: string,
    error: Error | unknown,
    requestId?: string,
    metadata?: Record<string, unknown>,
  ): void {
    const record: ErrorRecord = {
      category,
      message,
      error,
      requestId,
      timestamp: Date.now(),
      metadata,
    };

    // 更新统计
    let stat = this.stats.get(category);
    if (!stat) {
      stat = {
        category,
        count: 0,
        lastOccurrence: 0,
        samples: [],
      };
      this.stats.set(category, stat);
    }

    stat.count++;
    stat.lastOccurrence = record.timestamp;
    stat.samples.push(record);

    // 保持样本数量限制
    if (stat.samples.length > this.maxSamples) {
      stat.samples.shift();
    }

    // 记录日志
    const logFields = [
      logger.ErrorType(category),
      logger.String("message", message),
      logger.Err(error),
    ];

    if (requestId) {
      logFields.push(logger.String("request_id", requestId));
    }

    if (metadata) {
      logFields.push(logger.Any("metadata", metadata));
    }

    // 根据错误类型选择日志级别
    if (this.isCriticalError(category)) {
      logger.error(`[${category}] ${message}`, ...logFields);
    } else {
      logger.warn(`[${category}] ${message}`, ...logFields);
    }
  }

  /**
   * 判断是否为严重错误
   */
  private isCriticalError(category: ErrorCategory): boolean {
    return [
      ErrorCategory.AUTH_NO_AVAILABLE_TOKEN,
      ErrorCategory.SYSTEM_OUT_OF_MEMORY,
      ErrorCategory.UPSTREAM_UNAVAILABLE,
    ].includes(category);
  }

  /**
   * 获取错误统计
   */
  getStats(): Map<ErrorCategory, ErrorStats> {
    return new Map(this.stats);
  }

  /**
   * 获取特定类型的错误统计
   */
  getCategoryStats(category: ErrorCategory): ErrorStats | undefined {
    return this.stats.get(category);
  }

  /**
   * 重置统计
   */
  reset(): void {
    this.stats.clear();
  }

  /**
   * 获取错误摘要
   */
  getSummary(): Record<string, number> {
    const summary: Record<string, number> = {};
    for (const [category, stat] of this.stats) {
      summary[category] = stat.count;
    }
    return summary;
  }

  /**
   * 清理过期样本
   */
  cleanup(): void {
    const now = Date.now();
    const maxAge = 60 * 60 * 1000; // 1小时

    for (const stat of this.stats.values()) {
      stat.samples = stat.samples.filter(
        (sample) => now - sample.timestamp < maxAge,
      );
    }
  }
}

// 全局错误追踪器
export const errorTracker = new ErrorTracker();

// 定期清理过期样本
setInterval(() => errorTracker.cleanup(), 10 * 60 * 1000);

/**
 * 便捷函数：追踪认证错误
 */
export function trackAuthError(
  category: ErrorCategory,
  message: string,
  error: Error | unknown,
  requestId?: string,
): void {
  errorTracker.track(category, message, error, requestId);
}

/**
 * 便捷函数：追踪请求错误
 */
export function trackRequestError(
  category: ErrorCategory,
  message: string,
  error: Error | unknown,
  requestId?: string,
  metadata?: Record<string, unknown>,
): void {
  errorTracker.track(category, message, error, requestId, metadata);
}

/**
 * 便捷函数：追踪上游错误
 */
export function trackUpstreamError(
  message: string,
  error: Error | unknown,
  requestId?: string,
  statusCode?: number,
): void {
  errorTracker.track(
    ErrorCategory.UPSTREAM_ERROR,
    message,
    error,
    requestId,
    { statusCode },
  );
}

/**
 * 便捷函数：追踪流处理错误
 */
export function trackStreamError(
  category: ErrorCategory,
  message: string,
  error: Error | unknown,
  requestId?: string,
): void {
  errorTracker.track(category, message, error, requestId);
}
