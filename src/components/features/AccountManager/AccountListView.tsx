import { useRef, useMemo, memo, useState, useCallback } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Users, Plus, RefreshCw, Eye, Edit2, Trash2, Copy, UserX, ChevronUp, ChevronDown, Key, LogIn, LogOut } from 'lucide-react'
import { Checkbox } from '@/components/ui/checkbox'
import { useApp } from '../../../hooks/useApp'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { getQuota, getUsed, formatUsage, getAccountDisplayName } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus, isUnavailableStatus } from '../../../utils/accountStatus'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'
import ContextMenu from './ContextMenu'
import { buildAccountListMaps } from './utils/accountListMaps'
import type { Account, TagDefinition, GroupDefinition } from '../../../types/account'

const ROW_HEIGHT = 48

/** 紧凑徽章：统一各列徽章样式，避免每处复制一遍 class */
function Pill({ tone = 'muted', className = '', title, children }: {
  tone?: 'muted' | 'primary' | 'red' | 'orange' | 'green' | 'slate'
  className?: string
  title?: string
  children: React.ReactNode
}) {
  const toneClass: Record<string, string> = {
    muted: 'bg-muted text-muted-foreground border-border/50',
    primary: 'bg-primary text-primary-foreground border-primary/20',
    red: 'bg-red-500/10 text-red-500 border-red-500/20',
    orange: 'bg-orange-500 text-white border-orange-600',
    green: 'bg-green-500/10 text-green-500 border-green-500/20',
    slate: 'bg-slate-500/10 text-slate-500 border-slate-500/20',
  }
  return (
    <span title={title} className={`inline-flex items-center justify-center text-[10px] px-1.5 py-0.5 rounded font-bold border whitespace-nowrap ${toneClass[tone]} ${className}`}>
      {children}
    </span>
  )
}

interface ListRowProps {
  account: Account
  isSelected: boolean
  isCurrent: boolean
  isRefreshing: boolean
  isRefreshingToken: boolean
  isSwitching: boolean
  isTogglingOverage: boolean
  tagMap: Map<string, any>
  groupMap: Map<string, any>
  t: (key: string) => string
  maskEmail: (email: string) => string
  onSelectOne: (id: string, checked: any) => void
  onLogin: (account: Account) => void
  onLogout: (account: Account) => void
  onRefresh: (id: string) => void
  onRefreshToken: (id: string) => void
  onEdit: (account: Account) => void
  onEditLabel: (account: Account) => void
  onDelete: (id: string) => void
  onDeleteRemote?: (account: Account) => void
  onToggleEnabled?: (account: Account, enabled: boolean) => void
  onToggleOverage?: (account: Account, enabled: boolean) => void
  onCopy: (text: string, id: string) => void
}

