import { useEffect, useState } from 'react'
import {
  Server, Shield, Puzzle, Zap, Bot, Link2, FileText, Workflow,
  ChevronRight, type LucideIcon,
} from 'lucide-react'
import { getMcpToolStats, getMcpConfig } from '../../../api/mcpApi'
import { getPermissions } from '../../../api/settingsApi'

// Agent 配置总览：上半是配置体检（状态灯 + 洞察 + 跳转），下半是资产计数卡。
// 体检数据并行拉取（MCP 工具量 / 服务状态 / 权限放行口子），云托管状态由
// AgentConfig 读取后下传；计数与各 tab 徽标同源（AgentConfig 统一加载下传）。
// 权限规则由 PermissionsPanel 自管理，无计数，卡片只做入口。

interface OverviewProps {
  counts: {
    mcp: number
    steering: number
    skills: number
    hooks: number
    agents: number
    powers: number
    agentsMd: number
    specs: number
    workflows: number
  }
  projectDir: string | null
  cloudReadOnly: boolean
  accent: any
  onNavigate: (tab: string) => void
  t: (key: string) => string
}

const tr = (tpl: string, n: number) => tpl.replace('{n}', String(n))

function CheckRow({
  level, title, detail, onClick,
}: {
  level: 'ok' | 'warn'
  title: string
  detail: string
  onClick: () => void
}) {
  return (
    <button
      onClick={onClick}
      className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border hover:bg-muted/50 transition-colors cursor-pointer text-left focus:outline-none focus:ring-2 focus:ring-primary/30"
    >
      <span className={`w-2 h-2 rounded-full shrink-0 ${level === 'warn' ? 'bg-amber-500' : 'bg-emerald-500'}`} />
      <span className="text-sm font-medium text-foreground shrink-0 w-24">{title}</span>
      <span className={`text-xs flex-1 min-w-0 ${level === 'warn' ? 'text-amber-600 dark:text-amber-400' : 'text-muted-foreground'}`}>
        {detail}
      </span>
      <ChevronRight size={14} className="text-muted-foreground shrink-0" />
    </button>
  )
}

function StatCard({
  icon: Icon, label, count, onClick, accent, t,
}: {
  icon: LucideIcon
  label: string
  count?: number
  onClick: () => void
  accent: any
  t: (key: string) => string
}) {
  return (
    <button
      onClick={onClick}
      className={`glass-card border border-border rounded-xl p-4 flex items-center gap-3 text-left cursor-pointer hover:bg-muted/50 transition-colors focus:outline-none focus:ring-2 ${accent.ring}`}
    >
      <div className={`w-9 h-9 rounded-lg bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shrink-0`}>
        <Icon size={16} className="text-primary-foreground" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-foreground truncate">{label}</div>
        <div className="text-xs text-muted-foreground">
          {count !== undefined ? count : t('agentConfig.manage')}
        </div>
      </div>
      <ChevronRight size={14} className="text-muted-foreground shrink-0" />
    </button>
  )
}

function Group({
  title, children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">{title}</h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">{children}</div>
    </div>
  )
}

