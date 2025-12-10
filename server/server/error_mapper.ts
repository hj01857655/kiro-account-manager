import * as logger from "../logger/logger.ts";

// Claude错误响应结构
export interface ClaudeErrorResponse {
  type: string;
  message: string;
  stopReason?: string;
}

// CodeWhisperer错误响应体
interface CodeWhispererErrorBody {
  message: string;
  reason: string;
}

// 错误映射策略接口
interface ErrorMappingStrategy {
  mapError(statusCode: number, responseBody: string): { response: ClaudeErrorResponse | null; handled: boolean };
  getErrorType(): string;
}

// 内容长度超限错误映射策略
class ContentLengthExceedsStrategy implements ErrorMappingStrategy {
  mapError(statusCode: number, responseBody: string): { response: ClaudeErrorResponse | null; handled: boolean } {
    if (statusCode !== 400) {
      return { response: null, handled: false };
    }

    try {
      const errorBody: CodeWhispererErrorBody = JSON.parse(responseBody);
      
      if (errorBody.reason === "CONTENT_LENGTH_EXCEEDS_THRESHOLD") {
        return {
          response: {
            type: "message_delta",
            stopReason: "max_tokens",
            message: "Content length exceeds threshold, response truncated",
          },
          handled: true,
        };
      }
    } catch {
      // 解析失败，继续下一个策略
    }

    return { response: null, handled: false };
  }

  getErrorType(): string {
    return "content_length_exceeds";
  }
}

// 默认错误映射策略
class DefaultErrorStrategy implements ErrorMappingStrategy {
  mapError(_statusCode: number, responseBody: string): { response: ClaudeErrorResponse | null; handled: boolean } {
    return {
      response: {
        type: "error",
        message: `Upstream error: ${responseBody}`,
      },
      handled: true,
    };
  }

  getErrorType(): string {
    return "default";
  }
}

// 错误映射器
export class ErrorMapper {
  private strategies: ErrorMappingStrategy[];

  constructor() {
    this.strategies = [
      new ContentLengthExceedsStrategy(),
      new DefaultErrorStrategy(),
    ];
  }

  mapCodeWhispererError(statusCode: number, responseBody: string): ClaudeErrorResponse {
    for (const strategy of this.strategies) {
      const { response, handled } = strategy.mapError(statusCode, responseBody);
      if (handled && response) {
        logger.debug("错误映射成功",
          logger.String("strategy", strategy.getErrorType()),
          logger.Int("status_code", statusCode),
          logger.String("mapped_type", response.type),
          logger.String("stop_reason", response.stopReason || "")
        );
        return response;
      }
    }

    return {
      type: "error",
      message: "Unknown error",
    };
  }

  createErrorResponse(claudeError: ClaudeErrorResponse): Response {
    if (claudeError.stopReason === "max_tokens") {
      return this.createMaxTokensResponse(claudeError);
    }
    return this.createStandardErrorResponse(claudeError);
  }

  private createMaxTokensResponse(claudeError: ClaudeErrorResponse): Response {
    const response = {
      type: "message_delta",
      delta: {
        stop_reason: "max_tokens",
        stop_sequence: null,
      },
      usage: {
        input_tokens: 0,
        output_tokens: 0,
      },
    };

    logger.info("已发送max_tokens stop_reason响应",
      logger.String("stop_reason", "max_tokens"),
      logger.String("original_message", claudeError.message)
    );

    return Response.json(response);
  }

  private createStandardErrorResponse(claudeError: ClaudeErrorResponse): Response {
    const errorResp = {
      type: "error",
      error: {
        type: "overloaded_error",
        message: claudeError.message,
      },
    };

    return Response.json(errorResp, { status: 500 });
  }
}
