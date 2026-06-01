export interface FreeTrialInfo {
    currentUsage?: number;
    currentUsageWithPrecision?: number;
    freeTrialExpiry: number;
    freeTrialStatus: 'ACTIVE' | 'EXPIRED' | string;
    usageLimit?: number;
    usageLimitWithPrecision?: number;
}

export interface Bonus {
    status: 'ACTIVE' | string;
    expiresAt: number;
    displayName?: string;
    bonusCode?: string;
    description?: string;
    redeemedAt?: number;
    usageLimit?: number;
    currentUsage?: number;
}

export interface UsageBreakdown {
    bonuses?: Bonus[];
    currency?: string;
    currentOverages?: number;
    currentOveragesWithPrecision?: number;
    currentUsage?: number;
    currentUsageWithPrecision?: number;
    displayName?: string;
    displayNamePlural?: string;
    freeTrialInfo?: FreeTrialInfo;
    nextDateReset?: number;
    overageCap?: number;
    overageCapWithPrecision?: number;
    overageCharges?: number;
    overageRate?: number;
    resourceType?: string;
    unit?: string;
    usageLimit?: number;
    usageLimitWithPrecision?: number;
}

export interface AccountUsageData {
    daysUntilReset?: number;
    limits?: any[];
    nextDateReset?: number;
    overageConfiguration?: {
        overageLimit?: number | null;
        overageStatus?: string;
    };
    subscriptionInfo?: {
        overageCapability?: string;
        subscriptionManagementTarget?: string;
        subscriptionTitle: string;
        type?: string;
        upgradeCapability?: string;
    };
    totalUsage?: number | null;
    usageBreakdown?: any;
    usageBreakdownList?: UsageBreakdown[];
    userInfo?: {
        userId?: string;
        email?: string | null;
    };
}

export interface TagLink {
    tagId: string;
    tagName?: string;
    linkedAt?: string;
}

export interface AccountProxyConfig {
    enabled: boolean;
    protocol: 'http' | 'socks5';
    host: string;
    port: number;
    username?: string;
    password?: string;
}

export interface Account {
    id: string;
    email?: string;
    provider: string;
    refreshToken: string;
    accessToken?: string;
    label?: string;
    machineId?: string;
    groupId?: string;
    tagLinks?: TagLink[];
    usageData?: AccountUsageData;
    availableModelsCache?: {
        cachedAt: number;
        models?: any[];
        response?: any;
        modelProvider?: any;
    };
    expiresAt?: string;
    status?: string;
    lastError?: string;
    refreshing?: boolean;
    addedAt?: string;
    quota?: number;
    used?: number;
    // 扩展字段
    clientId?: string;
    clientSecret?: string;
    enabled?: boolean;
    proxyConfig?: AccountProxyConfig;
    _index?: number;
}

export interface TagDefinition {
    id: string;
    name: string;
    color: string;
}

export interface GroupDefinition {
    id: string;
    name: string;
    color: string;
}
