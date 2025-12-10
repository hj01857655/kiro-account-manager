/**
 * Cache Configuration
 * 
 * 集中管理缓存相关的配置参数
 */

/**
 * Token 缓存配置
 */
export const TOKEN_CACHE_CONFIG = {
  /**
   * Token 缓存的生存时间（毫秒）
   * 默认：24 小时
   */
  TTL_MS: 24 * 60 * 60 * 1000,

  /**
   * 缓存清理任务的执行间隔（毫秒）
   * 默认：1 小时
   */
  CLEANUP_INTERVAL_MS: 60 * 60 * 1000,

  /**
   * Token 过期缓冲时间（毫秒）
   * 在实际过期时间前提前刷新 token
   * 默认：5 分钟
   */
  EXPIRY_BUFFER_MS: 5 * 60 * 1000,
} as const;

/**
 * 流处理缓存配置
 */
export const STREAM_CACHE_CONFIG = {
  /**
   * 流处理缓冲区大小（字节）
   * 默认：64 KB
   */
  BUFFER_SIZE: 64 * 1024,

  /**
   * 最大缓冲块数量
   * 默认：1000
   */
  MAX_BUFFER_CHUNKS: 1000,
} as const;

/**
 * 响应缓存配置（预留）
 */
export const RESPONSE_CACHE_CONFIG = {
  /**
   * 是否启用响应缓存
   */
  ENABLED: false,

  /**
   * 缓存的最大数量
   */
  MAX_SIZE: 100,

  /**
   * 缓存的生存时间（毫秒）
   */
  TTL_MS: 5 * 60 * 1000, // 5 分钟
} as const;
