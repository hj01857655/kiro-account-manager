import type { AnthropicRequest, ToolChoice } from "../types/anthropic.ts";
import type { OpenAIRequest } from "../types/openai.ts";
import type { CodeWhispererRequest } from "../types/codewhisperer.ts";
import type { ContentBlock } from "../types/common.ts";
import { ModelNotFoundErrorType } from "../types/common.ts";
import { DEFAULTS, MODEL_MAP } from "../config/constants.ts";
import * as logger from "../logger/logger.ts";
import { validateAndProcessTools, convertOpenAIToolChoiceToAnthropic } from "./tools.ts";
import { validateImageContent } from "./content.ts";

export { convertAnthropicToOpenAI } from "./openai.ts";

// Validate CodeWhisperer request structure
// Match Go implementation in converter/codewhisperer.go:47-87
function validateCodeWhispererRequest(cwReq: CodeWhispererRequest): void {
  // Validate required fields
  if (!cwReq.conversationState.currentMessage.userInputMessage.modelId) {
    throw new Error("ModelId cannot be empty");
  }

  if (!cwReq.conversationState.conversationId) {
    throw new Error("ConversationId cannot be empty");
  }

  // Validate content completeness
  const trimmedContent = cwReq.conversationState.currentMessage.userInputMessage.content.trim();
  const hasImages = cwReq.conversationState.currentMessage.userInputMessage.images.length > 0;
  const hasTools = (cwReq.conversationState.currentMessage.userInputMessage.userInputMessageContext
    .tools?.length || 0) > 0;
  const hasToolResults = (cwReq.conversationState.currentMessage.userInputMessage
    .userInputMessageContext.toolResults?.length || 0) > 0;

  // If has tool results, allow empty content (tool execution feedback)
  if (hasToolResults) {
    logger.debug(
      "检测到工具结果，允许空内容",
      logger.Int(
        "tool_result_count",
        cwReq.conversationState.currentMessage.userInputMessage.userInputMessageContext.toolResults
          ?.length || 0,
      ),
    );
    return;
  }

  // If no content but has tools, inject placeholder
  if (!trimmedContent && !hasImages && hasTools) {
    cwReq.conversationState.currentMessage.userInputMessage.content = "执行工具任务";
    logger.warn("注入占位符内容以触发工具调用");
    return;
  }

  // Validate at least has content or images
  if (!trimmedContent && !hasImages) {
    throw new Error("User message content and images are both empty");
  }
}

// Determine chat trigger type following Go logic
function determineChatTriggerType(req: AnthropicRequest): string {
  if (req.tools && req.tools.length > 0) {
    const tc = req.tool_choice;
    if (tc && typeof tc === "object") {
      const tcObj = tc as unknown as Record<string, unknown>;
      if (tcObj.type === "any" || tcObj.type === "tool") return "AUTO";
    } else if (typeof tc === "string") {
      if (tc === "any") return "AUTO";
    }
  }
  return "MANUAL";
}

