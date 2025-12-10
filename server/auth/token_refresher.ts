import type { AuthConfig } from "./config.ts";
import type { TokenInfo } from "../types/common.ts";
import type { CachedToken } from "./token_cache.ts";
import { refreshToken } from "./refresh.ts";
import { UsageLimitsChecker, calculateAvailableCount } from "./usage_checker.ts";
import * as logger from "../logger/logger.ts";

export class TokenRefresher {
  private refreshLocks: Map<number, Promise<TokenInfo>> = new Map();

  async refresh(
    configIndex: number,
    config: AuthConfig,
  ): Promise<CachedToken> {
    const startTime = Date.now();
    
    logger.info(
      "开始刷新 token",
      logger.Int("config_index", configIndex),
      logger.String("auth_type", config.auth),
    );

    try {
      const token = await refreshToken(config);

      const checker = new UsageLimitsChecker();
      let available = 0;
      let usageInfo = null;

      try {
        usageInfo = await checker.checkUsageLimits(token);
        if (usageInfo) {
          available = calculateAvailableCount(usageInfo);
        }
      } catch (error) {
        logger.warn("检查使用限制失败", logger.Err(error));
      }

      const duration = Date.now() - startTime;
      logger.info(
        "Token 刷新成功",
        logger.Int("config_index", configIndex),
        logger.String("expires_at", token.expiresAt?.toISOString() || "unknown"),
        logger.Float("available", available),
        logger.Int("duration_ms", duration)
      );

      return {
        token,
        configIndex,
        cachedAt: new Date(),
        lastUsed: new Date(),
        available,
        usageInfo,
      };
    } catch (error) {
      const duration = Date.now() - startTime;
      logger.error(
        "Token 刷新失败",
        logger.Int("config_index", configIndex),
        logger.Int("duration_ms", duration),
        logger.Err(error)
      );
      throw error;
    }
  }

  async getOrRefresh(
    configIndex: number,
    config: AuthConfig,
    getCached: () => CachedToken | undefined,
    isExpired: (token: TokenInfo) => boolean,
  ): Promise<TokenInfo> {
    const cached = getCached();
    if (cached && !isExpired(cached.token)) {
      return cached.token;
    }

    let existingRefresh = this.refreshLocks.get(configIndex);
    if (existingRefresh) {
      logger.debug("等待现有刷新完成", logger.Int("config_index", configIndex));
      return await existingRefresh;
    }

    const refreshPromise = (async () => {
      const recheck = getCached();
      if (recheck && !isExpired(recheck.token)) {
        logger.debug("缓存已在等待期间刷新", logger.Int("config_index", configIndex));
        return recheck.token;
      }
      
      const cached = await this.refresh(configIndex, config);
      return cached.token;
    })();

    this.refreshLocks.set(configIndex, refreshPromise);

    try {
      return await refreshPromise;
    } catch (error) {
      this.refreshLocks.delete(configIndex);
      logger.error("Token 刷新失败，已清理锁", logger.Int("config_index", configIndex), logger.Err(error));
      throw error;
    } finally {
      this.refreshLocks.delete(configIndex);
    }
  }

  destroy(): void {
    for (const promise of this.refreshLocks.values()) {
      promise.catch(() => {});
    }
    this.refreshLocks.clear();
    logger.info("TokenRefresher 资源已清理");
  }
}
