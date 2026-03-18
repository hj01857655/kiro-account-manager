const ACTIVE_STATUSES = new Set(['active', '正常', '有效'])
const BANNED_STATUSES = new Set(['banned', '封禁', '已封禁'])
const INVALID_STATUSES = new Set(['invalid', '失效', '已失效', 'Token已失效', 'token已失效'])
const EXPIRED_STATUSES = new Set(['expired', '过期', '已过期'])

export function normalizeAccountStatus(status) {
  if (!status) return 'unknown'
  if (ACTIVE_STATUSES.has(status)) return 'active'
  if (BANNED_STATUSES.has(status)) return 'banned'
  if (INVALID_STATUSES.has(status)) return 'invalid'
  if (EXPIRED_STATUSES.has(status)) return 'expired'
  return status
}

export function isActiveStatus(status) {
  return normalizeAccountStatus(status) === 'active'
}

export function isBannedStatus(status) {
  return normalizeAccountStatus(status) === 'banned'
}

export function isInvalidStatus(status) {
  return normalizeAccountStatus(status) === 'invalid'
}

export function isExpiredStatus(status) {
  return normalizeAccountStatus(status) === 'expired'
}

export function isUnavailableStatus(status) {
  const normalized = normalizeAccountStatus(status)
  return normalized === 'banned' || normalized === 'invalid' || normalized === 'expired'
}

export function isAvailableStatus(status) {
  return !isUnavailableStatus(status)
}

export function getAccountStatusMeta(status, t) {
  const normalized = normalizeAccountStatus(status)

  switch (normalized) {
    case 'active':
      return { key: 'active', label: t?.('accounts.active') ?? '正常', tone: 'success' }
    case 'banned':
      return { key: 'banned', label: t?.('accounts.banned') ?? '封禁', tone: 'danger' }
    case 'invalid':
      return { key: 'invalid', label: t?.('accounts.invalid') ?? '失效', tone: 'warning' }
    case 'expired':
      return { key: 'expired', label: t?.('accounts.expired') ?? '过期', tone: 'warning' }
    default:
      return { key: normalized, label: status || (t?.('common.unknown') ?? '未知'), tone: 'warning' }
  }
}
