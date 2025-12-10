import type { AnthropicRequest } from "../types/anthropic.ts";
import type { TokenWithUsage } from "../types/common.ts";
import { calculateInputTokens } from "../utils/token_calculation.ts";
import { SSEStateManager } from "./sse_state_manager.ts";
import { StopReasonManager, getStopReasonDescription } from "./stop_reason_manager.ts";
import { ErrorMapper } from "./error_mapper.ts";
import * as logger from "../logger/logger.ts";
import { errorTracker, ErrorCategory } from "../logger/error_tracker.ts";
import { TimeoutError } from "../config/timeout.ts";
import { StreamEventConverter } from "./stream_event_converter.ts";
import { StreamTokenManager } from "./stream_token_manager.ts";
import { StreamTimeoutController } from "./stream_timeout_controller.ts";
import { StreamBufferProcessor } from "./stream_buffer_processor.ts";
import { BaseStreamProcessor } from "./base_stream_processor.ts";

/**
 * 流处理器上下文（重构版）
 * 封装流式请求处理的所有状态
 *
 * 重构改进：
 * 1. 职责分离：将大类拆分为多个专门的管理器
 * 2. 代码复用：提取公共逻辑到独立类
 * 3. 性能优化：使用增量解析和缓冲区复用
 * 4. 可测试性：每个管理器可独立测试
 */
export class StreamProcessorContext extends BaseStreamProcessor {
  // 请求信息
  public readonly anthropicReq: AnthropicRequest;
  public readonly inputTokens: number;

  // 专门的管理器（职责分离）
  private readonly sseStateManager: SSEStateManager;
  private readonly stopReasonManager: StopReasonManager;
  private readonly eventConverter: StreamEventConverter;
  private readonly tokenManager: StreamTokenManager;
  private readonly timeoutController: StreamTimeoutController;
  private readonly bufferProcessor: StreamBufferProcessor;

  // 工具调用追踪
  private readonly completedToolUseIds = new Set<string>();

  constructor(
    anthropicReq: AnthropicRequest,
    tokenWithUsage: TokenWithUsage,
    messageId: string,
    requestId: string,
    inputTokens: number,
  ) {
    super(tokenWithUsage, messageId, requestId);
    this.anthropicReq = anthropicReq;
    this.inputTokens = inputTokens;

    // 初始化所有管理器
    this.sseStateManager = new SSEStateManager(false);
    this.stopReasonManager = new StopReasonManager();
    this.eventConverter = new StreamEventConverter();
    this.tokenManager = new StreamTokenManager();
    this.timeoutController = new StreamTimeoutController();
    this.bufferProcessor = new StreamBufferProcessor();
  }

  /**
   * 清理资源（重构版）
   */
  cleanup(): void {
    // 清理所有管理器
    this.bufferProcessor.reset();
    this.eventConverter.reset();
    this.tokenManager.reset();
    this.timeoutController.cleanup();
    this.completedToolUseIds.clear();
  }

  /**
   * 发送初始事件
   */
  sendInitialEvents(
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): void {

    // message_start事件
    const startEvent = {
      type: "message_start",
      message: {
        id: this.messageId,
        type: "message",
        role: "assistant",
        model: this.anthropicReq.model,
        content: [],
        stop_reason: null,
        usage: { input_tokens: this.inputTokens, output_tokens: 0 },
      },
    };

    const validation = this.sseStateManager.validateAndSend(startEvent);
    if (validation.valid) {
      controller.enqueue(
        encoder.encode(
          `event: ${startEvent.type}\n` +
          `data: ${JSON.stringify(startEvent)}\n\n`,
        ),
      );
    }

    // ping事件
    const pingEvent = { type: "ping" };
    controller.enqueue(
      encoder.encode(
        `event: ping\n` +
        `data: ${JSON.stringify(pingEvent)}\n\n`,
      ),
    );
  }

  /**
   * 处理工具使用开始事件
   */
  private processToolUseStart(dataMap: Record<string, unknown>): void {
    const contentBlock = dataMap.content_block as Record<string, unknown>;
    if (!contentBlock || contentBlock.type !== "tool_use") {
      return;
    }

    const index = dataMap.index as number;
    const toolId = contentBlock.id as string;

    if (toolId) {
      this.eventConverter.getToolUseIdByBlockIndex().set(index, toolId);
      logger.debug(
        "转发tool_use开始",
        logger.String("request_id", this.requestId),
        logger.String("tool_use_id", toolId),
        logger.String("tool_name", contentBlock.name as string),
        logger.Int("index", index),
      );
    }
  }

  /**
   * 处理工具使用结束事件
   */
  private processToolUseStop(dataMap: Record<string, unknown>): void {
    const index = dataMap.index as number;
    const toolId = this.eventConverter.getToolUseIdByBlockIndex().get(index);

    if (toolId) {
      // 关键：先记录到完成集合，再删除映射
      this.completedToolUseIds.add(toolId);
      this.eventConverter.getToolUseIdByBlockIndex().delete(index);
    }
  }

