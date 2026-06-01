import { useState, useEffect, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { invoke } from '@tauri-apps/api/core'
import { Copy, Check, Folder, Plus, X, RefreshCw, Loader2, CheckCircle, Network } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { setAccountTags, setAccountGroup, getGroups, addGroup } from '../../../api/groupTag'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { TagSelector } from './GroupTagManager'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter} from '../../shared/dialog'
import { Button } from '../../shared/button'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { Account, AccountProxyConfig, GroupDefinition } from '../../../types/account'

const PRESET_COLORS = [
  '#3b82f6', '#10b981', '#f59e0b', '#ef4444', 
  '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'
]

interface GroupSelectorProps {
  groups: GroupDefinition[];
  value: string;
  onChange: (value: string) => void;
  onGroupsChange: (groups: GroupDefinition[]) => void;
}

function GroupSelector({ groups, value, onChange, onGroupsChange }: GroupSelectorProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const [newGroupName, setNewGroupName] = useState('')
  const [showInput, setShowInput] = useState(false)

  const handleAddGroup = async () => {
    const trimmed = newGroupName.trim().slice(0, 20)
    if (!trimmed) return
    if (groups.some(g => g.name === trimmed)) {
      setNewGroupName('')
      return
    }
    const color = PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)]
    try {
      const newGroup = await addGroup(trimmed, color) as GroupDefinition
      onGroupsChange([...groups, newGroup])
      onChange(newGroup.id)
      setNewGroupName('')
      setShowInput(false)
    } catch (e) {
      console.error('创建分组失败:', e)
    }
  }

  if (showInput) {
    return (
      <div className="flex gap-2">
        <input
          type="text"
          value={newGroupName}
          onChange={(e) => setNewGroupName(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleAddGroup()}
          placeholder={t('groups.newGroupPlaceholder') || '输入新分组名...'}
          className={`flex-1 px-4 py-2.5 border rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
        />
        <button
          onClick={handleAddGroup}
          disabled={!newGroupName.trim()}
          className={`p-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} disabled:opacity-50 cursor-pointer`}
        >
          <Check size={16} />
        </button>
        <button
          onClick={() => { setShowInput(false); setNewGroupName('') }}
          className={`p-2.5 rounded-xl hover:bg-muted/50 cursor-pointer`}
        >
          <X size={16} />
        </button>
      </div>
    )
  }

  return (
    <div className="flex gap-2">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`flex-1 px-4 py-2.5 border rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
      >
        <option value="">{t('groups.noGroup') || '无分组'}</option>
        {groups.map(g => (
          <option key={g.id} value={g.id}>{g.name}</option>
        ))}
      </select>
      <button
        onClick={() => setShowInput(true)}
        className={`p-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} cursor-pointer`}
      >
        <Plus size={16} />
      </button>
    </div>
  )
}

interface EditAccountModalProps {
  account: Account;
  onClose: () => void;
  onSuccess?: (account: Account) => void;
}

interface VerifyAccountResponse {
  usageData: any;
  accessToken: string;
  refreshToken: string;
}

interface AccountProxyTestResult {
  ok: boolean;
  status: number;
  response: string;
}

function decodeUrlPart(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function parseProxyQuickInput(value: string, currentProtocol: string) {
  const trimmed = value.trim()
  if (!trimmed) return null

  const hasProtocol = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed)
  const url = new URL(hasProtocol ? trimmed : `http://${trimmed}`)
  const protocol = url.protocol.replace(':', '').toLowerCase()
  if (hasProtocol && protocol !== 'http' && protocol !== 'socks5') {
    throw new Error('Unsupported proxy protocol')
  }
  if (!url.hostname || !url.port) {
    throw new Error('Proxy host and port are required')
  }

  const port = Number(url.port)
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error('Proxy port must be between 1 and 65535')
  }

  return {
    protocol: hasProtocol ? protocol : currentProtocol,
    host: url.hostname,
    port: url.port,
    username: decodeUrlPart(url.username),
    password: decodeUrlPart(url.password)
  }
}

