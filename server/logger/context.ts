/**
 * 日志上下文管理
 * 提供请求级别的上下文追踪
 */

import type { Field } from "./logger.ts";
import { String as LogString } from "./logger.ts";

/**
 * 请求上下文
 */
export interface RequestContext {
  requestId: string;
  method?: string;
  path?: string;
  model?: string;
  userId?: string;
  startTime: number;
}

/**
 * 上下文管理器
 */
class ContextManager {
  private contexts = new Map<string, RequestContext>();

  /**
   * 创建请求上下文
   */
  create(requestId: string, data?: Partial<RequestContext>): RequestContext {
    const ctx: RequestContext = {
      requestId,
      startTime: Date.now(),
      ...data,
    };
    this.contexts.set(requestId, ctx);
    return ctx;
  }

  /**
   * 获取请求上下文
   */
  get(requestId: string): RequestContext | undefined {
    return this.contexts.get(requestId);
  }

  /**
   * 删除请求上下文
   */
  delete(requestId: string): void {
    this.contexts.delete(requestId);
  }

  /**
   * 获取请求耗时
   */
  getElapsed(requestId: string): number {
    const ctx = this.contexts.get(requestId);
    return ctx ? Date.now() - ctx.startTime : 0;
  }

  /**
   * 将上下文转换为日志字段
   */
  toFields(requestId: string): Field[] {
    const ctx = this.contexts.get(requestId);
    if (!ctx) return [];

    const fields: Field[] = [
      LogString("request_id", ctx.requestId),
    ];

    if (ctx.method) fields.push(LogString("method", ctx.method));
    if (ctx.path) fields.push(LogString("path", ctx.path));
    if (ctx.model) fields.push(LogString("model", ctx.model));
    if (ctx.userId) fields.push(LogString("user_id", ctx.userId));

    return fields;
  }

  /**
   * 清理过期上下文（超过1小时）
   */
  cleanup(): void {
    const now = Date.now();
    const maxAge = 60 * 60 * 1000; // 1小时

    for (const [id, ctx] of this.contexts.entries()) {
      if (now - ctx.startTime > maxAge) {
        this.contexts.delete(id);
      }
    }
  }
}

// 全局上下文管理器
export const contextManager = new ContextManager();

// 定期清理过期上下文
setInterval(() => contextManager.cleanup(), 5 * 60 * 1000); // 每5分钟清理一次