// Convert Anthropic request to CodeWhisperer format
export function anthropicToCodeWhisperer(
  req: AnthropicRequest,
  conversationId: string,
  agentContinuationId?: string,
): CodeWhispererRequest {
  const effectiveAgentContinuationId = agentContinuationId || crypto.randomUUID();

  if (!req.messages || req.messages.length === 0) {
    const error = new Error("messages is empty");
    logger.error("请求验证失败", logger.Err(error));
    throw error;
  }

  // Get the model ID
  const modelId = MODEL_MAP[req.model];
  if (!modelId) {
    logger.error(
      "未知模型",
      logger.String("model", req.model),
      logger.Any("available_models", Object.keys(MODEL_MAP)),
    );
    throw new ModelNotFoundErrorType(req.model, effectiveAgentContinuationId);
  }
  
  logger.debug(
    "开始转换请求",
    logger.String("model", req.model),
    logger.String("model_id", modelId),
    logger.Int("message_count", req.messages.length),
    logger.Bool("has_tools", !!(req.tools && req.tools.length > 0)),
  );

  // Extract the last message content
  const lastMessage = req.messages[req.messages.length - 1];
  let content = "";

  if (typeof lastMessage.content === "string") {
    content = lastMessage.content;
  } else if (Array.isArray(lastMessage.content)) {
    const textBlock = lastMessage.content.find((b) => b.type === "text");
    content = textBlock?.text || "";
  }

  if (!content && req.tools && req.tools.length > 0) {
    content = "执行工具任务";
  }

  const images = extractImages(lastMessage.content);

  // Build history from previous messages (matching Go implementation)
  const history: unknown[] = [];

  // Add system messages to history if present
  if (req.system) {
    const systemContentParts: string[] = [];
    
    if (typeof req.system === "string") {
      systemContentParts.push(req.system);
    } else if (Array.isArray(req.system)) {
      for (const sysMsg of req.system) {
        if (typeof sysMsg === "string") {
          systemContentParts.push(sysMsg);
        } else if (typeof sysMsg === "object" && sysMsg.type === "text" && sysMsg.text) {
          systemContentParts.push(sysMsg.text);
        }
      }
    }

    if (systemContentParts.length > 0) {
      history.push({
        userInputMessage: {
          content: systemContentParts.join("\n").trim(),
          modelId,
          origin: DEFAULTS.ORIGIN,
          images: [],
          userInputMessageContext: {},
        },
      });

      history.push({
        assistantResponseMessage: {
          content: "OK",
          toolUses: null,
        },
      });
    }
  }

  // Buffer for collecting consecutive user messages
  let userMessagesBuffer: typeof req.messages = [];

  for (let i = 0; i < req.messages.length - 1; i++) {
    const msg = req.messages[i];

    if (msg.role === "user") {
      // Collect user messages in buffer
      userMessagesBuffer.push(msg);
      continue;
    }

    if (msg.role === "assistant") {
      // Process accumulated user messages when we encounter an assistant message
      if (userMessagesBuffer.length > 0) {
        // Merge all accumulated user messages
        const contentParts: string[] = [];
        let allImages: Array<{ format: string; source: { bytes: string } }> = [];
        let allToolResults: unknown[] = [];

        for (const userMsg of userMessagesBuffer) {
          const messageContent = extractTextContent(userMsg.content);
          const messageImages = extractImages(userMsg.content);

          if (messageContent) {
            contentParts.push(messageContent);
          }
          if (messageImages.length > 0) {
            allImages = allImages.concat(messageImages);
          }

          // Collect tool results
          const toolResults = extractToolResults(userMsg.content);
          if (toolResults.length > 0) {
            allToolResults = allToolResults.concat(toolResults);
          }
        }

        // Build merged user message
        const userInputMessageContext: Record<string, unknown> = {};
        if (allToolResults.length > 0) {
          userInputMessageContext.toolResults = allToolResults;
        }

        history.push({
          userInputMessage: {
            content: allToolResults.length > 0 ? "" : contentParts.join("\n"),
            modelId,
            origin: DEFAULTS.ORIGIN,
            images: allImages,
            userInputMessageContext,
          },
        });

        // Clear buffer
        userMessagesBuffer = [];

        // Add assistant message
        const textContent = extractTextContent(msg.content);
        const toolUses = extractToolUses(msg.content);

        const validToolUses = toolUses;

        history.push({
          assistantResponseMessage: {
            content: textContent || "",
            toolUses: validToolUses.length > 0 ? validToolUses : null,
          },
        });
      } else {
        // Orphaned assistant message - warn and skip
        logger.warn("孤立的助手消息，跳过", logger.Int("index", i));
      }
    }
  }

  // Handle orphaned user messages at the end
  if (userMessagesBuffer.length > 0) {
    logger.warn(
      "历史末尾的孤立用户消息，自动配对响应",
      logger.Int("message_count", userMessagesBuffer.length),
    );

    const contentParts: string[] = [];
    let allImages: Array<{ format: string; source: { bytes: string } }> = [];
    let allToolResults: unknown[] = [];

    for (const userMsg of userMessagesBuffer) {
      const messageContent = extractTextContent(userMsg.content);
      const messageImages = extractImages(userMsg.content);

      if (messageContent) {
        contentParts.push(messageContent);
      }
      if (messageImages.length > 0) {
        allImages = allImages.concat(messageImages);
      }

      const toolResults = extractToolResults(userMsg.content);
      if (toolResults.length > 0) {
        allToolResults = allToolResults.concat(toolResults);
      }
    }

    const userInputMessageContext: Record<string, unknown> = {};
    if (allToolResults.length > 0) {
      userInputMessageContext.toolResults = allToolResults;
    }

    history.push({
      userInputMessage: {
        content: allToolResults.length > 0 ? "" : contentParts.join("\n"),
        modelId,
        origin: DEFAULTS.ORIGIN,
        images: allImages,
        userInputMessageContext,
      },
    });

    // Auto-pair with OK response
    history.push({
      assistantResponseMessage: {
        content: "OK",
        toolUses: null,
      },
    });
  }

  const userInputMessageContext: Record<string, unknown> = {};
  if (req.tools && req.tools.length > 0) {
    userInputMessageContext.tools = convertToolsToCodeWhisperer(req.tools);
  }

  // Add tool results from current message if present
  const currentToolResults = extractToolResults(lastMessage.content);
  if (currentToolResults.length > 0) {
    userInputMessageContext.toolResults = currentToolResults;
    // When tool results are present, content should be empty
    content = "";
  }

const cwReq: CodeWhispererRequest = {
  conversationState: {
    agentContinuationId: effectiveAgentContinuationId,
    agentTaskType: DEFAULTS.AGENT_TASK_TYPE,
    chatTriggerType: determineChatTriggerType(req),
    currentMessage: {
      userInputMessage: {
        userInputMessageContext,
        content,
        modelId,
        images,
        origin: DEFAULTS.ORIGIN,
      },
    },
    conversationId,
    history,
  },
};

  // Validate request before sending
  try {
    validateCodeWhispererRequest(cwReq);
    
    logger.debug(
      "请求转换完成",
      logger.String("conversation_id", conversationId),
      logger.Int("history_length", history.length),
      logger.Bool("has_images", images.length > 0),
      logger.Bool("has_tool_results", currentToolResults.length > 0),
    );
  } catch (error) {
    logger.error(
      "请求验证失败",
      logger.Err(error),
      logger.LazyJson("request", cwReq),
    );
    throw error;
  }

  return cwReq;
}

