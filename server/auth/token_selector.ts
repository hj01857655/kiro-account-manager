import type { AuthConfig } from "./config.ts";
import type { TokenInfo, TokenWithUsage } from "../types/common.ts";
import type { TokenCache } from "./token_cache.ts";
import type { TokenRefresher } from "./token_refresher.ts";
import * as logger from "../logger/logger.ts";

export class TokenSelector {
  private currentIndex = 0;
  private exhausted: Set<number> = new Set();

  constructor(
    private configs: AuthConfig[],
    private cache: TokenCache,
    private refresher: TokenRefresher,
  ) {}

  async selectBest(): Promise<TokenWithUsage> {
    const maxAttempts = this.configs.length;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const configIndex = this.currentIndex;
      const config = this.configs[configIndex];

      try {
        const cached = this.cache.get(configIndex);
        if (cached && cached.available <= 0) {
          this.exhausted.add(configIndex);
          this.currentIndex = (this.currentIndex + 1) % this.configs.length;
          logger.debug(
            "Token 已耗尽，切换到下一个",
            logger.Int("exhausted_index", configIndex),
            logger.Int("next_index", this.currentIndex)
          );
          continue;
        }

        const token = await this.refresher.getOrRefresh(
          configIndex,
          config,
          () => this.cache.get(configIndex),
          (token) => this.cache.isExpired(token),
        );

        const available = this.cache.updateUsage(configIndex);

        logger.debug(
          "选择 token",
          logger.Int("config_index", configIndex),
          logger.Float("available_before", available),
          logger.Float("available_after", available - 1)
        );

        return {
          tokenInfo: token,
          configIndex,
          availableCount: available,
          isUsageExceeded: available <= 0,
        };
      } catch (error) {
        logger.error(
          "获取 token 失败",
          logger.Int("config_index", configIndex),
          logger.Err(error),
        );
        this.exhausted.add(configIndex);
        this.currentIndex = (this.currentIndex + 1) % this.configs.length;
      }
    }

    throw new Error("All token configurations failed");
  }

  removeFromExhausted(index: number): void {
    this.exhausted.delete(index);
  }

  destroy(): void {
    this.exhausted.clear();
    this.currentIndex = 0;
    logger.info("TokenSelector 资源已清理");
  }
}
