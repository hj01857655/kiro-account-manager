/**
 * Timeout Configuration
 * 
 * 集中管理超时控制相关的配置参数
 * 防止慢速客户端占用资源，提升系统稳定性
 */

/**
 * 流处理超时配置
 */
export const STREAM_TIMEOUT_CONFIG = {
  /**
   * 总超时时间（毫秒）
   * 默认：5 分钟
   * 
   * 防止流处理无限期挂起
   */
  TOTAL_TIMEOUT_MS: 5 * 60 * 1000,

  /**
   * 单次读取超时时间（毫秒）
   * 默认：30 秒
   * 
   * 防止单个读取操作阻塞
   */
  READ_TIMEOUT_MS: 30 * 1000,

  /**
   * 最大数据大小（字节）
   * 默认：100 MB
   * 
   * 防止内存耗尽
   */
  MAX_DATA_SIZE: 100 * 1024 * 1024,

  /**
   * 空闲超时时间（毫秒）
   * 默认：2 分钟
   * 
   * 如果在此时间内没有接收到任何数据，则认为连接已断开
   */
  IDLE_TIMEOUT_MS: 2 * 60 * 1000,

  /**
   * 是否启用超时控制
   * 默认：true
   */
  ENABLED: true,
} as const;

/**
 * 非流式请求超时配置
 */
export const NON_STREAM_TIMEOUT_CONFIG = {
  /**
   * 请求超时时间（毫秒）
   * 默认：2 分钟
   */
  REQUEST_TIMEOUT_MS: 2 * 60 * 1000,

  /**
   * 最大响应大小（字节）
   * 默认：50 MB
   */
  MAX_RESPONSE_SIZE: 50 * 1024 * 1024,

  /**
   * 是否启用超时控制
   * 默认：true
   */
  ENABLED: true,
} as const;

/**
 * 超时错误类型
 */
export class TimeoutError extends Error {
  constructor(
    message: string,
    public readonly timeoutType: "total" | "read" | "idle" | "size",
  ) {
    super(message);
    this.name = "TimeoutError";
  }
}
