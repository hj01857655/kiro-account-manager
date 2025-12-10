export interface ToolCall {
  id: string;
  type: "function";
  function: {
    name: string;
    arguments: string;
  };
}

export interface ToolCallResult {
  toolCallId: string;
  result: unknown;
}

export interface ToolCallError {
  toolCallId: string;
  error: string;
}

export interface SSEEvent {
  event: string;
  data: unknown;
}

export enum ToolStatus {
  Pending,
  Running,
  Completed,
  Error,
}

export interface ToolExecution {
  id: string;
  name: string;
  startTime: Date;
  endTime?: Date;
  status: ToolStatus;
  arguments: Record<string, unknown>;
  result?: unknown;
  error?: string;
  blockIndex: number;
}
