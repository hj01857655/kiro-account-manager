import {
  HeaderParser,
  HeaderValue,
  getMessageTypeFromHeaders,
  getEventTypeFromHeaders,
  getContentTypeFromHeaders,
} from "./header_parser.ts";
import * as logger from "../logger/logger.ts";

interface EventStreamMessage {
  headers: Record<string, HeaderValue>;
  payload: Uint8Array;
  messageType: string;
  eventType: string;
  contentType: string;
}

const MIN_MESSAGE_SIZE = 16;
const MAX_MESSAGE_SIZE = 16 * 1024 * 1024; // 16MB

export class RobustEventStreamParser {
  private buffer: Uint8Array = new Uint8Array(0);
  private errorCount = 0;
  private maxErrors = 100;
  private headerParser: HeaderParser;

  constructor() {
    this.headerParser = new HeaderParser();
  }

  setMaxErrors(maxErrors: number): void {
    this.maxErrors = maxErrors;
  }

  reset(): void {
    this.buffer = new Uint8Array(0);
    this.errorCount = 0;
    this.headerParser.reset();
  }

  parseStream(data: Uint8Array): EventStreamMessage[] {
    this.buffer = this.concatBuffers(this.buffer, data);
    const messages: EventStreamMessage[] = [];

    while (this.buffer.length >= MIN_MESSAGE_SIZE) {
      const view = new DataView(this.buffer.buffer, this.buffer.byteOffset);
      const totalLength = view.getUint32(0, false); // Big-endian

      if (totalLength < MIN_MESSAGE_SIZE || totalLength > MAX_MESSAGE_SIZE) {
        this.buffer = this.buffer.slice(1);
        this.errorCount++;
        continue;
      }

      if (this.buffer.length < totalLength) {
        break;
      }

      const messageData = this.buffer.slice(0, totalLength);
      this.buffer = this.buffer.slice(totalLength);

      try {
        // 每条消息开始前重置头部解析器
        this.headerParser.reset();
        const message = this.parseSingleMessage(messageData);
        if (message) {
          // 验证工具调用完整性
          this.validateToolUseIdIntegrity(message);
          messages.push(message);
        }
      } catch {
        this.errorCount++;
        if (this.errorCount >= this.maxErrors) {
          throw new Error(`Too many errors (${this.errorCount}), stopping parse`);
        }
      }
    }

    return messages;
  }

  private parseSingleMessage(data: Uint8Array): EventStreamMessage | null {
    if (data.length < MIN_MESSAGE_SIZE) {
      return null;
    }

    const view = new DataView(data.buffer, data.byteOffset);
    const totalLength = view.getUint32(0, false);
    const headerLength = view.getUint32(4, false);

    if (totalLength !== data.length) {
      throw new Error(`Length mismatch: expected ${totalLength}, got ${data.length}`);
    }

    const headerData = data.slice(12, 12 + headerLength);
    const payloadStart = 12 + headerLength;
    const payloadEnd = totalLength - 4;

    // 边界检查
    if (payloadStart > payloadEnd || payloadEnd > data.length) {
      throw new Error(`Payload boundary error: start=${payloadStart}, end=${payloadEnd}, len=${data.length}`);
    }

    const payloadData = data.slice(payloadStart, payloadEnd);

    // 使用新的 HeaderParser 解析头部，支持断点续传和容错
    let headers: Record<string, HeaderValue>;
    if (headerData.length === 0) {
      logger.debug("Empty header detected, using defaults");
      headers = this.headerParser.forceCompleteHeaderParsing(this.headerParser.getState());
    } else {
      try {
        headers = this.headerParser.parseHeaders(headerData);
        // 检查是否可以恢复
        if (Object.keys(headers).length === 0 && this.headerParser.isHeaderParseRecoverable(this.headerParser.getState())) {
          logger.warn("Header parsing incomplete, forcing completion");
          headers = this.headerParser.forceCompleteHeaderParsing(this.headerParser.getState());
        }
      } catch (err) {
        logger.warn("Header parsing failed, using defaults", logger.Err(err));
        headers = this.headerParser.forceCompleteHeaderParsing(this.headerParser.getState());
      }
    }

    // 添加 payload 调试信息
    if (payloadData.length > 0) {
      const payloadPreview = new TextDecoder().decode(payloadData.slice(0, Math.min(100, payloadData.length)));
      logger.debug(
        "Payload parsed",
        logger.Int("length", payloadData.length),
        logger.String("preview", payloadPreview.substring(0, 100))
      );
    }

    return {
      headers,
      payload: payloadData,
      messageType: getMessageTypeFromHeaders(headers),
      eventType: getEventTypeFromHeaders(headers),
      contentType: getContentTypeFromHeaders(headers),
    };
  }

