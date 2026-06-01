import { Account, AccountUsageData } from '../types/account';

const ACTIVE_STATUSES = new Set(['active', '正常', '有效'])
const CAPPED_STATUSES = new Set(['capped', '封顶', '已封顶'])
const OVERAGE_STATUSES = new Set(['overage', '超额', '超额中'])
const BANNED_STATUSES = new Set(['banned', '封禁', '已封禁'])
const INVALID_STATUSES = new Set(['invalid', '失效', '已失效', 'Token已失效', 'token已失效'])
const EXPIRED_STATUSES = new Set(['expired', '过期', '已过期'])

function resolveStatusInput(statusOrAccount: string | Account | any, usageData?: AccountUsageData) {
  if (statusOrAccount && typeof statusOrAccount === 'object' && !Array.isArray(statusOrAccount)) {
    return {
      status: (statusOrAccount as Account).status || (statusOrAccount as any).status,
      usageData: (statusOrAccount as Account).usageData || usageData
    }
  }

  return {
    status: statusOrAccount as string,
    usageData
  }
}

function getUsageNumber(source: any, integerKey: string, preciseKey: string): number | null {
  const precise = source?.[preciseKey]
  if (typeof precise === 'number') return precise

  const integer = source?.[integerKey]
  if (typeof integer === 'number') return integer

  return null
}

export function isUsageCapped(usageData?: AccountUsageData): boolean {
  const breakdown = usageData?.usageBreakdownList?.[0]
  if (!breakdown) return false

  const currentUsage = getUsageNumber(breakdown, 'currentUsage', 'currentUsageWithPrecision')
  const usageLimit = getUsageNumber(breakdown, 'usageLimit', 'usageLimitWithPrecision')

  if (typeof currentUsage !== 'number' || typeof usageLimit !== 'number' || usageLimit <= 0) {
    return false
  }

  // 基础额度是否用完
  const baseUsageFull = currentUsage >= usageLimit
  if (!baseUsageFull) return false

  // 检查超额情况
  const overageCapability = usageData?.subscriptionInfo?.overageCapability
  const overageStatus = usageData?.overageConfiguration?.overageStatus
  const currentOverages = breakdown.currentOverages ?? 0
  const overageCap = breakdown.overageCap ?? 0

  // 如果开启了超额且超额还有空间，不算 capped
  if (overageCapability === 'OVERAGE_CAPABLE' && overageStatus === 'ENABLED') {
    // 超额也用完了才算 capped
    if (overageCap > 0 && currentOverages >= overageCap) {
      return true
    }
    // 超额还有空间，不算 capped
    return false
  }

  // 没有超额能力或未开启，基础额度用完就是 capped
  return true
}

export function normalizeAccountStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): string {
  const { status, usageData: resolvedUsageData } = resolveStatusInput(statusOrAccount, usageData)
  if (!status) return 'unknown'

  let normalized = status
  if (ACTIVE_STATUSES.has(status)) normalized = 'active'
  else if (CAPPED_STATUSES.has(status)) normalized = 'capped'
  else if (OVERAGE_STATUSES.has(status)) normalized = 'overage'
  else if (BANNED_STATUSES.has(status)) normalized = 'banned'
  else if (INVALID_STATUSES.has(status)) normalized = 'invalid'
  else if (EXPIRED_STATUSES.has(status)) normalized = 'expired'

  if (normalized === 'active' && isUsageCapped(resolvedUsageData)) {
    return 'capped'
  }

  return normalized
}

export function isActiveStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return normalizeAccountStatus(statusOrAccount, usageData) === 'active'
}

export function isCappedStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return normalizeAccountStatus(statusOrAccount, usageData) === 'capped'
}

export function isBannedStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return normalizeAccountStatus(statusOrAccount, usageData) === 'banned'
}

export function isInvalidStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return normalizeAccountStatus(statusOrAccount, usageData) === 'invalid'
}

export function isExpiredStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return normalizeAccountStatus(statusOrAccount, usageData) === 'expired'
}

export function isUnavailableStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  const normalized = normalizeAccountStatus(statusOrAccount, usageData)
  return normalized === 'banned' || normalized === 'invalid' || normalized === 'expired'
}

export function isAvailableStatus(statusOrAccount: string | Account | any, usageData?: AccountUsageData): boolean {
  return !isUnavailableStatus(statusOrAccount, usageData)
}

export interface StatusMeta {
    key: string;
    label: string;
    tone: 'success' | 'warning' | 'danger';
}

export function getAccountStatusMeta(statusOrAccount: string | Account | any, t?: any, usageData?: AccountUsageData): StatusMeta {
  const normalized = normalizeAccountStatus(statusOrAccount, usageData)

  switch (normalized) {
    case 'active':
      return { key: 'active', label: t?.('accounts.active') ?? 'Active', tone: 'success' }
    case 'capped':
      return { key: 'capped', label: t?.('accounts.capped') ?? 'Capped', tone: 'danger' }
    case 'overage':
      return { key: 'overage', label: t?.('accounts.overage') ?? 'Overage', tone: 'warning' }
    case 'banned':
      return { key: 'banned', label: t?.('accounts.banned') ?? 'Banned', tone: 'danger' }
    case 'invalid':
      return { key: 'invalid', label: t?.('accounts.invalid') ?? 'Invalid', tone: 'danger' }
    case 'expired':
      return { key: 'expired', label: t?.('accounts.expired') ?? 'Expired', tone: 'warning' }
    default:
      const { status } = resolveStatusInput(statusOrAccount, usageData)
      return { key: normalized, label: status || (t?.('common.unknown') ?? 'Unknown'), tone: 'warning' }
  }
}
