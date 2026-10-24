import { useEffect, useState } from 'react'
import { ShieldCheck } from 'lucide-react'
import {
  getSandboxConfig,
  saveSandboxConfig,
  getEffectiveSandbox,
  saveProjectSandboxConfig,
  type SandboxConfig,
  type EffectiveSandbox,
} from '../../../api/settingsApi'
import { useDialog } from '../../../contexts/DialogContext'
import SectionCard from './SectionCard'
import SwitchRow from './SwitchRow'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'

// 命令沙箱（Kiro IDE 1.3+）：读写独立的 ~/.kiro/settings/sandbox.json，
// 不是 settings.json 键。三档出网模式 + stdio MCP 沙箱。
// 注意：全局激活时项目级只能收紧，项目级 sandbox.json 不在此面板管理范围。

const NETWORK_MODES = ['DEFAULT_BLOCKED', 'COMMON_DEPENDENCIES', 'DEFAULT_ALLOWED'] as const

export default function SandboxCard({ t }: { t: (key: string) => string }) {
  const { showError } = useDialog()
  const [config, setConfig] = useState<SandboxConfig>({
    enabled: false,
    networkAccess: 'DEFAULT_BLOCKED',
    mcpSandboxing: false,
  })
  const [loaded, setLoaded] = useState(false)
  // 项目级配置：路径由用户填写（设置页没有当前工作区概念），
  // 填了才显示两级合并结果与冲突提示。
  const [projectRoot, setProjectRoot] = useState('')
  const [projectCfg, setProjectCfg] = useState<SandboxConfig | null>(null)
  const [effective, setEffective] = useState<EffectiveSandbox | null>(null)

  useEffect(() => {
    getSandboxConfig()
      .then(setConfig)
      .catch(() => {})
      .finally(() => setLoaded(true))
  }, [])

  // 项目路径变化时重算两级合并（项目只能收紧）
  useEffect(() => {
    if (!projectRoot.trim()) {
      setEffective(null)
      setProjectCfg(null)
      return
    }
    getEffectiveSandbox(projectRoot.trim())
      .then(r => {
        setEffective(r)
        setProjectCfg(r.project ?? null)
      })
      .catch(() => {
        setEffective(null)
        setProjectCfg(null)
      })
  }, [projectRoot, config])

  const applyProject = async (patch: Partial<SandboxConfig>) => {
    if (!projectRoot.trim()) return
    const base = projectCfg ?? { enabled: config.enabled, networkAccess: config.networkAccess, mcpSandboxing: false }
    const next = { ...base, ...patch }
    setProjectCfg(next)
    try {
      await saveProjectSandboxConfig(projectRoot.trim(), next)
      const r = await getEffectiveSandbox(projectRoot.trim())
      setEffective(r)
      setProjectCfg(r.project ?? null)
    } catch (err: any) {
      showError(t('settings.saveFailed'), String(err?.message || err))
    }
  }

  const apply = async (patch: Partial<SandboxConfig>) => {
    const next = { ...config, ...patch }
    setConfig(next)
    try {
      await saveSandboxConfig(next)
    } catch (err: any) {
      showError(t('settings.saveFailed'), String(err?.message || err))
    }
  }

  return (
    <SectionCard
      title={t('settings.sandboxTitle')}
      accent="red"
      icon={<ShieldCheck size={14} className="text-red-500" />}
      desc={t('settings.sandboxDesc')}
    >
      {loaded && (
        <>
          <SwitchRow
            checked={config.enabled}
            onCheckedChange={checked => apply({ enabled: checked })}
            label={t('settings.sandboxEnabled')}
            hint={t('settings.sandboxEnabledDesc')}
          />

          {config.enabled && (
            <>
              <div>
                <Label className="block text-[11px] text-muted-foreground mb-1">{t('settings.sandboxNetwork')}</Label>
                <Select
                  value={config.networkAccess}
                  onValueChange={v => apply({ networkAccess: v })}
                >
                  <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {NETWORK_MODES.map(m => (
                      <SelectItem key={m} value={m}>{t(`settings.sandboxNetwork_${m}`)}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-[11px] text-muted-foreground mt-1">{t('settings.sandboxWindowsHint')}</p>
              </div>

              <SwitchRow
                checked={config.mcpSandboxing}
                onCheckedChange={checked => apply({ mcpSandboxing: checked })}
                label={t('settings.sandboxMcp')}
                hint={t('settings.sandboxMcpDesc')}
              />

              {/* 项目级配置：项目只能收紧，不能放宽 */}
              <div className="pt-2 mt-1 border-t border-border/60">
                <Label className="block text-[11px] text-muted-foreground mb-1">
                  {t('settings.sandboxProjectRoot')}
                </Label>
                <input
                  value={projectRoot}
                  onChange={e => setProjectRoot(e.target.value)}
                  placeholder="C:\\path\\to\\project"
                  className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground font-mono"
                />
                <p className="text-[11px] text-muted-foreground mt-1">
                  {t('settings.sandboxProjectHint')}
                </p>

                {projectRoot.trim() && (
                  <div className="mt-2 space-y-2">
                    <SwitchRow
                      checked={projectCfg?.enabled ?? false}
                      onCheckedChange={checked => applyProject({ enabled: checked })}
                      label={t('settings.sandboxProjectEnabled')}
                      hint={t('settings.sandboxProjectEnabledDesc')}
                    />
                    {projectCfg?.enabled && (
                      <>
                        <div>
                          <Label className="block text-[11px] text-muted-foreground mb-1">
                            {t('settings.sandboxNetwork')}
                          </Label>
                          <Select
                            value={projectCfg.networkAccess}
                            onValueChange={v => applyProject({ networkAccess: v })}
                          >
                            <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                            <SelectContent>
                              {NETWORK_MODES.map(m => (
                                <SelectItem key={m} value={m}>{t(`settings.sandboxNetwork_${m}`)}</SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                        <SwitchRow
                          checked={projectCfg.mcpSandboxing}
                          onCheckedChange={checked => applyProject({ mcpSandboxing: checked })}
                          label={t('settings.sandboxMcp')}
                          hint={t('settings.sandboxMcpDesc')}
                        />
                      </>
                    )}

                    {/* 合并结果与冲突提示（项目比全局宽松时后端会忽略并说明） */}
                    {effective && (
                      <div className="text-[11px] text-muted-foreground space-y-1 pt-1">
                        <div className="font-mono">
                          {t('settings.sandboxEffective')}: {effective.effective.networkAccess}
                          {effective.effective.mcpSandboxing ? ' + MCP' : ''}
                        </div>
                        {effective.notes.map((n, i) => (
                          <div key={i} className="text-amber-600 dark:text-amber-500">{n}</div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </>
          )}
        </>
      )}
    </SectionCard>
  )
}
