import type { ContentType, MessageStatus, UserIntent } from "./codewhisperer_enums.ts";

export interface CodeWhispererImage {
  format: string;
  source: {
    bytes: string;
  };
}

export interface ToolSpecification {
  name: string;
  description: string;
  inputSchema: {
    json: Record<string, unknown>;
  };
}

export interface CodeWhispererTool {
  toolSpecification: ToolSpecification;
}

export interface ToolResult {
  toolUseId: string;
  content: Array<Record<string, unknown>>;
  status: string;
  isError?: boolean;
}

export interface ToolUseEntry {
  toolUseId: string;
  name: string;
  input: Record<string, unknown>;
}

// History user message
export interface HistoryUserMessage {
  userInputMessage: {
    content: string;
    modelId: string;
    origin: string;
    images?: CodeWhispererImage[];
    userInputMessageContext: {
      toolResults?: ToolResult[];
      tools?: CodeWhispererTool[];
    };
  };
}

// History assistant message
export interface HistoryAssistantMessage {
  assistantResponseMessage: {
    content: string;
    toolUses: ToolUseEntry[];
  };
}

// CodeWhisperer request structure
export interface CodeWhispererRequest {
  conversationState: {
    agentContinuationId: string;
    agentTaskType: string;
    chatTriggerType: string;
    currentMessage: {
      userInputMessage: {
        userInputMessageContext: {
          toolResults?: ToolResult[];
          tools?: CodeWhispererTool[];
        };
        content: string;
        modelId: string;
        images: CodeWhispererImage[];
        origin: string;
      };
    };
    conversationId: string;
    history: unknown[];
  };
}

// CodeWhisperer event
export interface CodeWhispererEvent {
  "content-type": string;
  "message-type": string;
  content: string;
  "event-type": string;
}

// Content span
export interface ContentSpan {
  start: number;
  end: number;
}

// Supplementary web link
export interface SupplementaryWebLink {
  url: string;
  title?: string;
  snippet?: string;
  score?: number;
}

// Most relevant missed alternative
export interface MostRelevantMissedAlternative {
  url: string;
  licenseName?: string;
  repository?: string;
}

// Reference
export interface Reference {
  licenseName?: string;
  repository?: string;
  url?: string;
  information?: string;
  recommendationContentSpan?: ContentSpan;
  mostRelevantMissedAlternative?: MostRelevantMissedAlternative;
}

// Followup prompt
export interface FollowupPrompt {
  content: string;
  userIntent?: UserIntent;
}

// Programming language
export interface ProgrammingLanguage {
  languageName: string;
}

// Customization
export interface Customization {
  arn: string;
  name?: string;
}

// Code query
export interface CodeQuery {
  codeQueryId: string;
  programmingLanguage?: ProgrammingLanguage;
  userInputMessageId?: string;
}

// AWS CodeWhisperer assistant response event complete structure
export interface AssistantResponseEvent {
  // Core fields (optional to support streaming/partial events as in Go)
  conversationId?: string;
  messageId?: string;
  content?: string;
  contentType?: ContentType;
  messageStatus?: MessageStatus;

  // Reference and link fields
  supplementaryWebLinks?: SupplementaryWebLink[];
  references?: Reference[];
  codeReference?: Reference[];

  // Interaction fields
  followupPrompt?: FollowupPrompt;

  // Context fields
  programmingLanguage?: ProgrammingLanguage;
  customizations?: Customization[];
  userIntent?: UserIntent;
  codeQuery?: CodeQuery;
}