// Convert OpenAI request to Anthropic format
export function openAIToAnthropic(req: OpenAIRequest): AnthropicRequest {
  // Convert and filter tools
  const tools = req.tools ? validateAndProcessTools(req.tools) : undefined;

  // Convert tool_choice
  const toolChoice = req.tool_choice
    ? convertOpenAIToolChoiceToAnthropic(req.tool_choice)
    : undefined;

  // Convert messages with content block transformation
  const messages = req.messages.map((msg) => {
    let content = msg.content;

    // Convert content blocks if needed
    if (Array.isArray(content)) {
      content = convertOpenAIContentToAnthropic(content);
    }

    return {
      role: msg.role,
      content: content as string | ContentBlock[],
    };
  });

  // Extract system messages from messages array (OpenAI format)
const systemMessages: Array<{ type: string; text: string }> = [];
const nonSystemMessages = messages.filter((msg) => {
  if (msg.role === "system") {
    if (typeof msg.content === "string") {
      systemMessages.push({ type: "text", text: msg.content });
    } else if (Array.isArray(msg.content)) {
      const text = (msg.content as Array<{ type: string; text: string }>)
        .filter(b => b && b.type === "text" && typeof b.text === "string")
        .map(b => b.text)
        .join("");
      if (text) systemMessages.push({ type: "text", text });
    }
    return false;
  }
  return true;
});

  return {
    model: req.model,
    max_tokens: req.max_tokens || DEFAULTS.MAX_TOKENS,
    messages: nonSystemMessages,
    stream: req.stream ?? false,
    temperature: req.temperature,
    system: systemMessages.length > 0 ? systemMessages : undefined,
    tools,
    tool_choice: toolChoice as string | ToolChoice | undefined,
  };
}

// Extract text content from message
function extractTextContent(content: unknown): string {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .filter((b): b is Record<string, unknown> => typeof b === "object" && b !== null && (b as Record<string, unknown>).type === "text")
      .map(b => b.text as string)
      .join("");
  }
  return "";
}

// Extract images from content
function extractImages(content: unknown) {
  if (!Array.isArray(content)) return [];
  
return content
  .filter((b): b is Record<string, unknown> => {
    if (typeof b !== "object" || b === null) return false;
    const obj = b as Record<string, unknown>;
    return obj.type === "image" && !!obj.source && validateImageContent(obj.source);
  })
  .map(b => {
    const source = b.source as Record<string, unknown>;
    const mediaType = source.media_type as string;
    return {
      format: mediaType?.split("/")[1] || "png",
      source: { bytes: source.data as string }
    };
  });
}

// Extract tool results from content
function extractToolResults(content: unknown) {
  if (!Array.isArray(content)) return [];

  return content
    .filter((b): b is Record<string, unknown> => {
      if (typeof b !== "object" || b === null) return false;
      const obj = b as Record<string, unknown>;
      return obj.type === "tool_result" && !!obj.tool_use_id;
    })
    .map(b => {
      let contentArray: unknown[] = [];

      if (b.content !== undefined && b.content !== null) {
        if (typeof b.content === "string") {
          contentArray = [{ text: b.content }];
        } else if (Array.isArray(b.content)) {
          contentArray = b.content.map((item: unknown) => 
            typeof item === "string" ? { text: item } : item
          ).filter((item: unknown) => item);
        } else if (typeof b.content === "object") {
          contentArray = [b.content];
        } else {
          contentArray = [{ text: String(b.content) }];
        }
      }

      if (contentArray.length === 0) contentArray = [{ text: "" }];

      return {
        toolUseId: b.tool_use_id as string,
        content: contentArray,
        status: b.is_error ? "error" : "success",
        isError: (b.is_error as boolean) || false,
      };
    });
}

