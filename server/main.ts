import { AuthService } from "./auth/auth_service.ts";
import {
  handleMessages,
  handleModels,
  handleTokenStatus,
} from "./server/handlers.ts";
import {
  handleGetTokens,
  handleAddToken,
  handleDeleteToken,
  handleImportTokens,
  handleClearTokens,
} from "./routes/token_admin.ts";
import { handleCountTokens } from "./server/count_tokens_handler.ts";
import { handleOpenAINonStreamRequest } from "./server/openai_handlers.ts";
import { handleOpenAIStreamRequest } from "./server/openai_stream_processor.ts";
import { createRequestContext } from "./server/request_context.ts";
import { openAIToAnthropic } from "./converter/converter.ts";
import { respondError } from "./server/common.ts";
import type { OpenAIRequest } from "./types/openai.ts";
import { requestIDMiddleware, validateAPIKey, requiresAuth, getCORSHeaders } from "./server/middleware.ts";
import { DEFAULTS } from "./config/constants.ts";
import * as logger from "./logger/logger.ts";
import { join, normalize, resolve } from "https://deno.land/std@0.208.0/path/mod.ts";

// Middleware to check authorization
function checkAuth(req: Request, clientToken: string): boolean {
  const url = new URL(req.url);
  const protectedPrefixes = ["/v1"];
  
  // Skip auth for non-protected endpoints
  if (!requiresAuth(url.pathname, protectedPrefixes)) {
    return true;
  }

  return validateAPIKey(req, clientToken);
}

/**
 * 安全地提供静态文件服务
 * 防止路径遍历攻击
 */
async function serveStaticFile(pathname: string): Promise<Response> {
  try {
    // Remove leading slash and "static/" prefix if present
    const filePath = pathname.startsWith("/static/")
      ? pathname.substring("/static/".length)
      : pathname.substring(1);
    
    // 规范化路径，移除 .. 和 . 等
    const normalizedPath = normalize(filePath);
    
    // 检查是否包含路径遍历尝试
    if (normalizedPath.includes("..") || normalizedPath.startsWith("/")) {
      logger.warn(
        "路径遍历攻击尝试被阻止",
        logger.String("requested_path", pathname),
        logger.String("normalized", normalizedPath)
      );
      return new Response("Forbidden", { status: 403 });
    }
    
    // 使用 import.meta.resolve 获取嵌入文件的 URL
    const fileUrl = import.meta.resolve(`./static/${normalizedPath}`);
    const file = await Deno.readFile(new URL(fileUrl));
    
    // Determine content type based on file extension
    const ext = normalizedPath.split(".").pop()?.toLowerCase();
    const contentTypes: Record<string, string> = {
      "html": "text/html; charset=utf-8",
      "css": "text/css; charset=utf-8",
      "js": "application/javascript; charset=utf-8",
      "json": "application/json",
      "png": "image/png",
      "jpg": "image/jpeg",
      "jpeg": "image/jpeg",
      "gif": "image/gif",
      "svg": "image/svg+xml",
      "ico": "image/x-icon",
    };
    
    const contentType = contentTypes[ext || ""] || "application/octet-stream";
    
    return new Response(file, {
      status: 200,
      headers: {
        "Content-Type": contentType,
        "Cache-Control": "public, max-age=3600",
        "X-Content-Type-Options": "nosniff", // 安全头：防止 MIME 类型嗅探
      },
    });
  } catch (error) {
    // 详细的错误分类
    if (error instanceof Deno.errors.NotFound) {
      logger.debug("静态文件不存在", logger.String("path", pathname));
      return new Response("Not Found", { status: 404 });
    } else if (error instanceof Deno.errors.PermissionDenied) {
      logger.error(
        "文件权限错误",
        logger.String("path", pathname),
        logger.Err(error)
      );
      return new Response("Forbidden", { status: 403 });
    } else {
      logger.error(
        "读取静态文件失败",
        logger.String("path", pathname),
        logger.Err(error)
      );
      return new Response("Internal Server Error", { status: 500 });
    }
  }
}

