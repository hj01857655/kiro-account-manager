import { memo, useCallback, useMemo, useState } from 'react'
import { Layers3, Eye, Copy, Check, Clock, Repeat, Key, BarChart3, Package } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@mantine/core'
import { useTheme } from '../../../contexts/ThemeContext'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { DialogRoot, DialogBody, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '../../ui/dialog'
import { getUsagePercent, getProgressBarColor } from './hooks/useAccountStats'
import { getQuota, getUsed, getSubType, getSubPlan, formatUsage, getAccountDisplayName } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus, isUnavailableStatus } from '../../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import { getThemeAccent } from '../KiroConfig/themeAccent'

const AccountCard = memo(function AccountCard({
  account,
  selectedIdsSet,
  onSelect,
  copiedId,
  onCopy,
  onSwitch,
  onRefresh,
  onRefreshToken,
  onEdit,
  isRefreshing = false,
  isRefreshingToken = false,
  isSwitching = false,
  isCurrentAccount,
  tagDefinitions = [],
  groupDefinitions = [],
  availableModels = null,
  availableModelsLoading = false,
  availableModelsError = '',
  onLoadAvailableModels,
  onContextMenuOpen,
  index = 0,
}) {
  const { t } = useTranslation()
  const { theme, colors } = useTheme()
  const accent = getThemeAccent(theme)
  const { maskEmail } = usePrivacy()
  const [modelsModalOpen, setModelsModalOpen] = useState(false)

  // 从 Set 中计算是否选中
  const isSelected = selectedIdsSet?.has(account.id) ?? false

  // 预计算常用值，避免重复计算
  const cardData = useMemo(() => {
    const quota = getQuota(account)
    const used = getUsed(account)
    const subType = getSubType(account)
    const subPlan = getSubPlan(account)
    const percent = getUsagePercent(used, quota)
    const statusMeta = getAccountStatusMeta(account.status, t)
    const isBanned = isBannedStatus(account.status)
    const isNormal = statusMeta.key === 'active'
    const isUnavailable = isUnavailableStatus(account.status)
    const isExpired = account.expiresAt && new Date(account.expiresAt.replace(/\//g, '-')) < new Date()
    const breakdown = account.usageData?.usageBreakdownList?.[0]
    const nextDateReset = account.usageData?.nextDateReset
    
    return { quota, used, subType, subPlan, percent, statusMeta, isBanned, isNormal, isUnavailable, isExpired, breakdown, nextDateReset }
  }, [account, t])

  const { quota, used, subType, subPlan, percent, statusMeta, isBanned, isNormal, isUnavailable, isExpired, breakdown, nextDateReset } = cardData
  const hasLoadedAvailableModels = Array.isArray(availableModels)
  const availableModelsCache = account.availableModelsCache
  const availableModelsCachedAtText = availableModelsCache?.cachedAt
    ? new Date(availableModelsCache.cachedAt * 1000).toLocaleString()
    : ''

  // 右键菜单处理
  const handleContextMenu = useCallback((e) => {
    e.preventDefault()
    onContextMenuOpen(e.clientX, e.clientY)
  }, [onContextMenuOpen])

  const handleOpenAvailableModels = useCallback(async () => {
    if (availableModelsLoading) {
      return
    }

    setModelsModalOpen(true)

    if (!hasLoadedAvailableModels) {
      await onLoadAvailableModels?.(account.id).catch(() => {})
    }
  }, [account.id, availableModelsLoading, hasLoadedAvailableModels, onLoadAvailableModels])

  const handleRefreshAvailableModels = useCallback(async () => {
    if (availableModelsLoading) {
      return
    }

    if (!modelsModalOpen) {
      setModelsModalOpen(true)
    }

    await onLoadAvailableModels?.(account.id, { forceRefresh: true }).catch(() => {})
  }, [account.id, availableModelsLoading, modelsModalOpen, onLoadAvailableModels])

  // 状态光环颜色
  const glowColor = isCurrentAccount
    ? colors.cardGlowCurrent
    : isBanned
      ? colors.cardGlowBanned
      : isNormal
        ? ''
        : colors.cardGlowWarning

  return (
    <div
      onContextMenu={handleContextMenu}
      className={`relative rounded-2xl border hover:shadow-lg flex flex-col min-h-[240px] animate-stagger transition-all duration-300 ${glowColor} ${
      isSelected
        ? colors.cardSelected
        : isCurrentAccount
          ? colors.cardCurrent
          : isBanned
            ? colors.cardBanned
            : !isNormal
              ? colors.cardWarning
              : colors.cardNormal
    }`}
      style={{ animationDelay: `${Math.min(index, 20) * 30}ms` }}
    >
      {/* 选择框 */}
      <div className="absolute top-3 left-3 z-10">
        <Checkbox
          checked={isSelected}
          onChange={(e) => onSelect(e.currentTarget.checked)}
          size="sm"
          styles={{
            root: { cursor: 'pointer' },
            input: { 
              cursor: 'pointer',
            },
          }}
          aria-label={t('accountCard.selectAccount')}
        />
      </div>
      
      {/* 状态标签 */}
      <div className="absolute top-3 right-3 flex items-center gap-2">
        <span className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
          statusMeta.key === 'active'
            ? colors.badgeSuccess
            : statusMeta.tone === 'danger'
              ? colors.error
              : colors.badgeWarning
        }`}>{statusMeta.label}</span>
      </div>

      <div className="p-4 pt-9 flex-1 flex flex-col gap-2.5">
        {/* 头像和邮箱/用户ID */}
        <div className="flex items-start gap-2">
          <div className={`w-10 h-10 rounded-lg flex items-center justify-center text-base font-bold shadow-md ${
            account.provider === 'Google' ? colors.providerGoogle :
            isGitHubProvider(account.provider) ? colors.providerGithub :
            colors.badgeInfo
          }`}>
            {getAccountDisplayName(account)[0].toUpperCase()}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5 mb-0.5">
              <span className={`font-medium ${colors.text} text-[13px] truncate`}>
                {account.email ? maskEmail(account.email) : getAccountDisplayName(account)}
              </span>
              <button 
                onClick={() => onCopy(getAccountDisplayName(account), account.id)} 
                className={`btn-icon p-1 rounded-lg ${colors.cardHover} flex-shrink-0 transition-all hover:scale-110`}
              >
                {copiedId === account.id ? <Check size={11} className={colors.iconSuccess} /> : <Copy size={11} className={colors.textMuted} />}
              </button>
            </div>
            <div className={`text-[11px] ${colors.textMuted}`}>{account.label || getProviderDisplayName(account.provider) || t('common.noLabel')}</div>
          </div>
        </div>

        {/* 订阅类型和登录方式 */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className={`inline-flex px-2 py-0.5 rounded-lg text-[11px] font-medium shadow-sm ${
            (subType.includes('ENTERPRISE') || subPlan.toUpperCase().includes('ENTERPRISE'))
              ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white'
              : (subType.includes('PRO+') || subPlan.includes('PRO+'))
                ? `bg-gradient-to-r ${accent.gradientFrom} ${accent.gradientTo} text-white`
                : (subType.includes('PRO') || subPlan.includes('PRO'))
                  ? `${accent.solidBg} text-white`
                  : (subPlan.toUpperCase().includes('KIRO'))
                    ? `${accent.bgSoft} ${accent.text}`
                    : colors.badgeDisabled
          }`}>
            {subPlan || 'Free'}
          </span>
          <span className={`text-[11px] px-2 py-0.5 rounded-lg font-medium shadow-sm ${
            account.provider === 'Google' ? colors.providerGoogle
              : isGitHubProvider(account.provider) ? colors.providerGithub
              : account.provider === 'BuilderId' ? colors.providerBuilderId
              : account.provider === 'Enterprise' ? colors.providerEnterprise
              : colors.providerDefault
          }`}>
            {getProviderDisplayName(account.provider) || t('common.unknown')}
          </span>
          {isCurrentAccount && (
            <span className="text-[11px] px-2 py-0.5 rounded-lg bg-gradient-to-r from-green-500 to-emerald-500 text-white font-medium shadow-sm">
              {t('common.currentlyUsing')}
            </span>
          )}
        </div>

        {/* 分组 */}
        {account.groupId && (
          <div className="flex items-center gap-2">
            {(() => {
              const group = groupDefinitions.find(g => g.id === account.groupId)
              if (!group) return null
              return (
                <span 
                  className={`text-[11px] px-2.5 py-1 rounded-lg font-medium shadow-sm ${colors.cardSecondary}`}
                  style={{ 
                    backgroundColor: group.color ? `${group.color}20` : undefined,
                    color: group.color || colors.textMuted
                  }}
                >
                  {group.name}
                </span>
              )
            })()}
          </div>
        )}

        {/* 标签 */}
        {account.tagLinks && account.tagLinks.length > 0 && (
          <div className="flex items-center gap-2 flex-wrap">
            {account.tagLinks.map(tagLink => {
              const tag = tagDefinitions.find(t => t.id === tagLink.tagId)
              const tagName = tag?.name || tagLink.tagName || '未知标签'
              const tagColor = tag?.color || '#888888'
              const linkedAt = tagLink.linkedAt
              return (
                <span 
                  key={tagLink.tagId} 
                  className="text-[11px] px-2.5 py-0.5 rounded-full text-white cursor-default font-medium shadow-sm"
                  style={{ backgroundColor: tagColor }}
                  title={linkedAt ? `关联于 ${linkedAt}` : ''}
                >
                  {tagName}
                </span>
              )
            })}
          </div>
        )}

        {/* 配额进度 */}
        <div className={`p-2.5 rounded-lg ${colors.cardSecondary}`}>
          <div className="flex items-center justify-between text-[11px] mb-1.5">
            <span className={`${colors.text} font-medium`}>{t('common.usage')}</span>
            <span className={`font-semibold text-sm ${percent > 80 ? 'text-red-400' : percent > 50 ? 'text-yellow-400' : 'text-green-400'}`}>
              {Math.round(percent)}%
            </span>
          </div>
          <div className={`h-1.5 ${colors.cardSecondary} rounded-full overflow-hidden mb-1.5 shadow-inner`}>
            <div 
              className={`h-full rounded-full transition-all duration-500 ${getProgressBarColor(percent)}`} 
              style={{ width: `${Math.min(percent, 100)}%` }} 
            />
          </div>
          <div className="flex items-center justify-between text-[11px]">
            <span className={`font-semibold ${colors.text}`}>{formatUsage(used)} / {formatUsage(quota)}</span>
            <span className={`${colors.textMuted}`}>{t('common.remaining')} {formatUsage(quota - used)}</span>
          </div>
          {/* 日期信息 */}
          {(nextDateReset || (breakdown?.freeTrialInfo?.freeTrialExpiry && breakdown.freeTrialInfo.freeTrialStatus === 'ACTIVE') || breakdown?.bonuses?.some(b => b.status === 'ACTIVE' && b.expiresAt)) && (
            <div className={`mt-2 pt-2 border-t ${colors.cardBorder} flex items-center gap-1.5 flex-wrap text-[10px]`}>
              <Clock size={9} className={colors.textMuted} />
              {nextDateReset && (
                <span className={colors.dateReset}>{t('common.reset')} {new Date(nextDateReset * 1000).toLocaleDateString()}</span>
              )}
              {breakdown?.freeTrialInfo?.freeTrialExpiry && breakdown.freeTrialInfo.freeTrialStatus === 'ACTIVE' && (
                <span className={colors.dateTrial}>· {t('home.trial')} {new Date(breakdown.freeTrialInfo.freeTrialExpiry * 1000).toLocaleDateString()}</span>
              )}
              {breakdown?.bonuses?.filter(b => b.status === 'ACTIVE' && b.expiresAt).slice(0, 1).map((bonus, idx) => (
                <span key={idx} className={colors.dateBonus}>· {t('detail.bonusTotal')} {new Date(bonus.expiresAt * 1000).toLocaleDateString()}</span>
              ))}
            </div>
          )}
        </div>

        {/* 固定位置的附加信息区域 */}
        <div className="flex flex-col gap-1.5">
          <div className={`px-2.5 py-2 rounded-lg ${colors.cardSecondary} shadow-sm`}>
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0">
                <span className={`text-[11px] font-medium ${colors.text}`}>
                  {t('accountCard.availableModels')}
                </span>
                {availableModelsCachedAtText && (
                  <div className={`mt-1 text-[10px] ${colors.textMuted}`}>
                    {t('accountCard.cachedAt')}: {availableModelsCachedAtText}
                  </div>
                )}
              </div>
              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={handleOpenAvailableModels}
                  disabled={availableModelsLoading}
                  className={`inline-flex h-9 w-9 cursor-pointer items-center justify-center rounded-lg border transition-colors duration-200 ${colors.cardBorder} ${colors.cardHover} ${colors.text} disabled:cursor-not-allowed disabled:opacity-60`}
                  title={availableModelsLoading
                    ? t('accountCard.loadingModels')
                    : hasLoadedAvailableModels
                      ? `${availableModels.length} ${t('accountCard.modelCountSuffix')}`
                      : t('accountCard.loadModels')}
                  aria-label={availableModelsLoading
                    ? t('accountCard.loadingModels')
                    : hasLoadedAvailableModels
                      ? `${availableModels.length} ${t('accountCard.modelCountSuffix')}`
                      : t('accountCard.loadModels')}
                >
                  {availableModelsLoading ? (
                    <Repeat size={15} className="animate-spin" />
                  ) : (
                    <Layers3 size={15} />
                  )}
                </button>
              </div>
            </div>
            {availableModelsError && (
              <div className="mt-2 text-[10px] text-red-400 break-words">
                {t('accountCard.modelLoadFailed')}: {availableModelsError.slice(0, 120)}
              </div>
            )}
          </div>

          <DialogRoot open={modelsModalOpen} onOpenChange={setModelsModalOpen}>
            <DialogContent maxWidth="720px" className="overflow-hidden">
              <DialogHeader
                icon={Package}
                iconColor={accent.text}
                iconBg={accent.iconBadgeBg}
                className="border-b pb-4"
              >
                <div className="pr-12">
                  <DialogTitle>{t('accountCard.availableModels')}</DialogTitle>
                  <DialogDescription>
                    {account.label || getAccountDisplayName(account)}
                  </DialogDescription>
                </div>
              </DialogHeader>

              <DialogBody className="space-y-4">
                <div className="flex items-center justify-between gap-3 rounded-xl border px-4 py-3">
                  <div className={`min-w-0 text-sm ${colors.textMuted}`}>
                    <div className={`font-medium ${colors.text}`}>
                      {hasLoadedAvailableModels
                        ? `${availableModels.length} ${t('accountCard.modelCountSuffix')}`
                        : t('accountCard.loadModels')}
                    </div>
                    {availableModelsCachedAtText && (
                      <div className="mt-1 text-xs">
                        {t('accountCard.cachedAt')}: {availableModelsCachedAtText}
                      </div>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={handleRefreshAvailableModels}
                    disabled={availableModelsLoading}
                    className={`inline-flex h-10 w-10 cursor-pointer items-center justify-center rounded-xl border transition-colors duration-200 ${colors.cardBorder} ${colors.cardHover} ${colors.text} disabled:cursor-not-allowed disabled:opacity-60`}
                    title={t('accountCard.refreshModels')}
                    aria-label={t('accountCard.refreshModels')}
                  >
                    <Repeat size={16} className={availableModelsLoading ? 'animate-spin' : ''} />
                  </button>
                </div>

                {availableModelsError && (
                  <div className="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-400 break-words">
                    {t('accountCard.modelLoadFailed')}: {availableModelsError}
                  </div>
                )}

                {availableModelsLoading && !hasLoadedAvailableModels ? (
                  <div className={`flex min-h-40 items-center justify-center rounded-xl border ${colors.cardBorder} ${colors.cardSecondary}`}>
                    <div className={`flex items-center gap-2 text-sm ${colors.textMuted}`}>
                      <Repeat size={16} className="animate-spin" />
                      <span>{t('accountCard.loadingModels')}</span>
                    </div>
                  </div>
                ) : hasLoadedAvailableModels && availableModels.length > 0 ? (
                  <div className={`max-h-[380px] overflow-y-auto rounded-xl border p-4 ${colors.cardSecondary}`}>
                    <div className="flex flex-wrap gap-2">
                      {availableModels.map(model => {
                        const isDefaultModel = model.isDefault ?? model.is_default ?? false
                        const promptCaching = model.promptCaching ?? model.prompt_caching
                        const supportsPromptCaching = promptCaching?.supportsPromptCaching === true

                        return (
                          <span
                            key={model.modelId}
                            className={`inline-flex max-w-full items-center gap-1 rounded-full px-3 py-1.5 text-xs font-medium ${colors.cardHover} ${colors.text}`}
                            title={[model.modelName || model.modelId, model.description].filter(Boolean).join('\n')}
                          >
                            <span className="max-w-[260px] truncate">
                              {model.modelName || model.modelId}
                            </span>
                            {isDefaultModel && (
                              <span className="rounded-full bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-400">
                                {t('accountCard.defaultModel')}
                              </span>
                            )}
                            {supportsPromptCaching && (
                              <span className="rounded-full bg-sky-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-sky-400">
                                {t('accountCard.promptCaching')}
                              </span>
                            )}
                          </span>
                        )
                      })}
                    </div>
                  </div>
                ) : (
                  <div className={`rounded-xl border px-4 py-10 text-center text-sm ${colors.cardBorder} ${colors.textMuted}`}>
                    {t('accountCard.noAvailableModels')}
                  </div>
                )}
              </DialogBody>
            </DialogContent>
          </DialogRoot>

          {/* Token 过期时间 */}
          {account.expiresAt && (
            <div className={`text-[11px] ${isExpired ? colors.dateExpired : colors.textMuted} flex items-center gap-1 px-2 py-1 rounded-lg ${colors.cardSecondary}`}>
              <Clock size={10} />
              <span>Token: {account.expiresAt}</span>
              {isExpired && <span className={`${colors.dateExpired} font-medium`}>({t('accountCard.tokenExpired')})</span>}
            </div>
          )}

          {/* 机器码 */}
          {account.machineId && (
            <div className={`text-[11px] flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg ${colors.cardSecondary} shadow-sm`}>
              <span className={`font-medium shrink-0 ${colors.machineIdText}`}>机器码:</span>
              <span className={`font-mono text-[10px] break-all ${colors.machineIdTextSecondary}`}>{account.machineId}</span>
              <button 
                onClick={() => onCopy(account.machineId, `${account.id}-mid`)} 
                className={`btn-icon p-1 rounded-lg flex-shrink-0 ${colors.cardHover} transition-all hover:scale-110`}
              >
                {copiedId === `${account.id}-mid` ? <Check size={11} className={colors.iconSuccess} /> : <Copy size={11} className={colors.machineIdIcon} />}
              </button>
            </div>
          )}
        </div>

        {/* 底部操作栏 */}
        <div className={`mt-auto pt-3 border-t ${colors.cardBorder} flex items-center justify-between`}>
          <span className={`text-[10px] ${colors.textMuted} opacity-70`}>{t('accountCard.rightClickTip')}</span>
          <div className="flex items-center gap-1.5">
            <button
              onClick={(e) => { e.stopPropagation(); onEdit(account) }}
              className={`p-2 rounded-lg ${colors.cardHover} ${colors.actionView} transition-all hover:scale-110 shadow-sm`}
              title={t('accountCard.viewDetails')}
            >
<Layers3 size={16} />
            </button>
            <button
              onClick={(e) => { e.stopPropagation(); onRefreshToken?.(account.id) }}
              disabled={isRefreshingToken}
              className={`p-2 rounded-lg ${colors.cardHover} ${colors.actionRefresh} disabled:opacity-50 transition-all hover:scale-110 shadow-sm`}
              title={t('accountCard.refreshToken')}
            >
              <Key size={16} className={isRefreshingToken ? 'animate-spin' : ''} />
            </button>
            <button
              onClick={(e) => { e.stopPropagation(); onRefresh(account.id) }}
              disabled={isRefreshing}
              className={`p-2 rounded-lg ${colors.cardHover} ${colors.actionRefresh} disabled:opacity-50 transition-all hover:scale-110 shadow-sm`}
              title={t('accountCard.refreshQuota')}
            >
              <BarChart3 size={16} className={isRefreshing ? 'animate-spin' : ''} />
            </button>
            {!isCurrentAccount && (
              <button
                onClick={(e) => { e.stopPropagation(); onSwitch(account) }}
                disabled={isSwitching || isUnavailable}
                className={`p-2 rounded-lg ${colors.cardHover} ${colors.actionSwitch} disabled:opacity-50 disabled:cursor-not-allowed transition-all hover:scale-110 shadow-sm`}
                title={isUnavailable ? `${statusMeta.label}账号不可切换` : t('accountCard.switchAccount')}
              >
                <Repeat size={16} className={isSwitching ? 'animate-spin' : ''} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}, (prevProps, nextProps) => {
  // 自定义比较：只在关键 props 变化时重渲染
  const prevSelected = prevProps.selectedIdsSet?.has(prevProps.account.id) ?? false
  const nextSelected = nextProps.selectedIdsSet?.has(nextProps.account.id) ?? false
  
  return (
    prevProps.account === nextProps.account &&
    prevSelected === nextSelected &&
    prevProps.copiedId === nextProps.copiedId &&
    prevProps.isRefreshing === nextProps.isRefreshing &&
    prevProps.isRefreshingToken === nextProps.isRefreshingToken &&
    prevProps.isSwitching === nextProps.isSwitching &&
    prevProps.isCurrentAccount === nextProps.isCurrentAccount &&
    prevProps.tagDefinitions === nextProps.tagDefinitions &&
    prevProps.availableModels === nextProps.availableModels &&
    prevProps.availableModelsLoading === nextProps.availableModelsLoading &&
    prevProps.availableModelsError === nextProps.availableModelsError &&
    prevProps.onLoadAvailableModels === nextProps.onLoadAvailableModels
  )
})

export default AccountCard
