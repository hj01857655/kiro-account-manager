import * as logger from "../logger/logger.ts";

// Stop Reason管理器
export class StopReasonManager {
  private hasActiveToolCalls = false;
  private hasCompletedTools = false;
  private forcedStopReason: string | null = null;

  updateToolCallStatus(hasActiveCalls: boolean, hasCompleted: boolean): void {
    this.hasActiveToolCalls = hasActiveCalls;
    this.hasCompletedTools = hasCompleted;

    logger.debug("更新工具调用状态",
      logger.Bool("has_active_tools", hasActiveCalls),
      logger.Bool("has_completed_tools", hasCompleted)
    );
  }

  /**
   * 强制设置 stop_reason
   * 用于处理特殊情况，如异常映射（ContentLengthExceededException -> max_tokens）
   */
  forceStopReason(reason: string): void {
    this.forcedStopReason = reason;
    logger.debug("强制设置stop_reason", logger.String("reason", reason));
  }

  determineStopReason(): string {
    // 优先使用强制设置的 stop_reason（用于异常处理）
    if (this.forcedStopReason) {
      return this.forcedStopReason;
    }

    // 根据Claude规范，只要消息包含tool_use块，stop_reason就应该是tool_use
    if (this.hasActiveToolCalls || this.hasCompletedTools) {
      return "tool_use";
    }

    // 默认情况 - 自然完成响应
    return "end_turn";
  }

  determineStopReasonFromUpstream(upstreamStopReason: string): string {
    if (!upstreamStopReason) {
      return this.determineStopReason();
    }

    // 验证上游stop_reason是否符合Claude规范
    const validStopReasons = new Set([
      "end_turn",
      "max_tokens",
      "stop_sequence",
      "tool_use",
      "pause_turn",
      "refusal",
    ]);

    if (!validStopReasons.has(upstreamStopReason)) {
      logger.warn("上游提供了无效的stop_reason，使用本地逻辑",
        logger.String("upstream_stop_reason", upstreamStopReason)
      );
      return this.determineStopReason();
    }

    logger.debug("使用上游stop_reason",
      logger.String("upstream_stop_reason", upstreamStopReason)
    );
    return upstreamStopReason;
  }
}

// 获取stop_reason的描述
export function getStopReasonDescription(stopReason: string): string {
  const descriptions: Record<string, string> = {
    "end_turn": "Claude自然完成了响应",
    "max_tokens": "达到了token限制",
    "stop_sequence": "遇到了自定义停止序列",
    "tool_use": "Claude正在调用工具并期待执行",
    "pause_turn": "服务器工具操作暂停",
    "refusal": "Claude拒绝生成响应",
  };

  return descriptions[stopReason] || "未知的stop_reason";
}
