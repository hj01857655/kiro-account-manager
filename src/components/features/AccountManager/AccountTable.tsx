import { useRef, useMemo, useState, useEffect, useCallback, memo } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Users, Plus, Edit2, Copy, KeyRound , Eye , Key, Trash2, UserX, LogIn } from 'lucide-react'
import { Checkbox } from '@/components/ui/checkbox'
import { useApp } from '../../../hooks/useApp'
import { getAccountStatusMeta, isBannedStatus, isUnavailableStatus } from '../../../utils/accountStatus'
import AccountCard from './AccountCard'
import ContextMenu from './ContextMenu'
import React from 'react'

// 根据容器宽度计算列数
function getColumnCount(width: number) {
  if (width >= 1280) return 4
  if (width >= 1024) return 3
  if (width >= 768) return 2
  return 1
}

// 单独的行组件，避免重复渲染
const VirtualRow = memo(function VirtualRow({
  row,
  columns,
  selectedIdsSet,
  onSelectOne,
  copiedId,
  onCopy,
  onLogin,
  onLogout,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onToggleEnabled,
  onToggleOverage,
  onDelete,
  onDeleteRemote,
  onAdd,
  localToken,
  tagDefinitions,
  groupDefinitions,
  accountRowStateById,
  onLoadAvailableModels,
  t,
  onContextMenuOpen}: any) {
  return (
    <div className="gap-3 pb-4" style={{ display: 'grid', gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
      {row.map((item: any) => {
        if (item._isAddButton) {
          return <AddButton key="add" onClick={onAdd} t={t} />
        }
        return (
          <AccountCard
            key={item.id}
            account={item}
            selectedIdsSet={selectedIdsSet}
            onSelect={(checked: boolean) => onSelectOne(item.id, checked)}
            copiedId={copiedId}
            onCopy={onCopy}
            onLogin={onLogin}
            onLogout={onLogout}
            onRefresh={onRefresh}
            onRefreshToken={onRefreshToken}
            onEdit={onEdit}
            onEditLabel={onEditLabel}
            onToggleEnabled={onToggleEnabled}
            onToggleOverage={onToggleOverage}
            onDelete={onDelete}
            isRefreshing={Boolean(accountRowStateById?.[item.id]?.isRefreshing)}
            isRefreshingToken={Boolean(accountRowStateById?.[item.id]?.isRefreshingToken)}
            isSwitching={Boolean(accountRowStateById?.[item.id]?.isSwitching)}
            isTogglingOverage={Boolean(accountRowStateById?.[item.id]?.isTogglingOverage)}
            isCurrentAccount={localToken?.refreshToken && item.refreshToken === localToken.refreshToken}
            tagDefinitions={tagDefinitions}
            groupDefinitions={groupDefinitions}
            availableModels={accountRowStateById?.[item.id]?.availableModels ?? null}
            availableModelsLoading={Boolean(accountRowStateById?.[item.id]?.availableModelsLoading)}
            availableModelsError={accountRowStateById?.[item.id]?.availableModelsError ?? ''}
            onLoadAvailableModels={onLoadAvailableModels}
            onContextMenuOpen={(x: number, y: number) => onContextMenuOpen(item.id, x, y, item)}
          />
        )
      })}
    </div>
  )
}, (prev: any, next: any) => {
  if (prev.row !== next.row || prev.columns !== next.columns) return false
  if (prev.copiedId !== next.copiedId) return false
  if (prev.localToken !== next.localToken) return false
  if (prev.tagDefinitions !== next.tagDefinitions) return false
  if (prev.groupDefinitions !== next.groupDefinitions) return false
  if (prev.accountRowStateById !== next.accountRowStateById) return false
  if (prev.onLoadAvailableModels !== next.onLoadAvailableModels) return false
  for (const item of prev.row) {
    if (item._isAddButton) continue
    const prevSelected = prev.selectedIdsSet?.has(item.id)
    const nextSelected = next.selectedIdsSet?.has(item.id)
    if (prevSelected !== nextSelected) return false
  }
  return true
})

function AccountTable({
  accounts,
  totalCount,
  selectedIds,
  onSelectAll,
  onSelectOne,
  copiedId,
  onCopy,
  onLogin,
  onLogout,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onToggleEnabled,
  onToggleOverage,
  onDelete,
  onDeleteRemote,
  onAdd,
  localToken,
  tagDefinitions = [],
  groupDefinitions = [],
  accountRowStateById = {},
  onLoadAvailableModels}: any) {
  const { t } = useApp()
  const containerRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const [columns, setColumns] = useState(4)
  const [contextMenuState, setContextMenuState] = useState<any>(null) // { accountId, x, y, account }

  // 打开右键菜单
  const handleContextMenuOpen = useCallback((accountId: string, x: number, y: number, account: any) => {
    setContextMenuState({ accountId, x, y, account })
  }, [])

  // 关闭右键菜单
  const handleContextMenuClose = useCallback(() => {
    setContextMenuState(null)
  }, [])

  // 生成菜单项
  const getMenuItems = useCallback((account: any) => {
    const isBanned = isBannedStatus(account)
    const isUnavailable = isUnavailableStatus(account)
    const statusMeta = getAccountStatusMeta(account, t)
    const rowState = accountRowStateById?.[account.id] ?? {}

    const items: any[] = [
      { icon: Eye, label: t('accountCard.viewDetails'), onClick: () => onEdit(account) },
      { icon: Edit2, label: t('accountCard.editRemark'), onClick: () => onEditLabel(account) },
      { icon: Copy, label: t('accountCard.copyJson'), onClick: () => onCopy(JSON.stringify(account, null, 2), account.id) },
      { divider: true },
      { icon: Key , label: t('accountCard.refreshQuota'), onClick: () => onRefresh(account.id), disabled: Boolean(rowState.isRefreshing) },
      { icon: KeyRound , label: t('accountCard.refreshToken'), onClick: () => onRefreshToken?.(account.id), disabled: Boolean(rowState.isRefreshingToken) },
      { icon: LogIn, label: isUnavailable ? t('accountCard.unavailableCannotSwitch', { status: statusMeta.label }) : t('accountCard.LogIn'), onClick: () => onLogin(account), disabled: Boolean(rowState.isSwitching) || isUnavailable },
      { divider: true },
      { label: account.enabled === false ? t('accountCard.enableAccount') : t('accountCard.disableAccount'), onClick: () => onToggleEnabled?.(account, account.enabled === false) },
      { icon: Trash2, label: t('accountCard.delete'), onClick: () => onDelete(account.id), danger: true },
    ]

    if (account.provider !== 'Enterprise' && !isBanned && onDeleteRemote) {
      items.push({ icon: UserX, label: t('accountCard.deleteRemote'), onClick: () => onDeleteRemote(account), danger: true })
    }

    return items
  }, [t, onEdit, onEditLabel, onCopy, onRefreshToken, onRefresh, onLogin, onDelete, onDeleteRemote, accountRowStateById])

  useEffect(() => {
    if (!containerRef.current) return
    const updateColumns = () => {
      const width = containerRef.current?.offsetWidth ? containerRef.current.offsetWidth - 48 : 0
      setColumns(getColumnCount(width))
    }
    updateColumns()
    const observer = new ResizeObserver(updateColumns)
    observer.observe(containerRef.current)
    return () => observer.disconnect()
  }, [])

  const rows = useMemo(() => {
    const result = []
    const items = [...accounts, { _isAddButton: true }]
    for (let i = 0; i < items.length; i += columns) {
      result.push(items.slice(i, i + columns))
    }
    return result
  }, [accounts, columns])

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 220,
    overscan: 1})

  // 将 selectedIds 转为 Set 提高查找性能
  const selectedIdsSet = useMemo(() => new Set(selectedIds), [selectedIds])

  return (
    <div ref={containerRef} className="flex-1 flex flex-col overflow-hidden p-3">
      {accounts.length > 0 && (
        <div className="flex items-center justify-between mb-2 px-1 shrink-0">
          <label className="flex items-center gap-2 cursor-pointer">
            <Checkbox
              checked={selectedIds.length === accounts.length && accounts.length > 0}
              onCheckedChange={onSelectAll}
            />
            <span className="text-xs text-muted-foreground">
              {selectedIds.length > 0 ? `${t('common.selected')} ${selectedIds.length}` : t('common.selectAll')}
            </span>
          </label>
          <span className="text-xs text-muted-foreground">
            {accounts.length === totalCount ? t('accounts.totalCount', { count: totalCount }) : t('accounts.displayCount', { current: accounts.length, total: totalCount })}
          </span>
        </div>
      )}

      {accounts.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <div className="w-16 h-16 rounded-full bg-muted/30 flex items-center justify-center mb-3">
            <Users size={32} strokeWidth={1.5} className="opacity-50" />
          </div>
          <p className="text-sm font-medium mb-1">{t('common.noAccounts')}</p>
          <p className="text-xs opacity-75">{t('common.addAccountHint')}</p>
          <button onClick={onAdd} className="mt-3 px-3 h-8 rounded-md bg-muted/40 hover:bg-muted/60 transition-colors cursor-pointer text-xs inline-flex items-center gap-1.5">
            <Plus size={13} />
            {t('common.addAccount')}
          </button>
        </div>
      ) : (
        <div 
          ref={scrollRef} 
          className="flex-1 overflow-auto"
        >
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            <div
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${rowVirtualizer.getVirtualItems()[0]?.start ?? 0}px)`}}
            >
              {rowVirtualizer.getVirtualItems().map((virtualRow) => (
                <div 
                  key={virtualRow.key} 
                  data-index={virtualRow.index}
                >
                  <VirtualRow
                    row={rows[virtualRow.index]}
                    columns={columns}
                    selectedIdsSet={selectedIdsSet}
                    onSelectOne={onSelectOne}
                    copiedId={copiedId}
                    onCopy={onCopy}
                    onLogin={onLogin}
                    onLogout={onLogout}
                    onRefresh={onRefresh}
                    onRefreshToken={onRefreshToken}
                    onEdit={onEdit}
                    onEditLabel={onEditLabel}
                    onToggleEnabled={onToggleEnabled}
                    onToggleOverage={onToggleOverage}
                    onDelete={onDelete}
                    onDeleteRemote={onDeleteRemote}
                    onAdd={onAdd}
                    localToken={localToken}
                    tagDefinitions={tagDefinitions}
                    groupDefinitions={groupDefinitions}
                    accountRowStateById={accountRowStateById}
                    onLoadAvailableModels={onLoadAvailableModels}
                    t={t}
                    onContextMenuOpen={handleContextMenuOpen}
                  />
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
      
      {/* 全局右键菜单 */}
      {contextMenuState && (
        <ContextMenu
          x={contextMenuState.x}
          y={contextMenuState.y}
          onClose={handleContextMenuClose}
          items={getMenuItems(contextMenuState.account)}
        />
      )}
    </div>
  )
}

const AddButton = memo(function AddButton({ onClick, t }: any) {
  return (
    <button
      onClick={onClick}
      className="rounded-xl border-2 border-dashed border-border hover:border-primary/40 transition-colors min-h-[240px] flex flex-col items-center justify-center gap-2 cursor-pointer hover:bg-muted/20"
    >
      <div className="w-9 h-9 rounded-lg flex items-center justify-center bg-muted/40">
        <Plus size={16} className="text-muted-foreground" />
      </div>
      <span className="text-[11px] font-medium text-muted-foreground">{t('common.addAccount')}</span>
    </button>
  )
})

export default AccountTable
