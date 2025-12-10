/**
 * Token Estimation Configuration
 * 
 * 集中管理 Token 估算相关的配置参数
 */

/**
 * Token 估算系数配置
 */
export const TOKEN_ESTIMATION_CONFIG = {
  /**
   * 基础文本 Token 估算系数
   * 平均每个字符对应的 token 数量
   * 
   * 英文：约 0.25 tokens/char (4 chars/token)
   * 中文：约 0.5-0.7 tokens/char
   * 
   * 使用保守估计：0.35
   */
  TEXT_CHARS_PER_TOKEN: 3.5,

  /**
   * 工具定义的基础 Token 成本
   * 包括工具名称、描述等元数据
   */
  TOOL_DEFINITION_BASE_TOKENS: 100,

  /**
   * 工具输入 Schema 的 Token 估算系数
   * JSON Schema 通常比普通文本更冗长
   */
  TOOL_SCHEMA_CHARS_PER_TOKEN: 4.0,

  /**
   * 工具使用的基础 Token 成本
   * 包括工具调用的元数据
   */
  TOOL_USE_BASE_TOKENS: 50,

  /**
   * 工具输入参数的 Token 估算系数
   * JSON 格式的参数通常比普通文本更冗长
   */
  TOOL_INPUT_CHARS_PER_TOKEN: 4.0,

  /**
   * 工具结果的基础 Token 成本
   */
  TOOL_RESULT_BASE_TOKENS: 30,

  /**
   * 工具结果内容的 Token 估算系数
   */
  TOOL_RESULT_CHARS_PER_TOKEN: 3.5,

  /**
   * 系统消息的 Token 估算系数
   * 系统消息通常包含特殊指令，可能有额外开销
   */
  SYSTEM_CHARS_PER_TOKEN: 3.5,

  /**
   * 图片的估算 Token 数
   * 根据图片大小和分辨率，使用固定值
   */
  IMAGE_BASE_TOKENS: 1000,

  /**
   * 最小 Token 数量
   * 即使内容为空，也会有最小的 token 开销
   */
  MIN_TOKENS: 1,

  /**
   * 安全边界系数
   * 为估算结果添加一定的安全余量
   * 1.1 表示增加 10% 的安全边界
   */
  SAFETY_MARGIN: 1.1,
} as const;

/**
 * Token 限制配置
 */
export const TOKEN_LIMIT_CONFIG = {
  /**
   * Claude 3.5 Sonnet 的最大输入 Token 数
   */
  MAX_INPUT_TOKENS: 200000,

  /**
   * Claude 3.5 Sonnet 的最大输出 Token 数
   */
  MAX_OUTPUT_TOKENS: 8192,

  /**
   * 默认的最大输出 Token 数
   * 用于用户未指定时的默认值
   */
  DEFAULT_MAX_TOKENS: 4096,

  /**
   * 警告阈值
   * 当 token 使用量超过此比例时发出警告
   */
  WARNING_THRESHOLD: 0.9, // 90%
} as const;

/**
 * Token 估算调试配置
 */
export const TOKEN_ESTIMATION_DEBUG = {
  /**
   * 是否启用详细的 Token 估算日志
   */
  ENABLED: false,

  /**
   * 是否记录每个组件的 Token 分解
   */
  DETAILED_BREAKDOWN: false,
} as const;

/**
 * 动态配置类型
 */
export type TokenEstimationConfigType = {
  TEXT_CHARS_PER_TOKEN: number;
  TOOL_DEFINITION_BASE_TOKENS: number;
  TOOL_SCHEMA_CHARS_PER_TOKEN: number;
  TOOL_USE_BASE_TOKENS: number;
  TOOL_INPUT_CHARS_PER_TOKEN: number;
  TOOL_RESULT_BASE_TOKENS: number;
  TOOL_RESULT_CHARS_PER_TOKEN: number;
  SYSTEM_CHARS_PER_TOKEN: number;
  IMAGE_BASE_TOKENS: number;
  MIN_TOKENS: number;
  SAFETY_MARGIN: number;
};

/**
 * 根据环境变量动态调整配置
 */
export function getTokenEstimationConfig(): TokenEstimationConfigType {
  const config: TokenEstimationConfigType = {
    TEXT_CHARS_PER_TOKEN: TOKEN_ESTIMATION_CONFIG.TEXT_CHARS_PER_TOKEN,
    TOOL_DEFINITION_BASE_TOKENS: TOKEN_ESTIMATION_CONFIG.TOOL_DEFINITION_BASE_TOKENS,
    TOOL_SCHEMA_CHARS_PER_TOKEN: TOKEN_ESTIMATION_CONFIG.TOOL_SCHEMA_CHARS_PER_TOKEN,
    TOOL_USE_BASE_TOKENS: TOKEN_ESTIMATION_CONFIG.TOOL_USE_BASE_TOKENS,
    TOOL_INPUT_CHARS_PER_TOKEN: TOKEN_ESTIMATION_CONFIG.TOOL_INPUT_CHARS_PER_TOKEN,
    TOOL_RESULT_BASE_TOKENS: TOKEN_ESTIMATION_CONFIG.TOOL_RESULT_BASE_TOKENS,
    TOOL_RESULT_CHARS_PER_TOKEN: TOKEN_ESTIMATION_CONFIG.TOOL_RESULT_CHARS_PER_TOKEN,
    SYSTEM_CHARS_PER_TOKEN: TOKEN_ESTIMATION_CONFIG.SYSTEM_CHARS_PER_TOKEN,
    IMAGE_BASE_TOKENS: TOKEN_ESTIMATION_CONFIG.IMAGE_BASE_TOKENS,
    MIN_TOKENS: TOKEN_ESTIMATION_CONFIG.MIN_TOKENS,
    SAFETY_MARGIN: TOKEN_ESTIMATION_CONFIG.SAFETY_MARGIN,
  };
  
  // 可以从环境变量读取自定义配置
  const customFactor = Deno.env.get("TOKEN_ESTIMATION_FACTOR");
  if (customFactor) {
    const factor = parseFloat(customFactor);
    if (!isNaN(factor) && factor > 0) {
      config.TEXT_CHARS_PER_TOKEN = factor;
    }
  }
  
  return config;
}
