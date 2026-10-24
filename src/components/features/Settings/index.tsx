import { useState, useEffect, useCallback } from 'react'
import { getKiroSettings, getAppSettings, getCustomKiroPath, checkIdeInstallation, getAppDataDir, setKiroProxy, setKiroModel, clearCustomKiroPath, detectInstalledBrowsers, detectSystemProxy, openAppDataDir, openKiroSettingsFile, setKiroNotification, setKiroTelemetry } from '../../../api/settingsApi'
import { getSystemMachineGuid, resetSystemMachineGuid, restartAsAdmin } from '../../../api/kiroApi'
import { emit } from '@tauri-apps/api/event'
import { Palette, Settings as SettingsIcon, LayoutDashboard, Cpu, FileJson } from 'lucide-react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../ui/tabs'
import { Button } from '../../ui/button'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { useAppSettings } from '../../../contexts/AppSettingsContext'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { persistAppSettings, runKiroCommandWithAppSettings, makeAppBoolToggle, makeKiroBoolToggle } from './settingsActions'
import { isValidBrowserPath, isValidProxy } from './settingsValidators'
import {
  DEFAULT_NOTIFICATIONS,
  DEFAULT_TELEMETRY,
  NOTIFICATION_SETTINGS_FIELD_MAP,
  type NotificationState,
  type TelemetryState,
} from './settingsConstants'
import SettingsAppearance from './SettingsAppearance'
import SettingsGeneral from './SettingsGeneral'
import SettingsKiro from './SettingsKiro'

