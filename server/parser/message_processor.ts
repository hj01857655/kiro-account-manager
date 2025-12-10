import { SSEEvent, ToolCall } from "./event_stream_types.ts";
import { ToolLifecycleManager } from "./tool_lifecycle_manager.ts";
import { SessionManager } from "./session_manager.ts";
import { SonicStreamingJSONAggregator } from "./sonic_streaming_aggregator.ts";

interface EventStreamMessage {
  headers: Record<string, unknown>;
  payload: Uint8Array;
  messageType: string;
  eventType: string;
  contentType: string;
}

const MessageTypes = {
  EVENT: "event",
  ERROR: "error",
  EXCEPTION: "exception",
};

const EventTypes = {
  COMPLETION: "completion",
  COMPLETION_CHUNK: "completion_chunk",
  TOOL_CALL_REQUEST: "tool_call_request",
  TOOL_CALL_ERROR: "tool_call_error",
  SESSION_START: "session_start",
  SESSION_END: "session_end",
  ASSISTANT_RESPONSE_EVENT: "assistantResponseEvent",
  TOOL_USE_EVENT: "toolUseEvent",
};

export class MessageProcessor {
  private sessionManager: SessionManager;
  private toolManager: ToolLifecycleManager;
  private completionBuffer: string[] = [];
  private toolDataAggregator: SonicStreamingJSONAggregator;

  constructor() {
    this.sessionManager = new SessionManager();
    this.toolManager = new ToolLifecycleManager();
    
    this.toolDataAggregator = new SonicStreamingJSONAggregator(
      (toolUseId: string, fullParams: string) => {
        try {
          const args = JSON.parse(fullParams);
          this.toolManager.updateToolArguments(toolUseId, args);
        } catch {
          // Ignore parse errors
        }
      }
    );
  }

  processMessage(message: EventStreamMessage): SSEEvent[] {
    const { messageType, eventType } = message;

    switch (messageType) {
      case MessageTypes.EVENT:
        return this.processEventMessage(message, eventType);
      case MessageTypes.ERROR:
        return this.processErrorMessage(message);
      case MessageTypes.EXCEPTION:
        return this.processExceptionMessage(message);
      default:
        return [];
    }
  }

  private processEventMessage(message: EventStreamMessage, eventType: string): SSEEvent[] {
    switch (eventType) {
      case EventTypes.COMPLETION:
        return this.handleCompletion(message);
      case EventTypes.COMPLETION_CHUNK:
        return this.handleCompletionChunk(message);
      case EventTypes.TOOL_CALL_REQUEST:
        return this.handleToolCallRequest(message);
      case EventTypes.TOOL_CALL_ERROR:
        return this.handleToolCallError(message);
      case EventTypes.SESSION_START:
        return this.handleSessionStart(message);
      case EventTypes.SESSION_END:
        return this.handleSessionEnd(message);
      case EventTypes.ASSISTANT_RESPONSE_EVENT:
        return this.handleAssistantResponse(message);
      case EventTypes.TOOL_USE_EVENT:
        return this.handleToolUseEvent(message);
      default:
        return [];
    }
  }

  private handleCompletion(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    return [{
      event: "completion",
      data: {
        type: "completion",
        content: data.content || "",
        finish_reason: data.finish_reason || "",
        tool_calls: data.tool_calls || [],
        raw_data: data,
      },
    }];
  }

  private handleCompletionChunk(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    const content = data.content || data.delta || "";
    
    this.completionBuffer.push(content);

    return [{
      event: "content_block_delta",
      data: {
        type: "content_block_delta",
        index: 0,
        delta: {
          type: "text_delta",
          text: content,
        },
      },
    }];
  }

  private handleToolCallRequest(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    const toolCall: ToolCall = {
      id: data.toolCallId || data.tool_call_id || "",
      type: "function",
      function: {
        name: data.toolName || data.name || "",
        arguments: JSON.stringify(data.input || {}),
      },
    };

    return this.toolManager.handleToolCallRequest({ toolCalls: [toolCall] });
  }