const ListRow = memo(function ListRow({
  account,
  isSelected,
  isCurrent,
  isRefreshing,
  isRefreshingToken,
  isSwitching,
  isTogglingOverage,
  tagMap,
  groupMap,
  t,
  maskEmail,
  onSelectOne,
  onLogin,
  onLogout,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onDelete,
  onDeleteRemote,
  onToggleEnabled,
  onToggleOverage,
  onCopy,
}: ListRowProps) {
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null)
  const used = getUsed(account)
  const limit = getQuota(account)
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  const currentOverages = breakdown?.currentOverages ?? 0
  const isOverage = currentOverages > 0
  const usagePercent = limit > 0 ? Math.min((used / limit) * 100, 100) : 0
  const overageCapability = account.usageData?.subscriptionInfo?.overageCapability
  const overageStatus = account.usageData?.overageConfiguration?.overageStatus
  const isBanned = isBannedStatus(account)
  const isUnavailable = isUnavailableStatus(account)
  const statusMeta = getAccountStatusMeta(account, t)

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
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
    { icon: Key, label: t('accountCard.refreshToken'), onClick: () => onRefreshToken?.(account.id), disabled: isRefreshingToken },
    isCurrent
      ? { icon: LogOut, label: t('accountCard.LogOut'), onClick: () => onLogout(account), disabled: isSwitching, danger: true }
      : { icon: LogIn, label: t('accountCard.LogIn'), onClick: () => onLogin(account), disabled: isSwitching || isUnavailable },
    { divider: true },
    { label: account.enabled === false ? '启用账号' : '禁用账号', onClick: () => onToggleEnabled?.(account, account.enabled === false) },
    ...(overageCapability === 'OVERAGE_CAPABLE' ? [
      { label: overageStatus === 'ENABLED' ? '关闭超额' : '开启超额', onClick: () => onToggleOverage?.(account, overageStatus !== 'ENABLED'), disabled: isTogglingOverage },
    ] : []),
    { icon: Trash2, label: t('accountCard.delete'), onClick: () => onDelete(account.id), danger: true },
    ...(account.provider !== 'Enterprise' && !isBanned && onDeleteRemote ? [
      { icon: UserX, label: t('accountCard.deleteRemote'), onClick: () => onDeleteRemote(account), danger: true },
    ] : []),
  ], [t, account, handleCopyJson, onEdit, onEditLabel, onRefresh, onRefreshToken, onLogin, onLogout, onDelete, onDeleteRemote, onToggleEnabled, onToggleOverage, isRefreshing, isRefreshingToken, isSwitching, isTogglingOverage, isBanned, isUnavailable, isCurrent, overageCapability, overageStatus])

  const subscriptionTitle = account.usageData?.subscriptionInfo?.subscriptionTitle || ''
  const subscriptionTone: Parameters<typeof Pill>[0]['tone'] =
    subscriptionTitle.toUpperCase().includes('ENTERPRISE') ? 'orange'
    : subscriptionTitle.includes('PRO') ? 'primary'
    : 'muted'
  const providerTone: Parameters<typeof Pill>[0]['tone'] =
    account.provider === 'Google' ? 'red'
    : isGitHubProvider(account.provider) ? 'slate'
    : 'muted'
  const isExpired = account.expiresAt && new Date(account.expiresAt.replace(/\//g, '-')) < new Date()
  const trialExpiry = account.usageData?.usageBreakdownList?.[0]?.freeTrialInfo?.freeTrialExpiry

  return (
    <div
      onContextMenu={handleContextMenu}
      className={`group relative flex items-center gap-3 px-3 border-b border-border hover:bg-muted/30 cursor-context-menu animate-stagger ${isCurrent ? 'bg-green-500/5' : ''} ${account.enabled === false ? 'opacity-50' : ''}`}
      style={{ height: ROW_HEIGHT, animationDelay: `${Math.min(account._index || 0, 20) * 30}ms` }}
    >
      {contextMenu && (
        <ContextMenu x={contextMenu.x} y={contextMenu.y} onClose={() => setContextMenu(null)} items={getMenuItems()} />
      )}

      <Checkbox
        checked={isSelected}
        onCheckedChange={(checked) => onSelectOne(account.id, checked)}
        onClick={(e) => e.stopPropagation()}
        className="shrink-0 cursor-pointer"
      />

      {/* 邮箱 + 备注 */}
      <div className="w-36 shrink-0 min-w-0">
        <div className="flex items-center gap-1.5">
          <span className="text-xs font-medium truncate text-foreground">
            {account.email ? maskEmail(account.email) : getAccountDisplayName(account)}
          </span>
          {isCurrent && <Pill tone="green" className="bg-green-500 text-white border-green-600 px-1 shrink-0">LIVE</Pill>}
        </div>
        {account.label && (
          <span className="text-[10px] text-muted-foreground truncate block leading-tight">{account.label}</span>
        )}
      </div>

      {/* Provider */}
      <Pill tone={providerTone} className="w-16 shrink-0">
        {getProviderDisplayName(account.provider) || '—'}
      </Pill>

      {/* 订阅 */}
      <Pill tone={subscriptionTone} className="w-16 shrink-0">
        {subscriptionTitle || 'Free'}
      </Pill>

      {/* 配额 */}
      <div className="w-24 shrink-0">
        <div className="flex items-center justify-between">
          <span className={`text-[11px] font-bold ${isOverage ? 'text-purple-500' : used >= limit && limit > 0 ? 'text-red-500' : 'text-foreground'}`}>
            {isOverage ? formatUsage(limit) : formatUsage(used)}
          </span>
          <span className="text-[10px] text-muted-foreground">/{formatUsage(limit)}</span>
        </div>
        <div className="h-1 rounded-full bg-muted mt-0.5 overflow-hidden">
          <div
            className={`h-full rounded-full ${isOverage ? 'bg-purple-500' : usagePercent > 80 ? 'bg-red-500' : usagePercent > 50 ? 'bg-orange-500' : 'bg-green-500'}`}
            style={{ width: `${usagePercent}%` }}
          />
        </div>
        {isOverage && (
          <div className="flex items-center justify-between mt-0.5">
            <span className="text-[9px] text-purple-500 font-bold">⚡{formatUsage(currentOverages)}{breakdown?.overageCap ? `/${formatUsage(breakdown.overageCap)}` : ''}</span>
            {breakdown?.overageCharges != null && breakdown.overageCharges > 0 && (
              <span className="text-[9px] text-purple-500">${breakdown.overageCharges.toFixed(2)}</span>
            )}
          </div>
        )}
        {!isOverage && overageCapability === 'OVERAGE_CAPABLE' && (
          <span className={`text-[9px] mt-0.5 block ${overageStatus === 'ENABLED' ? 'text-green-500' : 'text-muted-foreground'}`}>
            {overageStatus === 'ENABLED' ? '⚡超额已开' : '⚡可开超额'}
          </span>
        )}
      </div>

      {/* 状态 */}
      <Pill
        tone={statusMeta.key === 'active' ? 'green' : 'red'}
        className="w-12 shrink-0 uppercase"
      >
        {statusMeta.label}
      </Pill>

      {/* 过期 / 试用 */}
      <div className="w-28 shrink-0 text-[10px] text-muted-foreground leading-tight">
        {account.expiresAt ? (
          <span className={isExpired ? 'text-red-500 font-bold' : ''}>
            {account.expiresAt.slice(5, 16).replace('/', '-')}
          </span>
        ) : '—'}
        {trialExpiry && (
          <span className="text-orange-500 ml-1" title="试用到期">
            · {new Date(trialExpiry * 1000).toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })}
          </span>
        )}
      </div>

      {/* 分组 */}
      <div className="w-16 shrink-0">
        {account.groupId
          ? (() => {
            const group = groupMap.get(account.groupId)
            return group ? (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded font-bold truncate block max-w-full bg-muted/50 border border-border/50"
                style={{ color: group.color }}
                title={group.name}
              >
                {group.name}
              </span>
            ) : <span className="text-xs text-muted-foreground">—</span>
          })()
          : <span className="text-xs text-muted-foreground">—</span>}
      </div>

      {/* 标签 */}
      <div className="flex-[1.5] min-w-[80px] min-w-0">
        {account.tagLinks && account.tagLinks.length > 0 ? (
          <div className="flex items-center gap-1 flex-wrap">
            {account.tagLinks.slice(0, 3).map(tagLink => {
              const tag = tagMap.get(tagLink.tagId)
              const tagName = tag?.name || tagLink.tagName || '标签'
              const tagColor = tag?.color || '#888888'
              return (
                <span
                  key={tagLink.tagId}
                  className="text-[10px] px-1.5 py-0.5 rounded-full font-medium truncate max-w-[100px] border border-border/20"
                  style={{ backgroundColor: `${tagColor}20`, color: tagColor }}
                >
                  {tagName}
                </span>
              )
            })}
            {account.tagLinks.length > 3 && <span className="text-[10px] text-muted-foreground">+{account.tagLinks.length - 3}</span>}
          </div>
        ) : <span className="text-xs text-muted-foreground">—</span>}
      </div>

      {/* 操作按钮（hover 时绝对定位浮起，避免占用列宽） */}
      <div className="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity bg-card/95 backdrop-blur-sm rounded-md shadow-sm border border-border px-1 py-1">
        {isCurrent ? (
          <button
            onClick={(e) => { e.stopPropagation(); onLogout(account) }}
            disabled={isSwitching}
            className="h-7 w-7 rounded-md inline-flex items-center justify-center bg-destructive/10 text-destructive hover:bg-destructive/20 transition-colors disabled:opacity-50"
            title={t('accountCard.LogOut')}
          >
            <LogOut size={13} className={isSwitching ? 'animate-spin' : ''} />
          </button>
        ) : (
          <button
            onClick={(e) => { e.stopPropagation(); onLogin(account) }}
            disabled={isSwitching || isUnavailable}
            className="h-7 w-7 rounded-md inline-flex items-center justify-center bg-primary/10 text-primary hover:bg-primary/20 transition-colors disabled:opacity-50"
            title={t('accountCard.LogIn')}
          >
            <LogIn size={13} className={isSwitching ? 'animate-spin' : ''} />
          </button>
        )}
        <button
          onClick={(e) => { e.stopPropagation(); onEdit(account) }}
          className="h-7 w-7 rounded-md inline-flex items-center justify-center hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
          title={t('accountCard.viewDetails')}
        >
          <Eye size={13} />
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onEditLabel(account) }}
          className="h-7 w-7 rounded-md inline-flex items-center justify-center hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
          title={t('accountCard.editRemark')}
        >
          <Edit2 size={13} />
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onRefresh(account.id) }}
          disabled={isRefreshing}
          className="h-7 w-7 rounded-md inline-flex items-center justify-center hover:bg-muted text-muted-foreground hover:text-primary transition-colors disabled:opacity-50"
          title={t('accountCard.refreshQuota')}
        >
          <RefreshCw size={13} className={isRefreshing ? 'animate-spin' : ''} />
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(account.id) }}
          className="h-7 w-7 rounded-md inline-flex items-center justify-center hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-colors"
          title={t('accountCard.delete')}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </div>
  )
}, (prev, next) => (
  prev.account === next.account &&
  prev.isSelected === next.isSelected &&
  prev.isCurrent === next.isCurrent &&
  prev.isRefreshing === next.isRefreshing &&
  prev.isRefreshingToken === next.isRefreshingToken &&
  prev.isSwitching === next.isSwitching &&
  prev.tagMap === next.tagMap &&
  prev.groupMap === next.groupMap
))

