import { Clock, Globe, Search, Shield, Shuffle, AlertTriangle, Eye, EyeOff, Repeat, RefreshCw, Check, Copy, Cpu, X, FolderOpen, ExternalLink, Users, User, Languages } from 'lucide-react'
import { Input } from '../../ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import React from 'react'
import { TFunction } from 'i18next'
import SectionCard from './SectionCard'
import SwitchRow from './SwitchRow'
import { useI18n } from '../../../hooks/useI18n'

interface BrowserInfo {
  name: string;
  path: string;
  incognitoArg?: string;
}

interface SystemMachineInfo {
  machineGuid: string;
  osType: string;
  requiresAdmin: boolean;
  canModify: boolean;
}

interface SettingsGeneralProps {
  autoRefresh: boolean;
  autoRefreshInterval: number;
  autoChangeMachineId: boolean;
  machineIdMode: string;
  privacyMode: boolean;
  setPrivacyMode: (checked: boolean) => void;
  autoSwitchEnabled: boolean;
  autoSwitchThreshold: number;
  autoSwitchInterval: number;
  closeToTray: boolean;
  browserPath: string;
  setBrowserPath: (path: string) => void;
  originalBrowserPath: string;
  savingBrowser: boolean;
  detectedBrowsers: BrowserInfo[];
  showBrowserList: boolean;
  setShowBrowserList: (show: boolean) => void;
  customKiroPath: string | null;
  handleBrowseKiroPath: () => void;
  handleClearKiroPath: () => void;
  systemMachineInfo: SystemMachineInfo | null;
  machineGuidAction: string | null;
  handleResetSystemMachineGuid: () => void;
  appDataDir: string;
  handleOpenAppDataDir: () => void;
  handleDetectBrowsers: () => void;
  handleApplyBrowser: () => void;
  handleAutoRefreshChange: (checked: boolean) => void;
  handleAutoRefreshIntervalChange: (value: string) => void;
  handleAutoChangeMachineIdChange: (checked: boolean) => void;
  handleMachineIdModeChange: (mode: string) => void;
  handleAutoSwitchEnabledChange: (checked: boolean) => void;
  handleAutoSwitchThresholdChange: (value: number) => void;
  handleAutoSwitchIntervalChange: (value: string) => void;
  handleCloseToTrayChange: (checked: boolean) => void;
  t: TFunction;
}

/// 紧凑开关行已抽到 ./SwitchRow