function EditAccountModal({ account, onClose, onSuccess }: EditAccountModalProps) {
  const { t, theme } = useApp()
  const { showError } = useDialog()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const isIdCAccount = account.provider === 'BuilderId' || account.provider === 'Enterprise'

  const [form, setForm] = useState({
    label: account.label || '',
    accessToken: account.accessToken || '',
    refreshToken: account.refreshToken || '',
    clientId: account.clientId || '',
    clientSecret: account.clientSecret || '',
    machineId: account.machineId || ''})
  const [proxyForm, setProxyForm] = useState({
    enabled: Boolean(account.proxyConfig?.enabled),
    protocol: account.proxyConfig?.protocol || 'http',
    host: account.proxyConfig?.host || '',
    port: account.proxyConfig?.port ? String(account.proxyConfig.port) : '',
    username: account.proxyConfig?.username || '',
    password: account.proxyConfig?.password || ''})

  const [selectedTagIds, setSelectedTagIds] = useState((account.tagLinks || []).map(link => link.tagId))
  const [selectedGroupId, setSelectedGroupId] = useState(account.groupId || '')
  const [groups, setGroups] = useState<GroupDefinition[]>([])
  const [saving, setSaving] = useState(false)
  const [verifying, setVerifying] = useState(false)
  const [testingProxy, setTestingProxy] = useState(false)
  const [proxyTestStatus, setProxyTestStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [proxyQuickInput, setProxyQuickInput] = useState('')
  const [proxyQuickInputError, setProxyQuickInputError] = useState('')
  const [copiedField, setCopiedField] = useState<string | null>(null)
  
  // 账号信息状态（验证后更新）
  const [accountInfo, setAccountInfo] = useState<{
    email: string;
    subscriptionType: string;
    usage: { current: number; limit: number };
    daysRemaining?: number;
  } | null>(null)

  useEffect(() => {
    getGroups().then(setGroups).catch(() => {})
    
    // 初始化账号信息
    if (account.usageData) {
      const usageData = account.usageData
      const userInfo = usageData.userInfo || {}
      const subscriptionInfo = usageData.subscriptionInfo
      const breakdown = usageData.usageBreakdownList?.[0]
      const nextReset = usageData.nextDateReset
      
      // 计算剩余天数
      let daysRemaining: number | undefined
      if (nextReset) {
        const resetDate = new Date(typeof nextReset === 'string' ? nextReset : (nextReset < 1e12 ? nextReset * 1000 : nextReset))
        daysRemaining = Math.max(0, Math.ceil((resetDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24)))
      }
      
      setAccountInfo({
        email: account.email || userInfo.email || '',
        subscriptionType: subscriptionInfo?.subscriptionTitle || subscriptionInfo?.type || 'Free',
        usage: {
          current: breakdown?.currentUsage ?? 0,
          limit: breakdown?.usageLimit ?? 0
        },
        daysRemaining
      })
    }
  }, [account])

  const handleCopy = async (text: string, field: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedField(field)
      setTimeout(() => setCopiedField(null), 2000)
    } catch (e) {
      console.error('复制失败:', e)
    }
  }

  const handleVerifyAndRefresh = async () => {
    if (!form.refreshToken) {
      await showError(t('editAccount.verifyFailed'), '请填写 Refresh Token')
      return
    }
    if (isIdCAccount && (!form.clientId || !form.clientSecret)) {
      await showError(t('editAccount.verifyFailed'), '请填写 Client ID 和 Client Secret')
      return
    }

    setVerifying(true)
    try {
      const result = await invoke<VerifyAccountResponse>('verify_account', {
        params: {
          accessToken: form.accessToken,
          refreshToken: form.refreshToken,
          provider: account.provider,
          clientId: isIdCAccount ? form.clientId : null,
          clientSecret: isIdCAccount ? form.clientSecret : null,
          region: null
        }
      })

      // 更新表单中的 token
      setForm(prev => ({
        ...prev,
        accessToken: result.accessToken,
        refreshToken: result.refreshToken
      }))

      // 更新账号信息显示
      const usageData = result.usageData
      const userInfo = usageData.userInfo || {}
      const subscriptionInfo = usageData.subscriptionInfo
      const verifyBreakdown = usageData.usageBreakdownList?.[0]
      const verifyNextReset = usageData.nextDateReset
      
      let verifyDaysRemaining: number | undefined
      if (verifyNextReset) {
        const resetDate = new Date(typeof verifyNextReset === 'string' ? verifyNextReset : (verifyNextReset < 1e12 ? verifyNextReset * 1000 : verifyNextReset))
        verifyDaysRemaining = Math.max(0, Math.ceil((resetDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24)))
      }
      
      setAccountInfo({
        email: userInfo.email || '',
        subscriptionType: subscriptionInfo?.subscriptionTitle || subscriptionInfo?.type || 'Free',
        usage: {
          current: verifyBreakdown?.currentUsage ?? 0,
          limit: verifyBreakdown?.usageLimit ?? 0
        },
        daysRemaining: verifyDaysRemaining
      })
    } catch (e) {
      await showError(t('editAccount.verifyFailed'), String(e))
    } finally {
      setVerifying(false)
    }
  }

  const buildProxyConfig = (): AccountProxyConfig => {
    const username = proxyForm.username.trim()
    const password = proxyForm.password
    return {
      enabled: proxyForm.enabled,
      protocol: proxyForm.protocol as AccountProxyConfig['protocol'],
      host: proxyForm.host.trim(),
      port: Number(proxyForm.port) || 0,
      ...(username ? { username } : {}),
      ...(password ? { password } : {})
    }
  }

  const handleProxyQuickInputChange = (value: string) => {
    setProxyQuickInput(value)
    setProxyTestStatus(null)
    if (!value.trim()) {
      setProxyQuickInputError('')
      return
    }

    try {
      const parsed = parseProxyQuickInput(value, proxyForm.protocol)
      if (!parsed) return
      setProxyForm({
        enabled: true,
        protocol: parsed.protocol,
        host: parsed.host,
        port: parsed.port,
        username: parsed.username,
        password: parsed.password
      })
      setProxyQuickInputError('')
    } catch {
      setProxyQuickInputError(t('editAccount.proxyQuickInputInvalid'))
    }
  }

  const validateProxyForm = async () => {
    if (!proxyForm.enabled) return true
    if (!proxyForm.host.trim()) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyHostRequired'))
      return false
    }
    const port = Number(proxyForm.port)
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyPortRequired'))
      return false
    }
    if (proxyForm.password && !proxyForm.username.trim()) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyUsernameRequired'))
      return false
    }
    return true
  }

  const handleTestProxy = async () => {
    if (!(await validateProxyForm())) return
    setTestingProxy(true)
    setProxyTestStatus(null)
    try {
      const result = await invoke<AccountProxyTestResult>('test_account_proxy', {
        proxyConfig: buildProxyConfig()
      })
      setProxyTestStatus({
        type: 'success',
        message: `${t('editAccount.proxyTestSuccess')} (${result.status})`
      })
    } catch (e) {
      const message = String(e)
      setProxyTestStatus({ type: 'error', message })
      await showError(t('editAccount.proxyTestFailed'), message)
    } finally {
      setTestingProxy(false)
    }
  }

  const handleSave = async () => {
    if (!(await validateProxyForm())) return
    setSaving(true)
    try {
      const params: any = {
        id: account.id,
        label: form.label || null,
        accessToken: form.accessToken || null,
        refreshToken: form.refreshToken || null,
        machineId: form.machineId || null,
        proxyConfig: buildProxyConfig()}
      if (isIdCAccount) {
        params.clientId = form.clientId || null
        params.clientSecret = form.clientSecret || null
      }
      const updatedAccount = await invoke<Account>('update_account', { params })
      await setAccountGroup(account.id, selectedGroupId || null)
      await setAccountTags(account.id, selectedTagIds)
      onSuccess?.(updatedAccount)
      onClose()
    } catch (e) {
      await showError(t('editAccount.saveFailed'), String(e))
    } finally {
      setSaving(false)
    }
  }

  const dialogContent = (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />

        <div className="relative w-full max-w-4xl max-h-[90vh] overflow-hidden bg-background rounded-2xl shadow-2xl z-10 animate-in zoom-in-95 duration-200 flex flex-col">
          {/* Sticky Header */}
          <div className="sticky top-0 bg-background/95 backdrop-blur-sm z-20 border-b border-border">
            <DialogHeader icon={Folder} iconColor={accent.text} iconBg={accent.iconBadgeBg}>
              <DialogTitle>{t('editAccount.title')}</DialogTitle>
              <DialogDescription>{getAccountDisplayName(account)}</DialogDescription>
            </DialogHeader>
            <button
              onClick={onClose}
              className="absolute top-4 right-4 p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer"
              aria-label="Close"
            >
              <X size={20} className="text-muted-foreground" />
            </button>
          </div>

          {/* Scrollable Body */}
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* 当前账号状态 */}
          {accountInfo && (
            <div className={`p-4 rounded-xl border space-y-3 ${accent.subtleBg} border-primary/10`}>
              <div className="flex items-center justify-between border-b border-primary/10 pb-2">
                <span className="text-sm font-semibold text-foreground/80">当前账号状态</span>
                <div className="px-2.5 py-0.5 rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 text-xs font-medium flex items-center gap-1.5">
                  <CheckCircle size={14} />
                  已验证
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground text-xs block mb-1">邮箱</span>
                  <span className="font-medium font-mono text-xs truncate block" title={accountInfo.email}>
                    {accountInfo.email}
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground text-xs block mb-1">订阅计划</span>
                  <span className="font-medium">{accountInfo.subscriptionType}</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-xs block mb-1">使用额度</span>
                  <span className="font-medium">
                    {accountInfo.usage.current.toLocaleString()} / {accountInfo.usage.limit.toLocaleString()}
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground text-xs block mb-1">剩余天数</span>
                  <span className="font-medium">{accountInfo.daysRemaining ?? '-'} 天</span>
                </div>
              </div>
            </div>
          )}

          {/* 账号别名 */}
          <div>
            <label className={`block text-sm font-medium text-foreground mb-2`}>
              {t('accounts.remark')}
            </label>
            <input
              type="text"
              placeholder={t('editAccount.labelPlaceholder')}
              value={form.label}
              onChange={(e) => setForm({ ...form, label: e.target.value })}
              className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
            />
          </div>

          {/* Refresh Token */}
          <div>
            <label className={`block text-sm font-medium text-foreground mb-2`}>
              Refresh Token {isIdCAccount && <span className="text-destructive">*</span>}
            </label>
            <div className="relative">
              <textarea
                placeholder="aorAAAAA..."
                value={form.refreshToken}
                onChange={(e) => setForm({ ...form, refreshToken: e.target.value })}
                rows={3}
                className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 resize-none outline-none font-mono`}
              />
              <button
                onClick={() => handleCopy(form.refreshToken, 'refreshToken')}
                className={`absolute right-3 top-3 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                title={copiedField === 'refreshToken' ? '已复制' : '复制'}
              >
                {copiedField === 'refreshToken' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
              </button>
            </div>
          </div>

          {/* Machine ID */}
          <div>
            <label className={`block text-sm font-medium text-foreground mb-2`}>
              {t('addAccount.machineId')}
            </label>
            <div className="relative">
              <input
                type="text"
                placeholder={t('addAccount.machineIdPlaceholder')}
                value={form.machineId}
                onChange={(e) => setForm({ ...form, machineId: e.target.value })}
                className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
              />
              <button
                onClick={() => handleCopy(form.machineId, 'machineId')}
                className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                title={copiedField === 'machineId' ? '已复制' : '复制'}
              >
                {copiedField === 'machineId' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
              </button>
            </div>
          </div>

          {/* Account Proxy */}
          <div className="p-4 rounded-xl border border-border bg-muted/20 space-y-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <div className={`p-2 rounded-lg ${accent.iconBadgeBg}`}>
                  <Network size={16} className={accent.text} />
                </div>
                <div>
                  <div className="text-sm font-semibold text-foreground">{t('editAccount.proxyTitle')}</div>
                  <div className="text-xs text-muted-foreground mt-0.5">{t('editAccount.proxyDescription')}</div>
                </div>
              </div>
              <label className="inline-flex items-center gap-2 text-sm text-foreground cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={proxyForm.enabled}
                  onChange={(e) => {
                    setProxyForm({ ...proxyForm, enabled: e.target.checked })
                    setProxyTestStatus(null)
                  }}
                  className="h-4 w-4 rounded border-input accent-primary cursor-pointer"
                />
                {t('editAccount.proxyEnabled')}
              </label>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                  {t('editAccount.proxyQuickInput')}
                </label>
                <input
                  type="text"
                  placeholder={t('editAccount.proxyQuickInputPlaceholder')}
                  value={proxyQuickInput}
                  onChange={(e) => handleProxyQuickInputChange(e.target.value)}
                  className={`w-full px-4 py-2.5 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                />
                <div className={`mt-1 text-[11px] ${proxyQuickInputError ? 'text-destructive' : 'text-muted-foreground'}`}>
                  {proxyQuickInputError || t('editAccount.proxyQuickInputHint')}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                {(['http', 'socks5'] as const).map(protocol => (
                  <button
                    key={protocol}
                    type="button"
                    disabled={!proxyForm.enabled}
                    onClick={() => {
                      setProxyForm({ ...proxyForm, protocol })
                      setProxyTestStatus(null)
                    }}
                    className={`h-10 rounded-xl border text-sm font-medium transition-colors cursor-pointer disabled:cursor-not-allowed ${
                      proxyForm.protocol === protocol
                        ? `${accent.solidBg} ${accent.solidHoverBg} text-white border-transparent`
                        : 'border-input bg-background hover:bg-muted/50 text-foreground'
                    }`}
                  >
                    {protocol.toUpperCase()}
                  </button>
                ))}
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-[1fr_120px] gap-3">
                <div>
                  <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                    {t('editAccount.proxyHost')}
                  </label>
                  <input
                    type="text"
                    disabled={!proxyForm.enabled}
                    placeholder="127.0.0.1"
                    value={proxyForm.host}
                    onChange={(e) => {
                      setProxyForm({ ...proxyForm, host: e.target.value })
                      setProxyTestStatus(null)
                    }}
                    className={`w-full px-4 py-2.5 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none disabled:cursor-not-allowed`}
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                    {t('editAccount.proxyPort')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={65535}
                    disabled={!proxyForm.enabled}
                    placeholder="7890"
                    value={proxyForm.port}
                    onChange={(e) => {
                      setProxyForm({ ...proxyForm, port: e.target.value })
                      setProxyTestStatus(null)
                    }}
                    className={`w-full px-4 py-2.5 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none disabled:cursor-not-allowed`}
                  />
                </div>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                    {t('editAccount.proxyUsername')}
                  </label>
                  <input
                    type="text"
                    disabled={!proxyForm.enabled}
                    placeholder={t('editAccount.proxyOptional')}
                    value={proxyForm.username}
                    onChange={(e) => {
                      setProxyForm({ ...proxyForm, username: e.target.value })
                      setProxyTestStatus(null)
                    }}
                    className={`w-full px-4 py-2.5 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none disabled:cursor-not-allowed`}
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                    {t('editAccount.proxyPassword')}
                  </label>
                  <input
                    type="password"
                    disabled={!proxyForm.enabled}
                    placeholder={t('editAccount.proxyOptional')}
                    value={proxyForm.password}
                    onChange={(e) => {
                      setProxyForm({ ...proxyForm, password: e.target.value })
                      setProxyTestStatus(null)
                    }}
                    className={`w-full px-4 py-2.5 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none disabled:cursor-not-allowed`}
                  />
                </div>
              </div>

              <div className="flex flex-col sm:flex-row sm:items-center gap-3">
                <Button
                  variant="secondary"
                  className="h-10 rounded-xl font-medium"
                  onClick={handleTestProxy}
                  disabled={!proxyForm.enabled || testingProxy}
                >
                  {testingProxy ? (
                    <>
                      <Loader2 size={16} className="mr-2 animate-spin" />
                      {t('editAccount.proxyTesting')}
                    </>
                  ) : (
                    <>
                      <RefreshCw size={16} className="mr-2" />
                      {t('editAccount.proxyTest')}
                    </>
                  )}
                </Button>
                {proxyTestStatus && (
                  <span
                    className={`text-xs font-medium ${
                      proxyTestStatus.type === 'success' ? 'text-green-600 dark:text-green-400' : 'text-destructive'
                    }`}
                  >
                    {proxyTestStatus.message}
                  </span>
                )}
              </div>
            </div>
          </div>

          {isIdCAccount && (
            <>
              <div>
                <label className={`block text-sm font-medium text-foreground mb-2`}>
                  Client ID <span className="text-destructive">*</span>
                </label>
                <div className="relative">
                  <input
                    type="text"
                    placeholder="刷新 Token 需要"
                    value={form.clientId}
                    onChange={(e) => setForm({ ...form, clientId: e.target.value })}
                    className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none font-mono`}
                  />
                  <button
                    onClick={() => handleCopy(form.clientId, 'clientId')}
                    className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                    title={copiedField === 'clientId' ? '已复制' : '复制'}
                  >
                    {copiedField === 'clientId' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
                  </button>
                </div>
              </div>
              <div>
                <label className={`block text-sm font-medium text-foreground mb-2`}>
                  Client Secret <span className="text-destructive">*</span>
                </label>
                <div className="relative">
                  <textarea
                    placeholder="刷新 Token 需要"
                    value={form.clientSecret}
                    onChange={(e) => setForm({ ...form, clientSecret: e.target.value })}
                    rows={2}
                    className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 resize-none outline-none font-mono`}
                  />
                  <button
                    onClick={() => handleCopy(form.clientSecret, 'clientSecret')}
                    className={`absolute right-3 top-3 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                    title={copiedField === 'clientSecret' ? '已复制' : '复制'}
                  >
                    {copiedField === 'clientSecret' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
                  </button>
                </div>
              </div>
            </>
          )}

          {/* 验证并刷新按钮 */}
          <Button
            variant="secondary"
            className="w-full h-10 rounded-xl font-medium"
            onClick={handleVerifyAndRefresh}
            disabled={verifying || !form.refreshToken || (isIdCAccount && (!form.clientId || !form.clientSecret))}
          >
            {verifying ? (
              <>
                <Loader2 size={16} className="mr-2 animate-spin" />
                验证中...
              </>
            ) : (
              <>
                <RefreshCw size={16} className="mr-2" />
                验证并刷新凭证信息
              </>
            )}
          </Button>

          {/* 分组 */}
          <div>
            <div className={`text-sm font-medium mb-2 flex items-center gap-1.5 text-foreground`}>
              <Folder size={14} />
              {t('groups.title') || '分组'}
            </div>
            <GroupSelector
              groups={groups}
              value={selectedGroupId}
              onChange={setSelectedGroupId}
              onGroupsChange={setGroups}
            />
          </div>

          {/* 标签 */}
          <div>
            <TagSelector 
              selectedTagIds={selectedTagIds} 
              onChange={setSelectedTagIds} 
            />
          </div>
        </div>

        {/* Sticky Footer */}
        <div className="sticky bottom-0 bg-background/95 backdrop-blur-sm p-4 border-t border-border flex justify-end gap-3 z-20">
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="success"
            onClick={handleSave}
            disabled={saving}
            loading={saving}
          >
            {t('common.save')}
          </Button>
        </div>
      </div>
    </div>
  </DialogRoot>
  )

  return createPortal(dialogContent, document.body)
}

export default EditAccountModal
