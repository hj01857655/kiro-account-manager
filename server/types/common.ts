// Usage statistics structure
export interface Usage {
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  // Anthropic format compatibility
  input_tokens?: number;
  output_tokens?: number;
}

// Convert usage to Anthropic format
export function toAnthropicFormat(usage: Usage): Usage {
  return {
    input_tokens: usage.prompt_tokens,
    output_tokens: usage.completion_tokens,
  };
}

// Convert usage to OpenAI format
export function toOpenAIFormat(usage: Usage): Usage {
  let total = (usage.prompt_tokens || 0) + (usage.completion_tokens || 0);
  if (total === 0) {
    total = (usage.input_tokens || 0) + (usage.output_tokens || 0);
  }
  return {
    prompt_tokens: (usage.prompt_tokens || 0) + (usage.input_tokens || 0),
    completion_tokens: (usage.completion_tokens || 0) + (usage.output_tokens || 0),
    total_tokens: total,
  };
}

// Model not found error structure
export interface ModelNotFoundErrorDetail {
  code: string;
  message: string;
  type: string;
}

export interface ModelNotFoundError {
  error: ModelNotFoundErrorDetail;
}

// Create model not found error
export function createModelNotFoundError(
  model: string,
  requestId: string
): ModelNotFoundError {
  return {
    error: {
      code: "model_not_found",
      message: `分组 default 下模型 ${model} 无可用渠道（distributor） (request id: ${requestId})`,
      type: "new_api_error",
    },
  };
}

// Model not found error class
export class ModelNotFoundErrorType extends Error {
  errorData: ModelNotFoundError;

  constructor(model: string, requestId: string) {
    const errorData = createModelNotFoundError(model, requestId);
    super(`model not found: ${errorData.error.message}`);
    this.name = "ModelNotFoundErrorType";
    this.errorData = errorData;
  }
}

// Image source structure
export interface ImageSource {
  type: string;
  media_type: string;
  data: string;
}

// Content block structure
export interface ContentBlock {
  type: string;
  text?: string;
  tool_use_id?: string;
  content?: unknown;
  name?: string;
  input?: unknown;
  id?: string;
  is_error?: boolean;
  source?: ImageSource;
}

// Model information (align with Go types as required fields)
export interface Model {
  id: string;
  object: string;
  created: number;
  owned_by: string;
  display_name: string;
  type: string;
  max_tokens: number;
}

export interface ModelsResponse {
  object: string;
  data: Model[];
}

// Re-export token types for backward compatibility
export type { Token, RefreshResponse, RefreshRequest, IdcRefreshRequest } from "./token.ts";
export { isTokenExpired, createTokenFromRefreshResponse } from "./token.ts";
export type { TokenWithUsage } from "./usage_limits.ts";

// Backward compatibility: TokenInfo is the same as Token
export type { Token as TokenInfo } from "./token.ts";
