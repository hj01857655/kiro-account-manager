import { Fragment, useState, useEffect, useMemo } from 'react'
import { getCustomAgents, getHooks, getPowers, getSkills, getSteeringFiles, scanAgentsMd, listSpecs, listWorkflows } from '../../../api/agentConfigApi'
import { readCloudConfigEnabled } from '../../../api/settingsApi'
import { Server, Settings2, FileText, Puzzle, Bot, Zap, Link2, Lock, Shield, LayoutDashboard, Brain } from 'lucide-react'
import { useProjectScope } from '../../../contexts/ProjectScopeContext'
import { useApp } from '../../../hooks/useApp'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import OverviewPanel from './OverviewPanel'
import MemoriesPanel from './MemoriesPanel'
import { listMemories } from '../../../api/memoryApi'
import MCPPanel from './MCPPanel'
import PermissionsPanel from '../Settings/PermissionsPanel'
import SteeringPanel from './SteeringPanel'
import SkillsPanel from './SkillsPanel'
import HooksPanel from './HooksPanel'
import AgentsPanel from './AgentsPanel'
import PowersPanel from './PowersPanel'
import AgentsMdPanel from './AgentsMdPanel'
import SpecsPanel from './SpecsPanel'
import WorkflowsPanel from './WorkflowsPanel'
import { handleUiError } from '../../../utils/errorLogger'
import { getThemeAccent } from './themeAccent'

