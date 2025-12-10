import * as logger from "../logger/logger.ts";

// 生成UUID
function generateUUID(): string {
  return crypto.randomUUID();
}

// RequestIDMiddleware 为每个请求注入request_id
export function requestIDMiddleware(req: Request): string {
  const requestId = req.headers.get("X-Request-ID") || `req_${generateUUID()}`;
  return requestId;
}

// 验证API密钥
export function validateAPIKey(req: Request, authToken: string): boolean {
  let apiKey = req.headers.get("Authorization") || req.headers.get("x-api-key");
  
  if (!apiKey) {
    logger.warn("请求缺少Authorization或x-api-key头");
    return false;
  }

  if (apiKey.startsWith("Bearer ")) {
    apiKey = apiKey.substring(7);
  }

  if (apiKey !== authToken) {
    logger.error("authToken验证失败");
    return false;
  }

  return true;
}

// 检查路径是否需要认证
export function requiresAuth(path: string, protectedPrefixes: string[]): boolean {
  return protectedPrefixes.some(prefix => path.startsWith(prefix));
}

// CORS头
export function getCORSHeaders(): HeadersInit {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type, Authorization, x-api-key",
  };
}
