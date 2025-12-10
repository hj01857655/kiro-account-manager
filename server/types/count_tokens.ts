import type { AnthropicRequestMessage, AnthropicSystemMessage, AnthropicTool } from "./anthropic.ts";

// Count tokens request structure following Anthropic API spec
// Reference: https://docs.anthropic.com/en/api/messages-count-tokens
export interface CountTokensRequest {
  model: string;
  messages: AnthropicRequestMessage[];
  system?: AnthropicSystemMessage[];
  tools?: AnthropicTool[];
}

// Count tokens response structure following Anthropic API spec
export interface CountTokensResponse {
  input_tokens: number;
}
