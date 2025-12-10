export enum TokenRefreshStatus {
  Idle = 0,
  Refreshing = 1,
  Completed = 2,
  Failed = 3,
}

interface TokenInfo {
  token: string;
  expires_at: Date;
}

interface RefreshResult {
  tokenInfo?: TokenInfo;
  error?: Error;
  success: boolean;
}

class RefreshingToken {
  status: TokenRefreshStatus = TokenRefreshStatus.Refreshing;
  startTime: Date = new Date();
  endTime?: Date;
  result: Promise<RefreshResult>;
  private resolveResult!: (result: RefreshResult) => void;
  tokenInfo?: TokenInfo;
  error?: Error;

  constructor() {
    this.result = new Promise((resolve) => {
      this.resolveResult = resolve;
    });
  }

  complete(tokenInfo?: TokenInfo, error?: Error): void {
    this.endTime = new Date();
    if (error) {
      this.status = TokenRefreshStatus.Failed;
      this.error = error;
      this.resolveResult({ error, success: false });
    } else {
      this.status = TokenRefreshStatus.Completed;
      this.tokenInfo = tokenInfo;
      this.resolveResult({ tokenInfo, success: true });
    }
  }
}

export class TokenRefreshManager {
  private refreshing = new Map<number, RefreshingToken>();
  private totalRefreshes = 0;
  private duplicatePrevented = 0;

  startRefresh(tokenIdx: number): { token: RefreshingToken; isNew: boolean } {
    const existing = this.refreshing.get(tokenIdx);
    if (existing) {
      this.duplicatePrevented++;
      return { token: existing, isNew: false };
    }

    const newRefreshing = new RefreshingToken();
    this.refreshing.set(tokenIdx, newRefreshing);
    this.totalRefreshes++;

    return { token: newRefreshing, isNew: true };
  }

  completeRefresh(tokenIdx: number, tokenInfo?: TokenInfo, error?: Error): void {
    const refreshingToken = this.refreshing.get(tokenIdx);
    if (!refreshingToken) return;

    refreshingToken.complete(tokenInfo, error);

    // Cleanup after delay
    setTimeout(() => {
      this.refreshing.delete(tokenIdx);
    }, 5000);
  }

  async waitForRefresh(tokenIdx: number, timeout: number): Promise<TokenInfo> {
    const refreshingToken = this.refreshing.get(tokenIdx);
    if (!refreshingToken) {
      throw new Error(`没有找到token ${tokenIdx}的刷新任务`);
    }

    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error(`等待token ${tokenIdx}刷新超时`)), timeout);
    });

    const result = await Promise.race([refreshingToken.result, timeoutPromise]);
    if (!result.success || !result.tokenInfo) {
      throw result.error || new Error("刷新失败");
    }

    return result.tokenInfo;
  }

  isRefreshing(tokenIdx: number): boolean {
    const token = this.refreshing.get(tokenIdx);
    return token?.status === TokenRefreshStatus.Refreshing;
  }

  getStats(): Record<string, unknown> {
    let activeRefreshes = 0;
    for (const token of this.refreshing.values()) {
      if (token.status === TokenRefreshStatus.Refreshing) {
        activeRefreshes++;
      }
    }

    const efficiencyRate = this.totalRefreshes > 0
      ? (this.duplicatePrevented / this.totalRefreshes * 100).toFixed(2)
      : "0.00";

    return {
      total_refreshes: this.totalRefreshes,
      duplicate_prevented: this.duplicatePrevented,
      active_refreshes: activeRefreshes,
      efficiency_rate: `${efficiencyRate}%`,
    };
  }

  clearExpiredRefreshes(maxAge: number): number {
    let cleared = 0;
    const now = Date.now();

    for (const [key, token] of this.refreshing.entries()) {
      if (
        token.status !== TokenRefreshStatus.Refreshing &&
        token.endTime &&
        now - token.endTime.getTime() > maxAge
      ) {
        this.refreshing.delete(key);
        cleared++;
      }
    }

    return cleared;
  }

  forceCancel(tokenIdx: number): boolean {
    const token = this.refreshing.get(tokenIdx);
    if (!token || token.status !== TokenRefreshStatus.Refreshing) {
      return false;
    }

    token.complete(undefined, new Error("刷新被强制取消"));
    this.refreshing.delete(tokenIdx);
    return true;
  }
}
