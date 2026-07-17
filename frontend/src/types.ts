export interface Account {
  id: string;
  email?: string;
  label: string;
  status: string;
  provider?: string;
  authMethod?: string;
  expiresAt?: string;
  region?: string;
  profileArn?: string;
  groupId?: string;
  machineId: string;
  enabled: boolean;
  proxyConfig?: Record<string, unknown>;
  usage?: unknown;
  models?: unknown;
  hasAccessToken: boolean;
  hasRefreshToken: boolean;
  failureCount: number;
  successCount: number;
  disabledReason?: string;
  lastFailureAt?: string;
  lastRefreshedAt?: string;
  lastCheckedAt?: string;
  createdAt: string;
  updatedAt: string;
  tags: Tag[];
}

export interface AccountInput {
  email?: string;
  label: string;
  status: string;
  provider?: string;
  authMethod?: string;
  region?: string;
  profileArn?: string;
  groupId?: string;
  machineId?: string;
  enabled: boolean;
  accessToken?: string;
  refreshToken?: string;
  clientId?: string;
  clientSecret?: string;
  proxyConfig?: Record<string, unknown>;
  tagIds: string[];
}

export interface Group { id: string; name: string; color?: string; sortOrder: number; createdAt: string; updatedAt: string }
export interface Tag { id: string; name: string; color: string; createdAt: string }
export interface DashboardStats { totalAccounts: number; availableAccounts: number; abnormalAccounts: number; totalQuota: number; usedQuota: number; lastRefreshAt?: string; gatewayRequestCount: number }
export interface AuditLog { id: number; level: string; action: string; targetType?: string; targetId?: string; message: string; clientIp?: string; createdAt: string }
