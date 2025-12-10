import type { OpenAIResponse, OpenAIChoice, OpenAIMessage, OpenAIToolCall } from "../types/openai.ts";

// Convert Anthropic response to OpenAI format
export function convertAnthropicToOpenAI(
  anthropicResp: Record<string, unknown>,
  model: string,
  messageId: string,
): OpenAIResponse {
  let content = "";
  const toolCalls: OpenAIToolCall[] = [];
  let finishReason = "stop";

  // Process content array
  const contentArray = anthropicResp.content;
  if (Array.isArray(contentArray) && contentArray.length > 0) {
    const textParts: string[] = [];

    for (const block of contentArray) {
      if (typeof block === "object" && block !== null) {
        const blockType = block.type;

        switch (blockType) {
          case "text":
            if (typeof block.text === "string") {
              textParts.push(block.text);
            }
            break;

          case "tool_use":
            finishReason = "tool_calls";
            if (block.id && block.name) {
              const inputJson = JSON.stringify(block.input || {});
              toolCalls.push({
                id: block.id,
                type: "function",
                function: {
                  name: block.name,
                  arguments: inputJson,
                },
              });
            }
            break;
        }
      }
    }

    content = textParts.join("");
  }

  // Calculate token usage
  let promptTokens = 0;
  let completionTokens = Math.floor(content.length / 4); // Simple estimation

  if (anthropicResp.usage && typeof anthropicResp.usage === "object") {
    const usage = anthropicResp.usage as Record<string, unknown>;
    if (typeof usage.input_tokens === "number") {
      promptTokens = usage.input_tokens;
    }
    if (typeof usage.output_tokens === "number") {
      completionTokens = usage.output_tokens;
    }
  }

  const message: OpenAIMessage = {
    role: "assistant",
    content,
  };

  // Only add tool_calls if present
  if (toolCalls.length > 0) {
    message.tool_calls = toolCalls;
  }

  const choice: OpenAIChoice = {
    index: 0,
    message,
    finish_reason: finishReason,
  };

  return {
    id: messageId,
    object: "chat.completion",
    created: Math.floor(Date.now() / 1000),
    model,
    choices: [choice],
    usage: {
      prompt_tokens: promptTokens,
      completion_tokens: completionTokens,
      total_tokens: promptTokens + completionTokens,
    },
  };
}
