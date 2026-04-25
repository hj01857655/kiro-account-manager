import { Clock, Globe, Search, Shield, Shuffle, AlertTriangle, Eye, EyeOff, Repeat, RefreshCw, Check, Copy, Link2 } from 'lucide-react'
import { Card, CardContent } from '../../ui/card'
import { Input } from '../../ui/input'
import { Switch } from '../../ui/switch'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import React from 'react'
import { TFunction } from 'i18next'

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
  browserPath: string;
  setBrowserPath: (path: string) => void;
  originalBrowserPath: string;
  savingBrowser: boolean;
  kiroProtocolPath: string;
  setKiroProtocolPath: (path: string) => void;
  originalKiroProtocolPath: string;
  savingKiroProtocol: boolean;
  detectedBrowsers: BrowserInfo[];
  showBrowserList: boolean;
  setShowBrowserList: (show: boolean) => void;
  systemMachineInfo: SystemMachineInfo | null;
  machineGuidAction: string | null;
  handleResetSystemMachineGuid: () => void;
  handleDetectBrowsers: () => void;
  handleApplyBrowser: () => void;
  handleApplyKiroProtocolPath: () => void;
  handleResetKiroProtocolPath: () => void;
  handleAutoRefreshChange: (checked: boolean) => void;
  handleAutoRefreshIntervalChange: (value: string) => void;
  handleAutoChangeMachineIdChange: (checked: boolean) => void;
  handleMachineIdModeChange: (mode: string) => void;
  handleAutoSwitchEnabledChange: (checked: boolean) => void;
  handleAutoSwitchThresholdChange: (value: number) => void;
  handleAutoSwitchIntervalChange: (value: string) => void;
  t: TFunction;
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
  browserPath, 
  setBrowserPath, 
  originalBrowserPath, 
  savingBrowser, 
  kiroProtocolPath,
  setKiroProtocolPath,
  originalKiroProtocolPath,
  savingKiroProtocol,
  detectedBrowsers, 
  showBrowserList, 
  setShowBrowserList, 
  systemMachineInfo, 
  machineGuidAction, 
  handleResetSystemMachineGuid, 
  handleDetectBrowsers, 
  handleApplyBrowser, 
  handleApplyKiroProtocolPath,
  handleResetKiroProtocolPath,
  handleAutoRefreshChange, 
  handleAutoRefreshIntervalChange, 
  handleAutoChangeMachineIdChange, 
  handleMachineIdModeChange, 
  handleAutoSwitchEnabledChange, 
  handleAutoSwitchThresholdChange, 
  handleAutoSwitchIntervalChange, 
  t
}: SettingsGeneralProps) {
  const accountToggleContainerClass = "bg-card hover:bg-muted/50 border border-border text-foreground"
  const browserChanged = browserPath !== originalBrowserPath
  const kiroProtocolChanged = kiroProtocolPath !== originalKiroProtocolPath

  const [copiedField, setCopiedField] = React.useState<string | null>(null)
  const copiedTimerRef = React.useRef<NodeJS.Timeout | null>(null)

  React.useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopiedField(field)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
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
    <>
      {/* 账号设置 */}
      <Card className="card-glow animate-slide-in-left delay-200 mb-6">
        <CardContent className="p-6">
          <h2 className="text-lg font-semibold text-foreground mb-1">{t('settings.account')}</h2>
          <p className="text-sm text-muted-foreground mb-4">{t('settings.accountDesc')}</p>

          {/* 自动刷新 Token + 刷新间隔 */}
          <div className="flex items-center gap-3 mb-4">
            <label className={`flex items-center gap-2 cursor-pointer px-4 py-3 rounded-xl border flex-shrink-0 ${accountToggleContainerClass}`} title={t('settings.autoRefreshDesc')}>
              <Switch checked={autoRefresh} onCheckedChange={handleAutoRefreshChange} />
              <Clock size={16} />
              <span className="text-sm font-medium whitespace-nowrap">{t('settings.autoRefresh')}</span>
            </label>
            <div className="relative flex-1">
              <Select value={String(autoRefreshInterval)} onValueChange={handleAutoRefreshIntervalChange} disabled={!autoRefresh}>
                <SelectTrigger className="text-foreground bg-background border-border focus:ring-primary/20">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-card border-border">
                  <SelectItem value="10" className="text-foreground">10 {t('common.minutes')}</SelectItem>
                  <SelectItem value="20" className="text-foreground">20 {t('common.minutes')}</SelectItem>
                  <SelectItem value="30" className="text-foreground">30 {t('common.minutes')}</SelectItem>
                  <SelectItem value="40" className="text-foreground">40 {t('common.minutes')}</SelectItem>
                  <SelectItem value="50" className="text-foreground">50 {t('common.minutes')} ({t('common.recommended')})</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* 机器码设置 */}
          <div className="flex items-center gap-3 mb-4">
            <label className={`flex items-center gap-2 cursor-pointer px-4 py-3 rounded-xl border flex-shrink-0 ${accountToggleContainerClass}`} title={t('settings.autoChangeMachineIdDesc')}>
              <Switch checked={autoChangeMachineId} onCheckedChange={handleAutoChangeMachineIdChange} />
              <Shuffle size={16} />
              <span className="text-sm font-medium whitespace-nowrap">{t('settings.autoChangeMachineId')}</span>
            </label>
            <div className="relative flex-1">
              <Select value={machineIdMode} onValueChange={handleMachineIdModeChange} disabled={!autoChangeMachineId}>
                <SelectTrigger className="text-foreground bg-background border-border focus:ring-primary/20">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-card border-border">
                  <SelectItem value="bind" className="text-foreground">{t('settings.machineIdBind')} ({t('common.recommended')})</SelectItem>
                  <SelectItem value="random" className="text-foreground">{t('settings.machineIdRandom')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* 隐私模式 */}
          <label className={`flex items-center gap-2 cursor-pointer px-4 py-3 rounded-xl border mb-4 ${accountToggleContainerClass}`} title={t('settings.privacyModeDesc')}>
            <Switch checked={privacyMode} onCheckedChange={setPrivacyMode} />
            {privacyMode ? <EyeOff size={16} /> : <Eye size={16} />}
            <span className="text-sm font-medium whitespace-nowrap">{t('settings.privacyMode')}</span>
            <span className="text-xs text-muted-foreground ml-1">({t('settings.privacyModeHint')})</span>
          </label>

          {/* 自动换号 */}
          <div>
            <label className={`flex items-center gap-2 cursor-pointer px-4 py-3 rounded-xl border mb-2 ${accountToggleContainerClass}`} title={t('settings.autoSwitchDesc')}>
              <Switch checked={autoSwitchEnabled} onCheckedChange={handleAutoSwitchEnabledChange} />
              <Repeat size={16} />
              <span className="text-sm font-medium whitespace-nowrap">{t('settings.autoSwitch')}</span>
            </label>
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground whitespace-nowrap">{t('settings.autoSwitchThreshold')}</span>
              <Input
                type="number"
                value={autoSwitchThreshold}
                onChange={(e) => handleAutoSwitchThresholdChange(parseFloat(e.target.value) || 0)}
                disabled={!autoSwitchEnabled}
                min={0}
                step={0.1}
                className="text-foreground bg-background border-border text-center w-16"
              />
              <Select value={String(autoSwitchInterval)} onValueChange={handleAutoSwitchIntervalChange} disabled={!autoSwitchEnabled}>
                <SelectTrigger className="text-foreground bg-background border-border flex-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-card border-border">
                  <SelectItem value="1" className="text-foreground">1 {t('common.minutes')}</SelectItem>
                  <SelectItem value="3" className="text-foreground">3 {t('common.minutes')}</SelectItem>
                  <SelectItem value="5" className="text-foreground">5 {t('common.minutes')}</SelectItem>
                  <SelectItem value="10" className="text-foreground">10 {t('common.minutes')}</SelectItem>
                  <SelectItem value="15" className="text-foreground">15 {t('common.minutes')}</SelectItem>
                  <SelectItem value="30" className="text-foreground">30 {t('common.minutes')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 浏览器设置 */}
      <Card className="card-glow animate-slide-in-left delay-250 mb-6">
        <CardContent className="p-6">
          <div className="flex items-center gap-2 mb-1">
            <Globe size={18} className="text-primary" />
            <h2 className="text-lg font-semibold text-foreground">{t('settings.browser')}</h2>
          </div>
          <p className="text-sm text-muted-foreground mb-4">{t('settings.browserDesc')}</p>

          <div className="mb-3">
            <label className="block text-sm text-muted-foreground mb-2">{t('settings.browserPath')}</label>
            <div className="flex gap-3">
              <Input
                value={browserPath}
                onChange={(e) => setBrowserPath(e.target.value)}
                placeholder={t('settings.browserPlaceholder')}
                className="text-foreground bg-background border-border flex-1"
              />
              <button
                onClick={handleDetectBrowsers}
                className="px-4 py-3 border rounded-xl bg-card hover:bg-muted/50 border-border text-foreground flex items-center gap-2"
                title={t('settings.detectBrowsersTitle')}
              >
                <Search size={16} />
                {t('settings.detect')}
              </button>
              <button
                onClick={handleApplyBrowser}
                disabled={savingBrowser || !browserChanged}
                className={`px-5 py-3 rounded-xl flex items-center gap-2 font-medium shadow-sm disabled:opacity-50 disabled:cursor-not-allowed border ${browserChanged
                  ? "bg-primary text-primary-foreground border-primary"
                  : "bg-muted text-muted-foreground border-border"
                  }`}
              >
                {savingBrowser ? <RefreshCw size={16} className="animate-spin" /> : <Check size={16} />}
                {savingBrowser ? t('settings.saving') : t('settings.apply')}
              </button>
            </div>
          </div>

          {/* 检测到的浏览器列表 */}
          {showBrowserList && detectedBrowsers.length > 0 && (
            <div className="mt-4 p-4 rounded-xl border border-border bg-muted/30">
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-medium text-foreground">{t('settings.detectedBrowsers')}</span>
                <button onClick={() => setShowBrowserList(false)} className="text-xs text-muted-foreground hover:underline">
                  {t('settings.close')}
                </button>
              </div>
              <div className="space-y-2">
                {detectedBrowsers.map((browser, index) => (
                  <div key={index} className="flex items-center justify-between p-3 rounded-lg bg-card hover:bg-muted/50 transition-colors border border-border">
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-foreground">{browser.name}</div>
                      <div className="text-xs text-muted-foreground truncate">{browser.path}</div>
                    </div>
                    <div className="flex gap-2 ml-3">
                      <button onClick={() => handleSelectBrowser(browser, true)} className="px-3 py-1.5 text-xs rounded-lg transition-colors border bg-primary text-primary-foreground border-primary">
                        {t('settings.selectBrowser')}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          <p className="text-xs text-muted-foreground mt-3">{t('settings.browserTip')}</p>
        </CardContent>
      </Card>

      {/* Kiro 协议映射 */}
      <Card className="card-glow animate-slide-in-left delay-280 mb-6">
        <CardContent className="p-6">
          <div className="flex items-center gap-2 mb-1">
            <Link2 size={18} className="text-primary" />
            <h2 className="text-lg font-semibold text-foreground">Kiro 协议映射</h2>
          </div>
          <p className="text-sm text-muted-foreground mb-4">
            管理 <code>kiro://</code> 与 <code>kiro-account-manager://</code> 的本机执行路径。安装路径变更后可在此修复。
          </p>

          <label className="block text-sm text-muted-foreground mb-2">协议执行文件路径</label>
          <div className="flex gap-3">
            <Input
              value={kiroProtocolPath}
              onChange={(e) => setKiroProtocolPath(e.target.value)}
              placeholder="例如：C:\\Program Files\\Kiro Account Manager\\kiro-account-manager.exe"
              className="text-foreground bg-background border-border flex-1"
            />
            <button
              onClick={handleApplyKiroProtocolPath}
              disabled={savingKiroProtocol || !kiroProtocolChanged}
              className={`px-5 py-3 rounded-xl flex items-center gap-2 font-medium shadow-sm disabled:opacity-50 disabled:cursor-not-allowed border ${kiroProtocolChanged
                ? "bg-primary text-primary-foreground border-primary"
                : "bg-muted text-muted-foreground border-border"
                }`}
            >
              {savingKiroProtocol ? <RefreshCw size={16} className="animate-spin" /> : <Check size={16} />}
              {savingKiroProtocol ? '保存中' : '应用'}
            </button>
            <button
              onClick={handleResetKiroProtocolPath}
              disabled={savingKiroProtocol}
              className="px-4 py-3 border rounded-xl bg-card hover:bg-muted/50 border-border text-foreground flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <RefreshCw size={16} />
              恢复当前程序
            </button>
          </div>

          <p className="text-xs text-muted-foreground mt-3">
            仅 Windows 需要此映射。若浏览器提示“打开 Electron”或找不到应用，通常是这里被旧软件占用。
          </p>
        </CardContent>
      </Card>

      {/* 系统机器码管理 */}
      <Card className="card-glow animate-slide-in-left delay-300 mb-6">
        <CardContent className="p-6">
          <div className="flex items-center gap-2 mb-1">
            <Shield size={18} className="text-orange-500" />
            <h2 className="text-lg font-semibold text-foreground">{t('settings.systemMachineGuid')}</h2>
            {systemMachineInfo?.osType && (
              <span className="text-xs px-2 py-0.5 rounded-full text-muted-foreground border border-border bg-muted/30">
                {resolveOsLabel(systemMachineInfo.osType, t('common.unknown'))}
              </span>
            )}
          </div>
          <p className="text-sm text-muted-foreground mb-5">
            {systemMachineInfo?.osType === 'macos'
              ? t('settings.machineGuidDescMac')
              : systemMachineInfo?.osType === 'linux'
                ? t('settings.machineGuidDescLinux')
                : t('settings.machineGuidDescWin')}
          </p>

          {/* 当前值 */}
          <div className="rounded-xl p-4 mb-4 border border-border bg-muted/30">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-foreground">{t('settings.currentMachineGuid')}</span>
              <button onClick={() => {}} className="p-1.5 rounded-lg hover:bg-muted/50 transition-colors">
                <RefreshCw size={14} className="text-muted-foreground" />
              </button>
            </div>
            <div className="flex items-center gap-2">
              <code className="flex-1 text-sm px-3 py-2 rounded-lg font-mono text-foreground border border-border bg-card">
                {systemMachineInfo?.machineGuid || t('common.loading')}
              </code>
              {systemMachineInfo?.machineGuid && (
                <button onClick={() => copyToClipboard(systemMachineInfo.machineGuid, 'sysMachineGuid')} className="p-2 rounded-lg hover:bg-muted/50 transition-colors">
                  {copiedField === 'sysMachineGuid' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className="text-muted-foreground" />}
                </button>
              )}
            </div>
          </div>

          {/* 警告提示 */}
          {systemMachineInfo?.requiresAdmin && (
            <div className="flex items-start gap-3 bg-warning/10 rounded-xl p-4 mb-4 border border-warning/20">
              <AlertTriangle size={18} className="text-orange-500 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <p className="font-medium text-orange-500 mb-2 text-sm">{t('settings.adminWarningTitle')}</p>
                <ul className="list-disc list-inside space-y-1 mb-3 text-xs text-muted-foreground">
                  <li>{t('settings.adminWarning1')}</li>
                  <li>{t('settings.adminWarning2')}</li>
                  <li>{t('settings.adminWarning3')}</li>
                </ul>
              </div>
            </div>
          )}

          {/* 操作按钮 */}
          {systemMachineInfo?.canModify && (
            <button
              onClick={handleResetSystemMachineGuid}
              disabled={machineGuidAction !== null}
              className="w-full px-4 py-3 rounded-xl flex items-center justify-center gap-2 font-medium bg-red-500 hover:bg-red-600 text-white disabled:opacity-50"
            >
              {machineGuidAction === 'reset' ? <RefreshCw size={16} className="animate-spin" /> : <Shuffle size={16} />}
              {t('common.reset')}
            </button>
          )}
        </CardContent>
      </Card>
    </>
  )
}

export default SettingsGeneral
