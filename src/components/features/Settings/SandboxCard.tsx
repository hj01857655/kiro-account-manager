import { useEffect, useState } from 'react'
import { ShieldCheck } from 'lucide-react'
import { getSandboxConfig, saveSandboxConfig, type SandboxConfig } from '../../../api/settingsApi'
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

  useEffect(() => {
    getSandboxConfig()
      .then(setConfig)
      .catch(() => {})
      .finally(() => setLoaded(true))
  }, [])

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
            onChange={checked => apply({ enabled: checked })}
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
                onChange={checked => apply({ mcpSandboxing: checked })}
                label={t('settings.sandboxMcp')}
                hint={t('settings.sandboxMcpDesc')}
              />
            </>
          )}
        </>
      )}
    </SectionCard>
  )
}
