import * as logger from "../logger/logger.ts";

// 内容块状态
interface BlockState {
  index: number;
  type: "text" | "tool_use";
  started: boolean;
  stopped: boolean;
  toolUseId?: string;
}

// SSE状态管理器
export class SSEStateManager {
  private messageStarted = false;
  private messageDeltaSent = false;
  private messageEnded = false;
  private activeBlocks = new Map<number, BlockState>();
  private nextBlockIndex = 0;
  private strictMode: boolean;

  constructor(strictMode = false) {
    this.strictMode = strictMode;
  }

  reset(): void {
    this.messageStarted = false;
    this.messageDeltaSent = false;
    this.messageEnded = false;
    this.activeBlocks.clear();
    this.nextBlockIndex = 0;
  }

  // 验证并发送事件
  validateAndSend(eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    const eventType = eventData.type as string;

    switch (eventType) {
      case "message_start":
        return this.handleMessageStart(eventData);
      case "content_block_start":
        return this.handleContentBlockStart(eventData);
      case "content_block_delta":
        return this.handleContentBlockDelta(eventData);
      case "content_block_stop":
        return this.handleContentBlockStop(eventData);
      case "message_delta":
        return this.handleMessageDelta(eventData);
      case "message_stop":
        return this.handleMessageStop(eventData);
      default:
        return { valid: true };
    }
  }

  private handleMessageStart(_eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    if (this.messageStarted) {
      const errMsg = "违规：message_start只能出现一次";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }
    this.messageStarted = true;
    return { valid: true };
  }

  private handleContentBlockStart(eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    if (!this.messageStarted) {
      const errMsg = "违规：content_block_start必须在message_start之后";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
    }

    if (this.messageEnded) {
      const errMsg = "违规：message已结束，不能发送content_block_start";
      logger.error(errMsg);
      return { valid: false };
    }

    const index = eventData.index as number ?? this.nextBlockIndex;
    const block = this.activeBlocks.get(index);

    if (block?.started && !block.stopped) {
      const errMsg = `违规：索引${index}的content_block已经started但未stopped`;
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    const contentBlock = eventData.content_block as Record<string, unknown>;
    const blockType = contentBlock?.type as string ?? "text";

    // 工具块启动前自动关闭文本块
    if (blockType === "tool_use") {
      for (const [blockIndex, block] of this.activeBlocks.entries()) {
        if (block.type === "text" && block.started && !block.stopped) {
          logger.debug("工具块启动前自动关闭文本块", logger.Int("text_block_index", blockIndex));
          block.stopped = true;
        }
      }
    }

    const toolUseId = blockType === "tool_use" ? contentBlock?.id as string : undefined;

    this.activeBlocks.set(index, {
      index,
      type: blockType as "text" | "tool_use",
      started: true,
      stopped: false,
      toolUseId,
    });

    if (index >= this.nextBlockIndex) {
      this.nextBlockIndex = index + 1;
    }

    return { valid: true };
  }

  private handleContentBlockDelta(eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    const index = eventData.index as number;
    if (index === undefined) {
      const errMsg = "content_block_delta缺少有效索引";
      logger.error(errMsg);
      return { valid: false };
    }

    let block = this.activeBlocks.get(index);

    // 自动启动未启动的块
    if (!block?.started) {
      logger.debug("检测到content_block_delta但块未启动，自动生成content_block_start");
      
      const delta = eventData.delta as Record<string, unknown>;
      const blockType = delta?.type === "input_json_delta" ? "tool_use" : "text";

      this.activeBlocks.set(index, {
        index,
        type: blockType,
        started: true,
        stopped: false,
      });

      block = this.activeBlocks.get(index);
    }

    if (block?.stopped) {
      const errMsg = `违规：索引${index}的content_block已停止，不能发送delta`;
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    return { valid: true };
  }

  private handleContentBlockStop(eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    const index = eventData.index as number;
    if (index === undefined) {
      const errMsg = "content_block_stop缺少有效索引";
      logger.error(errMsg);
      return { valid: false };
    }

    const block = this.activeBlocks.get(index);

    if (!block?.started) {
      const errMsg = `违规：索引${index}的content_block未启动就发送stop`;
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    if (block.stopped) {
      const errMsg = `违规：索引${index}的content_block重复停止`;
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    block.stopped = true;
    return { valid: true };
  }

  private handleMessageDelta(_eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    if (!this.messageStarted) {
      const errMsg = "违规：message_delta必须在message_start之后";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
    }

    if (this.messageDeltaSent) {
      const errMsg = "违规：message_delta只能出现一次";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    // 自动关闭未关闭的块
    const unclosedBlocks: number[] = [];
    for (const [index, block] of this.activeBlocks.entries()) {
      if (block.started && !block.stopped) {
        unclosedBlocks.push(index);
      }
    }

    if (unclosedBlocks.length > 0 && !this.strictMode) {
      logger.debug("message_delta前自动关闭未关闭的content_block");
      for (const index of unclosedBlocks) {
        const block = this.activeBlocks.get(index);
        if (block) {
          block.stopped = true;
        }
      }
    }

    this.messageDeltaSent = true;
    return { valid: true };
  }

  private handleMessageStop(_eventData: Record<string, unknown>): { valid: boolean; error?: string } {
    if (!this.messageStarted) {
      const errMsg = "违规：message_stop必须在message_start之后";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
    }

    if (this.messageEnded) {
      const errMsg = "违规：message_stop只能出现一次";
      logger.error(errMsg);
      if (this.strictMode) {
        return { valid: false, error: errMsg };
      }
      return { valid: false };
    }

    this.messageEnded = true;
    return { valid: true };
  }

  getActiveBlocks(): Map<number, BlockState> {
    return this.activeBlocks;
  }

  isMessageStarted(): boolean {
    return this.messageStarted;
  }

  isMessageEnded(): boolean {
    return this.messageEnded;
  }

  isMessageDeltaSent(): boolean {
    return this.messageDeltaSent;
  }
}
