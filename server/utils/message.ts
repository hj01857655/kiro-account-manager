export function parseToolResultContent(content: unknown): string {
  if (content === null || content === undefined) return "No content provided";

  if (typeof content === "string") {
    return content === "" ? "Tool executed with no output" : content;
  }

  if (Array.isArray(content)) {
    if (content.length === 0) return "Tool executed with empty result list";

    const parts: string[] = [];
    for (const item of content) {
      if (typeof item === "object" && item !== null) {
        const obj = item as Record<string, unknown>;
        if (obj.type === "text" && typeof obj.text === "string" && obj.text !== "") {
          parts.push(obj.text);
        } else if (typeof obj.text === "string" && obj.text !== "") {
          parts.push(obj.text);
        } else {
          parts.push(JSON.stringify(item));
        }
      } else if (typeof item === "string" && item !== "") {
        parts.push(item);
      } else {
        parts.push(String(item));
      }
    }

    const result = parts.join("\n").trim();
    return result === "" ? "Tool executed with empty content" : result;
  }

  if (typeof content === "object") {
    const obj = content as Record<string, unknown>;
    if (obj.type === "text" && typeof obj.text === "string") {
      return obj.text === "" ? "Tool executed with empty text" : obj.text;
    }
    if (typeof obj.text === "string") {
      return obj.text === "" ? "Tool executed with empty text field" : obj.text;
    }
    return JSON.stringify(content);
  }

  return String(content);
}

export function getMessageContent(content: unknown): string {
  if (typeof content === "string") {
    return content.length === 0 ? "answer for user question" : content;
  }

  if (Array.isArray(content)) {
    const texts: string[] = [];
    let hasImage = false;

    for (const block of content) {
      if (typeof block === "object" && block !== null) {
        const cb = block as Record<string, unknown>;

        if (cb.type === "tool_result") {
          let toolResultContent = parseToolResultContent(cb.content);
          if (cb.is_error === true) {
            toolResultContent = "Tool Error: " + toolResultContent;
          }
          if (typeof cb.tool_use_id === "string" && cb.tool_use_id !== "") {
            toolResultContent = `Tool result for ${cb.tool_use_id}: ${toolResultContent}`;
          }
          texts.push(toolResultContent);
        } else if (cb.type === "text" && typeof cb.text === "string") {
          texts.push(cb.text);
        } else if (cb.type === "image") {
          hasImage = true;
          if (cb.source && typeof cb.source === "object") {
            const source = cb.source as Record<string, unknown>;
            texts.push(`[图片: ${source.media_type || "unknown"}格式]`);
          } else {
            texts.push("[图片]");
          }
        }
      }
    }

    if (texts.length === 0 && hasImage) return "请描述这张图片的内容";
    if (texts.length === 0) return "answer for user question";
    return texts.join("\n");
  }

  return "answer for user question";
}
