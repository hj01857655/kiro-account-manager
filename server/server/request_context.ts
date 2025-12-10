import type { AuthService } from "../auth/auth_service.ts";
import type { TokenInfo, TokenWithUsage } from "../types/common.ts";
import * as logger from "../logger/logger.ts";
import { respondError } from "./common.ts";

/**
 * 请求处理上下文
 * 统一封装token获取、body读取和日志追踪的上下文对象
 * 
 * 设计原则：
 * 1. 单一职责：专注于请求处理的通用流程
 * 2. DRY原则：避免在每个handler中重复token获取和body读取代码
 * 3. 一致性：确保所有请求都经过相同的处理流程
 */
export class RequestContext {
  public readonly request: Request;
  public readonly authService: AuthService;
  public readonly requestType: string;
  public readonly requestId: string;

  constructor(
    request: Request,
    authService: AuthService,
    requestType: string,
    requestId: string,
  ) {
    this.request = request;
    this.authService = authService;
    this.requestType = requestType;
    this.requestId = requestId;
  }

  /**
   * 获取token和请求体
   * @returns [tokenInfo, body, error]
   */
  async getTokenAndBody(): Promise<
    [TokenInfo, Uint8Array] | [null, null, Response]
  > {
    try {
      // 获取token
      const tokenInfo = await this.authService.getToken();

      // 读取请求体
      const body = new Uint8Array(await this.request.arrayBuffer());

      // 记录请求日志
      logger.debug(
        `收到${this.requestType}请求`,
        logger.String("request_id", this.requestId),
        logger.String("direction", "client_request"),
        logger.Int("body_size", body.length),
        logger.String("remote_addr", this.getClientIP()),
        logger.String("user_agent", this.request.headers.get("User-Agent") || ""),
      );

      return [tokenInfo, body];
    } catch (error) {
      logger.error(
        "获取token或读取请求体失败",
        logger.String("request_id", this.requestId),
        logger.Err(error),
      );
      return [null, null, respondError("Internal server error", 500)];
    }
  }

  /**
   * 获取token（包含使用信息）和请求体
   * @returns [tokenWithUsage, body, error]
   */
  async getTokenWithUsageAndBody(): Promise<
    [TokenWithUsage, Uint8Array] | [null, null, Response]
  > {
    try {
      // 获取token（包含使用信息）
      const tokenWithUsage = await this.authService.getTokenWithUsage();

      // 读取请求体
      const body = new Uint8Array(await this.request.arrayBuffer());

      // 记录请求日志
      logger.debug(
        `收到${this.requestType}请求`,
        logger.String("request_id", this.requestId),
        logger.String("direction", "client_request"),
        logger.Int("body_size", body.length),
        logger.String("remote_addr", this.getClientIP()),
        logger.String("user_agent", this.request.headers.get("User-Agent") || ""),
        logger.Float("available_count", tokenWithUsage.availableCount || 0),
      );

      return [tokenWithUsage, body];
    } catch (error) {
      logger.error(
        "获取token或读取请求体失败",
        logger.String("request_id", this.requestId),
        logger.Err(error),
      );
      return [null, null, respondError("Internal server error", 500)];
    }
  }

  /**
   * 获取客户端IP地址
   */
  private getClientIP(): string {
    // 尝试从常见的代理头中获取真实IP
    const forwardedFor = this.request.headers.get("X-Forwarded-For");
    if (forwardedFor) {
      return forwardedFor.split(",")[0].trim();
    }

    const realIP = this.request.headers.get("X-Real-IP");
    if (realIP) {
      return realIP;
    }

    // 如果都没有，返回unknown
    return "unknown";
  }

  /**
   * 添加日志字段（用于统一日志格式）
   */
  addLogFields(...fields: unknown[]): unknown[] {
    return [
      logger.String("request_id", this.requestId),
      ...fields,
    ];
  }
}

/**
 * 创建请求上下文
 */
export function createRequestContext(
  request: Request,
  authService: AuthService,
  requestType: string,
  requestId: string,
): RequestContext {
  return new RequestContext(request, authService, requestType, requestId);
}