function LanguageSelector({ t }: { t: TFunction }) {
  const { locale, setLocale, supportedLanguages } = useI18n()

  return (
    <div className="flex items-center justify-between py-1">
      <div className="flex items-center gap-2">
        <Globe size={14} className="text-muted-foreground" />
        <span className="text-sm text-foreground">{t('settings.displayLanguage')}</span>
      </div>
      <Select value={locale} onValueChange={setLocale}>
        <SelectTrigger className="h-7 w-[140px] text-xs"><SelectValue /></SelectTrigger>
        <SelectContent>
          {supportedLanguages.map(lang => (
            <SelectItem key={lang.code} value={lang.code}>{lang.label}</SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

function SettingsGeneral({
  autoRefresh,
  autoRefreshInterval,
  autoChangeMachineId,
  machineIdMode,
  privacyMode,
  setPrivacyMode,
  autoSwitchEnabled,
  autoSwitchThreshold,
  autoSwitchInterval,
  closeToTray,
  browserPath,
  setBrowserPath,
  originalBrowserPath,
  savingBrowser,
  detectedBrowsers,
  showBrowserList,
  setShowBrowserList,
  customKiroPath,
  handleBrowseKiroPath,
  handleClearKiroPath,
  systemMachineInfo,
  machineGuidAction,
  handleResetSystemMachineGuid,
  appDataDir,
  handleOpenAppDataDir,
  handleDetectBrowsers,
  handleApplyBrowser,
  handleAutoRefreshChange,
  handleAutoRefreshIntervalChange,
  handleAutoChangeMachineIdChange,
  handleMachineIdModeChange,
  handleAutoSwitchEnabledChange,
  handleAutoSwitchThresholdChange,
  handleAutoSwitchIntervalChange,
  handleCloseToTrayChange,
  t,
}: SettingsGeneralProps) {
  const browserChanged = browserPath !== originalBrowserPath

  const [copiedField, setCopiedField] = React.useState<string | null>(null)
  const copiedTimerRef = React.useRef<NodeJS.Timeout | null>(null)

  React.useEffect(() => {
    return () => { if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current) }
  }, [])

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopiedField(field)
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current)
    copiedTimerRef.current = setTimeout(() => setCopiedField(null), 1500)
  }

  const handleSelectBrowser = (browser: BrowserInfo, useIncognito = true) => {
    const path = useIncognito && browser.incognitoArg
      ? `"${browser.path}" ${browser.incognitoArg}`
      : `"${browser.path}"`
    setBrowserPath(path)
    setShowBrowserList(false)
  }

  const resolveOsLabel = (osType: string, defaultLabel: string) => {
    switch (osType) {
      case 'windows': return 'Windows'
      case 'macos': return 'macOS'
      case 'linux': return 'Linux'
      default: return defaultLabel
    }
  }

  return (
    <div className="space-y-3">
      {/* 语言设置 */}
      <SectionCard
        title={t('settings.language')}
        icon={<Languages size={14} className="text-primary" />}
      >
        <LanguageSelector t={t} />
      </SectionCard>

      {/* 账号管理 */}
      <SectionCard
        title={t('settings.account')}
        icon={<Users size={14} className="text-primary" />}
      >
        <div className="space-y-2">
          {/* 自动刷新 Token */}
          <SwitchRow
            checked={autoRefresh}
            onCheckedChange={handleAutoRefreshChange}
            icon={<Clock size={14} />}
            label={t('settings.autoRefresh')}
            title={t('settings.autoRefreshDesc')}
            trailing={
              <Select value={String(autoRefreshInterval)} onValueChange={handleAutoRefreshIntervalChange} disabled={!autoRefresh}>
                <SelectTrigger className="h-7 w-[140px] text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="10">10 {t('common.minutes')}</SelectItem>
                  <SelectItem value="20">20 {t('common.minutes')}</SelectItem>
                  <SelectItem value="30">30 {t('common.minutes')}</SelectItem>
                  <SelectItem value="40">40 {t('common.minutes')}</SelectItem>
                  <SelectItem value="50">50 {t('common.minutes')} ({t('common.recommended')})</SelectItem>
                </SelectContent>
              </Select>
            }
          />

          {/* 机器码 */}
          <SwitchRow
            checked={autoChangeMachineId}
            onCheckedChange={handleAutoChangeMachineIdChange}
            icon={<Shuffle size={14} />}
            label={t('settings.autoChangeMachineId')}
            title={t('settings.autoChangeMachineIdDesc')}
            trailing={
              <Select value={machineIdMode} onValueChange={handleMachineIdModeChange} disabled={!autoChangeMachineId}>
                <SelectTrigger className="h-7 w-[160px] text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="bind">{t('settings.machineIdBind')} ({t('common.recommended')})</SelectItem>
                  <SelectItem value="random">{t('settings.machineIdRandom')}</SelectItem>
                </SelectContent>
              </Select>
            }
          />

          {/* 隐私模式 */}
          <SwitchRow
            checked={privacyMode}
            onCheckedChange={setPrivacyMode}
            icon={privacyMode ? <EyeOff size={14} /> : <Eye size={14} />}
            label={t('settings.privacyMode')}
            hint={`(${t('settings.privacyModeHint')})`}
            title={t('settings.privacyModeDesc')}
          />

          {/* 自动换号 */}
          <SwitchRow
            checked={autoSwitchEnabled}
            onCheckedChange={handleAutoSwitchEnabledChange}
            icon={<Repeat size={14} />}
            label={t('settings.autoSwitch')}
            title={t('settings.autoSwitchDesc')}
          />
          {autoSwitchEnabled && (
            <div className="flex items-center gap-2 pl-9 pr-3">
              <span className="text-xs text-muted-foreground whitespace-nowrap">{t('settings.autoSwitchThreshold')}</span>
              <Input
                type="number"
                value={autoSwitchThreshold}
                onChange={(e) => handleAutoSwitchThresholdChange(parseFloat(e.target.value) || 0)}
                min={0}
                step={0.1}
                className="h-7 w-20 text-center text-xs"
              />
              <Select value={String(autoSwitchInterval)} onValueChange={handleAutoSwitchIntervalChange}>
                <SelectTrigger className="h-7 flex-1 text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="1">1 {t('common.minutes')}</SelectItem>
                  <SelectItem value="3">3 {t('common.minutes')}</SelectItem>
                  <SelectItem value="5">5 {t('common.minutes')}</SelectItem>
                  <SelectItem value="10">10 {t('common.minutes')}</SelectItem>
                  <SelectItem value="15">15 {t('common.minutes')}</SelectItem>
                  <SelectItem value="30">30 {t('common.minutes')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          {/* 关闭到托盘（合并进账号管理）*/}
          <SwitchRow
            checked={closeToTray}
            onCheckedChange={handleCloseToTrayChange}
            label={t('settings.minimizeToTray')}
            hint={t('settings.minimizeToTrayHint')}
          />
        </div>
      </SectionCard>

      {/* 浏览器 + Kiro IDE 路径（双栏并列）*/}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <SectionCard
          title={t('settings.browser')}
          accent="green"
          icon={<Globe size={14} className="text-emerald-500" />}
        >
          <div className="flex gap-1.5">
            <Input
              value={browserPath}
              onChange={(e) => setBrowserPath(e.target.value)}
              placeholder={t('settings.browserPlaceholder')}
              className="h-8 text-xs flex-1"
            />
            <button
              onClick={handleDetectBrowsers}
              className="px-2.5 h-8 border rounded-md bg-card hover:bg-muted/50 border-border text-foreground transition-colors"
              title={t('settings.detectBrowsersTitle')}
            >
              <Search size={13} />
            </button>
            <button
              onClick={handleApplyBrowser}
              disabled={savingBrowser || !browserChanged}
              className={`px-3 h-8 rounded-md flex items-center gap-1 text-xs font-medium border transition-colors disabled:opacity-50 ${
                browserChanged
                  ? 'bg-primary text-primary-foreground border-primary hover:bg-primary/90'
                  : 'bg-muted text-muted-foreground border-border'
              }`}
            >
              {savingBrowser ? <RefreshCw size={12} className="animate-spin" /> : <Check size={12} />}
              <span className="hidden sm:inline">{savingBrowser ? t('settings.saving') : t('settings.apply')}</span>
            </button>
          </div>

          {showBrowserList && detectedBrowsers.length > 0 && (
            <div className="rounded-md border border-border bg-muted/30 p-2 mt-2 space-y-1">
              <div className="flex items-center justify-between mb-1 px-1">
                <span className="text-xs font-medium">{t('settings.detectedBrowsers')}</span>
                <button onClick={() => setShowBrowserList(false)} className="text-[10px] text-muted-foreground hover:text-foreground">
                  {t('settings.close')}
                </button>
              </div>
              {detectedBrowsers.map((browser, index) => (
                <div key={index} className="flex items-center justify-between p-1.5 rounded bg-card border border-border hover:bg-muted/50 transition-colors">
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium">{browser.name}</div>
                    <div className="text-[10px] text-muted-foreground truncate">{browser.path}</div>
                  </div>
                  <button onClick={() => handleSelectBrowser(browser, true)} className="ml-2 px-2 py-1 text-[10px] rounded bg-primary text-primary-foreground hover:bg-primary/90 transition-colors">
                    {t('settings.selectBrowser')}
                  </button>
                </div>
              ))}
            </div>
          )}

          <p className="text-[11px] text-muted-foreground">{t('settings.browserTip')}</p>
        </SectionCard>

        <SectionCard
          title={t('settings.kiroIdePath')}
          accent="violet"
          icon={<Cpu size={14} className="text-violet-500" />}
        >
          <div className="flex gap-1.5">
            <Input
              value={customKiroPath || ''}
              placeholder={t('settings.useDefaultPath')}
              readOnly
              className="h-8 text-xs bg-muted/50 cursor-not-allowed flex-1"
            />
            <button
              onClick={handleBrowseKiroPath}
              className="px-2.5 h-8 border rounded-md bg-card hover:bg-muted/50 border-border text-foreground transition-colors"
              title={t('settings.browse')}
            >
              <Search size={13} />
            </button>
            {customKiroPath && (
              <button
                onClick={handleClearKiroPath}
                className="px-2.5 h-8 border rounded-md bg-card hover:bg-red-500/10 border-border text-red-500 transition-colors"
                title={t('settings.clear')}
              >
                <X size={13} />
              </button>
            )}
          </div>
          <p className="text-[11px] text-muted-foreground">{t('settings.kiroPathTip')}</p>
        </SectionCard>
      </div>

      {/* App data directory and system machine GUID */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <SectionCard
          title={t('settings.appDataDir')}
          accent="blue"
          icon={<FolderOpen size={14} className="text-blue-500" />}
        >
          <div className="flex items-center gap-2">
            <code className="flex-1 text-xs px-2 py-1.5 rounded font-mono text-foreground border border-border bg-muted/30 break-all">
              {appDataDir || t('common.loading')}
            </code>
            {appDataDir && (
              <button
                onClick={() => copyToClipboard(appDataDir, 'appDataDir')}
                className="p-1.5 rounded border border-border hover:bg-muted/50 transition-colors flex-shrink-0"
                title={t('settings.copyPath')}
              >
                {copiedField === 'appDataDir' ? <Check size={13} className="text-green-500" /> : <Copy size={13} className="text-muted-foreground" />}
              </button>
            )}
            <button
              onClick={handleOpenAppDataDir}
              disabled={!appDataDir}
              className="h-8 w-8 rounded-md flex items-center justify-center bg-primary hover:bg-primary/90 text-primary-foreground disabled:opacity-50 transition-colors flex-shrink-0"
              title={t('settings.openInExplorer')}
            >
              <ExternalLink size={13} />
            </button>
          </div>
        </SectionCard>

        {/* System machine GUID */}
        <SectionCard
          title={t('settings.systemMachineGuid')}
          accent="orange"
          icon={<Shield size={14} className="text-orange-500" />}
          badge={
            systemMachineInfo?.osType ? (
              <span className="text-[10px] px-1.5 py-0.5 rounded text-muted-foreground border border-border bg-muted/30">
                {resolveOsLabel(systemMachineInfo.osType, t('common.unknown'))}
              </span>
            ) : null
          }
        >
          <div className="flex items-center gap-2">
            <code className="flex-1 text-xs px-2 py-1.5 rounded font-mono text-foreground border border-border bg-muted/30 break-all">
              {systemMachineInfo?.machineGuid || t('common.loading')}
            </code>
            {systemMachineInfo?.machineGuid && (
              <button
                onClick={() => copyToClipboard(systemMachineInfo.machineGuid, 'sysMachineGuid')}
                className="p-1.5 rounded border border-border hover:bg-muted/50 transition-colors flex-shrink-0"
                title={t('settings.copy')}
              >
                {copiedField === 'sysMachineGuid' ? <Check size={13} className="text-green-500" /> : <Copy size={13} className="text-muted-foreground" />}
              </button>
            )}
            {systemMachineInfo?.canModify && (
              <button
                onClick={handleResetSystemMachineGuid}
                disabled={machineGuidAction !== null}
                className="h-8 px-3 rounded-md flex items-center gap-1.5 text-xs font-medium bg-red-500 hover:bg-red-600 text-white disabled:opacity-50 transition-colors flex-shrink-0"
              >
                {machineGuidAction === 'reset' ? <RefreshCw size={12} className="animate-spin" /> : <Shuffle size={12} />}
                {t('common.reset')}
              </button>
            )}
          </div>

          {systemMachineInfo?.requiresAdmin && (
            <details className="text-xs">
              <summary className="cursor-pointer text-orange-500 hover:underline flex items-center gap-1.5 select-none">
                <AlertTriangle size={12} />
                {t('settings.adminWarningTitle')}
              </summary>
              <ul className="list-disc list-inside space-y-0.5 mt-1.5 ml-4 text-[11px] text-muted-foreground">
                <li>{t('settings.adminWarning1')}</li>
                <li>{t('settings.adminWarning2')}</li>
                <li>{t('settings.adminWarning3')}</li>
              </ul>
            </details>
          )}
        </SectionCard>
      </div>
    </div>
  )
}

export default SettingsGeneral
