import { useEffect, useState } from 'react'
import { Search, RefreshCw, Check, Sparkles, Bot, Network, Lock, Bell, ShieldQuestion } from 'lucide-react'
import { Input } from '../../ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'
import {
  AI_MODELS,
  NOTIFICATION_ROWS,
  TELEMETRY_ROWS,
  type NotificationState,
  type TelemetryState,
} from './settingsConstants'
import { getKiroSettings, setKiroAgentSetting } from '../../../api/settingsApi'
import { useDialog } from '../../../contexts/DialogContext'
import SectionCard from './SectionCard'
import SwitchRow from './SwitchRow'
import ToggleRow from './ToggleRow'
import { rowShell } from './rowStyles'
import KiroAgentAdvancedPanel from './KiroAgentAdvancedPanel'
import SandboxCard from './SandboxCard'

// Kiro 设置页（kiro tab）的全部内容。
//
// 布局原则：**按功能聚合，卡内控件至少 2 个**（避免「一个开关一张卡」的 section 通胀），
// **同主题的卡必须相邻**，命名空间只作为「让同类键相邻」的辅助目标：
//   1-7. kiroAgent.* —— AI 模型 / Agent 行为（含 kiro.startupMode）/ 配置 MCP / 通知
//                       / Agent 高级 / 实验与高级 / 信任与自动批准
//   8.   权限规则     —— 独立配置（permissions）；紧贴「信任与自动批准」，同属
//                       「agent 能否不经询问就动手」
//   9.   telemetry.* —— 遥测与隐私
//   10.  http.*/app  —— 网络代理（按约定压尾）
//
// 说明：`kiro.*` 在 Kiro 里只有 startupMode 一个键，凑不成独立 section，因此并入
// 「Agent 行为」——它决定 Kiro 是否直接进 Agent 聚焦，与 agentAutonomy 是一对。

interface SettingsKiroProps {
  // 模型/工具
  aiModel: string
  lockModel: boolean
  agentAutonomy: string
  // 代理
  httpProxy: string
  setHttpProxy: (value: string) => void
  originalProxy: string
  appProxyMode: string
  savingProxy: boolean
  detectingProxy: boolean
  savingModel: boolean
  // Agent 行为开关
  enableCodebaseIndexing: boolean
  enableTabAutocomplete: boolean
  usageSummary: boolean
  enableDebugLogs: boolean
  referenceTracker: boolean
  // 通知 / 遥测
  notifications: NotificationState
  telemetry: TelemetryState
  // handlers
  handleApplyModel: (model: string) => Promise<void>
  handleLockModelChange: (checked: boolean) => Promise<void>
  handleAgentAutonomyChange: (mode: string) => Promise<void>
  handleApplyProxy: () => Promise<void>
  handleDetectProxy: () => Promise<void>
  handleAppProxyModeChange: (mode: string) => Promise<void>
  handleCodebaseIndexingChange: (checked: boolean) => Promise<void>
  handleTabAutocompleteChange: (checked: boolean) => Promise<void>
  handleUsageSummaryChange: (checked: boolean) => Promise<void>
  handleDebugLogsChange: (checked: boolean) => Promise<void>
  handleReferenceTrackerChange: (checked: boolean) => Promise<void>
  handleNotificationChange: (key: string, checked: boolean, field: keyof NotificationState) => void
  handleTelemetryChange: (ideKey: string, checked: boolean, field: keyof TelemetryState) => void
  t: (key: string) => string
}

