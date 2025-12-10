import type { OpenAIRequest, OpenAIResponse, OpenAIMessage, OpenAIToolCall } from "../types/openai.ts";
import type { TokenInfo, TokenWithUsage } from "../types/common.ts";
import { openAIToAnthropic } from "../converter/converter.ts";
import { anthropicToCodeWhisperer, generateId } from "../converter/converter.ts";
import { respondError } from "./common.ts";
import { handleOpenAIStreamRequest as streamProcessor } from "./openai_stream_processor.ts";
import * as logger from "../logger/logger.ts";
import { metricsCollector } from "../logger/metrics.ts";
import { errorTracker, ErrorCategory } from "../logger/error_tracker.ts";
import { calculateInputTokens, calculateOutputTokens } from "../utils/token_calculation.ts";
import { parseEventStreamBinary } from "../utils/response_parser.ts";
import { sendCodeWhispererRequest } from "../utils/codewhisperer_client.ts";

// 处理OpenAI非流式请求
export async function handleOpenAINonStreamRequest(
  openaiReq: OpenAIRequest,
  tokenInfo: TokenInfo,
  requestId?: string,
): Promise<Response> {
  const rid = requestId || crypto.randomUUID();
  metricsCollector.startRequest(rid);

  try {
    // 转换为Anthropic格式
    metricsCollector.startPhase(rid, "convert_request");
    const anthropicReq = openAIToAnthropic(openaiReq);
    const conversationId = crypto.randomUUID();
    const cwReq = anthropicToCodeWhisperer(anthropicReq, conversationId);
    metricsCollector.endPhase(rid, "convert_request");

    // Calculate input tokens
    const inputTokens = calculateInputTokens(anthropicReq);

    logger.info(
      "处理 OpenAI 非流式请求",
      logger.String("request_id", rid),
      logger.String("model", openaiReq.model),
      logger.Int("input_tokens", inputTokens),
      logger.Bool("stream", false),
    );

    metricsCollector.startPhase(rid, "upstream_request");
    const response = await sendCodeWhispererRequest(cwReq, tokenInfo.accessToken, rid);
    metricsCollector.endPhase(rid, "upstream_request", { status: response.status });

    // 读取和解析响应
    metricsCollector.startPhase(rid, "parse_response");
    const responseBuffer = await response.arrayBuffer();
    const data = new Uint8Array(responseBuffer);

    const { content, toolUses } = parseEventStreamBinary(data);

    // Build content blocks
    const contentBlocks: Array<{ type: string; text?: string; id?: string; name?: string; input?: unknown }> = [
      ...(content ? [{ type: "text", text: content }] : []),
      ...toolUses,
    ];

    // Calculate output tokens
    const outputTokens = calculateOutputTokens(contentBlocks);
    metricsCollector.endPhase(rid, "parse_response", {
      outputTokens,
      toolUseCount: toolUses.length,
    });

    logger.info(
      "OpenAI 请求处理完成",
      logger.String("request_id", rid),
      logger.Int("input_tokens", inputTokens),
      logger.Int("output_tokens", outputTokens),
      logger.Int("tool_uses", toolUses.length),
    );

    // 构建Anthropic响应
    const anthropicResponse = {
      id: generateId("msg"),
      type: "message",
      role: "assistant",
      model: anthropicReq.model,
      content: contentBlocks,
      stop_reason: toolUses.length > 0 ? "tool_use" : "stop",
      usage: {
        input_tokens: inputTokens,
        output_tokens: outputTokens,
      },
    };

    // 转换为OpenAI格式
    const openaiResponse = convertAnthropicToOpenAI(anthropicResponse, openaiReq.model);

    metricsCollector.endRequest(rid, true);
    return Response.json(openaiResponse);
  } catch (error) {
    errorTracker.track(
      ErrorCategory.UNKNOWN,
      "处理 OpenAI 非流式请求失败",
      error,
      rid,
    );
    metricsCollector.endRequest(rid, false, error as Error);
    return respondError("Internal server error", 500);
  }
}

// 处理OpenAI流式请求 - 使用OpenAIStreamProcessor
export async function handleOpenAIStreamRequest(
  openaiReq: OpenAIRequest,
  tokenWithUsage: TokenWithUsage,
  requestId: string,
): Promise<Response> {
  // 直接使用OpenAIStreamProcessor处理
  return await streamProcessor(openaiReq, tokenWithUsage, requestId);
}

/**
 * 转换Anthropic响应为OpenAI格式
 */
function convertAnthropicToOpenAI(anthropicResp: Record<string, unknown>, model: string): OpenAIResponse {
  const choices = [];
  const message: OpenAIMessage = {
    role: "assistant",
    content: null,
  };

  // 处理内容
  if (anthropicResp.content && Array.isArray(anthropicResp.content)) {
    const textParts: string[] = [];
    const toolCalls: OpenAIToolCall[] = [];

    for (const item of anthropicResp.content) {
      if (item.type === "text") {
        textParts.push(item.text);
      } else if (item.type === "tool_use") {
        toolCalls.push({
          id: item.id,
          type: "function",
          function: {
            name: item.name,
            arguments: JSON.stringify(item.input),
          },
        });
      }
    }

    if (textParts.length > 0) {
      message.content = textParts.join("");
    }

    if (toolCalls.length > 0) {
      message.tool_calls = toolCalls;
    }
  }

  choices.push({
    index: 0,
    message,
    finish_reason: anthropicResp.stop_reason === "tool_use" ? "tool_calls" : "stop",
  });

  // Map Anthropic usage to OpenAI format
  const usageObj = anthropicResp.usage as Record<string, unknown> | undefined;
  const usage = usageObj ? {
    prompt_tokens: (usageObj.input_tokens as number) || 0,
    completion_tokens: (usageObj.output_tokens as number) || 0,
    total_tokens: ((usageObj.input_tokens as number) || 0) + ((usageObj.output_tokens as number) || 0),
  } : {
    prompt_tokens: 0,
    completion_tokens: 0,
    total_tokens: 0,
  };

  return {
    id: `chatcmpl-${Date.now()}`,
    object: "chat.completion",
    created: Math.floor(Date.now() / 1000),
    model,
    choices,
    usage,
  };
}