  private handleToolCallError(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    return this.toolManager.handleToolCallError({
      toolCallId: data.tool_call_id || data.toolCallId || "",
      error: data.error || "Unknown error",
    });
  }

  private handleSessionStart(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    const sessionId = data.sessionId || data.session_id;
    
    if (sessionId) {
      this.sessionManager.setSessionId(sessionId);
      this.sessionManager.startSession();
    }

    return [{
      event: EventTypes.SESSION_START,
      data,
    }];
  }

  private handleSessionEnd(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    const endEvents = this.sessionManager.endSession();

    return [
      {
        event: EventTypes.SESSION_END,
        data,
      },
      ...endEvents,
    ];
  }

  private handleAssistantResponse(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    
    if (this.isToolCallEvent(data)) {
      return this.handleToolUseEvent(message);
    }

    if (data.content) {
      return [{
        event: "content_block_delta",
        data: {
          type: "content_block_delta",
          index: 0,
          delta: {
            type: "text_delta",
            text: data.content,
          },
        },
      }];
    }

    return [];
  }

  private handleToolUseEvent(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    const toolUseId = data.toolUseId || data.tool_use_id || "";
    const name = data.name || "";
    const input = typeof data.input === "string" ? data.input : JSON.stringify(data.input || {});
    const stop = data.stop || false;

    const activeTools = this.toolManager.getActiveTools();
    const toolExists = activeTools.has(toolUseId);

    if (!toolExists) {
      const toolCall: ToolCall = {
        id: toolUseId,
        type: "function",
        function: {
          name,
          arguments: input,
        },
      };

      const events = this.toolManager.handleToolCallRequest({ toolCalls: [toolCall] });

      if (stop) {
        return events;
      }

      return events;
    }

    if (stop) {
      const { complete, fullInput } = this.toolDataAggregator.processToolData(
        toolUseId,
        name,
        "",
        stop,
        -1
      );

      if (complete && fullInput && fullInput !== "{}") {
        try {
          const args = JSON.parse(fullInput);
          this.toolManager.updateToolArguments(toolUseId, args);
        } catch {
          // Ignore parse errors
        }
      }

      return this.toolManager.handleToolCallResult({
        toolCallId: toolUseId,
        result: "Tool execution completed",
      });
    }

    if (input && input !== "{}") {
      this.toolDataAggregator.processToolData(toolUseId, name, input, stop, -1);
      
      const blockIndex = this.toolManager.getBlockIndex(toolUseId);
      if (blockIndex >= 0) {
        return [{
          event: "content_block_delta",
          data: {
            type: "content_block_delta",
            index: blockIndex,
            delta: {
              type: "input_json_delta",
              partial_json: input,
            },
          },
        }];
      }
    }

    return [];
  }

  private processErrorMessage(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    return [{
      event: "error",
      data: {
        type: "error",
        error_code: data.__type || "",
        error_message: data.message || "",
        raw_data: data,
      },
    }];
  }

  private processExceptionMessage(message: EventStreamMessage): SSEEvent[] {
    const data = this.parsePayload(message.payload);
    return [{
      event: "exception",
      data: {
        type: "exception",
        exception_type: data.__type || "",
        exception_message: data.message || "",
        raw_data: data,
      },
    }];
  }

  private parsePayload(payload: Uint8Array): Record<string, unknown> {
    try {
      const text = new TextDecoder().decode(payload);
      return JSON.parse(text);
    } catch {
      return {};
    }
  }

  private isToolCallEvent(data: Record<string, unknown>): boolean {
    return !!(data.toolUseId || data.tool_use_id || (data.name && data.input));
  }

  getSessionManager(): SessionManager {
    return this.sessionManager;
  }

  getToolManager(): ToolLifecycleManager {
    return this.toolManager;
  }

  getCompletionBuffer(): string {
    return this.completionBuffer.join("");
  }

  reset(): void {
    this.sessionManager.reset();
    this.toolManager.reset();
    this.completionBuffer = [];
  }
}