// Extract tool uses from content
function extractToolUses(content: unknown) {
  if (typeof content === "string" || !Array.isArray(content)) return [];

  return content
    .filter((b): b is Record<string, unknown> => {
      if (typeof b !== "object" || b === null) return false;
      const obj = b as Record<string, unknown>;
      return obj.type === "tool_use" && 
        !!obj.id && 
        !!obj.name && 
        obj.name !== "web_search" && 
        obj.name !== "websearch";
    })
    .map(b => ({
      toolUseId: b.id as string,
      name: b.name as string,
      input: (b.input && typeof b.input === "object" && !Array.isArray(b.input)) ? b.input as Record<string, unknown> : {},
    }));
}

// Convert Anthropic tools to CodeWhisperer format
function convertToolsToCodeWhisperer(tools: unknown[]) {
  return (tools as Array<{ name: string; description: string; input_schema: unknown }>)
    .filter(t => t.name !== "web_search" && t.name !== "websearch")
    .map(t => ({
      toolSpecification: {
        name: t.name,
        description: t.description || "",
        inputSchema: { json: t.input_schema },
      },
    }));
}



// Convert OpenAI content blocks to Anthropic format
function convertOpenAIContentToAnthropic(content: unknown[]): unknown[] {
  return content.map(block => {
    if (typeof block !== "object" || block === null) return block;
    const b = block as Record<string, unknown>;
    
    if (!b.type) return block;

    if (b.type === "image_url" && b.image_url && typeof b.image_url === "object") {
      const imageUrl = b.image_url as Record<string, unknown>;
      const url = imageUrl.url as string;
      if (url && url.startsWith("data:")) {
        const match = url.match(/^data:image\/(\w+);base64,(.+)$/);
        if (match) {
          return {
            type: "image",
            source: { type: "base64", media_type: `image/${match[1]}`, data: match[2] },
          };
        }
      }
      if (url) {
        logger.warn("跳过非base64图片URL", logger.String("url", url.substring(0, 50)));
      }
      return null;
    }

    if (b.type === "tool_use" && (b.name === "web_search" || b.name === "websearch")) {
      return null;
    }

    return block;
  }).filter(b => b !== null);
}

// Parse content block from raw object
export function parseContentBlock(block: Record<string, unknown>): ContentBlock {
  const blockType = block.type;
  if (!blockType || typeof blockType !== "string") {
    throw new Error("Content block missing type field");
  }

  const contentBlock: ContentBlock = { type: blockType };

  switch (blockType) {
    case "text":
      if (typeof block.text === "string") {
        contentBlock.text = block.text;
      } else {
        logger.warn("Text block missing text field or not a string");
      }
      break;

    case "image":
      if (block.source && typeof block.source === "object") {
        const src = block.source as Record<string, unknown>;
        contentBlock.source = {
          type: (src.type as string) || "base64",
          media_type: src.media_type as string,
          data: src.data as string,
        };
      }
      break;

    case "image_url":
      // Convert OpenAI image_url to Anthropic format
      if (block.image_url && typeof block.image_url === "object") {
        const imageUrl = block.image_url as Record<string, unknown>;
        const url = imageUrl.url as string;
        if (url && url.startsWith("data:")) {
          const match = url.match(/^data:image\/(\w+);base64,(.+)$/);
          if (match) {
            contentBlock.type = "image";
            contentBlock.source = {
              type: "base64",
              media_type: `image/${match[1]}`,
              data: match[2],
            };
          }
        }
      }
      break;

    case "tool_result":
      contentBlock.tool_use_id = block.tool_use_id as string;
      contentBlock.content = block.content;
      contentBlock.is_error = (block.is_error as boolean) || false;
      break;

    case "tool_use":
      contentBlock.id = block.id as string;
      contentBlock.name = block.name as string;
      contentBlock.input = block.input as Record<string, unknown> || {};
      break;
  }

  return contentBlock;
}

// Generate unique IDs
export function generateId(prefix: string): string {
  return `${prefix}_${crypto.randomUUID().replace(/-/g, "")}`;
}
