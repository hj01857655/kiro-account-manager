import { useState, useEffect } from 'react'
import { Tabs, Textarea, Stack, Group, Alert, Progress, FileButton, Button as MantineButton } from '@mantine/core'
import { Upload, FileJson, AlertCircle, CheckCircle, Loader2, Database, RefreshCw, LogIn } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { getConcurrency } from '../../../utils/concurrency'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { isBannedStatus } from '../../../utils/accountStatus'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter,
} from '../../ui/dialog'
import { Button } from '../../ui/button'

function validateAccount(item, index) {
  const errors = []
  const refreshToken = item.refreshToken
  if (!refreshToken) {
    errors.push(`第 ${index + 1} 条: 缺少 refreshToken`)
    return { valid: false, errors, type: null }
  }

  if (!refreshToken.startsWith('aor')) {
    errors.push(`第 ${index + 1} 条: refreshToken 格式无效（应以 aor 开头）`)
    return { valid: false, errors, type: null }
  }

  const hasClientCredentials = item.clientId && item.clientSecret
  const isIdC = hasClientCredentials
  const isSocial = !hasClientCredentials

  let provider = item.provider
  if (!provider) {
    if (isSocial) {
      // Social 账号必须明确指定 provider
      errors.push(`第 ${index + 1} 条: Social 账号必须指定 provider (Google/Github)`)
      return { valid: false, errors, type: null }
    } else {
      // IdC 账号：通过 startUrl 判断是 Enterprise 还是 BuilderId
      provider = item.startUrl ? 'Enterprise' : 'BuilderId'
    }
  }

  const validProviders = ['Google', 'Github', 'BuilderId', 'Enterprise']
  if (!validProviders.includes(provider)) {
    errors.push(`第 ${index + 1} 条: provider 必须是 ${validProviders.join('/')}`)
    return { valid: false, errors, type: null }
  }

  if (isSocial && !['Google', 'Github'].includes(provider)) {
    errors.push(`第 ${index + 1} 条: Social 账号的 provider 应为 Google/Github`)
    return { valid: false, errors, type: null }
  }

  if (isIdC && !['BuilderId', 'Enterprise'].includes(provider)) {
    errors.push(`第 ${index + 1} 条: IdC 账号的 provider 应为 BuilderId/Enterprise`)
    return { valid: false, errors, type: null }
  }

  // Enterprise 账号不需要额外校验（region 可选，默认 us-east-1）

  return { valid: true, errors: [], type: isSocial ? 'social' : 'idc', inferredProvider: provider }
}