function Settings() {
    const { t, theme, setTheme, settings, updateSettings } = useApp()
    const { showConfirm, showError, showSuccess } = useDialog()
    const density = settings?.density || 'comfortable'
    const handleDensityChange = (value: string) => {
        updateSettings({ density: value })
    }
    const uiScale = settings?.uiScale || 100
    const handleUiScaleChange = (value: number) => {
        updateSettings({ uiScale: value })
    }
    const reduceMotion = !!settings?.reduceMotion
    const handleReduceMotionChange = (value: boolean) => {
        updateSettings({ reduceMotion: value })
    }
    const { updateSettings: updateAppSettings } = useAppSettings()
    const { privacyMode, setPrivacyMode } = usePrivacy()
    const [activeTab, setActiveTab] = useState('ide')

    const [aiModel, setAiModel] = useState('claude-sonnet-4.5')
    const [lockModel, setLockModel] = useState(false)
    const [autoRefresh, setAutoRefresh] = useState(true)
    const [autoRefreshInterval, setAutoRefreshInterval] = useState(50) // 分钟
    const [httpProxy, setHttpProxy] = useState('')
    const [originalProxy, setOriginalProxy] = useState('') // 原始代理值，用于判断是否修改
    const [appProxyMode, setAppProxyMode] = useState('followKiro')
    const [savingProxy, setSavingProxy] = useState(false)
    const [savingModel, setSavingModel] = useState(false)
    const [browserPath, setBrowserPath] = useState('')
    const [originalBrowserPath, setOriginalBrowserPath] = useState('')
    const [savingBrowser, setSavingBrowser] = useState(false)
    const [detectedBrowsers, setDetectedBrowsers] = useState<any[]>([])
    const [showBrowserList, setShowBrowserList] = useState(false)
    const [customKiroPath, setCustomKiroPath] = useState<string | null>(null)
    const [detectingProxy, setDetectingProxy] = useState(false)
    const [enableCodebaseIndexing, setEnableCodebaseIndexing] = useState(false) // 对齐 Kiro 1.0 默认（实验特性，默认关）

    // Agent 设置
    const [agentAutonomy, setAgentAutonomy] = useState('Autopilot') // 'Autopilot' | 'Supervised'（对齐 Kiro 1.0 默认）
    const [enableTabAutocomplete, setEnableTabAutocomplete] = useState(false) // 对齐 Kiro 1.0 默认
    const [usageSummary, setUsageSummary] = useState(true)
    const [enableDebugLogs, setEnableDebugLogs] = useState(false)

    // 新增 Kiro IDE 设置
    const [referenceTracker, setReferenceTracker] = useState(false)

    // 通知 / 遥测（同样是 Kiro IDE settings.json 里的键，原独立「通知」tab）
    const [notifications, setNotifications] = useState<NotificationState>(DEFAULT_NOTIFICATIONS)
    const [telemetry, setTelemetry] = useState<TelemetryState>(DEFAULT_TELEMETRY)

    // 自动换号设置
    const [autoSwitchEnabled, setAutoSwitchEnabled] = useState(false)
    const [autoSwitchThreshold, setAutoSwitchThreshold] = useState(1)
    const [autoSwitchInterval, setAutoSwitchInterval] = useState(5)

    // 关闭窗口行为
    const [closeToTray, setCloseToTray] = useState(false)

    // 系统机器码
    const [systemMachineInfo, setSystemMachineInfo] = useState<any>(null)
    const [machineGuidAction, setMachineGuidAction] = useState<string | null>(null) // 'reset'

    // 应用数据目录
    const [appDataDir, setAppDataDir] = useState<string>('')

    // 加载设置（指纹延迟加载，不阻塞页面）
    const loadSettings = useCallback(async () => {
        try {
            // 先加载核心设置（快速）
            const [kiroSettings, appSettings, sysMachine, kiroPath, ideInfo, dataDir] = await Promise.all([
                getKiroSettings().catch(() => null),
                getAppSettings().catch(() => null),
                getSystemMachineGuid().catch(() => null),
                getCustomKiroPath().catch(() => null),
                checkIdeInstallation().catch(() => null),
                getAppDataDir().catch(() => '')
            ])
            setSystemMachineInfo(sysMachine)
            // 优先显示自定义路径，否则显示检测到的默认路径
            setCustomKiroPath(kiroPath || (ideInfo?.ide_path || null))
            setAppDataDir(dataDir)

            // 从 Kiro IDE 设置读取
            if (kiroSettings) {
                const proxy = kiroSettings.httpProxy || ''
                setHttpProxy(proxy)
                setOriginalProxy(proxy)
                setAiModel(kiroSettings.modelSelection || 'claude-sonnet-4.5')
                setEnableCodebaseIndexing(kiroSettings.enableCodebaseIndexing ?? false)
                // Agent 设置（默认值对齐 Kiro 1.0）
                setAgentAutonomy(kiroSettings.agentAutonomy || 'Autopilot')
                setEnableTabAutocomplete(kiroSettings.enableTabAutocomplete ?? false)
                setUsageSummary(kiroSettings.usageSummary ?? true)
                setEnableDebugLogs(kiroSettings.enableDebugLogs ?? false)
                // 新增设置
                setReferenceTracker(kiroSettings.referenceTracker ?? false)
                // 通知（默认值对齐 Kiro 1.0：failure / success 默认 false）
                setNotifications({
                    notifyActionRequired: kiroSettings.notifyActionRequired ?? true,
                    notifyFailure: kiroSettings.notifyFailure ?? false,
                    notifySuccess: kiroSettings.notifySuccess ?? false,
                    notifyBilling: kiroSettings.notifyBilling ?? true,
                })
                // 遥测
                setTelemetry({
                    telemetryContentCollection: kiroSettings.telemetryContentCollection ?? false,
                    telemetryUsageAnalytics: kiroSettings.telemetryUsageAnalytics ?? false,
                    telemetryEditStats: kiroSettings.telemetryEditStats ?? false,
                    telemetryFeedback: kiroSettings.telemetryFeedback ?? false,
                    telemetryPromptLogging: kiroSettings.telemetryPromptLogging ?? false,
                    telemetryEditStatsDetails: kiroSettings.telemetryEditStatsDetails ?? false,
                    telemetryEditStatsDecorations: kiroSettings.telemetryEditStatsDecorations ?? false,
                    telemetryEditStatsStatusBar: kiroSettings.telemetryEditStatsStatusBar ?? false,
                })
            }
            // 从应用设置读取
            if (appSettings) {
                setLockModel(appSettings.lockModel ?? false)
                setAutoRefresh(appSettings.autoRefresh ?? true)
                setAutoRefreshInterval(appSettings.autoRefreshInterval ?? 50)
                const browser = appSettings.browserPath || ''
                setBrowserPath(browser)
                setOriginalBrowserPath(browser)
                // 自动换号设置
                setAutoSwitchEnabled(appSettings.autoSwitchEnabled ?? false)
                setAutoSwitchThreshold(appSettings.autoSwitchThreshold ?? 1)
                setAutoSwitchInterval(appSettings.autoSwitchInterval ?? 5)
                // 关闭窗口行为
                setCloseToTray(appSettings.closeToTray ?? false)
                setAppProxyMode(appSettings.appProxyMode || 'followKiro')
            }
        } catch (err) {
            console.error('Failed to load settings:', err)
        }
    }, [])

    useEffect(() => {
        loadSettings()
    }, [loadSettings])

    const saveAppSettings = (updates: any, notifyChange = false) => persistAppSettings({
        updates,
        notifyChange,
        updateAppSettings,
        emitFn: emit,
        showError,
        t})

    const runKiroCommand = (command: string, commandArgs: any, appSettingsUpdates: any = null, notifyChange = false) => runKiroCommandWithAppSettings({
        command,
        commandArgs,
        appSettingsUpdates,
        notifyChange,
        persistSettings: ({ updates, notifyChange: shouldNotify }: any) => saveAppSettings(updates, shouldNotify),
        showError,
        t})

    const handleApplyProxy = async () => {
        if (!isValidProxy(httpProxy)) {
            await showError(t('settings.saveFailed'), t('settings.invalidProxyFormat'))
            return
        }

        setSavingProxy(true)
        try {
            await setKiroProxy(httpProxy)
            setOriginalProxy(httpProxy)
            await showSuccess(t('settings.saveSuccess'), httpProxy ? t('settings.proxyApplied') : t('settings.proxyCleared'))
        } catch (err: any) {
            await showError(t('settings.saveFailed'), t('settings.saveFailed') + ': ' + err)
        } finally {
            setSavingProxy(false)
        }
    }

    const handleAppProxyModeChange = async (mode: string) => {
        setAppProxyMode(mode)
        await saveAppSettings({ appProxyMode: mode })
    }

    const handleApplyModel = async (model: string) => {
        setAiModel(model)
        setSavingModel(true)
        try {
            await setKiroModel(model)
            if (lockModel) {
                await saveAppSettings({ lockedModel: model })
            }
        } catch (err: any) {
            await showError(t('settings.saveFailed'), t('settings.saveFailed') + ': ' + err)
        } finally {
            setSavingModel(false)
        }
    }

    const handleLockModelChange = async (checked: boolean) => {
        setLockModel(checked)
        await saveAppSettings({ lockModel: checked, lockedModel: checked ? aiModel : null })
    }

    const handleAutoRefreshChange = makeAppBoolToggle(setAutoRefresh, 'autoRefresh', saveAppSettings, true)

    const handleAutoRefreshIntervalChange = async (value: string) => {
        const interval = parseInt(value) || 50
        setAutoRefreshInterval(interval)
        await saveAppSettings({ autoRefreshInterval: interval }, true)
    }

    const handleAutoSwitchEnabledChange = makeAppBoolToggle(setAutoSwitchEnabled, 'autoSwitchEnabled', saveAppSettings, true)

    const handleAutoSwitchThresholdChange = async (value: string | number) => {
        // 空输入/非法值不落盘：此前 `parseFloat('') || 0` 会把阈值静默写成 0，
        // 效果等同于「额度降到 0 才换号」。保留上一个有效值（受控输入会回填）。
        const parsedValue = typeof value === 'number' ? value : parseFloat(value)
        if (!Number.isFinite(parsedValue) || parsedValue < 0) return
        setAutoSwitchThreshold(parsedValue)
        await saveAppSettings({ autoSwitchThreshold: parsedValue }, true)
    }

    const handleAutoSwitchIntervalChange = async (value: string) => {
        const interval = parseInt(value) || 5
        setAutoSwitchInterval(interval)
        await saveAppSettings({ autoSwitchInterval: interval }, true)
    }

    const handleCloseToTrayChange = makeAppBoolToggle(setCloseToTray, 'closeToTray', saveAppSettings)
    const switchTarget = settings?.switchTarget || 'ide'
    const handleSwitchTargetChange = (value: string) => {
        // Select 的候选值就是这三个，收窄回联合类型以匹配 updateSettings 的签名
        updateSettings({ switchTarget: value as 'ide' | 'cli' | 'both' })
    }

    const handleBrowseKiroPath = async () => {
        try {
            const { open } = await import('@tauri-apps/plugin-dialog')
            const selected = await open({
                directory: false,
                multiple: false,
                filters: [{
                    name: 'Kiro',
                    extensions: window.navigator.platform.toLowerCase().includes('win') ? ['exe'] : []
                }]
            })

            if (selected) {
                await setCustomKiroPath(selected)
                setCustomKiroPath(selected)
                showSuccess(t('settings.kiroPathSaved'))
            }
        } catch (error) {
            showError(String(error))
        }
    }

    const handleClearKiroPath = async () => {
        try {
            await clearCustomKiroPath()
            setCustomKiroPath(null)
            showSuccess(t('settings.kiroPathCleared'))
        } catch (error) {
            showError(String(error))
        }
    }

    const handleCodebaseIndexingChange = makeKiroBoolToggle(setEnableCodebaseIndexing, runKiroCommand, 'set_kiro_codebase_indexing', 'enableCodebaseIndexing')

    const handleAgentAutonomyChange = async (mode: string) => {
        setAgentAutonomy(mode)
        await runKiroCommand('set_kiro_agent_autonomy', { autonomy: mode })
    }

    const handleTabAutocompleteChange = makeKiroBoolToggle(setEnableTabAutocomplete, runKiroCommand, 'set_kiro_tab_autocomplete', 'enableTabAutocomplete')

    const handleUsageSummaryChange = makeKiroBoolToggle(setUsageSummary, runKiroCommand, 'set_kiro_usage_summary', 'usageSummary')

    const handleDebugLogsChange = makeKiroBoolToggle(setEnableDebugLogs, runKiroCommand, 'set_kiro_debug_logs', 'enableDebugLogs')

    const handleReferenceTrackerChange = makeKiroBoolToggle(setReferenceTracker, runKiroCommand, 'set_kiro_reference_tracker', 'referenceTracker')

    // 通知开关：写 Kiro IDE settings.json，并按字段映射同步 app-settings.json
    const handleNotificationChange = async (key: string, checked: boolean, field: keyof NotificationState) => {
        setNotifications(prev => ({ ...prev, [field]: checked }))
        try {
            await setKiroNotification(key, checked)
            const appField = (NOTIFICATION_SETTINGS_FIELD_MAP as any)[key]
            if (appField) {
                await updateAppSettings({ [appField]: checked })
            }
        } catch (err: any) {
            await showError(t('settings.saveFailed'), `${t('settings.saveFailed')}: ${err}`)
        }
    }

    // 遥测开关：写 Kiro IDE settings.json，并同步 app-settings.json（字段名同名）
    const handleTelemetryChange = async (ideKey: string, checked: boolean, field: keyof TelemetryState) => {
        setTelemetry(prev => ({ ...prev, [field]: checked }))
        try {
            await setKiroTelemetry(ideKey, checked)
            await updateAppSettings({ [field]: checked })
        } catch (err: any) {
            await showError(t('settings.saveFailed'), `${t('settings.saveFailed')}: ${err}`)
        }
    }

    const handleApplyBrowser = async () => {
        if (!isValidBrowserPath(browserPath)) {
            await showError(t('settings.saveFailed'), t('settings.invalidBrowserPath'))
            return
        }

        setSavingBrowser(true)
        try {
            await saveAppSettings({ browserPath: browserPath })
            setOriginalBrowserPath(browserPath)
            await showSuccess(t('settings.saveSuccess'), browserPath ? t('settings.browserSaved') : t('settings.defaultBrowser'))
        } catch (err: any) {
            await showError(t('settings.saveFailed'), err.toString())
        } finally {
            setSavingBrowser(false)
        }
    }

    const handleDetectBrowsers = async () => {
        try {
            const browsers = await detectInstalledBrowsers()
            setDetectedBrowsers(browsers)
            setShowBrowserList(true)
        } catch (err: any) {
            await showError(t('settings.detectFailed'), err.toString())
        }
    }

    const handleDetectProxy = async () => {
        setDetectingProxy(true)
        try {
            const proxyInfo = await detectSystemProxy()
            if (proxyInfo.enabled && proxyInfo.httpProxy) {
                setHttpProxy(proxyInfo.httpProxy)
                await showSuccess(t('settings.detectSuccess'), `${t('settings.systemProxyDetected')}: ${proxyInfo.httpProxy}`)
            } else {
                await showError(t('settings.noProxyDetected'), t('settings.noProxyConfigured'))
            }
        } catch (err: any) {
            await showError(t('settings.detectFailed'), err.toString())
        } finally {
            setDetectingProxy(false)
        }
    }

    const handleResetSystemMachineGuid = async () => {
        const confirmed = await showConfirm(
            `⚠️ ${t('settings.resetSystemMachineGuid')}`,
            t('settings.confirmResetSystemMachineGuid'),
            { confirmText: t('settings.confirmReset'), cancelText: t('common.cancel') }
        )
        if (!confirmed) return

        setMachineGuidAction('reset')
        try {
            const newGuid = await resetSystemMachineGuid()
            setSystemMachineInfo((prev: any) => ({ ...prev, machineGuid: newGuid }))
            await showSuccess(t('settings.resetSuccess'), `${t('settings.newMachineGuid')}: ${newGuid}`)
        } catch (err: any) {
            await showError(t('settings.resetFailed'), err.toString())
        } finally {
            // 成败都要复位：否则成功路径下 machineGuidAction 一直为 'reset'，
            // 按钮永久 disabled、转圈不停（须刷新页面才能再点）
            setMachineGuidAction(null)
        }
    }

    // 以管理员身份重启：提权实例会沿用当前数据目录（后端 --data-dir= 传递），
    // 因此即便 UAC 用的是其他管理员账号，也不会读不到账号数据。
    const handleRestartAsAdmin = async () => {
        const confirmed = await showConfirm(
            `⚠️ ${t('settings.restartAsAdmin')}`,
            t('settings.confirmRestartAsAdmin'),
            { confirmText: t('settings.restartAsAdmin'), cancelText: t('common.cancel') }
        )
        if (!confirmed) return

        try {
            await restartAsAdmin()
            // 提权进程确认启动后，后端会退出当前进程
        } catch (err: any) {
            await showError(t('settings.restartAsAdminFailed'), err.toString())
        }
    }

    const handleOpenAppDataDir = async () => {
        try {
            await openAppDataDir()
        } catch (err: any) {
            await showError(t('settings.openFailed'), err.toString())
        }
    }

    // 用系统默认程序打开 Kiro IDE 的 settings.json（原始配置文件）
    const handleOpenKiroSettingsFile = async () => {
        try {
            await openKiroSettingsFile()
        } catch (err: any) {
            await showError(t('settings.openFailed'), err.toString())
        }
    }

    return (
        <div className="h-full glass-main p-6 flex flex-col min-h-0">
            <div className="w-full relative flex-1 flex flex-col min-h-0">
                {/* Header（紧凑 + 装饰 ring）*/}
                <div className="mb-4 flex items-center gap-3 animate-slide-in-left shrink-0">
                    <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary/80 to-primary flex items-center justify-center shadow-md ring-1 ring-primary/20">
                        <SettingsIcon size={20} className="text-primary-foreground" />
                    </div>
                    <div className="flex flex-col">
                        <h1 className="text-lg font-semibold text-foreground leading-tight">{t('settings.title')}</h1>
                        <p className="text-sm text-muted-foreground leading-tight">{t('settings.subtitle')}</p>
                    </div>
                </div>

                <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
                    <TabsList className="glass-card mb-4 flex h-10 w-full justify-start overflow-x-auto rounded-lg border-none p-0.5 no-scrollbar lg:w-fit shrink-0">
                        <TabsTrigger value="ide" className="gap-1.5 px-3 h-9 shrink-0 text-sm font-medium data-[state=active]:shadow-sm">
                            <Cpu size={14} />
                            {t('settings.kiro')}
                        </TabsTrigger>
                        <TabsTrigger value="general" className="gap-1.5 px-3 h-9 shrink-0 text-sm font-medium data-[state=active]:shadow-sm">
                            <LayoutDashboard size={14} />
                            {t('settings.general')}
                        </TabsTrigger>
                        <TabsTrigger value="appearance" className="gap-1.5 px-3 h-9 shrink-0 text-sm font-medium data-[state=active]:shadow-sm">
                            <Palette size={14} />
                            {t('settings.appearance')}
                        </TabsTrigger>
                    </TabsList>

                    {/* 仅内容区滚动：标题与标签栏固定，不随内容一起滚 */}
                    <div className="flex-1 min-h-0 overflow-y-auto pr-1">
                    <TabsContent value="ide">
                        {/* 打开 Kiro IDE 原始 settings.json，便于直接编辑配置文件 */}
                        <div className="mb-3 flex items-center justify-end">
                            <Button variant="outline" size="sm" onClick={handleOpenKiroSettingsFile}>
                                <FileJson size={14} /> {t('settings.openSettingsFile')}
                            </Button>
                        </div>
                        {/* KiroAgentAdvancedPanel 已并入 SettingsKiro，
                            以便按命名空间统一排序（代理压尾）；
                            MCP 配置与权限规则已迁至侧边栏「Agent 配置」页 */}
                        <SettingsKiro
                            aiModel={aiModel}
                            lockModel={lockModel}
                            agentAutonomy={agentAutonomy}
                            httpProxy={httpProxy}
                            setHttpProxy={setHttpProxy}
                            originalProxy={originalProxy}
                            appProxyMode={appProxyMode}
                            savingProxy={savingProxy}
                            detectingProxy={detectingProxy}
                            savingModel={savingModel}
                            enableCodebaseIndexing={enableCodebaseIndexing}
                            enableTabAutocomplete={enableTabAutocomplete}
                            usageSummary={usageSummary}
                            enableDebugLogs={enableDebugLogs}
                            referenceTracker={referenceTracker}
                            notifications={notifications}
                            telemetry={telemetry}
                            handleApplyModel={handleApplyModel}
                            handleLockModelChange={handleLockModelChange}
                            handleAgentAutonomyChange={handleAgentAutonomyChange}
                            handleApplyProxy={handleApplyProxy}
                            handleDetectProxy={handleDetectProxy}
                            handleAppProxyModeChange={handleAppProxyModeChange}
                            handleCodebaseIndexingChange={handleCodebaseIndexingChange}
                            handleTabAutocompleteChange={handleTabAutocompleteChange}
                            handleUsageSummaryChange={handleUsageSummaryChange}
                            handleDebugLogsChange={handleDebugLogsChange}
                            handleReferenceTrackerChange={handleReferenceTrackerChange}
                            handleNotificationChange={handleNotificationChange}
                            handleTelemetryChange={handleTelemetryChange}
                            t={t}
                        />
                    </TabsContent>

                    <TabsContent value="general">
                        <SettingsGeneral
                            autoRefresh={autoRefresh}
                            autoRefreshInterval={autoRefreshInterval}
                            privacyMode={privacyMode}
                            setPrivacyMode={setPrivacyMode}
                            autoSwitchEnabled={autoSwitchEnabled}
                            autoSwitchThreshold={autoSwitchThreshold}
                            autoSwitchInterval={autoSwitchInterval}
                            closeToTray={closeToTray}
                            browserPath={browserPath}
                            setBrowserPath={setBrowserPath}
                            originalBrowserPath={originalBrowserPath}
                            savingBrowser={savingBrowser}
                            detectedBrowsers={detectedBrowsers}
                            showBrowserList={showBrowserList}
                            setShowBrowserList={setShowBrowserList}
                            customKiroPath={customKiroPath}
                            handleBrowseKiroPath={handleBrowseKiroPath}
                            handleClearKiroPath={handleClearKiroPath}
                            systemMachineInfo={systemMachineInfo}
                            machineGuidAction={machineGuidAction}
                            handleResetSystemMachineGuid={handleResetSystemMachineGuid}
                            handleRestartAsAdmin={handleRestartAsAdmin}
                            handleDetectBrowsers={handleDetectBrowsers}
                            handleApplyBrowser={handleApplyBrowser}
                            handleAutoRefreshChange={handleAutoRefreshChange}
                            handleAutoRefreshIntervalChange={handleAutoRefreshIntervalChange}
                            handleAutoSwitchEnabledChange={handleAutoSwitchEnabledChange}
                            handleAutoSwitchThresholdChange={handleAutoSwitchThresholdChange}
                            handleAutoSwitchIntervalChange={handleAutoSwitchIntervalChange}
                            switchTarget={switchTarget}
                            handleSwitchTargetChange={handleSwitchTargetChange}
                            handleCloseToTrayChange={handleCloseToTrayChange}
                            appDataDir={appDataDir}
                            handleOpenAppDataDir={handleOpenAppDataDir}
                            t={t}
                        />
                    </TabsContent>

                    <TabsContent value="appearance">
                        <SettingsAppearance
                            theme={theme}
                            setTheme={setTheme}
                            density={density}
                            setDensity={handleDensityChange}
                            uiScale={uiScale}
                            setUiScale={handleUiScaleChange}
                            reduceMotion={reduceMotion}
                            setReduceMotion={handleReduceMotionChange}
                            t={t}
                        />
                    </TabsContent>
                    </div>
                </Tabs>
            </div>
        </div>
    )
}

export default Settings
