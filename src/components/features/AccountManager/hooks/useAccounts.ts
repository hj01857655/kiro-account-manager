import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, emit, UnlistenFn } from '@tauri-apps/api/event'
import { isUnavailableStatus } from '../../../../utils/accountStatus'
import { normalizeAccountForUi, getSafeAccountDisplayName } from '../utils/accountRuntime'
import { Account } from '../../../../types/account'

export interface RefreshResult {
    email: string;
    success: boolean;
    message: string;
}

export interface RefreshProgress {
    current: number;
    total: number;
    currentEmail: string;
    results: RefreshResult[];
}

export function useAccounts() {
  const [accounts, setAccounts] = useState<Account[]>([])
  const [loading, setLoading] = useState(true)
  const [autoRefreshing, setAutoRefreshing] = useState(false)
  const [refreshProgress, setRefreshProgress] = useState<RefreshProgress>({ 
    current: 0, 
    total: 0, 
    currentEmail: '', 
    results: [] 
  })
  const [lastRefreshTime, setLastRefreshTime] = useState<string | null>(null)
  const [refreshingId, setRefreshingId] = useState<string | null>(null)
  const refreshTimerRef = useRef<NodeJS.Timeout | null>(null)

  // 判断账号是否即将过期（5分钟内）
  const isExpiringSoon = useCallback((account: Account) => {
    if (isUnavailableStatus(account)) return false
    if (!account.expiresAt) return false
    try {
      const expiresAt = new Date(account.expiresAt.replace(/\//g, '-'))
      if (isNaN(expiresAt.getTime())) return false
      return expiresAt.getTime() - Date.now() < 5 * 60 * 1000
    } catch {
      return false
    }
  }, [])

  const loadAccounts = useCallback(async () => {
    try {
      setLoading(true)
      const loadedAccounts = await invoke<any[]>('get_accounts')
      const normalizedAccounts = Array.isArray(loadedAccounts)
        ? loadedAccounts.map(normalizeAccountForUi)
        : []
      setAccounts(normalizedAccounts)
    } catch (e) {
      // 错误处理
    } finally {
      setLoading(false)
    }
  }, [])

  // 批量刷新账号
  const batchRefreshAccounts = useCallback(async (accountIds: string[], accountList: Account[]) => {
    if (autoRefreshing || accountList.length === 0) return
    
    const validAccounts = accountList.filter(acc => !isUnavailableStatus(acc))
    const accountsToRefresh = accountIds.length > 0
      ? validAccounts.filter(acc => accountIds.includes(acc.id))
      : validAccounts.filter(isExpiringSoon)
    
    if (accountsToRefresh.length === 0) return

    const count = accountsToRefresh.length
    const concurrency = Math.min(20, Math.max(3, Math.ceil(count / 10)))

    setAutoRefreshing(true)
    setRefreshProgress({ current: 0, total: accountsToRefresh.length, currentEmail: '', results: [] })

    const updatedAccounts = [...accountList]
    const results: RefreshResult[] = []
    let completed = 0

    const refreshOne = async (account: Account) => {
      let success = false, message = ''
      try {
        const syncResult = await invoke<{ account: any }>('sync_account', { id: account.id })
        const updated = normalizeAccountForUi(syncResult.account)
        const idx = updatedAccounts.findIndex(a => a.id === account.id)
        if (idx !== -1) updatedAccounts[idx] = updated
        success = true
        message = '同步成功'
      } catch (e) {
        const errorMsg = String(e)
        const idx = updatedAccounts.findIndex(a => a.id === account.id)
        if (errorMsg.includes('BANNED')) {
          message = '账号已封禁'
          if (idx !== -1) updatedAccounts[idx] = { ...updatedAccounts[idx], status: 'banned' }
        } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
          message = '账号已失效'
          if (idx !== -1) updatedAccounts[idx] = { ...updatedAccounts[idx], status: 'invalid' }
        } else {
          message = errorMsg.slice(0, 30)
        }
      }
      completed++
      results.push({ email: getSafeAccountDisplayName(account), success, message })
      setRefreshProgress({ current: completed, total: accountsToRefresh.length, currentEmail: '', results: [...results] })
      return { account, success, message }
    }

    for (let i = 0; i < accountsToRefresh.length; i += concurrency) {
      const batch = accountsToRefresh.slice(i, i + concurrency)
      setRefreshProgress(prev => ({
        ...prev,
        currentEmail: batch.map(a => getSafeAccountDisplayName(a).split('@')[0]).join(', ')
      }))
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

  const handleRefreshStatus = useCallback(async (id: string) => {
    setRefreshingId(id)
    try {
      const syncResult = await invoke<{ account: any }>('sync_account', { id })
      const updated = normalizeAccountForUi(syncResult.account)
      setAccounts(prev => prev.map(a => a.id === id ? updated : a))
      return { success: true, data: updated }
    } catch (e) {
      const errorMsg = String(e)
      if (errorMsg.includes('BANNED')) {
        try {
          await invoke('update_account', {
            params: {
              id,
              label: null,
              status: 'banned',
              accessToken: null,
              refreshToken: null,
              clientId: null,
              clientSecret: null,
              machineId: null,
            }
          })
          setAccounts(prev => prev.map(a => a.id === id ? { ...a, status: 'banned' } : a))
        } catch (updateErr) {}
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid')) {
        try {
          await invoke('update_account', {
            params: {
              id,
              label: null,
              status: 'invalid',
              accessToken: null,
              refreshToken: null,
              clientId: null,
              clientSecret: null,
              machineId: null,
            }
          })
          setAccounts(prev => prev.map(a => a.id === id ? { ...a, status: 'invalid' } : a))
        } catch (updateErr) {}
      }
      return { success: false, error: errorMsg }
    } finally {
      setRefreshingId(null)
    }
  }, [])

  const handleExport = useCallback(async (selectedIds: string[] = []) => {
    try {
      if (selectedIds.length === 0) return
      
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
      
      if (!filePath) return
      
      const json = await invoke<string>('export_accounts', { ids: selectedIds })
      await writeTextFile(filePath, json)
    } catch (e) {}
  }, [])

  useEffect(() => {
    let unlistenLoginSuccess: UnlistenFn | null = null
    let unlistenAccountsUpdated: UnlistenFn | null = null
    let unlistenKiroLoginData: UnlistenFn | null = null
    let mounted = true

    const setupListeners = async () => {
      unlistenLoginSuccess = await listen('login-success', () => {
        if (mounted) loadAccounts()
      })
      unlistenAccountsUpdated = await listen('accounts-updated', () => {
        if (mounted) loadAccounts()
      })
      unlistenKiroLoginData = await listen<any>('kiro-login-data', async (event) => {
        if (!mounted) return
        try {
          const data = typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload
          if (data?.refreshToken) {
            await invoke('add_account_by_social', {
              refreshToken: data.refreshToken,
              provider: data.idp || data.provider || null
            })
            if (mounted) loadAccounts()
          }
        } catch (e) {}
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

  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        clearTimeout(refreshTimerRef.current)
      }
    }
  }, [])

  return {
    accounts,
    setAccounts,
    loading,
    loadAccounts,
    autoRefreshing,
    refreshProgress,
    lastRefreshTime,
    refreshingId,
    batchRefreshAccounts,
    handleRefreshStatus,
    handleExport}
}
