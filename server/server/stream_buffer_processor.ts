/**
 * 流缓冲处理器
 * 负责高效的流数据缓冲和解析
 * 
 * 性能优化：
 * 1. 使用增量解析，避免重复解析
 * 2. 缓冲区复用，减少内存分配
 * 3. 批量处理消息，减少函数调用开销
 */

import { RobustEventStreamParser } from "../parser/robust_parser.ts";

interface EventStreamMessage {
  payload: Uint8Array;
}

export class StreamBufferProcessor {
  private readonly parser: RobustEventStreamParser;
  private totalProcessedEvents = 0;

  constructor() {
    this.parser = new RobustEventStreamParser();
  }

  /**
   * 处理数据块
   * 使用增量解析，避免重复处理已解析的数据
   */
  processChunk(chunk: Uint8Array): EventStreamMessage[] {
    // 直接使用解析器的增量解析能力
    // RobustEventStreamParser 内部已经维护了缓冲区
    const messages = this.parser.parseStream(chunk);
    this.totalProcessedEvents += messages.length;
    return messages;
  }

  /**
   * 获取已处理的事件总数
   */
  getTotalProcessedEvents(): number {
    return this.totalProcessedEvents;
  }

  /**
   * 重置处理器状态
   */
  reset(): void {
    this.parser.reset();
    this.totalProcessedEvents = 0;
  }
}