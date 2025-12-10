// Model mapping between public names and CodeWhisperer internal IDs
export const MODEL_MAP: Record<string, string> = {
  "claude-sonnet-4-5": "CLAUDE_SONNET_4_5_20250929_V1_0",
  "claude-sonnet-4-5-20250929": "CLAUDE_SONNET_4_5_20250929_V1_0",
  "claude-sonnet-4-20250514": "CLAUDE_SONNET_4_20250514_V1_0",
  "claude-3-7-sonnet-20250219": "CLAUDE_3_7_SONNET_20250219_V1_0",
  "claude-3-5-haiku-20241022": "auto",
  "claude-haiku-4-5-20251001": "auto",
};

// AWS endpoints
export const AWS_ENDPOINTS = {
  SOCIAL_REFRESH: "https://prod.us-east-1.auth.desktop.kiro.dev/refreshToken",
  IDC_REFRESH: "https://oidc.us-east-1.amazonaws.com/token",
  CODEWHISPERER: "https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse",
};

// Default values
export const DEFAULTS = {
  PORT: 8080,
  GIN_MODE: "release",
  LOG_LEVEL: "info",
  LOG_FORMAT: "json",
  MAX_TOKENS: 8192,
  AGENT_TASK_TYPE: "vibe",
  CHAT_TRIGGER_TYPE: "MANUAL",
  ORIGIN: "AI_EDITOR",
};

// AWS constants
export const AWS_CONSTANTS = {
  REGION: "us-east-1",
  SERVICE: "codewhisperer",
};
