import {
  LONG_TEXT_THRESHOLD,
  SHORT_TEXT_THRESHOLD,
  TOKEN_ESTIMATION_RATIO,
} from "../config/runtime.ts";

interface CountTokensRequest {
  system?: Array<{ text: string }>;
  messages: Array<{ content: unknown }>;
  tools?: Array<{
    name: string;
    description: string;
    input_schema?: unknown;
  }>;
}

export class TokenEstimator {
  estimateTokens(req: CountTokensRequest): number {
    let totalTokens = 0;

    // 1. System prompt
    if (req.system) {
      for (const sysMsg of req.system) {
        if (sysMsg.text) {
          totalTokens += this.estimateTextTokens(sysMsg.text);
          totalTokens += 2;
        }
      }
    }

    // 2. Messages
    for (const msg of req.messages) {
      totalTokens += 3; // Role overhead

      if (typeof msg.content === "string") {
        totalTokens += this.estimateTextTokens(msg.content);
      } else if (Array.isArray(msg.content)) {
        for (const block of msg.content) {
          totalTokens += this.estimateContentBlock(block);
        }
      } else {
        totalTokens += JSON.stringify(msg.content).length / TOKEN_ESTIMATION_RATIO;
      }
    }

    // 3. Tools
    const toolCount = req.tools?.length || 0;
    if (toolCount > 0) {
      let baseToolsOverhead: number;
      let perToolOverhead: number;

      if (toolCount === 1) {
        baseToolsOverhead = 0;
        perToolOverhead = 320;
      } else if (toolCount <= 5) {
        baseToolsOverhead = 100;
        perToolOverhead = 120;
      } else {
        baseToolsOverhead = 180;
        perToolOverhead = 60;
      }

      totalTokens += baseToolsOverhead;

      for (const tool of req.tools!) {
        totalTokens += this.estimateToolName(tool.name || "");
        totalTokens += this.estimateTextTokens(tool.description || "");

        if (tool.input_schema) {
          const jsonBytes = JSON.stringify(tool.input_schema);
          let schemaCharsPerToken: number;

          if (toolCount === 1) {
            schemaCharsPerToken = 1.9;
          } else if (toolCount <= 5) {
            schemaCharsPerToken = 2.2;
          } else {
            schemaCharsPerToken = 2.5;
          }

          let schemaTokens = Math.floor(jsonBytes.length / schemaCharsPerToken);

          if (jsonBytes.includes("$schema")) {
            schemaTokens += toolCount === 1 ? 10 : 5;
          }

          const minSchemaTokens = toolCount > 5 ? 30 : 50;
          if (schemaTokens < minSchemaTokens) {
            schemaTokens = minSchemaTokens;
          }

          totalTokens += schemaTokens;
        }

        totalTokens += perToolOverhead;
      }
    }

    // 4. Base request overhead
    totalTokens += 4;

    return totalTokens;
  }

  private estimateToolName(name: string): number {
    if (!name || typeof name !== "string") return 0;

    const baseTokens = Math.floor(String(name).length / 2);
    const underscoreCount = (name.match(/_/g) || []).length;
    const camelCaseCount = (name.match(/[A-Z]/g) || []).length;

    const totalTokens = baseTokens + underscoreCount + Math.floor(camelCaseCount / 2);
    return Math.max(totalTokens, 2);
  }

  estimateTextTokens(text: string): number {
    if (!text || typeof text !== "string") return 0;

    const runes = Array.from(String(text));
    const runeCount = runes.length;
    if (runeCount === 0) return 0;

    // Count Chinese characters
    let chineseChars = 0;
    for (const r of runes) {
      const code = r.codePointAt(0) || 0;
      if (code >= 0x4E00 && code <= 0x9FFF) {
        chineseChars++;
      }
    }

    const nonChineseChars = runeCount - chineseChars;
    const isPureChinese = nonChineseChars === 0;

    // Chinese tokens
    let chineseTokens = 0;
    if (chineseChars > 0) {
      chineseTokens = isPureChinese ? 1 + chineseChars : chineseChars;
    }

    // Non-Chinese tokens
    let nonChineseTokens = 0;
    if (nonChineseChars > 0) {
      let charsPerToken: number;
      if (nonChineseChars < 50) {
        charsPerToken = 2.8;
      } else if (nonChineseChars < SHORT_TEXT_THRESHOLD) {
        charsPerToken = 2.6;
      } else {
        charsPerToken = 2.5;
      }
      nonChineseTokens = Math.max(1, Math.floor(nonChineseChars / charsPerToken));
    }

    let tokens = chineseTokens + nonChineseTokens;

    // Long text compression
    if (runeCount >= LONG_TEXT_THRESHOLD) {
      tokens = Math.floor(tokens * 0.60);
    } else if (runeCount >= 500) {
      tokens = Math.floor(tokens * 0.70);
    } else if (runeCount >= 300) {
      tokens = Math.floor(tokens * 0.80);
    } else if (runeCount >= 200) {
      tokens = Math.floor(tokens * 0.85);
    } else if (runeCount >= SHORT_TEXT_THRESHOLD) {
      tokens = Math.floor(tokens * 0.90);
    } else if (runeCount >= 50) {
      tokens = Math.floor(tokens * 0.95);
    }

    return Math.max(tokens, 1);
  }

  private estimateContentBlock(block: unknown): number {
    if (typeof block !== "object" || block === null) return 10;

    const blockMap = block as Record<string, unknown>;
    const blockType = blockMap.type as string;

    switch (blockType) {
      case "text": {
        const textContent = typeof blockMap.text === "string" ? blockMap.text : "";
        return this.estimateTextTokens(textContent);
      }
      case "image":
        return 1500;
      case "document":
        return 500;
      case "tool_use": {
        const toolName = blockMap.name as string || "";
        const toolInput = blockMap.input as Record<string, unknown> || {};
        return this.estimateToolUseTokens(toolName, toolInput);
      }
      case "tool_result": {
        const content = blockMap.content;
        if (typeof content === "string") {
          return this.estimateTextTokens(content || "");
        } else if (Array.isArray(content)) {
          return content.reduce((sum, item) => sum + this.estimateContentBlock(item), 0);
        }
        return 50;
      }
      default:
        return Math.floor(JSON.stringify(block).length / TOKEN_ESTIMATION_RATIO);
    }
  }

  estimateToolUseTokens(toolName: string, toolInput: Record<string, unknown>): number {
    let totalTokens = 3 + 8 + 1; // type + id + name keyword
    totalTokens += this.estimateToolName(toolName);
    totalTokens += 1; // input keyword

    if (Object.keys(toolInput).length > 0) {
      totalTokens += Math.floor(JSON.stringify(toolInput).length / TOKEN_ESTIMATION_RATIO);
    } else {
      totalTokens += 1;
    }

    return totalTokens;
  }
}

export function isValidClaudeModel(model: string): boolean {
  if (!model) return false;

  const lowerModel = model.toLowerCase();
  const validPrefixes = [
    "claude-",
    "gpt-",
    "gemini-",
    "text-",
    "anthropic.claude",
  ];

  return validPrefixes.some((prefix) => lowerModel.startsWith(prefix));
}