  // 工具调用 ID 完整性验证
  private validateToolUseIdIntegrity(message: EventStreamMessage): void {
    if (!message || message.payload.length === 0) {
      return;
    }

    const payloadStr = new TextDecoder().decode(message.payload);

    if (payloadStr.includes("tool_use_id") || payloadStr.includes("toolUseId")) {
      const toolUseIds = this.extractToolUseIds(payloadStr);
      for (const toolUseId of toolUseIds) {
        if (!this.isValidToolUseIdFormat(toolUseId)) {
          logger.warn("Detected potentially corrupted tool_use_id", logger.String("tool_use_id", toolUseId));
        }
      }
    }
  }

  // 从 payload 中提取所有 tool_use_id
  private extractToolUseIds(payload: string): string[] {
    const toolUseIds: string[] = [];
    const searchStr = "tooluse_";
    let startPos = 0;

    while (true) {
      const idx = payload.indexOf(searchStr, startPos);
      if (idx === -1) break;

      // 确保前面是引号或其他分隔符
      if (idx > 0) {
        const prevChar = payload[idx - 1];
        if (prevChar !== '"' && prevChar !== ':' && prevChar !== ' ' && prevChar !== '{') {
          startPos = idx + 1;
          continue;
        }
      }

      // 查找 ID 的结束位置
      let end = idx + searchStr.length;
      while (end < payload.length) {
        const char = payload[end];
        const code = char.charCodeAt(0);
        // 有效字符: 字母、数字、下划线、连字符
        if (!(
          (code >= 97 && code <= 122) || // a-z
          (code >= 65 && code <= 90) ||  // A-Z
          (code >= 48 && code <= 57) ||  // 0-9
          char === '_' || char === '-'
        )) {
          break;
        }
        end++;
      }

      if (end > idx + searchStr.length) {
        const toolUseId = payload.substring(idx, end);
        if (this.isValidToolUseIdFormat(toolUseId)) {
          toolUseIds.push(toolUseId);
        } else {
          logger.warn("Skipping invalid tool_use_id", logger.String("tool_use_id", toolUseId));
        }
      }

      startPos = idx + 1;
    }

    return toolUseIds;
  }

  // 验证 tool_use_id 格式是否有效
  private isValidToolUseIdFormat(toolUseId: string): boolean {
    // 基本格式检查
    if (!toolUseId.startsWith("tooluse_")) {
      return false;
    }

    // 长度检查
    if (toolUseId.length < 20 || toolUseId.length > 50) {
      logger.debug("tool_use_id length abnormal", logger.Int("length", toolUseId.length));
      return false;
    }

    // 字符有效性检查
    const suffix = toolUseId.substring(8);
    for (let i = 0; i < suffix.length; i++) {
      const char = suffix[i];
      const code = char.charCodeAt(0);
      if (!(
        (code >= 97 && code <= 122) || // a-z
        (code >= 65 && code <= 90) ||  // A-Z
        (code >= 48 && code <= 57) ||  // 0-9
        char === '_' || char === '-'
      )) {
        logger.debug(
          "tool_use_id contains invalid character",
          logger.Int("position", i + 8),
          logger.String("character", char)
        );
        return false;
      }
    }

    // 检查明显的损坏模式
    if (toolUseId.includes("tooluluse_") || toolUseId.includes("tooluse_tooluse_")) {
      logger.warn("Detected obviously corrupted tool_use_id pattern", logger.String("tool_use_id", toolUseId));
      return false;
    }

    return true;
  }

  private concatBuffers(a: Uint8Array, b: Uint8Array): Uint8Array {
    const result = new Uint8Array(a.length + b.length);
    result.set(a, 0);
    result.set(b, a.length);
    return result;
  }
}