// Main request handler
async function handleRequest(
  req: Request,
  authService: AuthService,
  clientToken: string,
): Promise<Response> {
  const url = new URL(req.url);

  // Generate request ID
  const requestId = requestIDMiddleware(req);
  
  // CORS headers
  const corsHeaders = getCORSHeaders();

  // Handle OPTIONS preflight
  if (req.method === "OPTIONS") {
    return new Response(null, { headers: corsHeaders });
  }

  // Check authentication
  if (!checkAuth(req, clientToken)) {
    return new Response(JSON.stringify({ error: "Unauthorized" }), {
      status: 401,
      headers: { ...corsHeaders, "Content-Type": "application/json" },
    });
  }

  // Route handling
  try {
    let response: Response;

    if (url.pathname === "/v1/models" && req.method === "GET") {
      response = await handleModels();
    } else if (url.pathname === "/api/tokens" && req.method === "GET") {
      response = await handleTokenStatus(authService);
    } else if (url.pathname === "/api/admin/tokens" && req.method === "GET") {
      response = await handleGetTokens(req, authService);
    } else if (url.pathname === "/api/admin/tokens" && req.method === "POST") {
      response = await handleAddToken(req, authService);
    } else if (url.pathname === "/api/admin/tokens" && req.method === "DELETE") {
      response = await handleDeleteToken(req, authService);
    } else if (url.pathname === "/api/admin/tokens/import" && req.method === "POST") {
      response = await handleImportTokens(req, authService);
    } else if (url.pathname === "/api/admin/tokens/clear" && req.method === "POST") {
      response = await handleClearTokens(req, authService);
    } else if (url.pathname === "/v1/messages" && req.method === "POST") {
      response = await handleMessages(req, authService);
    } else if (url.pathname === "/v1/messages/count_tokens" && req.method === "POST") {
      response = await handleCountTokens(req);
    } else if (url.pathname === "/v1/chat/completions" && req.method === "POST") {
      // Handle OpenAI format requests using RequestContext
      const reqCtx = createRequestContext(req, authService, "OpenAI", requestId);
      const result = await reqCtx.getTokenWithUsageAndBody();
      
      // Check for errors
      if (result.length === 3) {
        response = result[2]; // Return error response
      } else {
        const [tokenWithUsage, body] = result;
        
        // Parse OpenAI request
        const openaiReq: OpenAIRequest = JSON.parse(new TextDecoder().decode(body));
        const anthropicReq = openAIToAnthropic(openaiReq);
        
        if (anthropicReq.stream) {
          response = await handleOpenAIStreamRequest(openaiReq, tokenWithUsage, requestId);
        } else {
          response = await handleOpenAINonStreamRequest(openaiReq, tokenWithUsage.tokenInfo, requestId);
        }
      }
    } else if (url.pathname === "/" && req.method === "GET") {
      // Serve the dashboard index page
      response = await serveStaticFile("/index.html");
    } else if (url.pathname === "/admin" && req.method === "GET") {
      // Serve the admin page
      response = await serveStaticFile("/admin.html");
    } else if (url.pathname.startsWith("/static/") && req.method === "GET") {
      // Serve static files (CSS, JS, images, etc.)
      response = await serveStaticFile(url.pathname);
    } else {
      response = new Response("Not Found", { status: 404 });
    }

    // Add CORS headers and request ID to response
    const headers = new Headers(response.headers);
    Object.entries(corsHeaders).forEach(([key, value]) => {
      headers.set(key, value as string);
    });
    headers.set("X-Request-ID", requestId);

    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    });
  } catch (error) {
    logger.error(
      "请求处理失败",
      logger.String("request_id", requestId),
      logger.String("method", req.method),
      logger.String("path", url.pathname),
      logger.Err(error),
    );
    const errorResponse = respondError("Internal server error", 500);
    const headers = new Headers(errorResponse.headers);
    Object.entries(corsHeaders).forEach(([key, value]) => {
      headers.set(key, value as string);
    });
    headers.set("X-Request-ID", requestId);
    
    return new Response(errorResponse.body, {
      status: errorResponse.status,
      statusText: errorResponse.statusText,
      headers,
    });
  }
}

// Global variables for cloud deployment
let globalAuthService: AuthService | null = null;
let globalClientToken: string | null | undefined = null;

