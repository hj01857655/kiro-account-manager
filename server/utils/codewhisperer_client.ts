/**
 * CodeWhisperer API 客户端
 */

import type { CodeWhispererRequest } from "../types/codewhisperer.ts";
import { AWS_ENDPOINTS } from "../config/constants.ts";
import { createCodeWhispererHeaders } from "./request_headers.ts";
import * as logger from "../logger/logger.ts";
import { errorTracker, ErrorCategory } from "../logger/error_tracker.ts";

/**
 * 发送请求到 CodeWhisperer
 */
export async function sendCodeWhispererRequest(
  cwReq: CodeWhispererRequest,
  accessToken: string,
  requestId: string
): Promise<Response> {
  const reqStr = JSON.stringify(cwReq);
  const startTime = Date.now();
  
  logger.debug(
    "发送请求到 CodeWhisperer",
    logger.String("request_id", requestId),
    logger.Bytes(reqStr.length),
    logger.String("model", cwReq.conversationState.currentMessage.userInputMessage.modelId),
  );

  // 调试工具信息
  if (Deno.env.get("DEBUG_TOOLS") === "true") {
    logger.debug("完整请求", logger.LazyJson("request", cwReq));
  }

  try {
    const response = await fetch(AWS_ENDPOINTS.CODEWHISPERER, {
      method: "POST",
      headers: createCodeWhispererHeaders(accessToken),
      body: reqStr,
    });

    const latency = Date.now() - startTime;

    if (!response.ok) {
      const errorText = await response.text();
      
      errorTracker.track(
        ErrorCategory.UPSTREAM_ERROR,
        `CodeWhisperer API 错误: ${response.status}`,
        new Error(errorText),
        requestId,
        { statusCode: response.status, latency },
      );
      
      throw new Error(`CodeWhisperer API error: ${response.status}`);
    }

    logger.debug(
      "CodeWhisperer 响应成功",
      logger.String("request_id", requestId),
      logger.HttpStatus(response.status),
      logger.Latency(latency),
    );

    return response;
  } catch (error) {
    const latency = Date.now() - startTime;
    
    if (error instanceof Error && !error.message.includes("CodeWhisperer API error")) {
      errorTracker.track(
        ErrorCategory.UPSTREAM_TIMEOUT,
        "CodeWhisperer 请求异常",
        error,
        requestId,
        { latency },
      );
    }
    
    throw error;
  }
}
