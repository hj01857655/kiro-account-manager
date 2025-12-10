/**
 * 流超时控制器
 * 负责管理流处理的超时检查和控制
 */

import { STREAM_TIMEOUT_CONFIG, TimeoutError } from "../config/timeout.ts";

export class StreamTimeoutController {
  private readonly startTime: number;
  private lastReadTime: number;
  private totalReadBytes = 0;
  private totalTimeout?: number;
  private readTimeout?: number;

  constructor() {
    this.startTime = Date.now();
    this.lastReadTime = this.startTime;
  }

  /**
   * 更新读取时间和字节数
   */
  updateReadStats(bytes: number): void {
    this.lastReadTime = Date.now();
    this.totalReadBytes += bytes;
  }

  /**
   * 检查超时
   */
  checkTimeouts(): void {
    if (!STREAM_TIMEOUT_CONFIG.ENABLED) return;

    const now = Date.now();
    const elapsed = now - this.startTime;
    const idleTime = now - this.lastReadTime;

    // 检查总超时
    if (elapsed > STREAM_TIMEOUT_CONFIG.TOTAL_TIMEOUT_MS) {
      throw new TimeoutError(
        `Total timeout exceeded: ${elapsed}ms > ${STREAM_TIMEOUT_CONFIG.TOTAL_TIMEOUT_MS}ms`,
        "total",
      );
    }

    // 检查空闲超时
    if (idleTime > STREAM_TIMEOUT_CONFIG.IDLE_TIMEOUT_MS) {
      throw new TimeoutError(
        `Idle timeout exceeded: ${idleTime}ms > ${STREAM_TIMEOUT_CONFIG.IDLE_TIMEOUT_MS}ms`,
        "idle",
      );
    }

    // 检查数据大小
    if (this.totalReadBytes > STREAM_TIMEOUT_CONFIG.MAX_DATA_SIZE) {
      throw new TimeoutError(
        `Max data size exceeded: ${this.totalReadBytes} > ${STREAM_TIMEOUT_CONFIG.MAX_DATA_SIZE}`,
        "size",
      );
    }
  }

  /**
   * 带超时的读取操作
   */
  async readWithTimeout(
    reader: ReadableStreamDefaultReader<Uint8Array>,
  ): Promise<ReadableStreamReadResult<Uint8Array>> {
    if (!STREAM_TIMEOUT_CONFIG.ENABLED) {
      return await reader.read();
    }

    return await Promise.race([
      reader.read(),
      new Promise<ReadableStreamReadResult<Uint8Array>>((_, reject) => {
        this.readTimeout = setTimeout(() => {
          reject(
            new TimeoutError(
              `Read timeout exceeded: ${STREAM_TIMEOUT_CONFIG.READ_TIMEOUT_MS}ms`,
              "read",
            ),
          );
        }, STREAM_TIMEOUT_CONFIG.READ_TIMEOUT_MS);
      }),
    ]);
  }

  /**
   * 清除读取超时定时器
   */
  clearReadTimeout(): void {
    if (this.readTimeout !== undefined) {
      clearTimeout(this.readTimeout);
      this.readTimeout = undefined;
    }
  }

  /**
   * 获取统计信息
   */
  getStats(): { startTime: number; totalReadBytes: number; elapsed: number } {
    return {
      startTime: this.startTime,
      totalReadBytes: this.totalReadBytes,
      elapsed: Date.now() - this.startTime,
    };
  }

  /**
   * 清理资源
   */
  cleanup(): void {
    if (this.totalTimeout !== undefined) {
      clearTimeout(this.totalTimeout);
      this.totalTimeout = undefined;
    }
    this.clearReadTimeout();
  }
}