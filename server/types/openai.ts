import type { Usage } from "./common.ts";

// OpenAI tool call structure
export interface OpenAIToolCall {
  id: string;
  type: string;
  function: {
    name: string;
    arguments: string;
  };
}

// OpenAI message structure
export interface OpenAIMessage {
  role: string;
  content: unknown; // can be string or ContentBlock[]
  tool_calls?: OpenAIToolCall[];
}

// OpenAI function definition
export interface OpenAIFunction {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  strict?: boolean;
}

// OpenAI tool definition
export interface OpenAITool {
  type: string;
  function: OpenAIFunction;
}

// OpenAI tool choice
export interface OpenAIToolChoice {
  type: string;
  function?: {
    name: string;
  };
}

// OpenAI API request structure
export interface OpenAIRequest {
  model: string;
  messages: OpenAIMessage[];
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
  tools?: OpenAITool[];
  tool_choice?: string | OpenAIToolChoice;
}

// OpenAI choice structure
export interface OpenAIChoice {
  index: number;
  message: OpenAIMessage;
  finish_reason: string;
}

// OpenAI response structure
export interface OpenAIResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: OpenAIChoice[];
  usage: Usage;
}