  /**
   * 处理单个AWS EventStream消息
   * 返回 true 表示应该终止流处理
   */
  processMessage(
    message: { payload: Uint8Array },
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): boolean {
    const event = this.parsePayload(message.payload);
    if (!event) return false;

    // 检查是否为异常事件
    if (this.handleExceptionEvent(event, controller, encoder)) {
      return true; // 终止流处理
    }

    // 将 CodeWhisperer 事件转换为 Anthropic SSE 格式
    const anthropicEvents = this.eventConverter.convertToAnthropicEvents(event);
    
    // 发送转换后的事件
    for (const dataMap of anthropicEvents) {
      this.processAnthropicEvent(dataMap, controller, encoder);
    }

    return false;
  }


  /**
   * 处理单个 Anthropic 事件
   */
  processAnthropicEvent(
    dataMap: Record<string, unknown>,
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): void {
    const eventType = dataMap.type as string;

    // 处理不同类型的事件
    switch (eventType) {
      case "content_block_start":
        this.processToolUseStart(dataMap);
        break;

      case "content_block_stop":
        this.processToolUseStop(dataMap);
        break;

      case "content_block_delta":
        // Token估算在验证发送后处理
        break;
    }

    // 验证并发送事件
    const validation = this.sseStateManager.validateAndSend(dataMap);
    if (validation.valid) {
      controller.enqueue(
        encoder.encode(
          `event: ${eventType}\n` +
          `data: ${JSON.stringify(dataMap)}\n\n`,
        ),
      );
    }

    // 累计token（基于实际发送的内容）
    this.tokenManager.accumulateTokens(dataMap);
  }

  /**
   * 处理上游异常事件
   * 返回 true 表示已处理异常并应终止流
   */
  private handleExceptionEvent(
    event: Record<string, unknown>,
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): boolean {
    if (!this.isContentLengthException(event)) {
      return false;
    }

    const exceptionType = (event.exception_type as string) || (event.__type as string);
    this.logException(exceptionType, "max_tokens");
    
    // 追踪异常
    errorTracker.track(
      ErrorCategory.UPSTREAM_ERROR,
      `上游异常: ${exceptionType}`,
      new Error(exceptionType),
      this.requestId,
      { exceptionType, stopReason: "max_tokens" },
    );

      // 强制设置 stop_reason 为 max_tokens
      this.stopReasonManager.forceStopReason("max_tokens");

      // 关闭所有活跃的 content_block
      for (const [index, block] of this.sseStateManager.getActiveBlocks().entries()) {
        if (block.started && !block.stopped) {
          const stopEvent = { type: "content_block_stop", index };
          const validation = this.sseStateManager.validateAndSend(stopEvent);
          if (validation.valid) {
            controller.enqueue(
              encoder.encode(
                `event: content_block_stop\n` +
                `data: ${JSON.stringify(stopEvent)}\n\n`,
              ),
            );
          }
        }
      }

      // 发送 message_delta 事件（max_tokens）
      const maxTokensEvent = {
        type: "message_delta",
        delta: {
          stop_reason: "max_tokens",
          stop_sequence: null,
        },
        usage: {
          output_tokens: Math.max(1, this.tokenManager.getTotalOutputTokens()),
        },
      };

      const validation1 = this.sseStateManager.validateAndSend(maxTokensEvent);
      if (validation1.valid) {
        controller.enqueue(
          encoder.encode(
            `event: message_delta\n` +
            `data: ${JSON.stringify(maxTokensEvent)}\n\n`,
          ),
        );
      }

      // 发送 message_stop 事件
      const messageStopEvent = { type: "message_stop" };
      const validation2 = this.sseStateManager.validateAndSend(messageStopEvent);
      if (validation2.valid) {
        controller.enqueue(
          encoder.encode(
            `event: message_stop\n` +
            `data: ${JSON.stringify(messageStopEvent)}\n\n`,
          ),
        );
      }

      return true; // 已处理异常，终止流
  }


