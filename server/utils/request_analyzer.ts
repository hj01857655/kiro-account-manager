import { getMessageContent } from "./message.ts";

export enum RequestComplexity {
  Simple = 0,
  Complex = 1,
}

interface AnthropicRequest {
  max_tokens?: number;
  messages: Array<{ content: unknown }>;
  tools?: unknown[];
  system?: string | Array<{ text: string }>;
}

export function analyzeRequestComplexity(req: AnthropicRequest): RequestComplexity {
  let complexityScore = 0;

  // 1. Check max tokens
  const maxTokens = req.max_tokens || 0;
  if (maxTokens > 4000) {
    complexityScore += 2;
  } else if (maxTokens > 1000) {
    complexityScore += 1;
  }

  // 2. Check message length
  let totalContentLength = 0;
  for (const msg of req.messages) {
    const content = getMessageContent(msg.content);
    totalContentLength += content.length;
  }

  if (totalContentLength > 10000) {
    complexityScore += 2;
  } else if (totalContentLength > 3000) {
    complexityScore += 1;
  }

  // 3. Check tools
  if (req.tools && req.tools.length > 0) {
    complexityScore += 2;
  }

  // 4. Check system prompt
  if (req.system) {
    const systemContent = typeof req.system === "string"
      ? req.system
      : req.system.map((s) => s.text).join("");
    if (systemContent.length > 2000) {
      complexityScore += 1;
    }
  }

  // 5. Check complex keywords
  const complexKeywords = [
    "分析",
    "analyze",
    "详细",
    "detail",
    "总结",
    "summary",
    "代码审查",
    "code review",
    "重构",
    "refactor",
    "优化",
    "optimize",
    "复杂",
    "complex",
    "深入",
    "comprehensive",
    "完整",
    "complete",
  ];

  for (const msg of req.messages) {
    const content = getMessageContent(msg.content).toLowerCase();
    if (complexKeywords.some((keyword) => content.includes(keyword))) {
      complexityScore += 1;
      break;
    }
  }

  return complexityScore >= 3 ? RequestComplexity.Complex : RequestComplexity.Simple;
}
