import { useRef, useMemo, memo, useState, useCallback } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Users, Plus, RefreshCw, Repeat, Eye, Edit2, Trash2, Copy, UserX, ChevronUp, ChevronDown } from 'lucide-react'
import { Checkbox } from '@mantine/core'
import { useApp } from '../../../hooks/useApp'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { getQuota, getUsed, formatUsage, getAccountDisplayName } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus, isUnavailableStatus } from '../../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import ContextMenu from './ContextMenu'
import { getThemeAccent, isLightTheme as checkIsLightTheme } from '../KiroConfig/themeAccent'

// 单行组件
const ListRow = memo(function ListRow({
  account,
  isSelected,
  isCurrent,
  isRefreshing,
  isSwitching,
  tagMap,
  groupMap,
  colors,
  isLightTheme,
  t,
  maskEmail,
  onSelectOne,
  onSwitch,
  onRefresh,
  onEdit,
  onEditLabel,
  onDelete,
  onDeleteRemote,
  onCopy,
}) {
  const [contextMenu, setContextMenu] = useState(null)
  const used = getUsed(account)
  const limit = getQuota(account)
  const remaining = Math.max(limit - used, 0)
  const usagePercent = limit > 0 ? Math.min((used / limit) * 100, 100) : 0
  const isBanned = isBannedStatus(account.status)
  const isUnavailable = isUnavailableStatus(account.status)
  const statusMeta = getAccountStatusMeta(account.status, t)

  const handleContextMenu = useCallback((e) => {
    e.preventDefault()
    setContextMenu({ x: e.clientX, y: e.clientY })
  }, [])

  const handleCopyJson = useCallback(() => {
    onCopy(JSON.stringify(account, null, 2), account.id)
  }, [account, onCopy])

  const getMenuItems = useCallback(() => [
    { icon: Eye, label: t('accountCard.viewDetails'), onClick: () => onEdit(account) },
    { icon: Edit2, label: t('accountCard.editRemark'), onClick: () => onEditLabel(account) },
    { icon: Copy, label: t('accountCard.copyJson'), onClick: handleCopyJson },
    { divider: true },
    { icon: RefreshCw, label: t('accountCard.refresh'), onClick: () => onRefresh(account.id), disabled: isRefreshing },
    { icon: Repeat, label: t('accountCard.switchAccount'), onClick: () => onSwitch(account), disabled: isSwitching || isUnavailable },
    { divider: true },
    { icon: Trash2, label: t('accountCard.delete'), onClick: () => onDelete(account.id), danger: true },
    ...(account.provider !== 'Enterprise' && !isBanned && onDeleteRemote ? [
      { icon: UserX, label: t('accountCard.deleteRemote'), onClick: () => onDeleteRemote(account), danger: true },
    ] : []),
  ], [t, account, handleCopyJson, onEdit, onEditLabel, onRefresh, onSwitch, onDelete, onDeleteRemote, isRefreshing, isSwitching, isBanned, isUnavailable])

  return (
    <div
      onContextMenu={handleContextMenu}
      className={`flex items-center gap-3 px-4 h-[56px] border-b ${colors.cardBorder} ${isCurrent ? colors.cardCurrent : ''} ${colors.cardHover} cursor-context-menu animate-stagger`}
      style={{ animationDelay: `${Math.min(account._index || 0, 20) * 30}ms` }}
    >
      {contextMenu && (
        <ContextMenu x={contextMenu.x} y={contextMenu.y} onClose={() => setContextMenu(null)} items={getMenuItems()} isLightTheme={isLightTheme} />
      )}
      <Checkbox 
        checked={isSelected} 
        onChange={(e) => onSelectOne(account.id, e.currentTarget.checked)} 
        onClick={(e) => e.stopPropagation()}
        className="shrink-0"
        styles={{
          input: {
            cursor: 'pointer',
          }
        }}
      />
      
      {/* 邮箱/用户ID */}
      <div className="w-48 shrink-0">
        <div className="flex items-center gap-2">
          <span className={`text-sm font-medium truncate ${colors.text}`}>
            {account.email ? maskEmail(account.email) : getAccountDisplayName(account)}
          </span>
          {isCurrent && <span className={`text-xs px-1.5 py-0.5 rounded shrink-0 ${colors.badgeInfo}`}>当前</span>}
        </div>
        {account.label && <span className={`text-xs ${colors.textMuted} truncate block mt-0.5`}>{account.label}</span>}
      </div>

      {/* 提供商 */}
      <span className={`text-xs px-2 py-1 rounded w-20 text-center shrink-0 ${
        account.provider === 'Google' ? colors.badgeWarning
        : isGitHubProvider(account.provider) ? colors.badgeDisabled
        : account.provider === 'BuilderId' ? colors.badgeWarning
        : colors.badgeDisabled
      }`}>{getProviderDisplayName(account.provider) || 'Unknown'}</span>

      {/* 订阅类型 */}
      <span className={`text-xs px-2 py-1 rounded w-20 text-center shrink-0 ${
        account.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('ENTERPRISE')
          ? colors.badgeWarning
          : account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO+')
            ? colors.badgePurple
            : account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO')
              ? colors.badgeInfo
              : account.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('KIRO')
                ? colors.badgeCyan
                : colors.badgeDisabled
      }`}>{account.usageData?.subscriptionInfo?.subscriptionTitle || 'Free'}</span>

      {/* 配额 */}
      <div className="w-24 shrink-0">
        <div className={`text-xs ${remaining > 0 ? 'text-green-500' : 'text-red-500'}`}>{formatUsage(used)}/{formatUsage(limit)}</div>
        <div className={`h-1 rounded-full ${colors.cardSecondary} mt-1`}>
          <div className={`h-full rounded-full ${remaining > 0 ? 'bg-green-500' : 'bg-red-500'}`} style={{ width: `${usagePercent}%` }} />
        </div>
      </div>

      {/* 状态 */}
      <span className={`text-xs px-2 py-1 rounded w-12 text-center shrink-0 ${
        statusMeta.tone === 'danger' ? colors.error
        : statusMeta.tone === 'success' ? colors.badgeSuccess
        : colors.badgeWarning
      }`}>{statusMeta.label}</span>

      {/* 机器码 */}
      <span className="text-xs font-mono w-14 text-center shrink-0 text-red-500">
        {account.machineId?.slice(0, 6) || '-'}
      </span>

      {/* Token|试用到期时间 */}
      <div className="w-28 shrink-0 text-[11px]">
        <span className={colors.textMuted} title="Token 过期">{account.expiresAt?.slice(11, 16) || '-'}</span>
        {account.usageData?.usageBreakdownList?.[0]?.freeTrialInfo?.freeTrialExpiry && (
          <>
            <span className={`${colors.textMuted} mx-1`}>|</span>
            <span className="text-orange-500" title="试用到期">
              {new Date(account.usageData.usageBreakdownList[0].freeTrialInfo.freeTrialExpiry * 1000).toLocaleDateString().slice(5)}
            </span>
          </>
        )}
      </div>

      {/* 分组 */}
      <div className="w-16 shrink-0">
        {account.groupId ? (() => {
          const group = groupMap.get(account.groupId)
          return group ? (
            <span 
              className="text-[10px] px-1.5 py-0.5 rounded font-medium truncate block max-w-full"
              style={{ backgroundColor: `${group.color}20`, color: group.color }}
              title={group.name}
            >
              {group.name}
            </span>
          ) : <span className={`text-xs ${colors.textMuted}`}>-</span>
        })() : <span className={`text-xs ${colors.textMuted}`}>-</span>}
      </div>

      {/* 标签 */}
      <div className="flex-1 min-w-0">
        {account.tagLinks?.length > 0 ? (
          <div className="flex items-center gap-1 flex-wrap">
            {account.tagLinks.slice(0, 3).map(tagLink => {
              const tag = tagMap.get(tagLink.tagId)
              const linkedAt = tagLink.linkedAt
              // 优先用标签定义的名称，如果标签被删除则用 tagLink 中存储的名称
              const tagName = tag?.name || tagLink.tagName || '未知标签'
              const tagColor = tag?.color || '#888888'
              return <span key={tagLink.tagId} className="text-[10px] px-1.5 py-0.5 rounded font-medium truncate max-w-[120px]" style={{ backgroundColor: `${tagColor}20`, color: tagColor }} title={linkedAt ? `${tagName} (关联于 ${linkedAt})` : tagName}>{tagName}</span>
            })}
            {account.tagLinks.length > 3 && <span className={`text-[10px] ${colors.textMuted}`}>+{account.tagLinks.length - 3}</span>}
          </div>
        ) : <span className={`text-xs ${colors.textMuted}`}>-</span>}
      </div>
    </div>
  )
}, (prev, next) => (
  prev.account === next.account && prev.isSelected === next.isSelected && prev.isCurrent === next.isCurrent &&
  prev.isRefreshing === next.isRefreshing && prev.isSwitching === next.isSwitching &&
  prev.tagMap === next.tagMap && prev.groupMap === next.groupMap && prev.isLightTheme === next.isLightTheme
))