// Initialize function (called once on startup)
async function initialize() {
  const startTime = Date.now();
  logger.info("开始初始化服务...");
  
  // Load environment variables from .env file if it exists (local only)
  try {
    const env = await Deno.readTextFile(".env");
    let loadedCount = 0;
    env.split("\n").forEach((line) => {
      const match = line.match(/^([^=]+)=(.*)$/);
      if (match) {
        const [, key, value] = match;
        Deno.env.set(key.trim(), value.trim());
        loadedCount++;
      }
    });
    logger.info(
      "已加载 .env 文件",
      logger.Int("env_count", loadedCount),
    );
  } catch {
    logger.info("未找到 .env 文件，使用环境变量");
  }

  // Reinitialize logger after loading env vars
  logger.reinitialize();
  
  logger.info(
    "日志系统配置",
    logger.String("level", Deno.env.get("LOG_LEVEL") || "info"),
    logger.String("format", Deno.env.get("LOG_FORMAT") || "json"),
    logger.Bool("console", Deno.env.get("LOG_CONSOLE") !== "false"),
    logger.String("file", Deno.env.get("LOG_FILE") || "none"),
  );

  // Get configuration
  globalClientToken = Deno.env.get("KIRO_CLIENT_TOKEN");

  if (!globalClientToken) {
    logger.fatal("致命错误: 未设置 KIRO_CLIENT_TOKEN 环境变量");
    throw new Error(
      "KIRO_CLIENT_TOKEN environment variable not set. Please configure it in your deployment settings.",
    );
  }

  // Create AuthService
  logger.info("正在创建 AuthService...");
  const authStartTime = Date.now();
  globalAuthService = await AuthService.create();
  const authDuration = Date.now() - authStartTime;
  
  logger.info(
    "AuthService 初始化成功",
    logger.Duration("duration", authDuration),
  );
  
  const totalDuration = Date.now() - startTime;
  logger.info(
    "服务初始化完成",
    logger.Duration("total_duration", totalDuration),
  );
}

// Request handler wrapper with lazy initialization
async function handleRequestWithInit(req: Request): Promise<Response> {
  try {
    // Initialize on first request if not already done
    if (!globalAuthService || !globalClientToken) {
      await initialize();
    }

    return await handleRequest(req, globalAuthService!, globalClientToken!);
  } catch (error) {
    logger.error("请求处理错误", logger.Err(error));
    return new Response(
      JSON.stringify({
        error: "Service initialization failed",
        message: error instanceof Error ? error.message : String(error),
      }),
      {
        status: 503,
        headers: { "Content-Type": "application/json" },
      },
    );
  }
}

// Check if running in Deno Deploy
const isDenoDeployment = Deno.env.get("DENO_DEPLOYMENT_ID") !== undefined;

// Main function for local development only
async function main() {
  if (isDenoDeployment) {
    logger.info("在 Deno Deploy 环境中运行，跳过本地服务器设置");
    return;
  }

  const port = parseInt(Deno.env.get("PORT") || String(DEFAULTS.PORT));

  try {
    await initialize();

    logger.info(`正在启动 HTTP 服务器...`, logger.Int("port", port));

    Deno.serve({
      port,
      hostname: "127.0.0.1",
      onListen: ({ hostname, port }) => {
        logger.info(
          `✅ 服务器启动成功`,
          logger.String("host", hostname),
          logger.Int("port", port),
          logger.String("env", isDenoDeployment ? "Deno Deploy" : "Local"),
        );
        
        console.log(`\n🚀 kiro2api-deno 已启动`);
        console.log(`🌐 地址: http://${hostname}:${port}`);
        console.log(`📊 管理面板: http://${hostname}:${port}`);
        console.log(`🔑 认证: 已启用`);
        console.log(`\n可用端点:`);
        console.log(`  GET  /                        - Web 管理界面`);
        console.log(`  GET  /api/tokens              - Token 池状态`);
        console.log(`  GET  /v1/models               - 模型列表`);
        console.log(`  POST /v1/messages             - Anthropic API`);
        console.log(`  POST /v1/chat/completions     - OpenAI API`);
        console.log(`\n按 Ctrl+C 停止服务器\n`);
      },
    }, handleRequestWithInit);
  } catch (error) {
    logger.fatal("启动服务器失败", logger.Err(error));
    throw error;
  }
}

// Export handler for Deno Deploy
export default { fetch: handleRequestWithInit };

// Run the server if executed directly and not in Deno Deploy
if (import.meta.main && !isDenoDeployment) {
  main().catch((error) => {
    logger.fatal("服务器启动失败", logger.Err(error));
  });
}
