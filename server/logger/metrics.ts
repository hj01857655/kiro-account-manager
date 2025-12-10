/**
 * 性能指标收集
 * 用于监控和排查性能问题
 */

import * as logger from "./logger.ts";

/**
 * 请求指标
 */
export interface RequestMetrics {
  requestId: string;
  startTime: number;
  endTime?: number;
  phases: Map<string, PhaseMetrics>;
}

/**
 * 阶段指标
 */
export interface PhaseMetrics {
  name: string;
  startTime: number;
  endTime?: number;
  duration?: number;
  metadata?: Record<string, unknown>;
}

/**
 * 指标收集器
 */
class MetricsCollector {
  private metrics = new Map<string, RequestMetrics>();

  /**
   * 开始请求追踪
   */
  startRequest(requestId: string): void {
    this.metrics.set(requestId, {
      requestId,
      startTime: Date.now(),
      phases: new Map(),
    });
  }

  /**
   * 开始阶段追踪
   */
  startPhase(requestId: string, phaseName: string): void {
    const metric = this.metrics.get(requestId);
    if (!metric) return;

    metric.phases.set(phaseName, {
      name: phaseName,
      startTime: Date.now(),
    });
  }

  /**
   * 结束阶段追踪
   */
  endPhase(
    requestId: string,
    phaseName: string,
    metadata?: Record<string, unknown>,
  ): void {
    const metric = this.metrics.get(requestId);
    if (!metric) return;

    const phase = metric.phases.get(phaseName);
    if (!phase) return;

    phase.endTime = Date.now();
    phase.duration = phase.endTime - phase.startTime;
    phase.metadata = metadata;

    // 记录阶段完成日志
    logger.debug(
      `阶段完成: ${phaseName}`,
      logger.String("request_id", requestId),
      logger.Phase(phaseName),
      logger.Duration("duration", phase.duration),
      ...(metadata ? [logger.Any("metadata", metadata)] : []),
    );
  }

  /**
   * 结束请求追踪并记录汇总
   */
  endRequest(requestId: string, success: boolean, error?: Error): void {
    const metric = this.metrics.get(requestId);
    if (!metric) return;

    metric.endTime = Date.now();
    const totalDuration = metric.endTime - metric.startTime;

    // 计算各阶段耗时
    const phaseDurations: Record<string, number> = {};
    for (const [name, phase] of metric.phases) {
      if (phase.duration !== undefined) {
        phaseDurations[name] = phase.duration;
      }
    }

    // 记录请求完成日志
    const logFields = [
      logger.String("request_id", requestId),
      logger.Bool("success", success),
      logger.Duration("total_duration", totalDuration),
      logger.Any("phase_durations", phaseDurations),
    ];

    if (error) {
      logFields.push(logger.Err(error));
    }

    if (success) {
      logger.info("请求完成", ...logFields);
    } else {
      logger.error("请求失败", ...logFields);
    }

    // 清理指标
    this.metrics.delete(requestId);
  }

  /**
   * 获取请求指标
   */
  getMetrics(requestId: string): RequestMetrics | undefined {
    return this.metrics.get(requestId);
  }

  /**
   * 清理过期指标
   */
  cleanup(): void {
    const now = Date.now();
    const maxAge = 60 * 60 * 1000; // 1小时

    for (const [id, metric] of this.metrics.entries()) {
      if (now - metric.startTime > maxAge) {
        this.metrics.delete(id);
      }
    }
  }
}

// 全局指标收集器
export const metricsCollector = new MetricsCollector();

// 定期清理过期指标
setInterval(() => metricsCollector.cleanup(), 5 * 60 * 1000);

/**
 * 性能追踪装饰器
 */
export function trackPerformance(phaseName: string) {
  return function (
    _target: unknown,
    _propertyKey: string,
    descriptor: PropertyDescriptor,
  ) {
    const originalMethod = descriptor.value;

    descriptor.value = async function (...args: unknown[]) {
      // 尝试从参数中提取 requestId
      const requestId = extractRequestId(args);

      if (requestId) {
        metricsCollector.startPhase(requestId, phaseName);
      }

      try {
        const result = await originalMethod.apply(this, args);
        if (requestId) {
          metricsCollector.endPhase(requestId, phaseName);
        }
        return result;
      } catch (error) {
        if (requestId) {
          metricsCollector.endPhase(requestId, phaseName, {
            error: error instanceof Error ? error.message : String(error),
          });
        }
        throw error;
      }
    };

    return descriptor;
  };
}

/**
 * 从参数中提取 requestId
 */
function extractRequestId(args: unknown[]): string | null {
  for (const arg of args) {
    if (typeof arg === "string" && arg.match(/^[a-f0-9-]{36}$/i)) {
      return arg;
    }
    if (arg && typeof arg === "object" && "requestId" in arg) {
      return (arg as { requestId: string }).requestId;
    }
  }
  return null;
}