function AgentConfig() {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const [activeTab, setActiveTab] = useState('overview')
  const [mcpCount, setMcpCount] = useState(0)
  const [steeringCount, setSteeringCount] = useState(0)

  const [skillsCount, setSkillsCount] = useState(0)
  const [hooksCount, setHooksCount] = useState(0)
  const [agentsCount, setAgentsCount] = useState(0)
  const [powersCount, setPowersCount] = useState(0)
  const [agentsMdCount, setAgentsMdCount] = useState(0)
  const [specsCount, setSpecsCount] = useState(0)
  const [workflowsCount, setWorkflowsCount] = useState(0)
  const [memoriesCount, setMemoriesCount] = useState(0)
  // 项目作用域全局化：状态与最近列表在 ProjectScopeContext（侧边栏选择器共享），本页只消费
  const { projectDir } = useProjectScope()
  // 云配置只读：kiroAgent.experiments.cloudConfig 开启时，steering/skills/extensions(MCP)/
  // powers/hooks/workflows/agents 由 Kiro Cloud 托管，本地写入无效，本管理端应禁用写操作。
  const [cloudReadOnly, setCloudReadOnly] = useState(false)

  // 初始加载数量
  useEffect(() => {
    getSteeringFiles(projectDir || null).then(files => setSteeringCount(files?.length || 0)).catch(() => {})
    getSkills(projectDir || null).then(skills => setSkillsCount(skills?.length || 0)).catch(() => {})
    getCustomAgents(projectDir || null).then(agents => setAgentsCount(agents?.length || 0)).catch(() => {})

    // 无项目时同样加载：HooksPanel 会展示用户级 hooks（~/.kiro/hooks），计数须与之一致。
    // 与上面 steering / skills / agents 的写法保持一致。
    getHooks(projectDir || null).then(hooks => setHooksCount(hooks?.length || 0)).catch(() => setHooksCount(0))

    getPowers().then(powers => setPowersCount(powers?.length || 0)).catch(() => {})

    // 嵌套 AGENTS.md 仅在工作区内扫描；无项目目录时不调用（后端要求 project_dir 非空），
    // 与 AgentsMdPanel 的守卫一致，避免把一次无效 RPC 算进计数。
    if (projectDir) {
      scanAgentsMd(projectDir).then(files => setAgentsMdCount(files?.length || 0)).catch(() => setAgentsMdCount(0))
    } else {
      setAgentsMdCount(0)
    }

    // Specs：用户级与项目级都会预载计数，便于 Tab 徽标显示数量（无需先进入面板）。
    const specScope = projectDir ? 'project' : 'user'
    listSpecs(specScope, projectDir).then(s => setSpecsCount(s?.length || 0)).catch(() => setSpecsCount(0))

    // Workflows：同上，预载计数。
    const wfScope = projectDir ? 'project' : 'user'
    listWorkflows(wfScope, projectDir).then(w => setWorkflowsCount(w?.length || 0)).catch(() => setWorkflowsCount(0))

    // Memories：memories.db 不存在（Memory 从未启用）按 0 条计。
    listMemories().then(m => setMemoriesCount(m?.length || 0)).catch(() => setMemoriesCount(0))
  }, [projectDir])

  // 进入时读取云配置开关（纯读取，不写回 settings.json）。
  useEffect(() => {
    readCloudConfigEnabled()
      .then(setCloudReadOnly)
      .catch(() => setCloudReadOnly(false))
  }, [])


  const TABS = [
    { id: 'overview', label: t('agentConfig.overview'), icon: LayoutDashboard, count: 0 },
    { id: 'mcp', label: t('agentConfig.mcp'), icon: Server, count: mcpCount, group: 'capabilities' },
    { id: 'powers', label: t('agentConfig.powers'), icon: Zap, count: powersCount, group: 'capabilities' },
    { id: 'agents', label: t('agentConfig.agents'), icon: Bot, count: agentsCount, group: 'capabilities' },
    { id: 'skills', label: t('agentConfig.skills'), icon: Puzzle, count: skillsCount, group: 'capabilities' },
    { id: 'memories', label: t('agentConfig.memories'), icon: Brain, count: memoriesCount, group: 'capabilities' },
    { id: 'hooks', label: t('agentConfig.hooks'), icon: Link2, count: hooksCount, group: 'behavior' },
    { id: 'steering', label: t('agentConfig.steering'), icon: FileText, count: steeringCount, group: 'behavior' },
    { id: 'agentsMd', label: t('agentConfig.agentsMd'), icon: FileText, count: agentsMdCount, group: 'behavior' },
    { id: 'specs', label: t('agentConfig.specs'), icon: FileText, count: specsCount, group: 'specs' },
    { id: 'workflows', label: t('agentConfig.workflows'), icon: FileText, count: workflowsCount, group: 'specs' },
    // 权限规则（permissions.yaml，独立于 settings.json / mcp.json），从设置页迁入
    { id: 'permissions', label: t('settings.permissions'), icon: Shield, count: 0, group: 'boundaries' },
  ]

  // 组名复用总览页的分组键（groupCapabilities 等已有三语）；boundaries 为本页新增
  const GROUP_LABELS: Record<string, string> = {
    capabilities: t('agentConfig.groupCapabilities'),
    behavior: t('agentConfig.groupBehavior'),
    specs: t('agentConfig.groupSpecs'),
    boundaries: t('agentConfig.groupBoundaries'),
  }

  return (
    <div className="h-full flex flex-col max-w-full overflow-x-hidden glass-main">
      {/* Header（紧凑）*/}
      <div className="px-5 py-3 border-b border-border flex items-center gap-3">
        <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20 flex-shrink-0`}>
          <Settings2 size={20} className="text-primary-foreground" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <h1 className="text-lg font-semibold text-foreground leading-tight">{t('agentConfig.title')}</h1>
          <p className="text-sm text-muted-foreground leading-tight truncate">{t('agentConfig.subtitle')}</p>
        </div>

      </div>

      {/* 云配置只读横幅：开启 experiments.cloudConfig 时，云端托管的配置本地写入无效 */}
      {cloudReadOnly && (
        <div className="mx-5 mb-1 px-3 py-2.5 rounded-lg border border-amber-500/30 bg-amber-500/10 flex items-start gap-2.5">
          <Lock size={16} className="mt-0.5 text-amber-500 shrink-0" />
          <div className="flex-1 text-xs leading-relaxed">
            <div className="font-semibold text-foreground mb-0.5">{t('agentConfig.cloudReadOnly')}</div>
            <div className="text-muted-foreground">{t('agentConfig.cloudReadOnlyDesc')}</div>
          </div>
        </div>
      )}

      {/* Tabs + Content */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <div className="px-5 pt-3 pb-2">
          <TabsList className="glass-card flex h-10 w-full justify-start overflow-x-auto rounded-lg border-none p-0.5 no-scrollbar lg:w-fit">
            {TABS.map((tab, i) => {
              const Icon = tab.icon
              const prev = i > 0 ? TABS[i - 1] : null
              // 组间分隔：组名小字 + 竖线，组名复用总览页分组键
              const showGroupSep = !!(prev && prev.group && tab.group && prev.group !== tab.group)
              return (
                <Fragment key={tab.id}>
                  {showGroupSep && (
                    <div className="mx-1 flex items-center gap-1.5 self-center shrink-0">
                      <span className="text-[10px] text-muted-foreground/70">{GROUP_LABELS[tab.group]}</span>
                      <span className="w-px h-5 bg-border" />
                    </div>
                  )}
                  <TabsTrigger
                    value={tab.id}
                    className="gap-1.5 px-3 h-9 shrink-0 text-sm font-medium data-[state=active]:shadow-sm"
                  >
                    <Icon size={14} />
                    {tab.label}
                    {tab.count > 0 && (
                      <span className={`text-[10px] px-1.5 py-0.5 rounded-full ${
                        activeTab === tab.id
                          ? 'bg-primary/20 text-primary'
                          : 'bg-muted/50 text-muted-foreground'
                      }`}>
                        {tab.count}
                      </span>
                    )}
                  </TabsTrigger>
                </Fragment>
              )
            })}
          </TabsList>
        </div>

        <div className="flex-1 min-h-0 overflow-hidden">
          <TabsContent value="overview" className="h-full m-0">
            <OverviewPanel
              counts={{
                mcp: mcpCount,
                steering: steeringCount,
                skills: skillsCount,
                hooks: hooksCount,
                agents: agentsCount,
                powers: powersCount,
                agentsMd: agentsMdCount,
                specs: specsCount,
                workflows: workflowsCount,
              }}
              projectDir={projectDir}
              cloudReadOnly={cloudReadOnly}
              accent={accent}
              onNavigate={setActiveTab}
              t={t}
            />
          </TabsContent>
          <TabsContent value="mcp" className="h-full m-0">
            {/* MCP 配置（settings.json 键）收进工具栏「MCP 配置」弹窗，列表独占 tab */}
            <MCPPanel onCountChange={setMcpCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="steering" className="h-full m-0">
            <SteeringPanel onCountChange={setSteeringCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="agentsMd" className="h-full m-0">
            <AgentsMdPanel onCountChange={setAgentsMdCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="specs" className="h-full m-0">
            <SpecsPanel onCountChange={setSpecsCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="workflows" className="h-full m-0">
            <WorkflowsPanel onCountChange={setWorkflowsCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="skills" className="h-full m-0">
            <SkillsPanel onCountChange={setSkillsCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="memories" className="h-full m-0 overflow-y-auto pr-1">
            <MemoriesPanel t={t} />
          </TabsContent>
          <TabsContent value="hooks" className="h-full m-0">
            {/* 不再要求先选项目：HooksPanel 支持无项目时只展示用户级 hooks（~/.kiro/hooks），
                项目级操作在面板内部按 projectDir 是否为空自行禁用。与同级面板保持一致。 */}
            <HooksPanel onCountChange={setHooksCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="agents" className="h-full m-0">
            <AgentsPanel onCountChange={setAgentsCount} projectDir={projectDir} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="powers" className="h-full m-0">
            <PowersPanel onCountChange={setPowersCount} readOnly={cloudReadOnly} />
          </TabsContent>
          <TabsContent value="permissions" className="h-full m-0 overflow-y-auto pr-1">
            <PermissionsPanel t={t} />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  )
}

export default AgentConfig
