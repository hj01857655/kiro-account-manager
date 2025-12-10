import type { TokenInfo } from "../types/common.ts";
import * as logger from "../logger/logger.ts";

// Usage limits types
export interface UsageLimits {
  usageBreakdownList: UsageBreakdown[];
  userInfo: UserInfo;
  daysUntilReset: number;
  nextDateReset: number;
  subscriptionInfo: SubscriptionInfo;
}

export interface UsageBreakdown {
  resourceType: string;
  unit: string;
  usageLimit: number;
  usageLimitWithPrecision: number;
  currentUsage: number;
  currentUsageWithPrecision: number;
  freeTrialInfo?: FreeTrialInfo;
  displayName: string;
}

export interface FreeTrialInfo {
  freeTrialExpiry: number;
  freeTrialStatus: string;
  usageLimit: number;
  usageLimitWithPrecision: number;
  currentUsage: number;
  currentUsageWithPrecision: number;
}

export interface UserInfo {
  email: string;
  userId: string;
}

export interface SubscriptionInfo {
  subscriptionTitle: string;
  type: string;
}

// UsageLimitsChecker
export class UsageLimitsChecker {
  async checkUsageLimits(token: TokenInfo): Promise<UsageLimits | null> {
    try {
      // Build request URL
      const baseURL = "https://codewhisperer.us-east-1.amazonaws.com/getUsageLimits";
      const params = new URLSearchParams({
        isEmailRequired: "true",
        origin: "AI_EDITOR",
        resourceType: "AGENTIC_REQUEST",
      });
      
      const requestURL = `${baseURL}?${params.toString()}`;
      
      // Generate invocation ID
      const invocationId = `${Date.now()}-${crypto.randomUUID()}`;
      
      // Make HTTP request
      const response = await fetch(requestURL, {
        method: "GET",
        headers: {
          "x-amz-user-agent": "aws-sdk-js/1.0.0 KiroIDE-0.2.13-66c23a8c5d15afabec89ef9954ef52a119f10d369df04d548fc6c1eac694b0d1",
          "user-agent": "aws-sdk-js/1.0.0 ua/2.1 os/darwin#24.6.0 lang/js md/nodejs#20.16.0 api/codewhispererruntime#1.0.0 m/E KiroIDE-0.2.13-66c23a8c5d15afabec89ef9954ef52a119f10d369df04d548fc6c1eac694b0d1",
          "host": "codewhisperer.us-east-1.amazonaws.com",
          "amz-sdk-invocation-id": invocationId,
          "amz-sdk-request": "attempt=1; max=1",
          "Authorization": `Bearer ${token.accessToken}`,
        },
      });
      
      if (!response.ok) {
        const body = await response.text();
        logger.error(
          "获取使用限制失败",
          logger.Int("status", response.status),
          logger.String("body", body)
        );
        return null;
      }
      
      const usageLimits: UsageLimits = await response.json();
      
      // Log usage information
      this.logUsageLimits(usageLimits);
      
      return usageLimits;
    } catch (error) {
      logger.error("检查使用限制时出错", logger.Err(error));
      return null;
    }
  }
  
  private logUsageLimits(limits: UsageLimits): void {
    for (const breakdown of limits.usageBreakdownList) {
      if (breakdown.resourceType === "CREDIT") {
        let totalLimit = breakdown.usageLimitWithPrecision;
        let totalUsed = breakdown.currentUsageWithPrecision;
        
        // Add free trial if active
        if (breakdown.freeTrialInfo && breakdown.freeTrialInfo.freeTrialStatus === "ACTIVE") {
          totalLimit += breakdown.freeTrialInfo.usageLimitWithPrecision;
          totalUsed += breakdown.freeTrialInfo.currentUsageWithPrecision;
        }
        
        const available = totalLimit - totalUsed;
        
        logger.info(
          "CREDIT 使用状态",
          logger.Float("total_limit", totalLimit),
          logger.Float("total_used", totalUsed),
          logger.Float("available", available),
          logger.String("user_email", limits.userInfo.email)
        );
        
        if (available <= 1) {
          logger.warn(
            "CREDIT 使用量即将耗尽",
            logger.Float("remaining", available)
          );
        }
        
        break;
      }
    }
  }
}

// Calculate available count from usage limits
export function calculateAvailableCount(limits: UsageLimits): number {
  for (const breakdown of limits.usageBreakdownList) {
    if (breakdown.resourceType === "CREDIT") {
      let totalAvailable = 0;
      
      // Add free trial if active
      if (breakdown.freeTrialInfo && breakdown.freeTrialInfo.freeTrialStatus === "ACTIVE") {
        const freeTrialAvailable = breakdown.freeTrialInfo.usageLimitWithPrecision - 
                                     breakdown.freeTrialInfo.currentUsageWithPrecision;
        totalAvailable += freeTrialAvailable;
      }
      
      // Add base quota
      const baseAvailable = breakdown.usageLimitWithPrecision - breakdown.currentUsageWithPrecision;
      totalAvailable += baseAvailable;
      
      return Math.max(0, totalAvailable);
    }
  }
  
  return 0;
}
