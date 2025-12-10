import { createHash } from "https://deno.land/std@0.208.0/crypto/mod.ts";
import { generateUUID } from "./uuid.ts";

function getLocalHourWindow(): string {
  const d = new Date();
  const yyyy = d.getFullYear();
  const MM = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const HH = String(d.getHours()).padStart(2, "0");
  return `${yyyy}${MM}${dd}${HH}`;
}

export class ConversationIDManager {
  private cache = new Map<string, string>();

  generateConversationID(clientIP: string, userAgent: string, customConvID?: string): string {
    if (customConvID) return customConvID;

    const timeWindow = getLocalHourWindow();
    const clientSignature = `${clientIP}|${userAgent}|${timeWindow}`;

    const cached = this.cache.get(clientSignature);
    if (cached) return cached;

    const hash = createHash("md5").update(clientSignature).digest();
    const conversationID = `conv-${
      Array.from(hash.slice(0, 8)).map((b) => b.toString(16).padStart(2, "0")).join("")
    }`;

    this.cache.set(clientSignature, conversationID);
    return conversationID;
  }

  invalidateOldSessions(): void {
    this.cache.clear();
  }
}

export const globalConversationIDManager = new ConversationIDManager();

export function generateStableConversationID(
  clientIP: string,
  userAgent: string,
  customConvID?: string,
): string {
  return globalConversationIDManager.generateConversationID(clientIP, userAgent, customConvID);
}

export function generateStableAgentContinuationID(
  clientIP?: string,
  userAgent?: string,
  customAgentID?: string,
): string {
  if (!clientIP || !userAgent) return generateUUID();
  if (customAgentID) return customAgentID;

  const timeWindow = getLocalHourWindow();
  const clientSignature = `agent|${clientIP}|${userAgent}|${timeWindow}`;

  return generateDeterministicGUID(clientSignature, "agent");
}

function generateDeterministicGUID(input: string, namespace: string): string {
  const namespacedInput = `${namespace}|${input}`;
  const hash = createHash("md5").update(namespacedInput).digest();

  hash[6] = (hash[6] & 0x0f) | 0x50; // Version 5
  hash[8] = (hash[8] & 0x3f) | 0x80; // Variant bits

  const hex = Array.from(hash).map((b) => b.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex.slice(6, 8).join("")}-${
    hex.slice(8, 10).join("")
  }-${hex.slice(10, 16).join("")}`;
}