interface AccountListViewProps {
  accounts: Account[]
  totalCount: number
  selectedIds: string[]
  selectedIdsSet?: Set<string>
  onSelectAll: (checked: any) => void
  onSelectOne: (id: string, checked: any) => void
  onLogin: (account: Account) => void
  onLogout: (account: Account) => void
  onRefresh: (id: string) => void
  onRefreshToken: (id: string) => void
  onEdit: (account: Account) => void
  onEditLabel: (account: Account) => void
  onDelete: (id: string) => void
  onDeleteRemote?: (account: Account) => void
  onToggleEnabled?: (account: Account, enabled: boolean) => void
  onToggleOverage?: (account: Account, enabled: boolean) => void
  onCopy: (text: string, id: string) => void
  onAdd: () => void
  accountRowStateById?: Record<string, { isRefreshing?: boolean; isRefreshingToken?: boolean; isSwitching?: boolean; isTogglingOverage?: boolean }>
  localToken?: { refreshToken?: string } | null
  tagDefinitions?: TagDefinition[]
  groupDefinitions?: GroupDefinition[]
  sortBy?: string
  onSortChange?: (sort: string) => void
  [key: string]: any
}

function AccountListView({
  accounts,
  totalCount,
  selectedIds,
  selectedIdsSet,
  onSelectAll,
  onSelectOne,
  onLogin,
  onLogout,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onDelete,
  onDeleteRemote,
  onToggleEnabled,
  onToggleOverage,
  onCopy,
  onAdd,
  accountRowStateById = {},
  localToken,
  tagDefinitions = [],
  groupDefinitions = [],
  sortBy,
  onSortChange,
}: AccountListViewProps) {
  const { t } = useApp()
  const { maskEmail } = usePrivacy()
  const scrollRef = useRef<HTMLDivElement | null>(null)

  const _selectedIdsSet = selectedIdsSet || useMemo(() => new Set(selectedIds), [selectedIds])
  const localRefreshToken = localToken?.refreshToken
  const { tagMap, groupMap } = useMemo(() => buildAccountListMaps({
    tagDefinitions,
    groupDefinitions,
  }), [tagDefinitions, groupDefinitions])

  const rowVirtualizer = useVirtualizer({
    count: accounts.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 5,
  })

  const handleSort = useCallback((field: string) => {
    if (!onSortChange) return
    if (sortBy === `${field}Asc`) {
      onSortChange(`${field}Desc`)
    } else if (sortBy === `${field}Desc`) {
      onSortChange('default')
    } else {
      onSortChange(`${field}Asc`)
    }
  }, [sortBy, onSortChange])

  const SortIcon = ({ field }: { field: string }) => {
    if (sortBy === `${field}Asc`) return <ChevronUp size={12} className="inline ml-0.5" />
    if (sortBy === `${field}Desc`) return <ChevronDown size={12} className="inline ml-0.5" />
    return null
  }

  if (accounts.length === 0) {
    return (
      <div className="flex-1 flex flex-col overflow-hidden p-3">
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <div className="w-16 h-16 rounded-full bg-muted/30 flex items-center justify-center mb-3">
            <Users size={32} strokeWidth={1.5} className="opacity-50" />
          </div>
          <p className="text-sm font-medium mb-1">{t('common.noAccounts')}</p>
          <button onClick={onAdd} className="mt-3 px-3 h-8 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:opacity-90 inline-flex items-center gap-1.5">
            <Plus size={13} />{t('common.addAccount')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden p-3">
      <div className="flex items-center justify-between mb-2 px-1 shrink-0">
        <label className="flex items-center gap-2 cursor-pointer">
          <Checkbox
            checked={selectedIds.length === accounts.length && accounts.length > 0}
            onCheckedChange={(checked) => onSelectAll(checked)}
          />
          <span className="text-xs text-muted-foreground">
            {selectedIds.length > 0 ? `${t('common.selected')} ${selectedIds.length}` : t('common.selectAll')}
          </span>
        </label>
        <span className="text-xs text-muted-foreground">
          {accounts.length === totalCount ? `共 ${totalCount} 个账号` : `${accounts.length} / ${totalCount} 个账号`}
        </span>
      </div>

      {/* 表头 */}
      <div className="flex items-center gap-3 px-3 h-9 bg-muted/50 border border-border rounded-t-md text-muted-foreground text-[10px] font-bold uppercase tracking-wider">
        <div className="w-4" />
        <div className="w-36">邮箱</div>
        <div className="w-16 text-center">来源</div>
        <div className="w-16 text-center">订阅</div>
        <button type="button" onClick={() => handleSort('usage')} className="w-24 text-left hover:text-primary transition-colors">
          配额<SortIcon field="usage" />
        </button>
        <div className="w-12 text-center">状态</div>
        <button type="button" onClick={() => handleSort('trial')} className="w-28 text-left hover:text-primary transition-colors">
          过期 / 试用<SortIcon field="trial" />
        </button>
        <div className="w-16">分组</div>
        <div className="flex-[1.5] min-w-[80px]">标签</div>
      </div>

      {/* 列表 */}
      <div ref={scrollRef} className="flex-1 overflow-auto border border-t-0 border-border rounded-b-md bg-card/30 no-scrollbar">
        <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
          {rowVirtualizer.getVirtualItems().map((vRow) => {
            const acc = accounts[vRow.index]
            return (
              <div
                key={acc.id}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  transform: `translateY(${vRow.start}px)`,
                }}
              >
                <ListRow
                  account={acc}
                  isSelected={_selectedIdsSet.has(acc.id)}
                  isCurrent={!!localRefreshToken && acc.refreshToken === localRefreshToken}
                  isRefreshing={accountRowStateById[acc.id]?.isRefreshing ?? false}
                  isRefreshingToken={accountRowStateById[acc.id]?.isRefreshingToken ?? false}
                  isSwitching={accountRowStateById[acc.id]?.isSwitching ?? false}
                  isTogglingOverage={accountRowStateById[acc.id]?.isTogglingOverage ?? false}
                  tagMap={tagMap}
                  groupMap={groupMap}
                  t={t}
                  maskEmail={maskEmail}
                  onSelectOne={onSelectOne}
                  onLogin={onLogin}
                  onLogout={onLogout}
                  onRefresh={onRefresh}
                  onRefreshToken={onRefreshToken}
                  onEdit={onEdit}
                  onEditLabel={onEditLabel}
                  onDelete={onDelete}
                  onDeleteRemote={onDeleteRemote}
                  onToggleEnabled={onToggleEnabled}
                  onToggleOverage={onToggleOverage}
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
