import type { ToolCall, ToolCallError, ToolCallResult } from "./event_stream_types.ts";
import { SSEEvent, ToolExecution, ToolStatus } from "./event_stream_types.ts";
/**
 * Manages the lifecycle of tool calls within a streaming session, mirroring the logic
 * from the Go implementation. It is responsible for tracking tool execution states
 * (pending, running, completed, errored) and generating Anthropic-compliant
 * Server-Sent Events (SSE) for each stage of the tool call.
 */
export class ToolLifecycleManager {
  private activeTools: Map<string, ToolExecution> = new Map();
  private completedTools: Map<string, ToolExecution> = new Map();
  private blockIndexMap: Map<string, number> = new Map();
  private nextBlockIndex = 1;
  private textIntroGenerated = false;

  public reset(): void {
    this.activeTools.clear();
    this.completedTools.clear();
    this.blockIndexMap.clear();
    this.nextBlockIndex = 1;
    this.textIntroGenerated = false;
  }

  public getActiveTools(): Map<string, ToolExecution> {
    return new Map(this.activeTools);
  }

  public getCompletedTools(): Map<string, ToolExecution> {
    return new Map(this.completedTools);
  }

  public getBlockIndex(toolId: string): number {
    return this.blockIndexMap.get(toolId) ?? -1;
  }

  public updateToolArguments(toolId: string, args: Record<string, unknown>): void {
    const execution = this.activeTools.get(toolId) || this.completedTools.get(toolId);
    if (execution) {
      execution.arguments = args;
    }
  }

  public generateToolSummary(): Record<string, unknown> {
    const activeCount = this.activeTools.size;
    const completedCount = this.completedTools.size;
    let errorCount = 0;
    let totalExecutionTime = 0;

    for (const tool of this.completedTools.values()) {
      if (tool.status === ToolStatus.Error) {
        errorCount++;
      }
      if (tool.endTime) {
        totalExecutionTime += tool.endTime.getTime() - tool.startTime.getTime();
      }
    }

    return {
      active_tools: activeCount,
      completed_tools: completedCount,
      error_tools: errorCount,
      total_execution_time: totalExecutionTime,
      success_rate: completedCount > 0 ? (completedCount - errorCount) / completedCount : 0,
    };
  }

  public handleToolCallRequest(request: { toolCalls: ToolCall[] }): SSEEvent[] {
    const events: SSEEvent[] = [];

    if (!this.textIntroGenerated && request.toolCalls.length > 0) {
      const textIntroEvents = this.generateTextIntroduction(request.toolCalls[0]);
      events.push(...textIntroEvents);
      this.textIntroGenerated = true;
    }

    for (const toolCall of request.toolCalls) {
      if (this.activeTools.has(toolCall.id)) {
        const existing = this.activeTools.get(toolCall.id)!;
        try {
          const argumentsParsed = JSON.parse(toolCall.function.arguments);
          if (Object.keys(argumentsParsed).length > 0) {
            existing.arguments = argumentsParsed;
          }
        } catch {
          // Ignore parsing errors
        }
        continue;
      }

      let argumentsParsed: Record<string, unknown>;
      try {
        argumentsParsed = JSON.parse(toolCall.function.arguments);
      } catch {
        argumentsParsed = {};
      }

      const execution: ToolExecution = {
        id: toolCall.id,
        name: toolCall.function.name,
        startTime: new Date(),
        status: ToolStatus.Pending,
        arguments: argumentsParsed,
        blockIndex: this.getOrAssignBlockIndex(toolCall.id),
      };

      this.activeTools.set(toolCall.id, execution);

      events.push({
        event: "content_block_start",
        data: {
          type: "content_block_start",
          index: execution.blockIndex,
          content_block: {
            type: "tool_use",
            id: toolCall.id,
            name: toolCall.function.name,
            input: argumentsParsed,
          },
        },
      });

      if (Object.keys(argumentsParsed).length > 0) {
        events.push({
          event: "content_block_delta",
          data: {
            type: "content_block_delta",
            index: execution.blockIndex,
            delta: {
              type: "input_json_delta",
              partial_json: JSON.stringify(argumentsParsed),
            },
          },
        });
      }
      execution.status = ToolStatus.Running;
    }
    return events;
  }

  public handleToolCallResult(result: ToolCallResult): SSEEvent[] {
    const execution = this.activeTools.get(result.toolCallId);
    if (!execution) {
      return [];
    }

    const now = new Date();
    execution.endTime = now;
    execution.result = result.result;
    execution.status = ToolStatus.Completed;

    this.completedTools.set(result.toolCallId, execution);
    this.activeTools.delete(result.toolCallId);

    return [{
      event: "content_block_stop",
      data: {
        type: "content_block_stop",
        index: execution.blockIndex,
      },
    }];
  }

  public handleToolCallError(errorInfo: ToolCallError): SSEEvent[] {
    const execution = this.activeTools.get(errorInfo.toolCallId);
    if (!execution) {
      return [];
    }

    const now = new Date();
    execution.endTime = now;
    execution.error = errorInfo.error;
    execution.status = ToolStatus.Error;

    this.completedTools.set(errorInfo.toolCallId, execution);
    this.activeTools.delete(errorInfo.toolCallId);

    return [
      {
        event: "error",
        data: {
          type: "error",
          error: {
            type: "tool_error",
            message: errorInfo.error,
            tool_call_id: errorInfo.toolCallId,
          },
        },
      },
      {
        event: "content_block_stop",
        data: {
          type: "content_block_stop",
          index: execution.blockIndex,
        },
      },
    ];
  }

  private getOrAssignBlockIndex(toolID: string): number {
    if (this.blockIndexMap.has(toolID)) {
      return this.blockIndexMap.get(toolID)!;
    }
    const index = this.nextBlockIndex++;
    this.blockIndexMap.set(toolID, index);
    return index;
  }

  private generateTextIntroduction(firstTool: ToolCall): SSEEvent[] {
    const introText = this.generateIntroText(firstTool.function.name);
    if (!introText) {
      return [];
    }

    return [{
      event: "content_block_delta",
      data: {
        type: "content_block_delta",
        index: 0,
        delta: {
          type: "text_delta",
          text: introText,
        },
      },
    }];
  }

  private generateIntroText(toolName: string): string {
    // This can be expanded to be more intelligent
    return `I will use the ${toolName} tool.`;
  }
}