function ImportAccountModal({ onClose, onSuccess, onNavigate }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const [activeTab, setActiveTab] = useState('json')
  const [osType, setOsType] = useState('')
  const [jsonText, setJsonText] = useState('')
  const [parseResult, setParseResult] = useState(null)
  const [importing, setImporting] = useState(false)
  const [importProgress, setImportProgress] = useState({ current: 0, total: 0 })
  const [importResult, setImportResult] = useState(null)

  // 从 Kiro 导入相关状态
  const [kiroAccounts, setKiroAccounts] = useState([])
  const [kiroLoading, setKiroLoading] = useState(false)
  const [kiroError, setKiroError] = useState(null)
  const [kiroImporting, setKiroImporting] = useState(false)
  const [kiroProgress, setKiroProgress] = useState({ current: 0, total: 0 })
  const [kiroResult, setKiroResult] = useState(null)

  // 从 kiro-cli 导入相关状态
  const [kiroCliDbPath, setKiroCliDbPath] = useState('')
  const [kiroCliDetected, setKiroCliDetected] = useState(false)
  const [kiroCliDetecting, setKiroCliDetecting] = useState(false)
  const [kiroCliImporting, setKiroCliImporting] = useState(false)
  const [kiroCliResult, setKiroCliResult] = useState(null)
  const isWindowsOs = osType === 'windows'

  useEffect(() => {
    let isMounted = true

    const detectOsType = async () => {
      try {
        const info = await invoke('get_system_machine_guid')
        if (isMounted && info?.osType) {
          setOsType(info.osType)
          return
        }
      } catch (_) {
        // ignore and fallback to userAgent detection
      }

      if (!isMounted) return
      const userAgent = (navigator.userAgent || '').toLowerCase()
      if (userAgent.includes('windows')) {
        setOsType('windows')
      } else if (userAgent.includes('mac os') || userAgent.includes('macos')) {
        setOsType('macos')
      } else if (userAgent.includes('linux')) {
        setOsType('linux')
      }
    }

    detectOsType()

    return () => {
      isMounted = false
    }
  }, [])

  // 自动检测 kiro-cli 数据库路径
  useEffect(() => {
    if (activeTab === 'kiro-cli' && !kiroCliDbPath) {
      detectKiroCliPath()
    }
  }, [activeTab, kiroCliDbPath])

  const detectKiroCliPath = async () => {
    setKiroCliDetecting(true)
    try {
      const defaultPath = await invoke('get_kiro_cli_default_path')
      if (defaultPath) {
        setKiroCliDbPath(defaultPath)
        setKiroCliDetected(true)
      } else {
        setKiroCliDetected(false)
      }
    } catch (e) {
      console.error('获取默认路径失败:', e)
      setKiroCliDetected(false)
    } finally {
      setKiroCliDetecting(false)
    }
  }

  // 自动检测 Kiro 账号
  useEffect(() => {
    if (activeTab === 'kiro') {
      detectKiroAccounts()
    }
  }, [activeTab])

  const detectKiroAccounts = async () => {
    setKiroLoading(true)
    setKiroError(null)
    try {
      const accounts = await invoke('read_kiro_accounts')
      setKiroAccounts(accounts)
    } catch (e) {
      setKiroError(String(e))
      setKiroAccounts([])
    } finally {
      setKiroLoading(false)
    }
  }

  const parseJson = (text) => {
    if (!text.trim()) {
      setParseResult(null)
      return
    }

    try {
      let data = JSON.parse(text)
      if (!Array.isArray(data)) data = [data]

      const valid = []
      const invalid = []
      const errors = []

      data.forEach((item, index) => {
        const result = validateAccount(item, index)
        if (result.valid) {
          valid.push({ ...item, _type: result.type, _index: index, _inferredProvider: result.inferredProvider })
        } else {
          invalid.push({ ...item, _index: index })
          errors.push(...result.errors)
        }
      })

      setParseResult({ valid, invalid, errors })
    } catch (e) {
      setParseResult({ valid: [], invalid: [], errors: [`JSON 解析失败: ${e.message}`] })
    }
  }

  const handleFileSelect = async (file) => {
    if (!file) return
    const text = await file.text()
    setJsonText(text)
    parseJson(text)
  }

  const runConcurrent = async (items, handler, onProgress) => {
    const results = []
    let completed = 0
    const concurrency = getConcurrency(items.length)

    for (let i = 0; i < items.length; i += concurrency) {
      const batch = items.slice(i, i + concurrency)
      const batchResults = await Promise.all(
        batch.map(async (item) => {
          const result = await handler(item)
          completed++
          onProgress(completed)
          return result
        })
      )
      results.push(...batchResults)
    }
    return results
  }

  const handleJsonImport = async () => {
    if (!parseResult?.valid.length) return

    setImporting(true)
    setImportProgress({ current: 0, total: parseResult.valid.length })

    const added = []
    const updated = []
    const failed = []

    const importOne = async (item) => {
      try {
        let result
        const provider = item._inferredProvider || item.provider
        if (item._type === 'social') {
          result = await invoke('add_account_by_social', {
            refreshToken: item.refreshToken,
            provider: provider,
            machineId: item.machineId || null,
            accessToken: item.accessToken || null
          })
        } else {
          // IdC 账号：统一调用 add_account_by_idc
          const params = {
            provider,  // BuilderId 或 Enterprise
            refreshToken: item.refreshToken,
            clientId: item.clientId,
            clientSecret: item.clientSecret,
            region: item.region || null,
            machineId: item.machineId || null,
            accessToken: item.accessToken || null,
            password: item.password || null,
            startUrl: item.startUrl || null,  // Enterprise 可能需要
            clientIdHash: item.clientIdHash || null  // Enterprise 可用 clientIdHash 替代 startUrl
          }

          result = await invoke('add_account_by_idc', params)
        }

        const account = result.account
        if (isBannedStatus(account.status)) {
          return { success: true, index: item._index + 1, email: getAccountDisplayName(account), isNew: result.isNew, banned: true }
        }
        return { success: true, index: item._index + 1, email: getAccountDisplayName(account), isNew: result.isNew }
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          return { success: false, index: item._index + 1, error: '账号已封禁', banned: true }
        }
        return { success: false, index: item._index + 1, error: errorMsg.slice(0, 50) }
      }
    }

    const results = await runConcurrent(
      parseResult.valid,
      importOne,
      (completed) => setImportProgress({ current: completed, total: parseResult.valid.length })
    )

    results.forEach(r => {
      if (r.success) {
        if (r.isNew) {
          added.push({ index: r.index, email: r.email })
        } else {
          updated.push({ index: r.index, email: r.email })
        }
      } else {
        failed.push({ index: r.index, error: r.error })
      }
    })

    setImportResult({ added, updated, failed })
    setImporting(false)
    if (added.length > 0 || updated.length > 0) onSuccess?.()
  }

  const handleKiroImport = async () => {
    if (kiroAccounts.length === 0) return

    setKiroImporting(true)
    setKiroProgress({ current: 0, total: kiroAccounts.length })

    const added = []
    const updated = []
    const failed = []

    const importOne = async (account) => {
      try {
        let result
        if (account.authMethod === 'IdC') {
          // IdC 账号：统一调用 add_account_by_idc
          const params = {
            provider: account.provider,  // BuilderId 或 Enterprise
            refreshToken: account.refreshToken,
            clientId: account.clientId,
            clientSecret: account.clientSecret,
            region: account.region || null,
            machineId: null,
            accessToken: account.accessToken || null,
            password: null,
            startUrl: null,  // 从 Kiro 导入时不需要 startUrl（使用 clientIdHash）
            clientIdHash: account.clientIdHash || null  // 使用 Kiro 提供的 clientIdHash
          }

          result = await invoke('add_account_by_idc', params)
        } else {
          result = await invoke('add_account_by_social', {
            refreshToken: account.refreshToken,
            provider: account.provider,
            machineId: null,
            accessToken: account.accessToken || null
          })
        }

        const acc = result.account
        if (isBannedStatus(acc.status)) {
          return { success: true, email: acc.email, isNew: result.isNew, banned: true }
        }
        return { success: true, email: acc.email, isNew: result.isNew }
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          return { success: false, error: '账号已封禁', banned: true }
        }
        return { success: false, error: errorMsg.slice(0, 80) }
      }
    }

    const results = await runConcurrent(
      kiroAccounts,
      importOne,
      (completed) => setKiroProgress({ current: completed, total: kiroAccounts.length })
    )

    results.forEach(r => {
      if (r.success) {
        if (r.isNew) {
          added.push({ email: r.email })
        } else {
          updated.push({ email: r.email })
        }
      } else {
        failed.push({ error: r.error })
      }
    })

    setKiroResult({ added, updated, failed })
    setKiroImporting(false)

    if (added.length > 0 || updated.length > 0) {
      onSuccess?.()
    }
  }

  const handleKiroCliImport = async () => {
    if (!kiroCliDbPath) return

    setKiroCliImporting(true)
    setKiroCliResult(null)

    try {
      const result = await invoke('import_from_kiro_cli', {
        dbPath: kiroCliDbPath
      })

      if (result.success) {
        setKiroCliResult({
          success: true,
          isNew: result.is_new,
          email: result.account?.email || result.account?.user_id || '未知账号'
        })

        onSuccess?.()
      } else {
        setKiroCliResult({
          success: false,
          error: result.error || '导入失败'
        })
      }
    } catch (e) {
      setKiroCliResult({
        success: false,
        error: String(e)
      })
    } finally {
      setKiroCliImporting(false)
    }
  }

  const renderResult = (result) => (
  <Stack gap="md" p="sm">
    {result.added && result.added.length > 0 && (
      <Alert icon={<CheckCircle size={20} />} color="teal" variant="light">
        <div className={`font-medium ${colors.text}`}>✅ 新增 {result.added.length} 个账号</div>
        {result.added.length > 0 && (
          <div className={`text-sm mt-2 ${colors.text}`}>{result.added.map(s => s.email).join(', ')}</div>
        )}
      </Alert>
    )}

    {result.updated && result.updated.length > 0 && (
      <Alert icon={<CheckCircle size={20} />} color="blue" variant="light">
        <div className={`font-medium ${colors.text}`}>📝 更新 {result.updated.length} 个账号</div>
        {result.updated.length > 0 && (
          <div className={`text-sm mt-2 ${colors.text}`}>{result.updated.map(s => s.email).join(', ')}</div>
        )}
      </Alert>
    )}

    {result.failed && result.failed.length > 0 && (
      <Alert icon={<AlertCircle size={20} />} color="red" variant="light">
        <div className={`font-medium ${colors.text}`}>❌ 失败 {result.failed.length} 个</div>
        <Stack gap={4} mt="xs" p={0}>
          {result.failed.map((f, i) => (
            <div key={i} className={`text-sm ${colors.text}`}>{f.error}</div>
          ))}
        </Stack>
      </Alert>
    )}
  </Stack>
)