// Convert plain object to AssistantResponseEvent (align with Go FromDict)
export function assistantResponseEventFromDict(data: Record<string, unknown>): AssistantResponseEvent {
  const are: AssistantResponseEvent = {};

  if (typeof data.conversationId === "string") are.conversationId = data.conversationId;
  if (typeof data.messageId === "string") are.messageId = data.messageId;
  if (typeof data.content === "string") are.content = data.content;

  if (typeof data.contentType === "string") {
    are.contentType = data.contentType as ContentType;
  } else {
    // default like Go: text/markdown
    are.contentType = "text/markdown" as ContentType;
  }

  if (typeof data.messageStatus === "string") {
    are.messageStatus = data.messageStatus as MessageStatus;
  } else {
    // default like Go: COMPLETED
    are.messageStatus = "COMPLETED" as MessageStatus;
  }

  if (typeof data.userIntent === "string") are.userIntent = data.userIntent as UserIntent;

  if (Array.isArray(data.supplementaryWebLinks)) {
    are.supplementaryWebLinks = data.supplementaryWebLinks as SupplementaryWebLink[];
  }
  if (Array.isArray(data.references)) {
    are.references = data.references as Reference[];
  }
  if (Array.isArray(data.codeReference)) {
    are.codeReference = data.codeReference as Reference[];
  }

  if (data.followupPrompt && typeof data.followupPrompt === "object") {
    are.followupPrompt = data.followupPrompt as FollowupPrompt;
  }

  if (data.programmingLanguage && typeof data.programmingLanguage === "object") {
    are.programmingLanguage = data.programmingLanguage as ProgrammingLanguage;
  }

  if (Array.isArray(data.customizations)) {
    are.customizations = data.customizations as Customization[];
  }

  if (data.codeQuery && typeof data.codeQuery === "object") {
    are.codeQuery = data.codeQuery as CodeQuery;
  }

  return are;
}

// Convert AssistantResponseEvent to plain object (align with Go ToDict)
export function assistantResponseEventToDict(event: AssistantResponseEvent): Record<string, unknown> {
  const result: Record<string, unknown> = {};

  if (event.conversationId) result.conversationId = event.conversationId;
  if (event.messageId) result.messageId = event.messageId;
  if (event.content !== undefined) result.content = event.content;
  if (event.contentType) result.contentType = event.contentType;
  if (event.messageStatus) result.messageStatus = event.messageStatus;

  if (event.supplementaryWebLinks?.length) result.supplementaryWebLinks = event.supplementaryWebLinks;
  if (event.references?.length) result.references = event.references;
  if (event.codeReference?.length) result.codeReference = event.codeReference;

  if (event.followupPrompt) result.followupPrompt = event.followupPrompt;
  if (event.programmingLanguage) result.programmingLanguage = event.programmingLanguage;
  if (event.customizations?.length) result.customizations = event.customizations;
  if (event.userIntent) result.userIntent = event.userIntent;
  if (event.codeQuery) result.codeQuery = event.codeQuery;

  return result;
}

// Validation function (enhanced to validate enum values like Go's Validate)
export function validateAssistantResponseEvent(
  event: Partial<AssistantResponseEvent>
): boolean {
  // For streaming responses, only content is required
  if (!event.conversationId && !event.messageId && event.content) {
    return true;
  }

  // For tool call events, only tool fields are required
  if (!event.conversationId && !event.messageId && event.codeQuery) {
    return true;
  }

  // Check if has any valid content
  const hasValidContent = !!(event.content ||
    event.codeQuery ||
    (event.supplementaryWebLinks && event.supplementaryWebLinks.length > 0) ||
    (event.references && event.references.length > 0) ||
    (event.codeReference && event.codeReference.length > 0) ||
    event.followupPrompt);

  if (!hasValidContent) {
    if (!event.conversationId || !event.messageId) {
      return false;
    }
  }

  // Enum validations
  if (event.messageStatus && !["COMPLETED","IN_PROGRESS","ERROR"].includes(event.messageStatus)) {
    return false;
  }
  if (event.contentType && !["text/markdown","text/plain","application/json"].includes(event.contentType)) {
    return false;
  }
  if (event.userIntent && ![
    "EXPLAIN_CODE_SELECTION",
    "SUGGEST_ALTERNATE_IMPLEMENTATION",
    "APPLY_COMMON_BEST_PRACTICES",
    "IMPROVE_CODE",
    "SHOW_EXAMPLES",
    "CITE_SOURCES",
    "EXPLAIN_LINE_BY_LINE",
  ].includes(event.userIntent)) {
    return false;
  }

  return true;
}