const STARTUP_MODES = ['code', 'agentFocus'] as const
function SettingsKiro({
  aiModel,
  lockModel,
  agentAutonomy,
  httpProxy,
  setHttpProxy,
  originalProxy,
  appProxyMode,
  savingProxy,
  detectingProxy,
  savingModel,
  enableCodebaseIndexing,
  enableTabAutocomplete,
  usageSummary,
  enableDebugLogs,
  referenceTracker,
  notifications,
  telemetry,
  handleApplyModel,
  handleLockModelChange,
  handleAgentAutonomyChange,
  handleApplyProxy,
  handleDetectProxy,
  handleAppProxyModeChange,
  handleCodebaseIndexingChange,
  handleTabAutocompleteChange,
  handleUsageSummaryChange,
  handleDebugLogsChange,
  handleReferenceTrackerChange,
  handleNotificationChange,
  handleTelemetryChange,
  t,
}: SettingsKiroProps) {
  const { showError } = useDialog()
  const proxyChanged = httpProxy !== originalProxy

  // 少量键（kiro.startupMode）自取自写，
  // 省掉一层 props 透传；写入后由后端双向同步到 settings.json 与 app-settings.json。
  const [extras, setExtras] = useState({ startupMode: 'code' })

  useEffect(() => {
    let cancelled = false
    getKiroSettings<any>()
      .then(v => {
        if (cancelled || !v) return
        setExtras({
          startupMode: v.startupMode ?? 'code',
        })
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  const patchExtras = (p: Partial<typeof extras>) => setExtras(prev => ({ ...prev, ...p }))

  const applyExtra = async (key: string, value: unknown) => {
    try {
      await setKiroAgentSetting(key, value)
    } catch (err) {
      await showError(t('settings.saveFailed'), `${t('settings.saveFailed')}: ${err}`)
    }
  }

  return (
    <div className="space-y-3">
      {/* ========== 1-6. kiroAgent.* ========== */}

      {/* 1. AI 模型 */}
      <SectionCard
        title={t('settings.aiModel')}
        accent="violet"
        icon={<Sparkles size={14} className="text-violet-500" />}
        badge={savingModel ? <span className="text-[10px] text-primary animate-pulse">{t('settings.saving')}</span> : undefined}
      >
        <Select value={aiModel} onValueChange={handleApplyModel} disabled={savingModel}>
          <SelectTrigger className="h-9 text-xs"><SelectValue /></SelectTrigger>
          <SelectContent>
            {AI_MODELS.map(m => (
              <SelectItem key={m.value} value={m.value}>
                {m.recommended ? `${m.label} (⭐ ${t('common.recommended')})` : m.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <SwitchRow
          checked={lockModel}
          onCheckedChange={handleLockModelChange}
          icon={<Lock size={13} />}
          label={t('settings.lockModel')}
          hint={t('settings.lockModelDesc')}
        />
      </SectionCard>

      {/* 2. Agent 行为（含 kiro.startupMode）*/}
      <SectionCard
        title={t('settings.agentSettings')}
        accent="blue"
        icon={<Bot size={14} className="text-blue-500" />}
        desc={t('settings.agentSettingsDesc')}
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('settings.agentStartupMode')}</Label>
            <Select
              value={extras.startupMode}
              onValueChange={v => {
                patchExtras({ startupMode: v })
                void applyExtra('kiro.startupMode', v)
              }}
            >
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                {STARTUP_MODES.map(m => (
                  <SelectItem key={m} value={m}>{t(`settings.agentStartupMode_${m}`)}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('settings.agentAutonomy')}</Label>
            <Select value={agentAutonomy} onValueChange={handleAgentAutonomyChange}>
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="Supervised">{t('settings.agentSupervised')}</SelectItem>
                <SelectItem value="Autopilot">{t('settings.agentAutopilot')}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <p className="text-[11px] text-muted-foreground">{t('settings.agentStartupModeDesc')}</p>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          <ToggleRow
            checked={enableCodebaseIndexing}
            onChange={handleCodebaseIndexingChange}
            label={t('settings.enableCodebaseIndexing')}
          />
          <ToggleRow
            checked={enableTabAutocomplete}
            onChange={handleTabAutocompleteChange}
            label={t('settings.enableTabAutocomplete')}
          />
          <ToggleRow
            checked={usageSummary}
            onChange={handleUsageSummaryChange}
            label={t('settings.usageSummary')}
          />
          <ToggleRow
            checked={referenceTracker}
            onChange={handleReferenceTrackerChange}
            label={t('settings.referenceTracker')}
          />
          <ToggleRow
            checked={enableDebugLogs}
            onChange={handleDebugLogsChange}
            label={t('settings.enableDebugLogs')}
          />
        </div>
      </SectionCard>

      {/* 4. 通知 */}
      <SectionCard
        title={t('settings.notifications')}
        accent="blue"
        icon={<Bell size={14} className="text-blue-500" />}
        desc={t('settings.notificationsDesc')}
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {NOTIFICATION_ROWS.map(row => (
            <ToggleRow
              key={row.key}
              checked={notifications[row.field]}
              onChange={checked => handleNotificationChange(row.key, checked, row.field)}
              label={t(row.label)}
            />
          ))}
        </div>
      </SectionCard>

      {/* 5-7. Agent 高级 / 实验与高级 / 信任与自动批准（子面板）*/}
      <KiroAgentAdvancedPanel t={t} />

      {/* 8. 命令沙箱（sandbox.json，独立文件——Kiro IDE 1.3+，非 settings.json 键）*/}
      <SandboxCard t={t} />

      {/* ========== 9. telemetry.* ========== */}
      <SectionCard
        title={t('settings.telemetry')}
        accent="orange"
        icon={<ShieldQuestion size={14} className="text-orange-500" />}
        desc={t('settings.telemetryDesc')}
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {TELEMETRY_ROWS.map(row => (
            <ToggleRow
              key={row.ideKey}
              checked={telemetry[row.field]}
              onChange={checked => handleTelemetryChange(row.ideKey, checked, row.field)}
              label={t(row.label)}
            />
          ))}
        </div>
      </SectionCard>

      {/* ========== 10. 网络代理（压尾）========== */}
      <SectionCard
        title={t('settings.proxy')}
        accent="green"
        icon={<Network size={14} className="text-emerald-500" />}
        desc={t('settings.proxyTip')}
      >
        <div className="space-y-3">
          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('settings.httpProxy')}</Label>
            <div className="flex gap-1.5">
              <Input
                value={httpProxy}
                onChange={e => setHttpProxy(e.target.value)}
                placeholder="http://127.0.0.1:7897"
                className="h-8 text-xs flex-1 font-mono"
              />
              <button
                onClick={handleDetectProxy}
                disabled={detectingProxy}
                className="px-2.5 h-8 border rounded-md bg-card hover:bg-muted/50 border-border text-foreground transition-colors disabled:opacity-50 inline-flex items-center justify-center cursor-pointer"
                title={t('settings.detectProxyTitle')}
                aria-label={t('settings.detectProxyTitle')}
              >
                {detectingProxy ? <RefreshCw size={12} className="animate-spin" /> : <Search size={12} />}
              </button>
              <button
                onClick={handleApplyProxy}
                disabled={savingProxy || !proxyChanged}
                className={`px-3 h-8 rounded-md inline-flex items-center gap-1 text-xs font-medium border transition-colors disabled:opacity-50 cursor-pointer ${
                  proxyChanged
                    ? 'bg-primary text-primary-foreground border-primary hover:bg-primary/90'
                    : 'bg-muted text-muted-foreground border-border'
                }`}
              >
                {savingProxy ? <RefreshCw size={12} className="animate-spin" /> : <Check size={12} />}
                <span className="hidden sm:inline">{savingProxy ? t('settings.saving') : t('settings.apply')}</span>
              </button>
            </div>
          </div>

          <div className={rowShell('default')}>
            <div className="min-w-0">
              <span className="text-sm font-medium text-foreground whitespace-nowrap">{t('settings.appProxyMode')}</span>
              <p className="text-[11px] text-muted-foreground mt-0.5">{t('settings.appProxyModeDesc')}</p>
            </div>
            <Select value={appProxyMode} onValueChange={handleAppProxyModeChange}>
              <SelectTrigger className="h-8 text-xs ml-auto w-[180px]"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="followKiro">{t('settings.appProxyFollowKiro')}</SelectItem>
                <SelectItem value="disabled">{t('settings.appProxyDisabled')}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </SectionCard>
    </div>
  )
}

export default SettingsKiro
