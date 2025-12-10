import type { TokenWithUsage } from "../types/common.ts";
import * as logger from "../logger/logger.ts";

/**
 * 流处理器基类
 * 提取 Anthropic 和 OpenAI 流处理器的公共逻辑
 */
export abstract class BaseStreamProcessor {
  // 请求信息
  public readonly tokenWithUsage: TokenWithUsage;
  public readonly messageId: string;
  public readonly requestId: string;

  // 统计信息
  public totalProcessedEvents = 0;

  // 异常处理
  protected forcedFinishReason: string | null = null;

  constructor(
    tokenWithUsage: TokenWithUsage,
    messageId: string,
    requestId: string,
  ) {
    this.tokenWithUsage = tokenWithUsage;
    this.messageId = messageId;
    this.requestId = requestId;
  }

  /**
   * 清理资源（子类实现）
   */
  abstract cleanup(): void;

  /**
   * 发送初始事件（子类实现）
   */
  abstract sendInitialEvents(
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): void;

  /**
   * 发送结束事件（子类实现）
   */
  abstract sendFinalEvents(
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): void;

  /**
   * 处理单个消息（子类实现）
   * 返回 true 表示应该终止流处理
   */
  abstract processMessage(
    message: { payload: Uint8Array },
    controller: ReadableStreamDefaultController,
    encoder: TextEncoder,
  ): boolean;

  /**
   * 解析 AWS EventStream payload
   */
  protected parsePayload(payload: Uint8Array): Record<string, unknown> | null {
    try {
      const decoded = JSON.parse(new TextDecoder().decode(payload));
      return decoded.assistantResponseEvent || decoded;
    } catch {
      return null;
    }
  }

  /**
   * 检查是否为内容长度超限异常
   */
  protected isContentLengthException(event: Record<string, unknown>): boolean {
    const exceptionType = (event.exception_type as string) || (event.__type as string);
    return exceptionType === "ContentLengthExceededException" ||
      exceptionType?.includes("CONTENT_LENGTH_EXCEEDS") || false;
  }

  /**
   * 记录异常日志
   */
  protected logException(exceptionType: string, finishReason: string): void {
    logger.info(
      "检测到内容长度超限异常",
      logger.String("request_id", this.requestId),
      logger.String("exception_type", exceptionType),
      logger.String("finish_reason", finishReason),
    );
  }

  /**
   * 处理事件流（通用逻辑）
   */
  async processEventStream(
    reader: ReadableStreamDefaultReader<Uint8Array>,
    controller: ReadableStreamDefaultController,
    parseChunk: (chunk: Uint8Array) => Array<{ payload: Uint8Array }>,
  ): Promise<boolean> {
    const encoder = new TextEncoder();
    let shouldTerminate = false;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const messages = parseChunk(value);

      for (const message of messages) {
        shouldTerminate = this.processMessage(message, controller, encoder);
        if (shouldTerminate) {
          logger.info(
            "检测到终止信号，停止流处理",
            logger.String("request_id", this.requestId),
          );
          break;
        }
      }

      if (shouldTerminate) break;
    }

    logger.debug(
      "响应流结束",
      logger.String("request_id", this.requestId),
      logger.Bool("terminated_by_exception", shouldTerminate),
      logger.Int("processed_events", this.totalProcessedEvents),
    );

    return shouldTerminate;
  }
}
