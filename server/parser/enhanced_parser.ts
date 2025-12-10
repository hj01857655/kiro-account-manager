import { SSEEvent, ToolExecution } from "./event_stream_types.ts";
import { RobustEventStreamParser } from "./robust_parser.ts";
import { MessageProcessor } from "./message_processor.ts";
import { SessionInfo } from "./session_manager.ts";

export interface ParseResult {
  messages: Array<{ headers: Record<string, unknown>; payload: Uint8Array; messageType: string; eventType: string; contentType: string }>;
  events: SSEEvent[];
  toolExecutions: Map<string, ToolExecution>;
  activeTools: Map<string, ToolExecution>;
  sessionInfo: SessionInfo;
  summary: ParseSummary;
  errors: Error[];
}

export interface ParseSummary {
  totalMessages: number;
  totalEvents: number;
  messageTypes: Record<string, number>;
  eventTypes: Record<string, number>;
  hasToolCalls: boolean;
  hasCompletions: boolean;
  hasErrors: boolean;
  hasSessionEvents: boolean;
  toolSummary: Record<string, unknown>;
}

export class EnhancedEventStreamParser {
  private robustParser: RobustEventStreamParser;
  private messageProcessor: MessageProcessor;

  constructor() {
    this.robustParser = new RobustEventStreamParser();
    this.messageProcessor = new MessageProcessor();
  }

  setMaxErrors(maxErrors: number): void {
    this.robustParser.setMaxErrors(maxErrors);
  }

  reset(): void {
    this.robustParser.reset();
    this.messageProcessor.reset();
  }

  parseResponse(streamData: Uint8Array): ParseResult {
    const messages = this.robustParser.parseStream(streamData);
    const allEvents: SSEEvent[] = [];
    const errors: Error[] = [];

    for (let i = 0; i < messages.length; i++) {
      const message = messages[i];
      try {
        const events = this.messageProcessor.processMessage(message);
        allEvents.push(...events);
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        console.warn(`Failed to process message ${i}:`, {
          messageType: message.messageType,
          eventType: message.eventType,
          error: error.message,
        });
        errors.push(error);
      }
    }

    const toolManager = this.messageProcessor.getToolManager();
    const sessionManager = this.messageProcessor.getSessionManager();

    const result: ParseResult = {
      messages,
      events: allEvents,
      toolExecutions: toolManager.getCompletedTools(),
      activeTools: toolManager.getActiveTools(),
      sessionInfo: sessionManager.getSessionInfo(),
      summary: this.generateSummary(messages, allEvents, toolManager),
      errors,
    };

    if (errors.length > 0) {
      console.debug(`Parsing completed with errors:`, {
        successMessages: messages.length,
        totalEvents: allEvents.length,
        errorCount: errors.length,
      });
    }

    return result;
  }

  parseStream(data: Uint8Array): SSEEvent[] {
    const messages = this.robustParser.parseStream(data);
    const allEvents: SSEEvent[] = [];

    for (const message of messages) {
      try {
        const events = this.messageProcessor.processMessage(message);
        allEvents.push(...events);
      } catch (err) {
        console.warn(`Streaming message processing failed:`, err);
        // Continue processing other messages
      }
    }

    return allEvents;
  }

  private generateSummary(
    messages: Array<{ headers: Record<string, unknown>; payload: Uint8Array; messageType: string; eventType: string; contentType: string }>,
    events: SSEEvent[],
    toolManager: { generateToolSummary: () => Record<string, unknown> }
  ): ParseSummary {
    const summary: ParseSummary = {
      totalMessages: messages.length,
      totalEvents: events.length,
      messageTypes: {},
      eventTypes: {},
      hasToolCalls: false,
      hasCompletions: false,
      hasErrors: false,
      hasSessionEvents: false,
      toolSummary: {},
    };

    // 统计消息类型 - 更详细的分类
    for (const message of messages) {
      const msgType = message.messageType;
      summary.messageTypes[msgType] = (summary.messageTypes[msgType] || 0) + 1;

      // 检测错误消息
      if (msgType === "error" || msgType === "exception") {
        summary.hasErrors = true;
      }

      const eventType = message.eventType;
      if (eventType) {
        summary.eventTypes[eventType] = (summary.eventTypes[eventType] || 0) + 1;

        // 更精确的事件类型检测（参考 Go 版本）
        switch (eventType) {
          case "toolCallRequest":
          case "toolCallError":
          case "tool_call_request":
          case "tool_call_error":
            summary.hasToolCalls = true;
            break;
          case "completion":
          case "completionChunk":
          case "completion_chunk":
            summary.hasCompletions = true;
            break;
          case "sessionStart":
          case "sessionEnd":
          case "session_start":
          case "session_end":
            summary.hasSessionEvents = true;
            break;
          case "assistantResponseEvent":
          case "assistant_response_event":
            // 旧格式的助手响应事件也算作补全内容
            summary.hasCompletions = true;
            break;
        }

        // 模糊匹配（保持兼容性）
        if (eventType.toLowerCase().includes("tool")) {
          summary.hasToolCalls = true;
        }
        if (eventType.toLowerCase().includes("completion")) {
          summary.hasCompletions = true;
        }
        if (eventType.toLowerCase().includes("session")) {
          summary.hasSessionEvents = true;
        }
      }
    }

    // 统计事件类型 - 包括内容块检测
    for (const event of events) {
      const eventName = event.event;
      summary.eventTypes[eventName] = (summary.eventTypes[eventName] || 0) + 1;

      // 检测工具调用相关的内容块
      if (eventName === "content_block_start" || 
          eventName === "content_block_stop" ||
          eventName === "content_block_delta") {
        const data = event.data as Record<string, unknown>;
        const contentBlock = data?.content_block as { type?: string } | undefined;
        if (contentBlock?.type === "tool_use") {
          summary.hasToolCalls = true;
        }
        // 检测 delta 中的工具调用
        const delta = data?.delta as { type?: string } | undefined;
        if (delta?.type === "tool_use") {
          summary.hasToolCalls = true;
        }
      }
    }

    // 获取工具执行统计
    summary.toolSummary = toolManager.generateToolSummary();

    return summary;
  }

  getToolManager() {
    return this.messageProcessor.getToolManager();
  }

  getCompletionText(result: ParseResult): string {
    let text = "";
    for (const event of result.events) {
      if (event.event === "content_block_delta") {
        const data = event.data as Record<string, unknown>;
        const delta = data?.delta as { text?: string } | undefined;
        if (delta?.text) {
          text += delta.text;
        }
      }
    }
    return text;
  }

  getToolCalls(result: ParseResult): ToolExecution[] {
    return [
      ...Array.from(result.toolExecutions.values()),
      ...Array.from(result.activeTools.values()),
    ];
  }
}
