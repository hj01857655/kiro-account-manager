import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Wrench } from 'lucide-react'
import { Textarea } from '../../ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'
import { getKiroSettings, setKiroAgentSetting } from '../../../api/settingsApi'
import { useDialog } from '../../../contexts/DialogContext'
import SectionCard from '../Settings/SectionCard'

// MCP 相关的两项 settings.json 键：配置 MCP 开关（set_kiro_configure_mcp，后端双向同步）
// 与 kiroAgent.mcpApprovedEnvVars。从设置页迁到 Kiro 配置页的 MCP tab，与 MCPPanel
// （mcp.json 管理）同屏——MCP 的开关、环境变量白名单、服务列表都属于同一主题。
// 自取自写，不依赖设置页的状态透传。

const toLines = (v: string): string[] =>
  v.split('\n').map(s => s.trim()).filter(Boolean)

export default function MCPSettingsCard({ t }: { t: (key: string) => string }) {
  const { showError } = useDialog()
  const [configureMcp, setConfigureMcp] = useState('Enabled')
  const [envVars, setEnvVars] = useState('')

  useEffect(() => {
    let cancelled = false
    getKiroSettings<any>()
      .then(v => {
        if (cancelled || !v) return
        setConfigureMcp(v.configureMcp ?? 'Enabled')
        setEnvVars((v.mcpApprovedEnvVars ?? []).join('\n'))
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  const handleConfigureMcpChange = async (mode: string) => {
    setConfigureMcp(mode)
    try {
      await invoke('set_kiro_configure_mcp', { mode })
    } catch (err) {
      await showError(t('settings.saveFailed'), `${t('settings.saveFailed')}: ${err}`)
    }
  }

  const applyEnvVars = async () => {
    try {
      await setKiroAgentSetting('kiroAgent.mcpApprovedEnvVars', toLines(envVars))
    } catch (err) {
      await showError(t('settings.saveFailed'), `${t('settings.saveFailed')}: ${err}`)
    }
  }

  return (
    <SectionCard
      title={t('settings.configureMCP')}
      accent="amber"
      icon={<Wrench size={14} className="text-amber-500" />}
      desc={t('settings.configureMCPDesc')}
    >
      <Select value={configureMcp} onValueChange={handleConfigureMcpChange}>
        <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem value="Enabled">{t('settings.configureMCPEnabled')}</SelectItem>
          <SelectItem value="Disabled">{t('settings.configureMCPDisabled')}</SelectItem>
        </SelectContent>
      </Select>

      <div>
        <Label className="block text-[11px] text-muted-foreground mb-1">
          {t('settings.agentMcpApprovedEnvVars')}
        </Label>
        <Textarea
          value={envVars}
          onChange={e => setEnvVars(e.target.value)}
          onBlur={applyEnvVars}
          placeholder="GITHUB_TOKEN"
          className="font-mono text-xs"
          rows={2}
        />
        <p className="text-[11px] text-muted-foreground mt-1">{t('settings.agentMcpApprovedEnvVarsDesc')}</p>
      </div>
    </SectionCard>
  )
}
