import * as logger from "../logger/logger.ts";
import { createTokenPreview, maskEmail } from "../utils/privacy.ts";

/**
 * 标准化错误响应
 * @param message 错误消息
 * @param status HTTP状态码
 * @param code 错误码（可选）
 * @returns Response对象
 */
export function respondError(
  message: string,
  status = 500,
  code?: string,
): Response {
  const errorCode = code || getErrorType(status);
  return Response.json({
    error: {
      message,
      type: errorCode,
      code: errorCode,
    },
  }, { status });
}

/**
 * 根据状态码获取错误类型
 */
function getErrorType(status: number): string {
  switch (status) {
    case 400:
      return "bad_request";
    case 401:
      return "unauthorized";
    case 403:
      return "forbidden";
    case 404:
      return "not_found";
    case 429:
      return "rate_limited";
    default:
      return "internal_error";
  }
}

/**
 * 格式化错误响应（支持格式化字符串）
 */
export function respondErrorf(
  status: number,
  format: string,
  ...args: unknown[]
): Response {
  const message = formatString(format, ...args);
  return respondError(message, status);
}

/**
 * 简单的字符串格式化
 */
function formatString(format: string, ...args: unknown[]): string {
  return format.replace(/%s/g, () => String(args.shift() || ""));
}

// 日志字段辅助函数
export interface LogContext {
  requestId?: string;
  messageId?: string;
}

export function addLogFields(ctx: LogContext, ...fields: unknown[]): unknown[] {
  const result = [];
  if (ctx.requestId) {
    result.push(logger.String("request_id", ctx.requestId));
  }
  if (ctx.messageId) {
    result.push(logger.String("message_id", ctx.messageId));
  }
  result.push(...fields);
  return result;
}

// 提取相关请求头
export function extractRelevantHeaders(headers: Headers): Record<string, string> {
  const relevantHeaders: Record<string, string> = {};
  
  const headerKeys = [
    "Content-Type",
    "Authorization",
    "X-API-Key",
    "X-Request-ID",
    "X-Forwarded-For",
    "Accept",
    "Accept-Encoding",
  ];

  for (const key of headerKeys) {
    const value = headers.get(key);
    if (value) {
      // 对敏感信息进行脱敏处理
      if (key === "Authorization" && value.length > 20) {
        relevantHeaders[key] = value.substring(0, 10) + "***" + value.substring(value.length - 7);
      } else if (key === "X-API-Key" && value.length > 10) {
        relevantHeaders[key] = value.substring(0, 5) + "***" + value.substring(value.length - 3);
      } else {
        relevantHeaders[key] = value;
      }
    }
  }

  return relevantHeaders;
}

// Re-export privacy utilities for convenience
export { createTokenPreview, maskEmail };
