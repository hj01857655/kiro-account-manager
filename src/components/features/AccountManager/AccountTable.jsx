import { useRef, useMemo, useState, useEffect, useCallback, memo } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Users, Plus, Eye, Edit2, Copy, Key, BarChart3, Repeat, Trash2, UserX } from 'lucide-react'
import { Checkbox } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { useApp } from '../../../hooks/useApp'
import { useTheme } from '../../../contexts/ThemeContext'
import { getAccountStatusMeta, isBannedStatus, isUnavailableStatus } from '../../../utils/accountStatus'
import AccountCard from './AccountCard'
import ContextMenu from './ContextMenu'

// 根据容器宽度计算列数
function getColumnCount(width) {
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
  onSwitch,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onDelete,
  onDeleteRemote,
  onAdd,
  refreshingId,
  refreshingTokenId,
  switchingId,
  localToken,
  tagDefinitions,
  groupDefinitions,
  availableModelsById,
  availableModelsLoadingById,
  availableModelsErrorById,
  onLoadAvailableModels,
  colors,
  t,
  onContextMenuOpen,
}) {
  return (
    <div className="gap-6 pb-6" style={{ display: 'grid', gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
      {row.map(item => {
        if (item._isAddButton) {
          return <AddButton key="add" onClick={onAdd} colors={colors} t={t} />
        }
        return (
          <AccountCard
            key={item.id}
            account={item}
            selectedIdsSet={selectedIdsSet}
            onSelect={(checked) => onSelectOne(item.id, checked)}
            copiedId={copiedId}
            onCopy={onCopy}
            onSwitch={onSwitch}
            onRefresh={onRefresh}
            onRefreshToken={onRefreshToken}
            onEdit={onEdit}
            refreshingId={refreshingId}
            refreshingTokenId={refreshingTokenId}
            switchingId={switchingId}
            isCurrentAccount={localToken?.refreshToken && item.refreshToken === localToken.refreshToken}
            tagDefinitions={tagDefinitions}
            groupDefinitions={groupDefinitions}
            availableModels={availableModelsById?.[item.id] ?? null}
            availableModelsLoading={Boolean(availableModelsLoadingById?.[item.id])}
            availableModelsError={availableModelsErrorById?.[item.id] ?? ''}
            onLoadAvailableModels={onLoadAvailableModels}
            onContextMenuOpen={(x, y) => onContextMenuOpen(item.id, x, y, item)}
          />
        )
      })}
    </div>
  )
}, (prev, next) => {
  // 只在行数据或关键状态变化时重渲染
  if (prev.row !== next.row || prev.columns !== next.columns) return false
  if (prev.copiedId !== next.copiedId) return false
  if (prev.refreshingId !== next.refreshingId) return false
  if (prev.refreshingTokenId !== next.refreshingTokenId) return false
  if (prev.switchingId !== next.switchingId) return false
  if (prev.localToken !== next.localToken) return false
  if (prev.tagDefinitions !== next.tagDefinitions) return false
  if (prev.groupDefinitions !== next.groupDefinitions) return false
  if (prev.availableModelsById !== next.availableModelsById) return false
  if (prev.availableModelsLoadingById !== next.availableModelsLoadingById) return false
  if (prev.availableModelsErrorById !== next.availableModelsErrorById) return false
  if (prev.onLoadAvailableModels !== next.onLoadAvailableModels) return false
  // selectedIdsSet 比较：检查行内账号的选中状态是否变化
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
  onSwitch,
  onRefresh,
  onRefreshToken,
  onEdit,
  onEditLabel,
  onDelete,
  onDeleteRemote,
  onAdd,
  refreshingId,
  refreshingTokenId,
  switchingId,
  localToken,
  tagDefinitions = [],
  groupDefinitions = [],
  availableModelsById = {},
  availableModelsLoadingById = {},
  availableModelsErrorById = {},
  onLoadAvailableModels,
}) {
  const { t, colors } = useApp()
  const containerRef = useRef(null)
  const scrollRef = useRef(null)
  const [columns, setColumns] = useState(4)
  const [contextMenuState, setContextMenuState] = useState(null) // { accountId, x, y, account }

  // 打开右键菜单
  const handleContextMenuOpen = useCallback((accountId, x, y, account) => {
    setContextMenuState({ accountId, x, y, account })
  }, [])

  // 关闭右键菜单
  const handleContextMenuClose = useCallback(() => {
    setContextMenuState(null)
  }, [])

  // 生成菜单项
  const getMenuItems = useCallback((account) => {
    const isBanned = isBannedStatus(account.status)
    const isUnavailable = isUnavailableStatus(account.status)
    const statusMeta = getAccountStatusMeta(account.status, t)
    
    const items = [
      { icon: Eye, label: t('accountCard.viewDetails'), onClick: () => onEdit(account) },
      { icon: Edit2, label: t('accountCard.editRemark'), onClick: () => onEditLabel(account) },
      { icon: Copy, label: t('accountCard.copyJson'), onClick: () => onCopy(JSON.stringify(account, null, 2), account.id) },
      { divider: true },
      { icon: Key, label: t('accountCard.refreshToken'), onClick: () => onRefreshToken?.(account.id), disabled: refreshingTokenId === account.id },
      { icon: BarChart3, label: t('accountCard.refreshQuota'), onClick: () => onRefresh(account.id), disabled: refreshingId === account.id },
      { icon: Repeat, label: isUnavailable ? `${statusMeta.label}账号不可切换` : t('accountCard.switchAccount'), onClick: () => onSwitch(account), disabled: switchingId === account.id || isUnavailable },
      { divider: true },
      { icon: Trash2, label: t('accountCard.delete'), onClick: () => onDelete(account.id), danger: true },
    ]
    
    // Enterprise 账号或已封禁账号不显示远程删除
    if (account.provider !== 'Enterprise' && !isBanned && onDeleteRemote) {
      items.push({ icon: UserX, label: t('accountCard.deleteRemote'), onClick: () => onDeleteRemote(account), danger: true })
    }
    
    return items
  }, [t, onEdit, onEditLabel, onCopy, onRefreshToken, onRefresh, onSwitch, onDelete, onDeleteRemote, refreshingTokenId, refreshingId, switchingId])

  useEffect(() => {
    if (!containerRef.current) return
    const updateColumns = () => {
      const width = containerRef.current?.offsetWidth - 48 || 0
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
    estimateSize: () => 290,
    overscan: 1,
    measureElement: (el) => el?.getBoundingClientRect().height ?? 290,
  })

  // 将 selectedIds 转为 Set 提高查找性能
  const selectedIdsSet = useMemo(() => new Set(selectedIds), [selectedIds])

  return (
    <div ref={containerRef} className="flex-1 flex flex-col overflow-hidden p-6">
      {accounts.length > 0 && (
        <div className="flex items-center justify-between mb-4 px-1 shrink-0">
          <Checkbox
            checked={selectedIds.length === accounts.length && accounts.length > 0}
            onChange={(e) => onSelectAll(e.currentTarget.checked)}
            label={
              <span className={`text-sm ${colors.textMuted}`}>
                {selectedIds.length > 0 ? `${t('common.selected')} ${selectedIds.length}` : t('common.selectAll')}
              </span>
            }
            styles={{
              root: { cursor: 'pointer' },
              input: { cursor: 'pointer' },
              label: { cursor: 'pointer' },
            }}
          />
          <span className={`text-sm ${colors.textMuted}`}>
            {accounts.length === totalCount ? `共 ${totalCount} 个账号` : `${accounts.length} / ${totalCount} 个账号`}
          </span>
        </div>
      )}

      {accounts.length === 0 ? (
        <div className={`flex flex-col items-center justify-center py-20 ${colors.textMuted}`}>
          <div className={`w-20 h-20 rounded-full ${colors.cardSecondary} flex items-center justify-center animate-float mb-4`}>
            <Users size={40} strokeWidth={1} className="opacity-50" />
          </div>
          <p className="font-medium mb-1">{t('common.noAccounts')}</p>
          <p className="text-sm opacity-75">{t('common.addAccountHint')}</p>
          <button onClick={onAdd} className={`mt-4 px-4 py-2 rounded-xl ${colors.cardSecondary} ${colors.cardHover} transition-colors`}>
            <Plus size={16} className="inline mr-1" />
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
                transform: `translateY(${rowVirtualizer.getVirtualItems()[0]?.start ?? 0}px)`,
              }}
            >
              {rowVirtualizer.getVirtualItems().map((virtualRow) => (
                <div 
                  key={virtualRow.key} 
                  data-index={virtualRow.index}
                  ref={rowVirtualizer.measureElement}
                >
                  <VirtualRow
                    row={rows[virtualRow.index]}
                    columns={columns}
                    selectedIdsSet={selectedIdsSet}
                    onSelectOne={onSelectOne}
                    copiedId={copiedId}
                    onCopy={onCopy}
                    onSwitch={onSwitch}
                    onRefresh={onRefresh}
                    onRefreshToken={onRefreshToken}
                    onEdit={onEdit}
                    onEditLabel={onEditLabel}
                    onDelete={onDelete}
                    onDeleteRemote={onDeleteRemote}
                    onAdd={onAdd}
                    refreshingId={refreshingId}
                    refreshingTokenId={refreshingTokenId}
            switchingId={switchingId}
            localToken={localToken}
            tagDefinitions={tagDefinitions}
            groupDefinitions={groupDefinitions}
            availableModelsById={availableModelsById}
            availableModelsLoadingById={availableModelsLoadingById}
            availableModelsErrorById={availableModelsErrorById}
            onLoadAvailableModels={onLoadAvailableModels}
            colors={colors}
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

const AddButton = memo(function AddButton({ onClick, colors, t }) {
  return (
    <button
      onClick={onClick}
      className={`rounded-2xl border-2 border-dashed min-h-[240px] flex flex-col items-center justify-center gap-2.5 ${colors.dashedBorder} ${colors.dashedBorderHover}`}
    >
      <div className={`w-10 h-10 rounded-full flex items-center justify-center ${colors.cardSecondary}`}>
        <Plus size={20} className={colors.textMuted} />
      </div>
      <span className={`text-xs font-medium ${colors.textMuted}`}>{t('common.addAccount')}</span>
    </button>
  )
})

export default AccountTable