  /**
   * 发送结束事件
   */
  sendFinalEvents(
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): void {

    // 关闭文本块（如果已启动）
    if (this.eventConverter.isTextBlockStarted()) {
      const stopEvent = { type: "content_block_stop", index: 0 };
      const validation = this.sseStateManager.validateAndSend(stopEvent);
      if (validation.valid) {
        controller.enqueue(
          encoder.encode(
            `event: content_block_stop\n` +
            `data: ${JSON.stringify(stopEvent)}\n\n`,
          ),
        );
      }
    }

    // 关闭所有未关闭的content_block
    for (const [index, block] of this.sseStateManager.getActiveBlocks().entries()) {
      if (block.started && !block.stopped) {
        const stopEvent = { type: "content_block_stop", index };
        const validation = this.sseStateManager.validateAndSend(stopEvent);
        if (validation.valid) {
          controller.enqueue(
            encoder.encode(
              `event: content_block_stop\n` +
              `data: ${JSON.stringify(stopEvent)}\n\n`,
            ),
          );
        }
      }
    }

    // 更新工具调用状态
    const hasActiveTools = this.eventConverter.getToolUseIdByBlockIndex().size > 0;
    const hasCompletedTools = this.completedToolUseIds.size > 0;

    this.stopReasonManager.updateToolCallStatus(hasActiveTools, hasCompletedTools);

    // 最小token保护
    let outputTokens = this.tokenManager.getTotalOutputTokens();
    if (outputTokens < 1 && (hasActiveTools || hasCompletedTools)) {
      outputTokens = 1;
    }

    // 确定stop_reason
    const stopReason = this.stopReasonManager.determineStopReason();

    logger.debug(
      "流式响应stop_reason决策",
      logger.String("request_id", this.requestId),
      logger.String("stop_reason", stopReason),
      logger.String("description", getStopReasonDescription(stopReason)),
      logger.Int("output_tokens", outputTokens),
    );

    // 发送message_delta事件
    const stopEvent = {
      type: "message_delta",
      delta: { stop_reason: stopReason, stop_sequence: null },
      usage: { output_tokens: Math.max(1, outputTokens) },
    };
    const validation2 = this.sseStateManager.validateAndSend(stopEvent);
    if (validation2.valid) {
      controller.enqueue(
        encoder.encode(
          `event: message_delta\n` +
          `data: ${JSON.stringify(stopEvent)}\n\n`,
        ),
      );
    }

    // 发送message_stop事件
    const messageStopEvent = { type: "message_stop" };
    const validation3 = this.sseStateManager.validateAndSend(messageStopEvent);
    if (validation3.valid) {
      controller.enqueue(
        encoder.encode(
          `event: message_stop\n` +
          `data: ${JSON.stringify(messageStopEvent)}\n\n`,
        ),
      );
    }
  }


  /**
   * 处理事件流（带超时控制）
   */
  async processEventStreamWithTimeout(
    reader: ReadableStreamDefaultReader<Uint8Array>,
    controller: ReadableStreamDefaultController,
  ): Promise<void> {
    const encoder = new TextEncoder();
    let shouldTerminate = false;

    try {
      while (true) {
        this.timeoutController.checkTimeouts();
        const { done, value } = await this.timeoutController.readWithTimeout(reader);
        this.timeoutController.clearReadTimeout();

        if (done) break;

        this.timeoutController.updateReadStats(value.length);
        const messages = this.bufferProcessor.processChunk(value);

        for (const message of messages) {
          shouldTerminate = this.processMessage(message, controller, encoder);
          if (shouldTerminate) break;
        }

        if (shouldTerminate) break;
      }

      const stats = this.timeoutController.getStats();
      logger.debug(
        "响应流结束",
        logger.String("request_id", this.requestId),
        logger.Int("total_read_bytes", stats.totalReadBytes),
        logger.Int("total_messages", this.bufferProcessor.getTotalProcessedEvents()),
        logger.Int("duration_ms", stats.elapsed),
        logger.Bool("terminated_by_exception", shouldTerminate),
      );
    } catch (error) {
      if (error instanceof TimeoutError) {
        const stats = this.timeoutController.getStats();
        errorTracker.track(
          ErrorCategory.STREAM_TIMEOUT,
          "流处理超时",
          error,
          this.requestId,
          {
            timeoutType: error.timeoutType,
            elapsedMs: stats.elapsed,
            totalBytes: stats.totalReadBytes,
          },
        );
      } else {
        errorTracker.track(
          ErrorCategory.STREAM_INTERRUPTED,
          "流处理中断",
          error,
          this.requestId,
        );
      }
      throw error;
    }
  }
}

/**
 * 处理流式请求
 */
export function handleStreamRequest(
  anthropicReq: AnthropicRequest,
  tokenWithUsage: TokenWithUsage,
  requestId: string,
  upstreamResponse: Response,
): Response {
  const messageId = `msg_${crypto.randomUUID().replace(/-/g, "")}`;

  // 计算输入tokens
  const inputTokens = calculateInputTokens(anthropicReq);

  const stream = new ReadableStream({
    async start(controller) {
      // 创建流处理上下文
      const ctx = new StreamProcessorContext(
        anthropicReq,
        tokenWithUsage,
        messageId,
        requestId,
        inputTokens,
      );

      try {
        // 检查上游响应
        if (!upstreamResponse.ok || !upstreamResponse.body) {
          const errorMapper = new ErrorMapper();
          const errorText = await upstreamResponse.text();
          const claudeError = errorMapper.mapCodeWhispererError(
            upstreamResponse.status,
            errorText,
          );
          const errorResp = errorMapper.createErrorResponse(claudeError);
          controller.enqueue(new TextEncoder().encode(await errorResp.text()));
          controller.close();
          return;
        }

        const encoder = new TextEncoder();

        // 发送初始事件
        ctx.sendInitialEvents(controller, encoder);

        // 处理事件流
        const reader = upstreamResponse.body.getReader();
        await ctx.processEventStreamWithTimeout(reader, controller);

        // 发送结束事件
        ctx.sendFinalEvents(controller, encoder);

        controller.close();
      } catch (error) {
        logger.error(
          "流式请求处理失败",
          logger.String("request_id", requestId),
          logger.Err(error),
        );
        controller.error(error);
      } finally {
        // 清理资源
        ctx.cleanup();
      }
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      "Connection": "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
