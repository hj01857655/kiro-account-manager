/**
 * 流 Token 管理器
 * 负责累计和估算流式响应中的 token 数量
 */

import { TokenEstimator } from "../utils/token_estimator.ts";

export class StreamTokenManager {
  private totalOutputTokens = 0;
  private readonly tokenEstimator: TokenEstimator;

  constructor() {
    this.tokenEstimator = new TokenEstimator();
  }

  /**
   * 累计 token 数
   */
  accumulateTokens(dataMap: Record<string, unknown>): void {
    const eventType = dataMap.type as string;

    switch (eventType) {
      case "content_block_delta":
        this.accumulateContentDelta(dataMap);
        break;

      case "content_block_start":
        this.accumulateContentStart(dataMap);
        break;
    }
  }

  /**
   * 累计内容增量的 token
   */
  private accumulateContentDelta(dataMap: Record<string, unknown>): void {
    const delta = dataMap.delta as Record<string, unknown>;
    if (!delta) return;

    const deltaType = delta.type as string;
    if (deltaType === "text_delta" && delta.text) {
      // 文本内容增量
      this.totalOutputTokens += this.tokenEstimator.estimateTextTokens(
        delta.text as string,
      );
    } else if (deltaType === "input_json_delta" && delta.partial_json) {
      // 工具调用参数JSON增量
      const jsonText = delta.partial_json as string;
      this.totalOutputTokens += Math.ceil(jsonText.length / 4);
    }
  }

  /**
   * 累计内容开始的 token
   */
  private accumulateContentStart(dataMap: Record<string, unknown>): void {
    const contentBlock = dataMap.content_block as Record<string, unknown>;
    if (!contentBlock) return;

    const blockType = contentBlock.type as string;
    if (blockType === "tool_use") {
      // 工具调用结构开销：12 tokens (type+id+name)
      this.totalOutputTokens += 12;
      
      // 工具名称token
      const toolName = contentBlock.name as string;
      if (toolName) {
        this.totalOutputTokens += this.tokenEstimator.estimateTextTokens(toolName);
      }
    }
  }

  /**
   * 获取总输出 token 数
   */
  getTotalOutputTokens(): number {
    return this.totalOutputTokens;
  }

  /**
   * 重置 token 计数
   */
  reset(): void {
    this.totalOutputTokens = 0;
  }
}