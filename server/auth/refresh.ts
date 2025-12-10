import type { AuthConfig } from "./config.ts";
import type { RefreshResponse, TokenInfo } from "../types/common.ts";
import { AWS_ENDPOINTS } from "../config/constants.ts";
import * as logger from "../logger/logger.ts";
import { metricsCollector } from "../logger/metrics.ts";
import { errorTracker, ErrorCategory } from "../logger/error_tracker.ts";

// Refresh Social authentication token
async function refreshSocialToken(config: AuthConfig): Promise<TokenInfo> {
  const startTime = Date.now();
  
  logger.debug(
    "开始刷新 Social Token",
    logger.String("auth_type", "Social"),
  );
  
  try {
    const response = await fetch(AWS_ENDPOINTS.SOCIAL_REFRESH, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        refreshToken: config.refreshToken,
      }),
    });

    const latency = Date.now() - startTime;

    if (!response.ok) {
      errorTracker.track(
        ErrorCategory.AUTH_REFRESH_FAILED,
        `Social Token 刷新失败: ${response.status}`,
        new Error(`HTTP ${response.status}: ${response.statusText}`),
        undefined,
        { authType: "Social", statusCode: response.status, latency },
      );
      throw new Error(`Social token refresh failed: ${response.status} ${response.statusText}`);
    }

    const data: RefreshResponse = await response.json();
    
    logger.info(
      "Social Token 刷新成功",
      logger.String("auth_type", "Social"),
      logger.Latency(latency),
      logger.Int("expires_in", data.expiresIn),
    );

    return {
      accessToken: data.accessToken,
      refreshToken: config.refreshToken,
      expiresAt: new Date(Date.now() + data.expiresIn * 1000),
      expiresIn: data.expiresIn,
      profileArn: data.profileArn,
    };
  } catch (error) {
    const latency = Date.now() - startTime;
    errorTracker.track(
      ErrorCategory.AUTH_REFRESH_FAILED,
      "Social Token 刷新异常",
      error,
      undefined,
      { authType: "Social", latency },
    );
    throw error;
  }
}

// Refresh IdC authentication token
async function refreshIdCToken(config: AuthConfig): Promise<TokenInfo> {
  if (!config.clientId || !config.clientSecret) {
    const error = new Error("IdC authentication requires clientId and clientSecret");
    errorTracker.track(
      ErrorCategory.AUTH_TOKEN_INVALID,
      "IdC 配置缺少必需字段",
      error,
    );
    throw error;
  }

  const startTime = Date.now();
  
  logger.debug(
    "开始刷新 IdC Token",
    logger.String("auth_type", "IdC"),
  );
  
  try {
    const response = await fetch(AWS_ENDPOINTS.IDC_REFRESH, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Host": "oidc.us-east-1.amazonaws.com",
        "Connection": "keep-alive",
        "x-amz-user-agent": "aws-sdk-js/3.738.0 ua/2.1 os/other lang/js md/browser#unknown_unknown api/sso-oidc#3.738.0 m/E KiroIDE",
        "Accept": "*/*",
        "Accept-Language": "*",
        "sec-fetch-mode": "cors",
        "User-Agent": "node",
        "Accept-Encoding": "br, gzip, deflate",
      },
      body: JSON.stringify({
        clientId: config.clientId,
        clientSecret: config.clientSecret,
        grantType: "refresh_token",
        refreshToken: config.refreshToken,
      }),
    });

    const latency = Date.now() - startTime;

    if (!response.ok) {
      errorTracker.track(
        ErrorCategory.AUTH_REFRESH_FAILED,
        `IdC Token 刷新失败: ${response.status}`,
        new Error(`HTTP ${response.status}: ${response.statusText}`),
        undefined,
        { authType: "IdC", statusCode: response.status, latency },
      );
      throw new Error(`IdC token refresh failed: ${response.status} ${response.statusText}`);
    }

    const data: RefreshResponse = await response.json();
    
    logger.info(
      "IdC Token 刷新成功",
      logger.String("auth_type", "IdC"),
      logger.Latency(latency),
      logger.Int("expires_in", data.expiresIn),
    );

    return {
      accessToken: data.accessToken,
      refreshToken: config.refreshToken,
      expiresAt: new Date(Date.now() + data.expiresIn * 1000),
      expiresIn: data.expiresIn,
    };
  } catch (error) {
    const latency = Date.now() - startTime;
    errorTracker.track(
      ErrorCategory.AUTH_REFRESH_FAILED,
      "IdC Token 刷新异常",
      error,
      undefined,
      { authType: "IdC", latency },
    );
    throw error;
  }
}

// Main token refresh function
export async function refreshToken(config: AuthConfig): Promise<TokenInfo> {
  try {
    if (config.auth === "Social") {
      return await refreshSocialToken(config);
    } else if (config.auth === "IdC") {
      return await refreshIdCToken(config);
    } else {
      throw new Error(`Unknown auth type: ${config.auth}`);
    }
  } catch (error) {
    logger.error("Token 刷新失败", logger.String("auth_type", config.auth), logger.Err(error));
    throw error;
  }
}