function AccountListView({
  accounts, totalCount, selectedIds, selectedIdsSet, onSelectAll, onSelectOne, onSwitch, onRefresh, onEdit, onEditLabel, onDelete, onDeleteRemote, onCopy, onAdd, refreshingId, switchingId, localToken, tagDefinitions = [], groupDefinitions = [], copiedId, sortBy, onSortChange,
}) {
  const { t, theme, colors } = useApp()
  const accent = getThemeAccent(theme)
  const { maskEmail } = usePrivacy()
  const isLightTheme = checkIsLightTheme(theme)
  const scrollRef = useRef(null)

  // 优化：如果父组件传递了 selectedIdsSet，直接使用；否则创建
  const _selectedIdsSet = selectedIdsSet || useMemo(() => new Set(selectedIds), [selectedIds])
  const localRefreshToken = localToken?.refreshToken
  const tagMap = useMemo(() => new Map(tagDefinitions.map(tag => [tag.id, tag])), [tagDefinitions])
  const groupMap = useMemo(() => new Map(groupDefinitions.map(group => [group.id, group])), [groupDefinitions])

  const rowVirtualizer = useVirtualizer({
    count: accounts.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 56,
    overscan: 5,
    scrollMargin: 0, // 防止触底反弹
  })

  // 表头排序点击处理
  const handleSort = useCallback((field) => {
    if (!onSortChange) return
    // 点击同一列切换升降序，点击不同列默认升序
    if (sortBy === `${field}Asc`) {
      onSortChange(`${field}Desc`)
    } else if (sortBy === `${field}Desc`) {
      onSortChange('default')
    } else {
      onSortChange(`${field}Asc`)
    }
  }, [sortBy, onSortChange])

  // 排序图标
  const SortIcon = ({ field }) => {
    if (sortBy === `${field}Asc`) return <ChevronUp size={12} className="inline ml-0.5" />
    if (sortBy === `${field}Desc`) return <ChevronDown size={12} className="inline ml-0.5" />
    return null
  }

  if (accounts.length === 0) {
    return (
      <div className="flex-1 flex flex-col overflow-hidden p-6">
        <div className={`flex flex-col items-center justify-center py-20 ${colors.textMuted}`}>
          <div className={`w-20 h-20 rounded-full ${colors.cardSecondary} flex items-center justify-center mb-4`}>
            <Users size={40} strokeWidth={1} className="opacity-50" />
          </div>
          <p className="font-medium mb-1">{t('common.noAccounts')}</p>
          <p className="text-sm opacity-75">{t('common.addAccountHint')}</p>
          <button onClick={onAdd} className={`mt-4 px-4 py-2 rounded-xl ${colors.cardSecondary} ${colors.cardHover}`}>
            <Plus size={16} className="inline mr-1" />{t('common.addAccount')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden p-6">
      <div className="flex items-center justify-between mb-2 px-1 shrink-0">
        <label className="flex items-center gap-2 cursor-pointer">
          <Checkbox 
            checked={selectedIds.length === accounts.length && accounts.length > 0} 
            onChange={(e) => onSelectAll(e.currentTarget.checked)}
            styles={{
              input: {
                cursor: 'pointer',
              }
            }}
          />
          <span className={`text-sm ${colors.textMuted}`}>{selectedIds.length > 0 ? `${t('common.selected')} ${selectedIds.length}` : t('common.selectAll')}</span>
        </label>
        <span className={`text-sm ${colors.textMuted}`}>{accounts.length === totalCount ? `共 ${totalCount} 个账号` : `${accounts.length} / ${totalCount} 个账号`}</span>
      </div>

      {/* 表头 */}
      <div className={`flex items-center gap-3 px-4 py-3 ${colors.cardSecondary} border ${colors.cardBorder} rounded-t-xl ${colors.textMuted} text-xs font-semibold uppercase tracking-wider`}>
        <div className="w-4" />
        <div className="w-48">邮箱</div>
        <div className="w-20 text-center">账号类型</div>
        <div className="w-20 text-center">订阅类型</div>
        <button
          type="button"
          onClick={() => handleSort('usage')}
          className={`w-24 text-left ${accent.text} select-none cursor-pointer transition-colors duration-200 focus:ring-2 ${accent.ring} rounded`}
        >
          配额<SortIcon field="usage" />
        </button>
        <div className="w-12 text-center">状态</div>
        <div className="w-14 text-center text-red-500">机器码</div>
        <button
          type="button"
          onClick={() => handleSort('trial')}
          className={`w-28 text-left ${accent.text} select-none cursor-pointer transition-colors duration-200 focus:ring-2 ${accent.ring} rounded`}
        >
          token|试用过期<SortIcon field="trial" />
        </button>
        <div className="w-16">分组</div>
        <div className="flex-1">标签</div>
      </div>

      <div ref={scrollRef} className={`flex-1 overflow-auto border border-t-0 ${colors.cardBorder} rounded-b-xl ${colors.card}`} style={{ overscrollBehavior: 'contain' }}>
        <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
          {rowVirtualizer.getVirtualItems().map((vRow) => {
            const acc = accounts[vRow.index]
            return (
              <div key={acc.id} style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${vRow.start}px)` }}>
                <ListRow
                  account={acc}
                  isSelected={_selectedIdsSet.has(acc.id)}
                  isCurrent={localRefreshToken && acc.refreshToken === localRefreshToken}
                  isRefreshing={refreshingId === acc.id}
                  isSwitching={switchingId === acc.id}
                  tagMap={tagMap}
                  groupMap={groupMap}
                  colors={colors}
                  isLightTheme={isLightTheme}
                  t={t}
                  maskEmail={maskEmail}
                  onSelectOne={onSelectOne}
                  onSwitch={onSwitch}
                  onRefresh={onRefresh}
                  onEdit={onEdit}
                  onEditLabel={onEditLabel}
                  onDelete={onDelete}
                  onDeleteRemote={onDeleteRemote}
                  onCopy={onCopy}
                />
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

export default AccountListView