export default function OverviewPanel({
  counts, projectDir, cloudReadOnly, accent, onNavigate, t,
}: OverviewProps) {
  const [toolCount, setToolCount] = useState(0)
  const [serverTotal, setServerTotal] = useState(0)
  const [serverDisabled, setServerDisabled] = useState(0)
  const [openRules, setOpenRules] = useState(0)
  const [rulesLoaded, setRulesLoaded] = useState(false)

  useEffect(() => {
    let cancelled = false
    Promise.all([
      getMcpToolStats(projectDir || null).catch(() => null),
      getMcpConfig(projectDir || null).catch(() => null),
      getPermissions<any>().catch(() => null),
    ]).then(([toolStats, mcpCfg, policy]) => {
      if (cancelled) return
      const servers: any[] = mcpCfg?.mcpServers ? Object.values(mcpCfg.mcpServers) : []
      const disabled = servers.filter((s: any) => s?.disabled).length
      const rules: any[] = policy?.rules || []
      // effect=allow 且 match 为空 = 该能力完全放行（无范围限制）
      const open = rules.filter((r: any) => r?.effect === 'allow' && (!r?.match || r.match.length === 0)).length
      setToolCount(toolStats?.estimatedTools ?? 0)
      setServerTotal(servers.length)
      setServerDisabled(disabled)
      setOpenRules(open)
      setRulesLoaded(!!policy)
    })
    return () => {
      cancelled = true
    }
  }, [projectDir])

  const checks: { level: 'ok' | 'warn'; title: string; detail: string; tab: string }[] = [
    {
      level: toolCount > 50 ? 'warn' : 'ok',
      title: t('agentConfig.checkMcpTools'),
      detail: tr(t(toolCount > 50 ? 'agentConfig.checkMcpToolsHigh' : 'agentConfig.checkMcpToolsOk'), toolCount),
      tab: 'mcp',
    },
    {
      level: serverTotal === 0 ? 'warn' : 'ok',
      title: t('agentConfig.checkMcpServers'),
      detail: serverTotal === 0
        ? t('agentConfig.checkMcpNone')
        : tr(t('agentConfig.checkMcpOk'), serverTotal) + (serverDisabled > 0 ? tr(t('agentConfig.checkMcpDisabled'), serverDisabled) : ''),
      tab: 'mcp',
    },
    ...(rulesLoaded ? [{
      level: openRules > 0 ? ('warn' as const) : ('ok' as const),
      title: t('agentConfig.checkPerms'),
      detail: openRules > 0
        ? tr(t('agentConfig.checkPermsOpen'), openRules)
        : t('agentConfig.checkPermsOk'),
      tab: 'permissions',
    }] : []),
    {
      level: cloudReadOnly ? 'warn' : 'ok',
      title: t('agentConfig.checkCloud'),
      detail: cloudReadOnly ? t('agentConfig.checkCloudOn') : t('agentConfig.checkCloudOff'),
      tab: 'mcp',
    },
  ]

  return (
    <div className="h-full overflow-y-auto p-5 space-y-5">
      {/* 配置体检 */}
      <div>
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">{t('agentConfig.checkTitle')}</h3>
        <div className="space-y-2">
          {checks.map(c => (
            <CheckRow key={c.title} level={c.level} title={c.title} detail={c.detail} onClick={() => onNavigate(c.tab)} />
          ))}
        </div>
      </div>

      <Group title={t('agentConfig.groupCapabilities')}>
        <StatCard icon={Server} label={t('agentConfig.mcp')} count={counts.mcp} accent={accent} t={t} onClick={() => onNavigate('mcp')} />
        <StatCard icon={Shield} label={t('settings.permissions')} accent={accent} t={t} onClick={() => onNavigate('permissions')} />
        <StatCard icon={Puzzle} label={t('agentConfig.skills')} count={counts.skills} accent={accent} t={t} onClick={() => onNavigate('skills')} />
        <StatCard icon={Zap} label={t('agentConfig.powers')} count={counts.powers} accent={accent} t={t} onClick={() => onNavigate('powers')} />
        <StatCard icon={Bot} label={t('agentConfig.agents')} count={counts.agents} accent={accent} t={t} onClick={() => onNavigate('agents')} />
      </Group>

      <Group title={t('agentConfig.groupBehavior')}>
        <StatCard icon={Link2} label={t('agentConfig.hooks')} count={counts.hooks} accent={accent} t={t} onClick={() => onNavigate('hooks')} />
        <StatCard icon={FileText} label={t('agentConfig.steering')} count={counts.steering} accent={accent} t={t} onClick={() => onNavigate('steering')} />
        <StatCard icon={FileText} label={t('agentConfig.agentsMd')} count={counts.agentsMd} accent={accent} t={t} onClick={() => onNavigate('agentsMd')} />
      </Group>

      <Group title={t('agentConfig.groupSpecs')}>
        <StatCard icon={FileText} label={t('agentConfig.specs')} count={counts.specs} accent={accent} t={t} onClick={() => onNavigate('specs')} />
        <StatCard icon={Workflow} label={t('agentConfig.workflows')} count={counts.workflows} accent={accent} t={t} onClick={() => onNavigate('workflows')} />
      </Group>
    </div>
  )
}
