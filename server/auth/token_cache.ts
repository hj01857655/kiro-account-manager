import type { TokenInfo } from "../types/common.ts";
import * as logger from "../logger/logger.ts";
import { TOKEN_CACHE_CONFIG } from "../config/cache.ts";

export interface CachedToken {
  token: TokenInfo;
  configIndex: number;
  cachedAt: Date;
  lastUsed: Date;
  available: number;
  usageInfo?: unknown;
}

export class TokenCache {
  private cache: Map<number, CachedToken> = new Map();
  private cleanupTimer?: number;
  private isDestroyed = false;

  constructor() {
    this.startCleanup();
  }

  private startCleanup(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanup();
    }, TOKEN_CACHE_CONFIG.CLEANUP_INTERVAL_MS);
    
    logger.debug("缓存清理任务已启动", logger.Int("interval_ms", TOKEN_CACHE_CONFIG.CLEANUP_INTERVAL_MS));
  }

  private cleanup(): void {
    const now = Date.now();
    let cleanedCount = 0;
    let expiredCount = 0;
    let staleCount = 0;
    
    for (const [key, cached] of this.cache.entries()) {
      const age = now - cached.cachedAt.getTime();
      const isExpired = this.isExpired(cached.token);
      const isStale = age > TOKEN_CACHE_CONFIG.TTL_MS;
      
      if (isExpired || isStale) {
        this.cache.delete(key);
        cleanedCount++;
        if (isExpired) expiredCount++;
        if (isStale) staleCount++;
      }
    }
    
    if (cleanedCount > 0) {
      logger.info(
        "清理过期缓存",
        logger.Int("total_cleaned", cleanedCount),
        logger.Int("expired", expiredCount),
        logger.Int("stale", staleCount),
        logger.Int("remaining", this.cache.size)
      );
    }
  }

  get(index: number): CachedToken | undefined {
    return this.cache.get(index);
  }

  set(index: number, cached: CachedToken): void {
    this.cache.set(index, cached);
  }

  delete(index: number): void {
    this.cache.delete(index);
  }

  has(index: number): boolean {
    return this.cache.has(index);
  }

  get size(): number {
    return this.cache.size;
  }

  isExpired(token: TokenInfo): boolean {
    if (!token.expiresAt) return false;
    const now = new Date();
    return now.getTime() >= token.expiresAt.getTime() - TOKEN_CACHE_CONFIG.EXPIRY_BUFFER_MS;
  }

  updateUsage(index: number): number {
    const cached = this.cache.get(index);
    if (!cached) return 0;
    
    const available = cached.available;
    if (cached.available > 0) {
      cached.available--;
    }
    cached.lastUsed = new Date();
    
    return available;
  }

  destroy(): void {
    if (this.isDestroyed) return;
    
    this.isDestroyed = true;
    
    if (this.cleanupTimer !== undefined) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = undefined;
    }
    
    this.cache.clear();
    logger.info("TokenCache 资源已清理");
  }
}
