import type { AnthropicRequest } from "../types/anthropic.ts";
import type { TokenInfo } from "../types/common.ts";
import { AuthService } from "../auth/auth_service.ts";
import { anthropicToCodeWhisperer, generateId } from "../converter/converter.ts";
import { MODEL_MAP, AWS_ENDPOINTS } from "../config/constants.ts";
import { handleStreamRequest } from "./stream_processor.ts";
import { respondError } from "./common.ts";
import * as logger from "../logger/logger.ts";
import { metricsCollector } from "../logger/metrics.ts";
import { errorTracker, ErrorCategory } from "../logger/error_tracker.ts";
import type { TokenWithUsage } from "../types/common.ts";
import { RobustEventStreamParser } from "../parser/robust_parser.ts";
import { calculateInputTokens, calculateOutputTokens } from "../utils/token_calculation.ts";
import { parseEventStreamResponse } from "../utils/response_parser.ts";
import { sendCodeWhispererRequest } from "../utils/codewhisperer_client.ts";
import { createCodeWhispererHeaders } from "../utils/request_headers.ts";

// Handle /v1/models endpoint
export function handleModels(): Response {
  const models = Object.keys(MODEL_MAP).map((id) => ({
    id,
    object: "model",
    created: 1234567890,
    owned_by: "anthropic",
    display_name: id,
    type: "text",
    max_tokens: 200000,
  }));

  return Response.json({
    object: "list",
    data: models,
  });
}

// Handle /api/tokens endpoint
export async function handleTokenStatus(authService: AuthService): Promise<Response> {
  const status = await authService.getTokenPoolStatus();
  return Response.json(status);
}

// Handle /v1/messages endpoint (Anthropic format)
export async function handleMessages(
  req: Request,
  authService: AuthService,
): Promise<Response> {
  const requestId = crypto.randomUUID();
  metricsCollector.startRequest(requestId);

  try {
    metricsCollector.startPhase(requestId, "parse_request");
    const anthropicReq: AnthropicRequest = await req.json();
    metricsCollector.endPhase(requestId, "parse_request");

    // Validate request
    if (!anthropicReq.messages || anthropicReq.messages.length === 0) {
      errorTracker.track(
        ErrorCategory.REQUEST_INVALID_PARAMS,
        "messages array cannot be empty",
        new Error("Empty messages array"),
        requestId,
      );
      metricsCollector.endRequest(requestId, false);
      return respondError("messages array cannot be empty", 400);
    }

    // Get token with usage
    metricsCollector.startPhase(requestId, "get_token");
    const tokenWithUsage = await authService.getTokenWithUsage();
    metricsCollector.endPhase(requestId, "get_token");

    if (anthropicReq.stream) {
      // Use StreamProcessor for streaming requests
      return await handleStreamingRequest(anthropicReq, tokenWithUsage, requestId);
    } else {
      return await handleNonStreamRequest(anthropicReq, tokenWithUsage.tokenInfo, requestId);
    }
  } catch (error) {
    errorTracker.track(
      ErrorCategory.UNKNOWN,
      "处理 messages 请求失败",
      error,
      requestId,
    );
    metricsCollector.endRequest(requestId, false, error as Error);
    return respondError("Internal server error", 500);
  }
}


// Handle streaming requests using StreamProcessor
async function handleStreamingRequest(
  anthropicReq: AnthropicRequest,
  tokenWithUsage: TokenWithUsage,
  requestId: string,
): Promise<Response> {
  const conversationId = crypto.randomUUID();
  
  metricsCollector.startPhase(requestId, "convert_request");
  const cwReq = anthropicToCodeWhisperer(anthropicReq, conversationId);
  metricsCollector.endPhase(requestId, "convert_request");

  logger.info(
    "发送流式请求到 CodeWhisperer",
    logger.String("request_id", requestId),
    logger.String("model", anthropicReq.model),
    logger.Bool("stream", true),
  );

  metricsCollector.startPhase(requestId, "upstream_request");
  const startTime = Date.now();
  
  try {
    const upstreamResponse = await fetch(AWS_ENDPOINTS.CODEWHISPERER, {
      method: "POST",
      headers: createCodeWhispererHeaders(tokenWithUsage.tokenInfo.accessToken),
      body: JSON.stringify(cwReq),
    });
    
    const latency = Date.now() - startTime;
    metricsCollector.endPhase(requestId, "upstream_request", {
      status: upstreamResponse.status,
      latency,
    });

    logger.info(
      "收到上游响应",
      logger.String("request_id", requestId),
      logger.HttpStatus(upstreamResponse.status),
      logger.Latency(latency),
    );

    // Use StreamProcessor to handle the streaming response
    return await handleStreamRequest(
      anthropicReq,
      tokenWithUsage,
      requestId,
      upstreamResponse,
    );
  } catch (error) {
    const latency = Date.now() - startTime;
    metricsCollector.endPhase(requestId, "upstream_request", {
      error: error instanceof Error ? error.message : String(error),
      latency,
    });
    
    errorTracker.track(
      ErrorCategory.UPSTREAM_ERROR,
      "上游请求失败",
      error,
      requestId,
      { latency },
    );
    throw error;
  }
}

