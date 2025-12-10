import type { ContentBlock, Usage } from "./common.ts";

// Anthropic tool structure
export interface AnthropicTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

// Tool choice strategy
export interface ToolChoice {
  type: string; // "auto", "any", "tool"
  name?: string; // specified tool name when type is "tool"
}

// Anthropic system message
export interface AnthropicSystemMessage {
  type: string;
  text: string;
}

// Anthropic request message
export interface AnthropicRequestMessage {
  role: string;
  content: string | ContentBlock[];
}

// Anthropic API request structure
export interface AnthropicRequest {
  model: string;
  max_tokens: number;
  messages: AnthropicRequestMessage[];
  system?: AnthropicSystemMessage[];
  tools?: AnthropicTool[];
  tool_choice?: string | ToolChoice;
  stream: boolean;
  temperature?: number;
  metadata?: Record<string, unknown>;
}

// Anthropic stream response
export interface AnthropicStreamResponse {
  type: string;
  index: number;
  delta?: {
    text: string;
    type: string;
  };
  content?: Array<{
    text: string;
    type: string;
  }>;
  stop_reason?: string;
  stop_sequence?: string;
  usage?: Usage;
}
