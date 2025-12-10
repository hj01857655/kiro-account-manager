// Message status enum
export enum MessageStatus {
  COMPLETED = "COMPLETED",
  IN_PROGRESS = "IN_PROGRESS",
  ERROR = "ERROR",
}

// User intent enum
export enum UserIntent {
  EXPLAIN_CODE_SELECTION = "EXPLAIN_CODE_SELECTION",
  SUGGEST_ALTERNATE_IMPLEMENTATION = "SUGGEST_ALTERNATE_IMPLEMENTATION",
  APPLY_COMMON_BEST_PRACTICES = "APPLY_COMMON_BEST_PRACTICES",
  IMPROVE_CODE = "IMPROVE_CODE",
  SHOW_EXAMPLES = "SHOW_EXAMPLES",
  CITE_SOURCES = "CITE_SOURCES",
  EXPLAIN_LINE_BY_LINE = "EXPLAIN_LINE_BY_LINE",
}

// Content type enum
export enum ContentType {
  MARKDOWN = "text/markdown",
  PLAIN = "text/plain",
  JSON = "application/json",
}