return (
  <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
    <DialogContent maxWidth="700px">
      <DialogHeader>
        <div className="flex items-center gap-3">
          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ${accent.shadow}`}>
            <Upload size={20} className="text-white" strokeWidth={2} />
          </div>
          <div>
            <DialogTitle>{t('import.title')}</DialogTitle>
            <DialogDescription>{t('import.subtitle') || '批量导入账号数据'}</DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <DialogBody noPadding>
        {importResult || kiroResult || kiroCliResult ? (
          <div className="px-6 py-4">
            {importResult && renderResult(importResult)}
            {kiroResult && renderResult(kiroResult)}
            {kiroCliResult && (
              <Alert
                icon={kiroCliResult.success ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
                color={kiroCliResult.success ? "teal" : "red"}
                variant="light"
              >
                <div className={`text-sm font-medium ${colors.text}`}>
                  {kiroCliResult.success
                    ? (kiroCliResult.isNew
                      ? `✅ 新增账号: ${kiroCliResult.email}`
                      : `📝 更新账号: ${kiroCliResult.email}`)
                    : '❌ 导入失败'}
                </div>
                {kiroCliResult.error && (
                  <div className={`text-xs mt-1 ${colors.textMuted}`}>{kiroCliResult.error}</div>
                )}
              </Alert>
            )}
          </div>
        ) : importing || kiroImporting || kiroCliImporting ? (
          <div className="px-6 py-6">
            <div className={`p-5 rounded-xl ${colors.cardSecondary} border ${colors.cardBorder}`}>
              <div className="flex items-center gap-4 mb-4">
                <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${colors.cardSecondary}`}>
                  <Loader2 size={20} className={colors.primary} />
                </div>
                <div>
                  <div className={`font-medium ${colors.text}`}>
                    {importing ? t('import.importing') : kiroImporting ? '正在从 Kiro 导入...' : '正在从 kiro-cli 导入...'}
                  </div>
                  <div className={`text-sm ${colors.textMuted}`}>
                    {kiroCliImporting ? '请稍候...' : `${(importing ? importProgress : kiroProgress).current}/${(importing ? importProgress : kiroProgress).total}`}
                  </div>
                </div>
              </div>
              <Progress
                value={(importing ? importProgress : kiroProgress).current /
                  (importing ? importProgress : kiroProgress).total * 100}
                size="lg"
                radius="xl"
                classNames={{
                  root: colors.cardSecondary,
                  bar: colors.primary
                }}
              />
            </div>
          </div>
        ) : (
          <Tabs value={activeTab} onChange={setActiveTab}>
            <Tabs.List className="px-6 pt-2 pb-3 border-b-0">
              <div className={`grid grid-cols-3 gap-1 p-1 rounded-xl border ${colors.cardBorder} ${colors.cardSecondary}`}>
                <button
                  onClick={() => setActiveTab('json')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium ${activeTab === 'json'
                      ? `${colors.card} shadow-sm ring-1 ${colors.ringColor} ${colors.text}`
                      : `${colors.cardHover} ${colors.textMuted}`
                    }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <FileJson size={16} />
                    <span>{t('import.jsonTab')}</span>
                  </div>
                </button>
                <button
                  onClick={() => setActiveTab('kiro')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium ${activeTab === 'kiro'
                      ? `${colors.card} shadow-sm ring-1 ${colors.ringColor} ${colors.text}`
                      : `${colors.cardHover} ${colors.textMuted}`
                    }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Database size={16} />
                    <span>{t('import.kiroTab')}</span>
                  </div>
                </button>
                <button
                  onClick={() => setActiveTab('kiro-cli')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium ${activeTab === 'kiro-cli'
                      ? `${colors.card} shadow-sm ring-1 ${colors.ringColor} ${colors.text}`
                      : `${colors.cardHover} ${colors.textMuted}`
                    }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Database size={16} />
                    <span>{t('import.kiroCliTab')}</span>
                  </div>
                </button>
              </div>
            </Tabs.List>

            <Tabs.Panel value="json" pt="md" className="px-6 pb-4">
              <Stack gap="lg">
                <Group>
                  <FileButton onChange={handleFileSelect} accept=".json">
                    {(props) => <MantineButton {...props} variant="light" leftSection={<FileJson size={16} />}>{t('import.selectFile')}</MantineButton>}
                  </FileButton>
                  <MantineButton variant="light" color="blue" size="sm" onClick={() => setJsonText(JSON.stringify([{ refreshToken: "", provider: "Google" }], null, 2))}>
                    Social 模板
                  </MantineButton>
                  <MantineButton variant="light" color="violet" size="sm" onClick={() => setJsonText(JSON.stringify([{ refreshToken: "", clientId: "", clientSecret: "", provider: "BuilderId" }], null, 2))}>
                    BuilderId 模板
                  </MantineButton>
                  <MantineButton variant="light" color="grape" size="sm" onClick={() => setJsonText(JSON.stringify([{ refreshToken: "", clientId: "", clientSecret: "", provider: "Enterprise" }], null, 2))}>
                    Enterprise 模板
                  </MantineButton>
                </Group>

                <Textarea
                  label={t('import.orPaste')}
                  value={jsonText}
                  onChange={(e) => { setJsonText(e.target.value); parseJson(e.target.value) }}
                  rows={10}
                  placeholder={`[{"refreshToken": "aor...", "provider": "Google"}]`}
                  classNames={{
                    input: `${colors.text} ${colors.input} ${colors.inputFocus} font-mono`
                  }}
                />

                {parseResult && (
                  <Stack gap="xs">
                    {parseResult.valid.length > 0 && (
                      <Alert icon={<CheckCircle size={16} />} color="teal" variant="light">
                        {t('import.parseSuccess')}: {parseResult.valid.length} {t('import.validRecords')}
                      </Alert>
                    )}
                    {parseResult.errors.length > 0 && (
                      <Alert icon={<AlertCircle size={16} />} color="red" variant="light">
                        <div className={`text-sm font-medium ${colors.text}`}>{t('import.validationError')}</div>
                        <Stack gap={2} mt="xs">
                          {parseResult.errors.slice(0, 5).map((err, i) => (
                            <div key={i} className={`text-xs ${colors.text}`}>{err}</div>
                          ))}
                          {parseResult.errors.length > 5 && (
                            <div className={`text-xs ${colors.text}`}>{t('import.moreErrors', { count: parseResult.errors.length - 5 })}</div>
                          )}
                        </Stack>
                      </Alert>
                    )}
                  </Stack>
                )}
              </Stack>
            </Tabs.Panel>

            <Tabs.Panel value="kiro" pt="md" className="px-6 pb-4">
              <Stack gap="lg">
                <Alert color="indigo" variant="light">
                  <div className={`text-sm font-medium ${colors.text}`}>从 Kiro IDE 导入账号</div>
                  <div className={`text-xs mt-1 ${colors.textMuted}`}>
                    自动读取 Kiro IDE 缓存的账号信息（~/.aws/sso/cache/kiro-auth-token.json）
                  </div>
                </Alert>

                {kiroLoading ? (
                  <div className={`p-5 rounded-xl ${colors.cardSecondary} border ${colors.cardBorder}`}>
                    <div className="flex items-center gap-3">
                      <Loader2 size={20} className={`animate-spin ${accent.text}`} />
                      <div className={`text-sm ${colors.text}`}>正在检测 Kiro IDE 账号...</div>
                    </div>
                  </div>
                ) : kiroError ? (
                  <Alert icon={<AlertCircle size={16} />} color="red" variant="light">
                    <div className={`text-sm font-medium ${colors.text}`}>检测失败</div>
                    <div className={`text-xs mt-1 ${colors.textMuted}`}>{kiroError}</div>
                    <MantineButton
                      variant="light"
                      color="red"
                      size="xs"
                      mt="sm"
                      leftSection={<RefreshCw size={14} />}
                      onClick={detectKiroAccounts}
                    >
                      重新检测
                    </MantineButton>
                  </Alert>
                ) : kiroAccounts.length > 0 ? (
                  <>
                    <Alert icon={<CheckCircle size={16} />} color="teal" variant="light">
                      <div className={`text-sm font-medium ${colors.text}`}>检测到 {kiroAccounts.length} 个账号</div>
                    </Alert>

                    <div className={`p-4 rounded-xl ${colors.cardSecondary} border ${colors.cardBorder}`}>
                      <Stack gap="sm">
                        {kiroAccounts.map((account, index) => (
                          <div key={index} className={`p-3 rounded-lg ${colors.card} border ${colors.cardBorder}`}>
                            <div className="flex items-center justify-between">
                              <div>
                                <div className={`text-sm font-medium ${colors.text}`}>
                                  {account.provider} ({account.authMethod})
                                </div>
                                <div className={`text-xs ${colors.textMuted}`}>
                                  {getAccountDisplayName(account)}
                                </div>
                              </div>
                              <div className={`px-2 py-1 rounded text-xs ${colors.badgeInfo}`}>
                                {account.authMethod === 'IdC' ? 'IdC' : 'Social'}
                              </div>
                            </div>
                          </div>
                        ))}
                      </Stack>
                    </div>
                  </>
                ) : (
                  <Alert icon={<AlertCircle size={16} />} color="gray" variant="light">
                    <div className={`text-sm ${colors.text}`}>未检测到 Kiro IDE 账号</div>
                    <div className={`text-xs mt-1 ${colors.textMuted}`}>
                      请先在 Kiro IDE 中登录账号
                    </div>
                  </Alert>
                )}
              </Stack>
            </Tabs.Panel>

            <Tabs.Panel value="kiro-cli" pt="md" className="px-6 pb-4">
              <Stack gap="lg">
                <Alert color="violet" variant="light">
                  <div className={`text-sm font-medium ${colors.text}`}>{t('import.kiroCliTitle')}</div>
                  <div className={`text-xs mt-1 ${colors.textMuted}`}>
                    {t('import.kiroCliHint')}
                  </div>
                  <div className={`text-xs mt-1 ${colors.textMuted}`}>
                    {t('import.kiroCliInstallPrefix')} <code>{t('import.kiroCliInstallCommand')}</code>
                  </div>
                </Alert>

                {isWindowsOs && (
                  <Alert icon={<AlertCircle size={16} />} color="orange" variant="light">
                    <div className={`text-sm font-medium ${colors.text}`}>{t('import.kiroCliWindowsWslTitle')}</div>
                    <div className={`text-xs mt-1 ${colors.textMuted}`}>
                      {t('import.kiroCliWindowsWslHint')}
                    </div>
                  </Alert>
                )}

                {kiroCliDetecting ? (
                  <div className={`p-5 rounded-xl ${colors.cardSecondary} border ${colors.cardBorder}`}>
                    <div className="flex items-center gap-3">
                      <Loader2 size={20} className={`animate-spin ${accent.text}`} />
                      <div className={`text-sm ${colors.text}`}>{t('import.kiroCliDetecting')}</div>
                    </div>
                  </div>
                ) : kiroCliDetected ? (
                  <Alert icon={<CheckCircle size={16} />} color="teal" variant="light">
                    <div className={`text-sm font-medium ${colors.text}`}>{t('import.kiroCliDetected')}</div>
                    <div className={`text-xs mt-1 ${colors.textMuted}`}>{kiroCliDbPath}</div>
                  </Alert>
                ) : (
                  <Alert icon={<AlertCircle size={16} />} color="gray" variant="light">
                    <div className={`text-sm ${colors.text}`}>{t('import.kiroCliNotDetected')}</div>
                    <div className={`text-xs mt-1 ${colors.textMuted}`}>
                      {t('import.kiroCliPathHintManual')}
                    </div>
                  </Alert>
                )}

                <div className={`p-4 rounded-xl ${colors.cardSecondary} border ${colors.cardBorder}`}>
                  <Stack gap="md">
                    <div>
                      <label className={`text-sm font-medium ${colors.text} block mb-2`}>
                        {t('import.kiroCliPathLabel')}
                      </label>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          value={kiroCliDbPath}
                          onChange={(e) => {
                            setKiroCliDbPath(e.target.value)
                            setKiroCliDetected(false)
                          }}
                          placeholder={t('import.kiroCliPathPlaceholder')}
                          className={`flex-1 px-4 py-3 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2 transition-all`}
                        />
                        <FileButton
                          onChange={(file) => {
                            if (file) {
                              setKiroCliDbPath(file.path)
                              setKiroCliDetected(false)
                            }
                          }}
                          accept=".sqlite3,.db"
                        >
                          {(props) => (
                            <MantineButton
                              {...props}
                              variant="light"
                              className="px-4"
                            >
                              {t('import.browse')}
                            </MantineButton>
                          )}
                        </FileButton>
                      </div>
                      <div className={`text-xs mt-1 ${colors.textMuted}`}>
                        {kiroCliDetected ? t('import.kiroCliPathHintDetected') : t('import.kiroCliPathHintManual')}
                      </div>
                    </div>

                    {kiroCliResult && (
                      <Alert
                        icon={kiroCliResult.success ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
                        color={kiroCliResult.success ? "teal" : "red"}
                        variant="light"
                      >
                        <div className={`text-sm font-medium ${colors.text}`}>
                          {kiroCliResult.success 
                            ? (kiroCliResult.isNew 
                                ? `✅ 新增账号: ${kiroCliResult.email}`
                                : `📝 更新账号: ${kiroCliResult.email}`)
                            : '❌ 导入失败'}
                        </div>
                        {kiroCliResult.error && (
                          <div className={`text-xs mt-1 ${colors.textMuted}`}>{kiroCliResult.error}</div>
                        )}
                      </Alert>
                    )}
                  </Stack>
                </div>
              </Stack>
            </Tabs.Panel>
          </Tabs>
        )}
      </DialogBody>

      <DialogFooter>
        <div></div>
        {importResult || kiroResult || kiroCliResult ? (
          <div className="flex gap-3">
            <Button
              variant="secondary"
              onClick={() => {
                setImportResult(null)
                setKiroResult(null)
                setKiroCliResult(null)
                setJsonText('')
                setParseResult(null)
              }}
            >
              {t('import.continueImport')}
            </Button>
            <Button variant="success" onClick={onClose}>
              {t('import.done')}
            </Button>
          </div>
        ) : importing || kiroImporting || kiroCliImporting ? (
          <div></div>
        ) : activeTab === 'json' ? (
          <div className="flex justify-between w-full">
            <Button
              variant="secondary"
              onClick={() => {
                onClose()
                onNavigate?.('desktopOAuth')
              }}
              className="flex items-center gap-2"
            >
              <LogIn size={16} />
              在线登录
            </Button>
            <div className="flex gap-3">
              <Button variant="secondary" onClick={onClose}>
                {t('common.cancel')}
              </Button>
              <Button
                onClick={handleJsonImport}
                disabled={!parseResult?.valid.length}
                className="flex items-center gap-2"
              >
                <Upload size={16} />
                {t('import.import')} {parseResult?.valid.length ? `(${parseResult.valid.length})` : ''}
              </Button>
            </div>
          </div>
        ) : activeTab === 'kiro' ? (
          <div className="flex justify-between w-full">
            <Button
              variant="secondary"
              onClick={() => {
                onClose()
                onNavigate?.('desktopOAuth')
              }}
              className="flex items-center gap-2"
            >
              <LogIn size={16} />
              在线登录
            </Button>
            <div className="flex gap-3">
              <Button variant="secondary" onClick={onClose}>
                {t('common.cancel')}
              </Button>
              <Button
                onClick={handleKiroImport}
                disabled={kiroAccounts.length === 0}
                className="flex items-center gap-2"
              >
                <Database size={16} />
                导入 ({kiroAccounts.length})
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex justify-between w-full">
            <Button
              variant="secondary"
              onClick={() => {
                onClose()
                onNavigate?.('desktopOAuth')
              }}
              className="flex items-center gap-2"
            >
              <LogIn size={16} />
              在线登录
            </Button>
            <div className="flex gap-3">
              <Button variant="secondary" onClick={onClose}>
                {t('common.cancel')}
              </Button>
              <Button
                onClick={handleKiroCliImport}
                disabled={!kiroCliDbPath}
                className="flex items-center gap-2"
              >
                <Database size={16} />
                导入
              </Button>
            </div>
          </div>
        )}
      </DialogFooter>
    </DialogContent>
  </DialogRoot>
  )
}

export default ImportAccountModal


