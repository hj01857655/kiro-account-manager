import { getEnvWithDefault } from "./env.ts";
import { HTTP_CLIENT_KEEP_ALIVE, HTTP_CLIENT_TLS_HANDSHAKE_TIMEOUT } from "../config/tuning.ts";

export function shouldSkipTLSVerify(): boolean {
  return getEnvWithDefault("GIN_MODE", "") === "debug";
}

export function createHTTPClient(): Deno.HttpClient | undefined {
  const skipTLS = shouldSkipTLSVerify();

  // Build client options aligned with Go defaults
  const opts: Record<string, unknown> = {
    // Prefer HTTP/1.1 to match Go's ForceAttemptHTTP2: false
    http2: false,
    // Timeouts (ms)
    // @ts-ignore: Deno runtime may not support all custom timeout fields
    connectTimeout: HTTP_CLIENT_TLS_HANDSHAKE_TIMEOUT,
    // @ts-ignore: readTimeout is not in standard Deno.HttpClient options
    readTimeout: 60_000,
    // @ts-ignore: writeTimeout is not in standard Deno.HttpClient options
    writeTimeout: 60_000,
    // Keep-alive tuning
    // @ts-ignore: keepAliveTimeout is not in standard Deno.HttpClient options
    keepAliveTimeout: HTTP_CLIENT_KEEP_ALIVE,
  };

  if (skipTLS) {
    console.warn("[WARNING] TLS证书验证已禁用 - 仅适用于开发/调试环境");
    // NOTE: Deno does not expose programmatic 'ignore certificate errors' in stable.
    // We proceed without custom CA; use defaults.
  }

  try {
    // @ts-ignore: Deno specific API with extended options
    return Deno.createHttpClient(opts);
  } catch {
    return undefined;
  }
}

export const sharedHTTPClient = createHTTPClient();

export function doRequest(url: string | URL, init?: RequestInit): Promise<Response> {
  if (sharedHTTPClient) {
    return fetch(url, { ...init, client: sharedHTTPClient });
  }
  return fetch(url, init);
}
