/**
 * 统一的响应解析器
 */

import { isObject, isString, parseJsonSafely, extractToolInput } from "../types/guards.ts";
import * as logger from "../logger/logger.ts";

export interface ParsedContent {
  content: string;
  toolUses: Array<{ type: string; id: string; name: string; input: unknown }>;
}

/**
 * 解析 AWS EventStream 响应
 */
export function parseEventStreamResponse(
  messages: Array<{ payload: Uint8Array }>,
  requestId: string
): ParsedContent {
  let content = "";
  const toolUsesMap = new Map<string, { type: string; id: string; name: string; input: unknown }>();
  const toolInputBuffers = new Map<string, string>();

  for (const message of messages) {
    try {
      const payloadStr = new TextDecoder().decode(message.payload);
      const payload = parseJsonSafely(payloadStr);
      
      if (!isObject(payload)) continue;
      
      const event = isObject(payload.assistantResponseEvent) 
        ? payload.assistantResponseEvent 
        : payload;
      
      if (!isObject(event)) continue;

      // 提取内容
      if (isString(event.content)) {
        content += event.content;
      }

      // 提取工具使用信息
      if (isString(event.toolUseId) && isString(event.name)) {
        const toolId = event.toolUseId;
        const toolName = event.name;

        // 过滤 web_search
        if (toolName === "web_search" || toolName === "websearch") {
          continue;
        }

        // 获取或创建工具条目
        if (!toolUsesMap.has(toolId)) {
          toolUsesMap.set(toolId, {
            type: "tool_use",
            id: toolId,
            name: toolName,
            input: {},
          });
          toolInputBuffers.set(toolId, "");
        }

        // 累积输入
        if (event.input !== undefined && event.input !== null) {
          const tool = toolUsesMap.get(toolId)!;

          if (isObject(event.input)) {
            tool.input = event.input;
          } else if (isString(event.input)) {
            const currentBuffer = toolInputBuffers.get(toolId) || "";
            toolInputBuffers.set(toolId, currentBuffer + event.input);
          }
        }

        // 工具停止时解析累积的输入
        if (event.stop === true) {
          const tool = toolUsesMap.get(toolId);
          const inputBuffer = toolInputBuffers.get(toolId);

          if (tool && inputBuffer && inputBuffer.trim()) {
            tool.input = extractToolInput(inputBuffer);
          }
        }
      }
    } catch (e) {
      logger.debug(
        "跳过无效消息",
        logger.String("request_id", requestId),
        logger.Err(e),
      );
    }
  }

  // 最终解析所有剩余的缓冲输入
  for (const [toolId, buffer] of toolInputBuffers.entries()) {
    if (buffer && buffer.trim()) {
      const tool = toolUsesMap.get(toolId);
      if (tool && isObject(tool.input) && Object.keys(tool.input).length === 0) {
        tool.input = extractToolInput(buffer);
      }
    }
  }

  return {
    content,
    toolUses: Array.from(toolUsesMap.values()),
  };
}

/**
 * 简化的二进制解析（用于 OpenAI 处理器）
 */
export function parseEventStreamBinary(data: Uint8Array): ParsedContent {
  let content = "";
  const toolUsesMap = new Map<string, { type: string; id: string; name: string; input: unknown }>();
  let offset = 0;

  while (offset < data.length) {
    if (offset + 16 > data.length) break;

    const totalLength = new DataView(data.buffer, offset, 4).getUint32(0, false);
    const headerLength = new DataView(data.buffer, offset + 4, 4).getUint32(0, false);

    if (offset + totalLength > data.length) break;

    const payloadStart = offset + 12 + headerLength;
    const payloadEnd = offset + totalLength - 4;
    const payloadData = data.slice(payloadStart, payloadEnd);

    try {
      const payload = JSON.parse(new TextDecoder().decode(payloadData));
      const event = payload.assistantResponseEvent || payload;

      if (event.content) {
        content += event.content;
      }

      if (event.toolUseId && event.name) {
        const toolId = event.toolUseId;
        if (!toolUsesMap.has(toolId)) {
          toolUsesMap.set(toolId, {
            type: "tool_use",
            id: toolId,
            name: event.name,
            input: {},
          });
        }

        if (event.input !== undefined && event.input !== null) {
          const tool = toolUsesMap.get(toolId)!;
          if (typeof event.input === "object") {
            tool.input = event.input;
          }
        }
      }
    } catch {
      // 跳过无效 payload
    }

    offset += totalLength;
  }

  return {
    content,
    toolUses: Array.from(toolUsesMap.values()),
  };
}
