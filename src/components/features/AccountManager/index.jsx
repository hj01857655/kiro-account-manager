import { useState, useCallback, useMemo, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Upload } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { useAccounts } from './hooks/useAccounts'
import { useSwitchAccount } from './hooks/useSwitchAccount'
import { getTags, getGroups } from '../../../api/groupTag'
import { applyFilters } from './utils/filterUtils'
import { cn } from '../../../utils/cn'
import { showSuccess, showError } from '../../../utils/toast.jsx'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { isActiveStatus, isBannedStatus, isExpiredStatus, isInvalidStatus } from '../../../utils/accountStatus'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import AccountHeader from './AccountHeader'
import AccountTable from './AccountTable'
import AccountListView from './AccountListView'
import ImportAccountModal from './ImportAccountModal'
import RefreshProgressModal from './RefreshProgressModal'
import AccountDetailModal from '../../modals/AccountDetailModal'
import EditAccountModal from './EditAccountModal'
import BatchTagModal from './BatchTagModal'
import ConfirmModal from './ConfirmModal'
import { AccountListSkeleton, AccountTableSkeleton } from '../../shared/Skeleton'

function AccountManager({ onNavigate }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const { showConfirm } = useDialog()
  const [searchTerm, setSearchTerm] = useState('')
  const [selectedIds, setSelectedIds] = useState([])
  
  // 优化：将 selectedIds 转为 Set，提升查找性能（O(1) vs O(n)）
  const selectedIdsSet = useMemo(() => new Set(selectedIds), [selectedIds])
  const [editingAccount, setEditingAccount] = useState(null)
  const [editingLabelAccount, setEditingLabelAccount] = useState(null)
  const [showImportModal, setShowImportModal] = useState(false)
  const [showBatchTagModal, setShowBatchTagModal] = useState(false)
  const [copiedId, setCopiedId] = useState(null)
  const [selectedTag, setSelectedTag] = useState(null)
  const [selectedStatus, setSelectedStatus] = useState(null)
  const [viewMode, setViewMode] = useState(() => localStorage.getItem('accountViewMode') || 'card')
  const [tagDefinitions, setTagDefinitions] = useState([])
  const [groupDefinitions, setGroupDefinitions] = useState([])
  const [selectedGroup, setSelectedGroup] = useState(null)
  const [advancedFilters, setAdvancedFilters] = useState({
    subscriptions: [],
    statuses: [],
    providers: [],
    usageRange: null
  })
  const [sortBy, setSortBy] = useState('trialAsc')
  const [refreshingTokenId, setRefreshingTokenId] = useState(null)
  
  // 当前登录的本地 token
  const [localToken, setLocalToken] = useState(null)
  
  // 用于管理复制提示的timer
  const copiedTimerRef = useRef(null)
  
  // 切换账号 hook
  const {
    switchingId,
    setSwitchingId,
    switchDialog,
    handleSwitchAccount,
    confirmSwitch,
    closeSwitchDialog,
  } = useSwitchAccount(setLocalToken)
  
  useEffect(() => {
    invoke('get_kiro_local_token').then(setLocalToken).catch(() => setLocalToken(null))
  }, [])

  // 加载标签定义
  const loadTagDefinitions = useCallback(() => {
    getTags()
      .then(tags => {
        setTagDefinitions(tags)
      })
      .catch(err => {
        // 静默处理
      })
  }, [])

  // 加载分组定义
  const loadGroupDefinitions = useCallback(() => {
    getGroups().then(setGroupDefinitions).catch(() => {})
  }, [])

  useEffect(() => {
    loadTagDefinitions()
    loadGroupDefinitions()
  }, [])

  // 清理timer
  useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  const {
    accounts,
    loading,
    loadAccounts,
    autoRefreshing,
    refreshProgress,
    lastRefreshTime,
    refreshingId,
    batchRefreshAccounts,
    handleRefreshStatus,
    handleExport,
  } = useAccounts()

  // 包装刷新函数，添加 toast 通知
  const handleRefreshWithNotify = useCallback(async (id) => {
    const result = await handleRefreshStatus(id)
    if (result.success) {
      showSuccess(t('accounts.refreshSuccess'))
    } else if (result.error) {
      const errorMsg = result.error
      if (errorMsg.includes('BANNED')) {
        showError(t('accounts.accountBanned'))
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
        showError(t('accounts.tokenInvalid'))
      } else if (errorMsg.includes('error sending request') || errorMsg.includes('connection') || errorMsg.includes('network') || errorMsg.includes('timeout')) {
        showError('❌ 网络连接失败\n\n可能原因：\n• 网络不稳定\n• 代理设置有误\n• 防火墙拦截\n\n解决方法：\n1. 检查网络连接\n2. 检查代理设置\n3. 关闭防火墙或添加白名单')
      } else {
        showError(errorMsg.slice(0, 100))
      }
    }
    return result
  }, [handleRefreshStatus, t])

  // 刷新 Token
  const handleRefreshToken = useCallback(async (id) => {
    setRefreshingTokenId(id)
    try {
      const account = await invoke('refresh_account_token', { id })
      showSuccess('Token 刷新成功')
      return { success: true, account }
    } catch (e) {
      const errorMsg = String(e)
      if (errorMsg.includes('BANNED')) {
        showError('账号已封禁')
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
        showError('Token 无效，刷新失败')
      } else if (errorMsg.includes('error sending request') || errorMsg.includes('connection') || errorMsg.includes('network') || errorMsg.includes('timeout')) {
        showError('❌ 网络连接失败\n\n可能原因：\n• 网络不稳定\n• 代理设置有误\n• 防火墙拦截\n\n解决方法：\n1. 检查网络连接\n2. 检查代理设置\n3. 关闭防火墙或添加白名单')
      } else {
        showError(errorMsg.slice(0, 100))
      }
      return { success: false, error: errorMsg }
    } finally {
      await loadAccounts()
      setRefreshingTokenId(null)
    }
  }, [loadAccounts])

  // 获取所有标签（从标签定义中获取）
  const allTags = useMemo(() => {
    // 收集账号中使用的标签 ID（从 tagLinks 中提取）
    const usedTagIds = new Set()
    accounts.forEach(a => {
      if (a.tagLinks) a.tagLinks.forEach(link => usedTagIds.add(link.tagId))
    })
    // 返回被使用的标签定义
    return tagDefinitions.filter(t => usedTagIds.has(t.id))
  }, [accounts, tagDefinitions])

  // 当选中的标签不存在时，重置筛选（排除 __none__ 和 __has__ 特殊值）
  useEffect(() => {
    if (selectedTag && selectedTag !== '__none__' && selectedTag !== '__has__' && !allTags.find(t => t.id === selectedTag)) {
      setSelectedTag(null)
    }
  }, [allTags, selectedTag])

  // 获取试用到期时间戳（使用 useCallback 缓存）
  const getTrialExpiry = useCallback((account) => {
    const expiry = account.usageData?.usageBreakdownList?.[0]?.freeTrialInfo?.freeTrialExpiry
    if (!expiry) return Infinity // 没有试用的排最后
    return expiry
  }, [])

  // 获取使用量（已用绝对值）（使用 useCallback 缓存）
  const getUsageAmount = useCallback((account) => {
    const breakdown = account.usageData?.usageBreakdownList?.[0]
    if (!breakdown) return 0
    const mainUsed = breakdown.currentUsage || 0
    const trialUsed = breakdown.freeTrialInfo?.currentUsage || 0
    const bonusUsed = (breakdown.bonuses || []).reduce((sum, b) => sum + (b.currentUsage || 0), 0)
    return mainUsed + trialUsed + bonusUsed
  }, [])

  // 优化：将 tagDefinitions 转为 Map，提升查找性能
  const tagDefinitionsMap = useMemo(() => {
    return new Map(tagDefinitions.map(t => [t.id, t]))
  }, [tagDefinitions])

  const filteredAccounts = useMemo(() => {
    let result = accounts.filter(a => {
      const term = searchTerm.toLowerCase()
      // 搜索过滤：邮箱/用户ID、备注、标签名称（从 tagLinks 中提取）
      const displayName = getAccountDisplayName(a).toLowerCase()
      // 优化：使用 Map 查找标签名称，避免多次 find
      const tagNames = (a.tagLinks || [])
        .map(link => tagDefinitionsMap.get(link.tagId)?.name || '')
        .join(' ')
        .toLowerCase()
      const matchSearch = displayName.includes(term) ||
        a.label.toLowerCase().includes(term) ||
        tagNames.includes(term)
      // 分组过滤（__none__ 表示无分组，__has__ 表示有分组）
      const matchGroup = !selectedGroup ||
        (selectedGroup === '__none__' ? !a.groupId :
         selectedGroup === '__has__' ? !!a.groupId :
         a.groupId === selectedGroup)
      // 标签过滤（按 ID，__none__ 表示筛选无标签账号，__has__ 表示筛选有标签账号）
      const tagIds = (a.tagLinks || []).map(link => link.tagId)
      const matchTag = !selectedTag || 
        (selectedTag === '__none__' ? tagIds.length === 0 : 
         selectedTag === '__has__' ? tagIds.length > 0 :
         tagIds.includes(selectedTag))
      // 状态过滤
      const matchStatus = !selectedStatus || 
        (selectedStatus === 'active' && isActiveStatus(a.status)) ||
        (selectedStatus === 'banned' && isBannedStatus(a.status)) ||
        (selectedStatus === 'invalid' && isInvalidStatus(a.status)) ||
        (selectedStatus === 'expired' && isExpiredStatus(a.status))
      return matchSearch && matchGroup && matchTag && matchStatus
    })
    // 应用高级筛选
    result = applyFilters(result, advancedFilters)
    
    // 排序
    if (sortBy !== 'default') {
      result = [...result].sort((a, b) => {
        switch (sortBy) {
          case 'trialAsc':
            return getTrialExpiry(a) - getTrialExpiry(b)
          case 'trialDesc':
            return getTrialExpiry(b) - getTrialExpiry(a)
          case 'usageAsc':
            return getUsageAmount(a) - getUsageAmount(b)
          case 'usageDesc':
            return getUsageAmount(b) - getUsageAmount(a)
          case 'addedAsc':
            return new Date(a.addedAt || 0) - new Date(b.addedAt || 0)
          case 'addedDesc':
            return new Date(b.addedAt || 0) - new Date(a.addedAt || 0)
          default:
            return 0
        }
      })
    }
    return result
  }, [accounts, searchTerm, selectedGroup, selectedTag, selectedStatus, tagDefinitionsMap, advancedFilters, sortBy, getTrialExpiry, getUsageAmount])

  const handleSearchChange = useCallback((term) => { setSearchTerm(term) }, [])
  const handleGroupFilter = useCallback((group) => { setSelectedGroup(group) }, [])
  const handleTagFilter = useCallback((tag) => { setSelectedTag(tag) }, [])
  const handleStatusFilter = useCallback((status) => { setSelectedStatus(status) }, [])
  const handleViewModeChange = useCallback((mode) => {
    setViewMode(mode)
    localStorage.setItem('accountViewMode', mode)
  }, [])
  const handleSelectAll = useCallback((checked) => {
    setSelectedIds(checked ? filteredAccounts.map(a => a.id) : [])
  }, [filteredAccounts])

  const handleSelectOne = useCallback((id, checked) => {
    setSelectedIds(prev => checked ? [...prev, id] : prev.filter(i => i !== id))
  }, [])
  const handleCopy = useCallback((text, id) => { 
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopiedId(id)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
    copiedTimerRef.current = setTimeout(() => setCopiedId(null), 1500)
  }, [])
  
  // 删除单个账号
  const handleDelete = useCallback(async (id) => {
    // 防呆：检查是否是当前账号
    const account = accounts.find(a => a.id === id)
    const isCurrent = localToken?.refreshToken && account?.refreshToken === localToken.refreshToken
    
    if (isCurrent) {
      const confirmed = await showConfirm(
        '⚠️ 删除当前账号',
        '您正在删除当前使用的账号！\n\n删除后 Kiro IDE 将无法使用，需要重新登录。\n\n确定要删除吗？'
      )
      if (!confirmed) return
    } else {
      const confirmed = await showConfirm(t('accounts.delete'), t('accounts.confirmDelete'))
      if (!confirmed) return
    }
    
    await invoke('delete_account', { id })
    loadAccounts()
  }, [accounts, localToken, showConfirm, loadAccounts, t])

  // 远程删除账号（从 AWS 服务端注销）
  const handleDeleteRemote = useCallback(async (account) => {
    const confirmed = await showConfirm(
      '⚠️ ' + t('accountCard.deleteRemote'),
      '远程删除将从 AWS 服务端注销此账号！\n\n此操作不可恢复，账号将永久失效。\n\n' + t('accountCard.deleteRemoteConfirm')
    )
    if (confirmed) {
      try {
        await invoke('delete_account_remote', { id: account.id, deleteLocal: true })
        loadAccounts()
      } catch (e) {
        // 错误已通过 showError 显示
      }
    }
  }, [showConfirm, loadAccounts, t])

  // 批量删除
  const onBatchDelete = useCallback(async () => {
    if (selectedIds.length === 0) return
    
    // 防呆：检查是否包含当前账号
    const currentAccount = accounts.find(a => localToken?.refreshToken && a.refreshToken === localToken.refreshToken)
    const includesCurrent = currentAccount && selectedIds.includes(currentAccount.id)
    
    if (includesCurrent) {
      const confirmed = await showConfirm(
        '⚠️ 批量删除包含当前账号',
        `您选择了 ${selectedIds.length} 个账号，其中包含当前使用的账号！\n\n删除后 Kiro IDE 将无法使用，需要重新登录。\n\n确定要删除吗？`
      )
      if (!confirmed) return
    } else {
      const confirmed = await showConfirm(t('accounts.batchDelete'), t('accounts.confirmDeleteMultiple', { count: selectedIds.length }))
      if (!confirmed) return
    }
    
    await invoke('delete_accounts', { ids: selectedIds })
    setSelectedIds([])
    loadAccounts()
  }, [accounts, selectedIds, localToken, showConfirm, loadAccounts, t])

  return (
    <div className={cn('h-full flex flex-col', colors.main)}>
      <div className="flex-1 flex flex-col min-h-0">
      <AccountHeader
        searchTerm={searchTerm}
        onSearchChange={handleSearchChange}
        selectedCount={selectedIds.length}
        onBatchDelete={onBatchDelete}
        onBatchTag={() => setShowBatchTagModal(true)}
        onImport={() => setShowImportModal(true)}
        onExport={async () => {
          if (selectedIds.length === 0) {
            showError(t('accounts.exportSelectFirst') || '请先选择要导出的账号')
            return
          }
          handleExport(selectedIds)
        }}
        onRefresh={loadAccounts}
        onRefreshAll={async () => {
          if (selectedIds.length === 0) {
            showError(t('accounts.refreshSelectFirst') || '请先选择要刷新的账号')
            return
          }
          batchRefreshAccounts(selectedIds, accounts)
        }}
        autoRefreshing={autoRefreshing}
        lastRefreshTime={lastRefreshTime}
        refreshProgress={refreshProgress}
        allGroups={groupDefinitions}
        selectedGroup={selectedGroup}
        onGroupFilter={handleGroupFilter}
        allTags={allTags}
        selectedTag={selectedTag}
        onTagFilter={handleTagFilter}
        selectedStatus={selectedStatus}
        onStatusFilter={handleStatusFilter}
        sortBy={sortBy}
        onSortChange={setSortBy}
        viewMode={viewMode}
        onViewModeChange={handleViewModeChange}
        advancedFilters={advancedFilters}
        onAdvancedFiltersChange={setAdvancedFilters}
        totalCount={filteredAccounts.length}
        onSelectAll={handleSelectAll}
      />
      <div className="flex-1 flex flex-col min-h-0">
      {loading ? (
        viewMode === 'card' ? <AccountListSkeleton count={8} /> : <AccountTableSkeleton count={8} />
      ) : filteredAccounts.length === 0 ? (
        <div className={`flex-1 flex items-center justify-center ${colors.main}`}>
          <div className="text-center max-w-md px-6">
            <div className={`w-24 h-24 mx-auto mb-6 rounded-2xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-lg ${accent.shadow} animate-float`}>
              <svg width="48" height="48" viewBox="0 0 40 40" fill="none">
                <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white"/>
              </svg>
            </div>
            <h3 className={`text-xl font-bold ${colors.text} mb-2`}>
              {searchTerm || selectedGroup || selectedTag || selectedStatus ? '没有找到匹配的账号' : '还没有账号'}
            </h3>
            <p className={`text-sm ${colors.textMuted} mb-6`}>
              {searchTerm || selectedGroup || selectedTag || selectedStatus
                ? '试试调整筛选条件或搜索关键词'
                : '导入账号开始管理你的 Kiro IDE 账户'}
            </p>
            {!searchTerm && !selectedGroup && !selectedTag && !selectedStatus && (
              <button
                onClick={() => setShowImportModal(true)}
                className={`px-6 py-3 rounded-xl text-sm font-medium text-white bg-gradient-to-r ${accent.gradientFrom} ${accent.gradientTo} shadow-lg ${accent.shadow} hover:shadow-xl transition-all duration-200 hover:scale-105 flex items-center gap-2 mx-auto`}
              >
                <Upload size={18} />
                导入账号
              </button>
            )}
          </div>
        </div>
      ) : viewMode === 'card' ? (
        <AccountTable
          accounts={filteredAccounts}
          totalCount={accounts.length}
          selectedIds={selectedIds}
          selectedIdsSet={selectedIdsSet}
          onSelectAll={handleSelectAll}
          onSelectOne={handleSelectOne}
          copiedId={copiedId}
          onCopy={handleCopy}
          onSwitch={handleSwitchAccount}
          onRefresh={handleRefreshWithNotify}
          onRefreshToken={handleRefreshToken}
          onEdit={setEditingAccount}
          onEditLabel={setEditingLabelAccount}
          onDelete={handleDelete}
          onDeleteRemote={handleDeleteRemote}
          onAdd={() => setShowImportModal(true)}
          refreshingId={refreshingId}
          refreshingTokenId={refreshingTokenId}
          switchingId={switchingId}
          localToken={localToken}
          tagDefinitions={tagDefinitions}
          groupDefinitions={groupDefinitions}
        />
      ) : (
        <AccountListView
          accounts={filteredAccounts}
          totalCount={accounts.length}
          selectedIds={selectedIds}
          selectedIdsSet={selectedIdsSet}
          onSelectAll={handleSelectAll}
          onSelectOne={handleSelectOne}
          onSwitch={handleSwitchAccount}
          onRefresh={handleRefreshWithNotify}
          onRefreshToken={handleRefreshToken}
          onEdit={setEditingAccount}
          onEditLabel={setEditingLabelAccount}
          onDelete={handleDelete}
          onDeleteRemote={handleDeleteRemote}
          onCopy={handleCopy}
          onAdd={() => setShowImportModal(true)}
          refreshingId={refreshingId}
          refreshingTokenId={refreshingTokenId}
          switchingId={switchingId}
          localToken={localToken}
          tagDefinitions={tagDefinitions}
          groupDefinitions={groupDefinitions}
          copiedId={copiedId}
          sortBy={sortBy}
          onSortChange={setSortBy}
        />
      )}
      </div>
      {editingAccount && (
        <AccountDetailModal
          account={editingAccount}
          onClose={() => { setEditingAccount(null); loadAccounts() }}
        />
      )}
      {editingLabelAccount && (<EditAccountModal account={editingLabelAccount} onClose={() => setEditingLabelAccount(null)} onSuccess={() => { loadAccounts(); loadTagDefinitions(); loadGroupDefinitions() }} />)}
      {showImportModal && (<ImportAccountModal onClose={() => setShowImportModal(false)} onSuccess={loadAccounts} onNavigate={onNavigate} />)}
      {showBatchTagModal && (<BatchTagModal accountIds={selectedIds} accounts={accounts} onClose={() => setShowBatchTagModal(false)} onSuccess={() => { loadAccounts(); loadTagDefinitions(); setSelectedIds([]) }} />)}
      {autoRefreshing && (<RefreshProgressModal refreshProgress={refreshProgress} />)}
      
      {/* 切换账号弹窗 */}
      {switchDialog && (
        <ConfirmModal
          type={switchDialog.type}
          title={switchDialog.title}
          message={switchDialog.message}
          onConfirm={switchDialog.type === 'confirm' ? confirmSwitch : closeSwitchDialog}
          onCancel={closeSwitchDialog}
          confirmText={switchDialog.type === 'confirm' ? t('switch.confirmBtn') : t('common.ok')}
        />
      )}
      </div>
    </div>
  )
}

export default AccountManager
