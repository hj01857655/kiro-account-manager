/**
 * Token 计算辅助函数
 */

import type { AnthropicRequest } from "../types/anthropic.ts";
import { TokenEstimator } from "./token_estimator.ts";

/**
 * 计算请求的输入 tokens
 */
export function calculateInputTokens(anthropicReq: AnthropicRequest): number {
  const estimator = new TokenEstimator();
  const systemMessages = anthropicReq.system
    ? typeof anthropicReq.system === "string"
      ? [{ text: anthropicReq.system }]
      : anthropicReq.system.map(s => ({ text: typeof s === "string" ? s : s.text }))
    : undefined;
  
  return estimator.estimateTokens({
    system: systemMessages,
    messages: anthropicReq.messages,
    tools: anthropicReq.tools,
  });
}

/**
 * 计算输出 tokens（基于内容块）
 */
export function calculateOutputTokens(
  contentBlocks: Array<{ type: string; text?: string; name?: string; input?: unknown }>,
  estimator?: TokenEstimator
): number {
  const est = estimator || new TokenEstimator();
  let outputTokens = 0;

  for (const contentBlock of contentBlocks) {
    switch (contentBlock.type) {
      case "text":
        if (contentBlock.text) {
          outputTokens += est.estimateTextTokens(contentBlock.text);
        }
        break;

      case "tool_use":
        if (contentBlock.name) {
          outputTokens += est.estimateToolUseTokens(
            contentBlock.name,
            contentBlock.input as Record<string, unknown> || {},
          );
        }
        break;
    }
  }

  return Math.max(1, outputTokens);
}