// Handle non-streaming requests with TokenEstimator
async function handleNonStreamRequest(
  anthropicReq: AnthropicRequest,
  tokenInfo: TokenInfo,
  requestId: string,
): Promise<Response> {
  const conversationId = crypto.randomUUID();
  
  metricsCollector.startPhase(requestId, "convert_request");
  const cwReq = anthropicToCodeWhisperer(anthropicReq, conversationId);
  metricsCollector.endPhase(requestId, "convert_request");

  // Calculate input tokens
  const inputTokens = calculateInputTokens(anthropicReq);

  logger.info(
    "发送非流式请求到 CodeWhisperer",
    logger.String("request_id", requestId),
    logger.String("model", anthropicReq.model),
    logger.Int("input_tokens", inputTokens),
    logger.Bool("stream", false),
  );

  metricsCollector.startPhase(requestId, "upstream_request");
  const startTime = Date.now();
  
  try {
    const response = await sendCodeWhispererRequest(cwReq, tokenInfo.accessToken, requestId);
    const latency = Date.now() - startTime;
    
    metricsCollector.endPhase(requestId, "upstream_request", {
      status: response.status,
      latency,
    });

    // Read and parse response
    metricsCollector.startPhase(requestId, "parse_response");
    const responseBuffer = await response.arrayBuffer();
    const data = new Uint8Array(responseBuffer);
    
    logger.debug(
      "收到上游响应",
      logger.String("request_id", requestId),
      logger.HttpStatus(response.status),
      logger.Latency(latency),
      logger.Bytes(data.length),
    );

    const parser = new RobustEventStreamParser();
    const messages = parser.parseStream(data);
    metricsCollector.endPhase(requestId, "parse_response", {
      messageCount: messages.length,
    });

    // Parse content and tool uses
    metricsCollector.startPhase(requestId, "extract_content");
    const { content, toolUses } = parseEventStreamResponse(messages, requestId);
    metricsCollector.endPhase(requestId, "extract_content", {
      contentLength: content.length,
      toolUseCount: toolUses.length,
    });

    // Build content blocks
    const contentBlocks: Array<{ type: string; text?: string; id?: string; name?: string; input?: unknown }> = [];
    if (content) {
      contentBlocks.push({ type: "text", text: content });
    }
    contentBlocks.push(...toolUses);

    // Calculate output tokens
    const outputTokens = calculateOutputTokens(contentBlocks);

    logger.info(
      "请求处理完成",
      logger.String("request_id", requestId),
      logger.Int("input_tokens", inputTokens),
      logger.Int("output_tokens", outputTokens),
      logger.Int("tool_uses", toolUses.length),
    );

    metricsCollector.endRequest(requestId, true);

    // Convert response to Anthropic format
    const anthropicResponse = {
      id: generateId("msg"),
      type: "message",
      role: "assistant",
      model: anthropicReq.model,
      content: contentBlocks,
      stop_reason: toolUses.length > 0 ? "tool_use" : "end_turn",
      usage: {
        input_tokens: inputTokens,
        output_tokens: outputTokens,
      },
    };

    return Response.json(anthropicResponse);
  } catch (error) {
    const latency = Date.now() - startTime;
    metricsCollector.endPhase(requestId, "upstream_request", {
      error: error instanceof Error ? error.message : String(error),
      latency,
    });
    
    errorTracker.track(
      ErrorCategory.UPSTREAM_ERROR,
      "非流式请求失败",
      error,
      requestId,
      { latency },
    );
    
    metricsCollector.endRequest(requestId, false, error as Error);
    throw error;
  }
}
