import { normalizeAccountStatus } from '../../../../utils/accountStatus'

// 筛选器应用函数
export function applyFilters(accounts, filters) {
  if (!filters) return accounts
  const hasFilters = !!(
    (filters.subscriptions?.length > 0) ||
    (filters.statuses?.length > 0) ||
    (filters.providers?.length > 0) ||
    filters.usageRange
  )
  
  if (!hasFilters) return accounts

  const result = accounts.filter(account => {
    // 订阅类型筛选 - 使用 subscriptionTitle 字段
    if (filters.subscriptions?.length > 0) {
      const subTitle = (account.usageData?.subscriptionInfo?.subscriptionTitle || '').toUpperCase()
      let subType = 'FREE'
      if (subTitle.includes('ENTERPRISE')) {
        subType = 'KIRO ENTERPRISE'
      } else if (subTitle.includes('PRO+')) {
        subType = 'KIRO PRO+'
      } else if (subTitle.includes('PRO')) {
        subType = 'KIRO PRO'
      } else if (subTitle.includes('KIRO')) {
        subType = 'KIRO FREE'
      }
      if (!filters.subscriptions.includes(subType)) return false
    }

    // 状态筛选
    if (filters.statuses?.length > 0) {
      const normalizedStatus = normalizeAccountStatus(account.status)
      let status = 'normal'
      if (normalizedStatus === 'banned') {
        status = 'banned'
      } else if (normalizedStatus === 'invalid') {
        status = 'invalid'
      } else if (normalizedStatus === 'expired') {
        status = 'expired'
      }
      if (!filters.statuses.includes(status)) return false
    }

    // 提供商筛选
    if (filters.providers?.length > 0) {
      const provider = account.provider || 'Google'
      const matchProvider = filters.providers.some(p => 
        p.toLowerCase() === provider.toLowerCase()
      )
      if (!matchProvider) return false
    }

    // 使用量范围筛选 - 字符串格式 '0-10', '10-30' 等
    if (filters.usageRange && typeof filters.usageRange === 'string') {
      const [minStr, maxStr] = filters.usageRange.split('-')
      const min = parseInt(minStr, 10)
      const max = maxStr === '+' ? Infinity : parseInt(maxStr, 10)
      
      // 计算总使用量（已用绝对值）
      const breakdown = account.usageData?.usageBreakdownList?.[0]
      if (!breakdown) return false
      
      const mainUsed = breakdown.currentUsage || 0
      const trialUsed = breakdown.freeTrialInfo?.currentUsage || 0
      const bonusUsed = (breakdown.bonuses || []).reduce((sum, b) => sum + (b.currentUsage || 0), 0)
      
      const totalUsed = mainUsed + trialUsed + bonusUsed
      
      if (max === Infinity) {
        // 50+ 的情况
        if (totalUsed < min) return false
      } else {
        // 0-10, 10-30, 30-50 的情况
        if (totalUsed < min || totalUsed >= max) return false
      }
    }

    return true
  })
  
  return result
}
