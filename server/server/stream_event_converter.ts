/**
 * 流事件转换器
 * 负责将 CodeWhisperer 事件转换为 Anthropic SSE 格式
 */

export class StreamEventConverter {
  private textBlockStarted = false;
  private readonly toolUseIdByBlockIndex = new Map<number, string>();

  /**
   * 将 CodeWhisperer 事件转换为 Anthropic SSE 事件
   */
  convertToAnthropicEvents(event: Record<string, unknown>): Array<Record<string, unknown>> {
    const events: Array<Record<string, unknown>> = [];
    
    // 处理文本内容
    if (event.content && typeof event.content === "string") {
      this.addTextEvents(events, event.content as string);
    }
    
    // 处理工具调用
    if (event.toolUseId && event.name) {
      this.addToolEvents(events, event);
    }
    
    return events;
  }

  /**
   * 添加文本事件
   */
  private addTextEvents(events: Array<Record<string, unknown>>, content: string): void {
    // 如果文本块还没开始，先发送 content_block_start
    if (!this.textBlockStarted) {
      events.push({
        type: "content_block_start",
        index: 0,
        content_block: {
          type: "text",
          text: "",
        },
      });
      this.textBlockStarted = true;
    }
    
    events.push({
      type: "content_block_delta",
      index: 0,
      delta: {
        type: "text_delta",
        text: content,
      },
    });
  }

  /**
   * 添加工具调用事件
   */
  private addToolEvents(events: Array<Record<string, unknown>>, event: Record<string, unknown>): void {
    const toolUseId = event.toolUseId as string;
    const toolName = event.name as string;
    
    // 获取或分配块索引
    let blockIndex = this.findToolBlockIndex(toolUseId);
    
    // 如果是新工具，生成 content_block_start
    if (blockIndex === -1) {
      blockIndex = this.toolUseIdByBlockIndex.size + 1;
      this.toolUseIdByBlockIndex.set(blockIndex, toolUseId);
      
      events.push({
        type: "content_block_start",
        index: blockIndex,
        content_block: {
          type: "tool_use",
          id: toolUseId,
          name: toolName,
          input: {},
        },
      });
    }
    
    // 处理工具参数增量
    if (event.input && typeof event.input === "string") {
      events.push({
        type: "content_block_delta",
        index: blockIndex,
        delta: {
          type: "input_json_delta",
          partial_json: event.input,
        },
      });
    }
    
    // 处理工具结束
    if (event.stop) {
      events.push({
        type: "content_block_stop",
        index: blockIndex,
      });
    }
  }

  /**
   * 查找工具块索引
   */
  private findToolBlockIndex(toolUseId: string): number {
    for (const [index, id] of this.toolUseIdByBlockIndex.entries()) {
      if (id === toolUseId) {
        return index;
      }
    }
    return -1;
  }

  /**
   * 获取工具映射（用于外部追踪）
   */
  getToolUseIdByBlockIndex(): Map<number, string> {
    return this.toolUseIdByBlockIndex;
  }

  /**
   * 检查文本块是否已启动
   */
  isTextBlockStarted(): boolean {
    return this.textBlockStarted;
  }

  /**
   * 重置状态
   */
  reset(): void {
    this.textBlockStarted = false;
    this.toolUseIdByBlockIndex.clear();
  }
}