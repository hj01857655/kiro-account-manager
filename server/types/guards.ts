/**
 * Type Guard Functions
 * 
 * 提供运行时类型验证，减少类型断言和运行时错误
 * 遵循 TypeScript 类型守卫最佳实践
 */

/**
 * 检查值是否为非空对象
 */
export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * 检查值是否为字符串
 */
export function isString(value: unknown): value is string {
  return typeof value === "string";
}

/**
 * 检查值是否为数字
 */
export function isNumber(value: unknown): value is number {
  return typeof value === "number" && !isNaN(value);
}

/**
 * 检查值是否为布尔值
 */
export function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

/**
 * 检查值是否为数组
 */
export function isArray(value: unknown): value is unknown[] {
  return Array.isArray(value);
}

/**
 * 检查值是否为Error实例
 */
export function isError(value: unknown): value is Error {
  return value instanceof Error;
}

/**
 * 检查对象是否包含指定属性
 */
export function hasProperty<K extends string>(
  obj: unknown,
  key: K,
): obj is Record<K, unknown> {
  return isObject(obj) && key in obj;
}

/**
 * 检查对象是否包含字符串属性
 */
export function hasStringProperty<K extends string>(
  obj: unknown,
  key: K,
): obj is Record<K, string> {
  return hasProperty(obj, key) && isString(obj[key]);
}

/**
 * 检查对象是否包含数字属性
 */
export function hasNumberProperty<K extends string>(
  obj: unknown,
  key: K,
): obj is Record<K, number> {
  return hasProperty(obj, key) && isNumber(obj[key]);
}

/**
 * 检查是否为有效的 AWS EventStream 消息
 */
export interface EventStreamMessage {
  payload: Uint8Array;
}

export function isEventStreamMessage(value: unknown): value is EventStreamMessage {
  return isObject(value) && 
         hasProperty(value, "payload") && 
         value.payload instanceof Uint8Array;
}

/**
 * 检查是否为有效的 AssistantResponseEvent
 */
export interface AssistantResponseEvent {
  content?: string;
  toolUseId?: string;
  name?: string;
  input?: unknown;
  stop?: boolean;
  [key: string]: unknown;
}

export function isAssistantResponseEvent(value: unknown): value is AssistantResponseEvent {
  if (!isObject(value)) return false;
  
  // 至少需要有一个有效字段
  return hasProperty(value, "content") ||
         hasProperty(value, "toolUseId") ||
         hasProperty(value, "name") ||
         hasProperty(value, "stop");
}

/**
 * 检查是否为包含 assistantResponseEvent 的 payload
 */
export interface PayloadWithAssistantEvent {
  assistantResponseEvent: AssistantResponseEvent;
}

export function hasAssistantResponseEvent(value: unknown): value is PayloadWithAssistantEvent {
  return hasProperty(value, "assistantResponseEvent") && 
         isAssistantResponseEvent(value.assistantResponseEvent);
}

/**
 * 检查是否为工具使用对象
 */
export interface ToolUse {
  type: string;
  id: string;
  name: string;
  input: unknown;
}

export function isToolUse(value: unknown): value is ToolUse {
  return isObject(value) &&
         hasStringProperty(value, "type") &&
         value.type === "tool_use" &&
         hasStringProperty(value, "id") &&
         hasStringProperty(value, "name") &&
         hasProperty(value, "input");
}

/**
 * 检查是否为历史消息对象
 */
export interface HistoryMessage {
  assistantResponseMessage?: {
    toolUses?: unknown[];
  };
}

export function isHistoryMessage(value: unknown): value is HistoryMessage {
  if (!isObject(value)) return false;
  
  if (!hasProperty(value, "assistantResponseMessage")) return true;
  
  const arm = value.assistantResponseMessage;
  if (!isObject(arm)) return false;
  
  if (!hasProperty(arm, "toolUses")) return true;
  
  return isArray(arm.toolUses);
}

/**
 * 检查是否为有效的内容块
 */
export interface ContentBlock {
  type: string;
  text?: string;
  id?: string;
  name?: string;
  input?: unknown;
}

export function isContentBlock(value: unknown): value is ContentBlock {
  return isObject(value) && hasStringProperty(value, "type");
}

/**
 * 检查是否为有效的 token 缓存
 */
export interface TokenCache {
  token: Record<string, unknown>;
  configIndex: number;
  cachedAt: Date;
  lastUsed: Date;
  available: number;
  usageInfo?: unknown;
}

export function isTokenCache(value: unknown): value is TokenCache {
  if (!isObject(value)) return false;
  
  return hasProperty(value, "token") &&
         isObject(value.token) &&
         hasNumberProperty(value, "configIndex") &&
         hasProperty(value, "cachedAt") &&
         value.cachedAt instanceof Date &&
         hasProperty(value, "lastUsed") &&
         value.lastUsed instanceof Date &&
         hasNumberProperty(value, "available");
}

/**
 * 安全的 JSON 解析，返回类型守卫
 */
export function parseJsonSafely(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

/**
 * 安全获取对象属性值
 */
export function getProperty<T = unknown>(
  obj: unknown,
  key: string,
  defaultValue: T,
): T {
  if (!hasProperty(obj, key)) return defaultValue;
  return obj[key] as T;
}

/**
 * 安全获取字符串属性
 */
export function getStringProperty(
  obj: unknown,
  key: string,
  defaultValue = "",
): string {
  if (!hasStringProperty(obj, key)) return defaultValue;
  return obj[key];
}

/**
 * 安全获取数字属性
 */
export function getNumberProperty(
  obj: unknown,
  key: string,
  defaultValue = 0,
): number {
  if (!hasNumberProperty(obj, key)) return defaultValue;
  return obj[key];
}

/**
 * 验证并提取工具输入
 */
export function extractToolInput(value: unknown): Record<string, unknown> {
  if (isObject(value)) return value;
  
  // 尝试解析字符串
  if (isString(value)) {
    const parsed = parseJsonSafely(value);
    if (isObject(parsed)) return parsed;
  }
  
  // 返回空对象作为fallback
  return {};
}
