import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, emit } from '@tauri-apps/api/event'
import { isBannedStatus } from '../../../../utils/accountStatus'

export function useAccounts() {
  const [accounts, setAccounts] = useState([])
  const [loading, setLoading] = useState(true)
  const [autoRefreshing, setAutoRefreshing] = useState(false)
  const [refreshProgress, setRefreshProgress] = useState({ current: 0, total: 0, currentEmail: '', results: [] })
  const [lastRefreshTime, setLastRefreshTime] = useState(null)
  const [refreshingId, setRefreshingId] = useState(null)
  const refreshTimerRef = useRef(null)

  // 判断账号是否即将过期（5分钟内）
  const isExpiringSoon = useCallback((account) => {
    // 跳过已封禁账号
    if (isBannedStatus(account.status)) return false
    // 没有过期时间的不刷新
    if (!account.expiresAt) return false
    try {
      const expiresAt = new Date(account.expiresAt.replace(/\//g, '-'))
      // 检查日期是否有效
      if (isNaN(expiresAt.getTime())) return false
      return expiresAt.getTime() - Date.now() < 5 * 60 * 1000
    } catch {
      return false
    }
  }, [])

  const loadAccounts = useCallback(async () => {
    try {
      setLoading(true)
      const loadedAccounts = await invoke('get_accounts')
      setAccounts(loadedAccounts)
    } catch (e) {
      // 静默处理
    } finally {
      setLoading(false)
    }
  }, [])

  // 批量刷新账号
  // accountIds: 指定要刷新的账号ID列表，为空则刷新所有即将过期的账号
  // accountList: 所有账号列表
  const batchRefreshAccounts = useCallback(async (accountIds, accountList) => {
    if (autoRefreshing || accountList.length === 0) return
    
    const validAccounts = accountList.filter(acc => !isBannedStatus(acc.status))
    // 如果指定了账号ID，强制刷新这些账号；否则只刷新即将过期的
    const accountsToRefresh = accountIds.length > 0
      ? validAccounts.filter(acc => accountIds.includes(acc.id))
      : validAccounts.filter(isExpiringSoon)
    
    if (accountsToRefresh.length === 0) return

    // 动态计算并发数：数量/10，最小3，最大20
    const count = accountsToRefresh.length
    const concurrency = Math.min(20, Math.max(3, Math.ceil(count / 10)))

    setAutoRefreshing(true)
    setRefreshProgress({ current: 0, total: accountsToRefresh.length, currentEmail: '', results: [] })

    const updatedAccounts = [...accountList]
    const results = []
    let completed = 0

    // 单个账号刷新任务
    const refreshOne = async (account) => {
      let success = false, message = ''
      try {
        const syncResult = await invoke('sync_account', { id: account.id })
        const updated = syncResult.account
        const idx = updatedAccounts.findIndex(a => a.id === account.id)
        if (idx !== -1) updatedAccounts[idx] = updated
        success = true
        message = '同步成功'
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          message = '账号已封禁'
          // 更新账号状态为封禁
          const idx = updatedAccounts.findIndex(a => a.id === account.id)
          if (idx !== -1) updatedAccounts[idx] = { ...updatedAccounts[idx], status: 'banned' }
        } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
          message = 'Token已失效'
          const idx = updatedAccounts.findIndex(a => a.id === account.id)
          if (idx !== -1) updatedAccounts[idx] = { ...updatedAccounts[idx], status: 'Token已失效' }
        } else {
          message = errorMsg.slice(0, 30)
        }
      }
      completed++
      results.push({ email: account.email, success, message })
      setRefreshProgress({ current: completed, total: accountsToRefresh.length, currentEmail: '', results: [...results] })
      return { account, success, message }
    }

    // 并发控制：分批执行
    for (let i = 0; i < accountsToRefresh.length; i += concurrency) {
      const batch = accountsToRefresh.slice(i, i + concurrency)
      setRefreshProgress(prev => ({ ...prev, currentEmail: batch.map(a => a.email.split('@')[0]).join(', ') }))
      await Promise.all(batch.map(refreshOne))
    }

    setAccounts(updatedAccounts)
    setLastRefreshTime(new Date().toLocaleTimeString())
    emit('accounts-updated')
    if (refreshTimerRef.current) {
      clearTimeout(refreshTimerRef.current)
    }
    refreshTimerRef.current = setTimeout(() => {
      setAutoRefreshing(false)
      setRefreshProgress({ current: 0, total: 0, currentEmail: '', results: [] })
    }, 1500)
  }, [autoRefreshing, isExpiringSoon])


  const handleRefreshStatus = useCallback(async (id) => {
    setRefreshingId(id)
    try {
      const syncResult = await invoke('sync_account', { id })
      const updated = syncResult.account
      setAccounts(prev => prev.map(a => a.id === id ? updated : a))
      return { success: true, data: updated }
    } catch (e) {
      const errorMsg = String(e)
      // 只有封禁时才更新状态
      if (errorMsg.includes('BANNED')) {
        try {
          await invoke('update_account', { id, status: 'banned' })
          setAccounts(prev => prev.map(a => a.id === id ? { ...a, status: 'banned' } : a))
        } catch (updateErr) {
          // 静默处理
        }
      }
      // 其他错误只返回错误信息，不更新状态
      return { success: false, error: errorMsg }
    } finally {
      setRefreshingId(null)
    }
  }, [])

  const handleExport = useCallback(async (selectedIds = []) => {
    try {
      // 必须选中账号才能导出
      if (selectedIds.length === 0) {
        return
      }
      
      const { save } = await import('@tauri-apps/plugin-dialog')
      const { writeTextFile } = await import('@tauri-apps/plugin-fs')
      const { downloadDir } = await import('@tauri-apps/api/path')
      
      const defaultName = `kiro-accounts-${selectedIds.length}-${new Date().toISOString().slice(0, 10)}.json`
      const defaultDir = await downloadDir()
      const sep = defaultDir.includes('\\') ? '\\' : '/'
      
      const filePath = await save({
        defaultPath: `${defaultDir}${sep}${defaultName}`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
        title: '导出账号数据'
      })
      
      if (!filePath) return // 用户取消
      
      const json = await invoke('export_accounts', { ids: selectedIds })
      await writeTextFile(filePath, json)
    } catch (e) {
      // 错误已通过 showError 显示
    }
  }, [])

  // 注意：handleDelete, handleBatchDelete, handleSwitchAccount 已移动到 AccountManager/index.jsx 中
  // switchingId 已移动到 useSwitchAccount.js hook 中

  // 初始化和事件监听
  // 注意：自动刷新定时器已移至 App.jsx 统一管理，避免重复刷新
  useEffect(() => {
    let unlistenLoginSuccess = null
    let unlistenAccountsUpdated = null
    let unlistenKiroLoginData = null
    let mounted = true

    const setupListeners = async () => {
      unlistenLoginSuccess = await listen('login-success', () => {
        if (mounted) loadAccounts()
      })
      // 监听账号数据更新事件（来自 App.jsx 自动刷新或其他地方）
      unlistenAccountsUpdated = await listen('accounts-updated', () => {
        if (mounted) loadAccounts()
      })
      // 监听 Kiro 登录数据（通过 refresh_token 添加账号）
      unlistenKiroLoginData = await listen('kiro-login-data', async (event) => {
        if (!mounted) return
        try {
          const data = typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload
          if (data?.refreshToken) {
            // 使用正确的命令：add_account_by_social
            await invoke('add_account_by_social', {
              refreshToken: data.refreshToken,
              provider: data.idp || data.provider || null
            })
            if (mounted) loadAccounts()
          }
        } catch (e) {
          console.error('Failed to handle kiro-login-data:', e)
        }
      })
    }

    loadAccounts()
    setupListeners()

    return () => {
      mounted = false
      if (unlistenLoginSuccess) unlistenLoginSuccess()
      if (unlistenAccountsUpdated) unlistenAccountsUpdated()
      if (unlistenKiroLoginData) unlistenKiroLoginData()
    }
  }, [loadAccounts])

  // 清理刷新timer
  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        clearTimeout(refreshTimerRef.current)
      }
    }
  }, [])

  return {
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
  }
}
