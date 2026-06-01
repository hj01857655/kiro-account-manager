import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useApp } from '../../../hooks/useApp'
import { Server, Settings2, FileText, Puzzle, Bot, Zap, FolderOpen, Link2, X } from 'lucide-react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import MCPPanel from './MCPPanel'
import SteeringPanel from './SteeringPanel'
import SkillsPanel from './SkillsPanel'
import HooksPanel from './HooksPanel'
import AgentsPanel from './AgentsPanel'
import PowersPanel from './PowersPanel'
import { handleUiError } from '../../../utils/errorLogger'
import { getThemeAccent } from './themeAccent'

function KiroConfig() {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const [activeTab, setActiveTab] = useState('mcp')
  const [mcpCount, setMcpCount] = useState(0)
  const [steeringCount, setSteeringCount] = useState(0)

  const [skillsCount, setSkillsCount] = useState(0)
  const [hooksCount, setHooksCount] = useState(0)
  const [agentsCount, setAgentsCount] = useState(0)
  const [powersCount, setPowersCount] = useState(0)
  const [projectDir, setProjectDir] = useState<string | null>(null)

  // 初始加载数量
  useEffect(() => {
    invoke<any[]>('get_steering_files', { projectDir: projectDir || null }).then(files => setSteeringCount(files?.length || 0)).catch(() => {})
    invoke<any[]>('get_skills', { projectDir: projectDir || null }).then(skills => setSkillsCount(skills?.length || 0)).catch(() => {})
    invoke<any[]>('get_custom_agents', { projectDir: projectDir || null }).then(agents => setAgentsCount(agents?.length || 0)).catch(() => {})

    if (projectDir) {
      invoke<any[]>('get_hooks', { projectDir }).then(hooks => setHooksCount(hooks?.length || 0)).catch(() => setHooksCount(0))
    } else {
      setHooksCount(0)
    }

    invoke<any[]>('get_powers').then(powers => setPowersCount(powers?.length || 0)).catch(() => {})
  }, [projectDir])

  useEffect(() => {
    if (!projectDir && activeTab === 'hooks') {
      setActiveTab('mcp')
    }
  }, [projectDir, activeTab])


  const handleSelectProjectDir = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: t('kiroConfig.selectProjectDir') })
      if (selected) {
        setProjectDir(selected as string)
      }
    } catch (e) {
      handleUiError('select project directory failed', e, { userMessage: t('kiroConfig.selectProjectDirFailed') })
    }
  }

  const TABS = [
    { id: 'mcp', label: t('kiroConfig.mcp'), icon: Server, count: mcpCount },
    { id: 'powers', label: t('kiroConfig.powers'), icon: Zap, count: powersCount },
    { id: 'agents', label: t('kiroConfig.agents'), icon: Bot, count: agentsCount },
    { id: 'skills', label: t('kiroConfig.skills'), icon: Puzzle, count: skillsCount },
    { id: 'hooks', label: t('kiroConfig.hooks'), icon: Link2, count: hooksCount, disabled: !projectDir },
    { id: 'steering', label: t('kiroConfig.steering'), icon: FileText, count: steeringCount },
  ]

  return (
    <div className="h-full flex flex-col max-w-full overflow-x-hidden glass-main">
      {/* Header（紧凑）*/}
      <div className="px-5 py-3 border-b border-border flex items-center gap-3">
        <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20 flex-shrink-0`}>
          <Settings2 size={20} className="text-primary-foreground" />
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <h1 className="text-lg font-semibold text-foreground leading-tight">{t('kiroConfig.title')}</h1>
          <p className="text-sm text-muted-foreground leading-tight truncate">{t('kiroConfig.subtitle')}</p>
        </div>

        {/* 项目目录选择器 */}
        <div className="flex items-center gap-1.5 flex-shrink-0">
          <button
            onClick={handleSelectProjectDir}
            className={`flex items-center gap-1.5 px-2.5 h-8 rounded-md text-xs transition-colors cursor-pointer hover:bg-muted/50 border border-border text-foreground focus:outline-none focus:ring-2 ${accent.ring}`}
            title={t('kiroConfig.selectProjectDir')}
          >
            <FolderOpen size={13} className="text-amber-500" />
            {projectDir ? (
              <span className="max-w-[160px] truncate">{projectDir.split(/[/\\]/).pop()}</span>
            ) : (
              <span className="text-muted-foreground">{t('kiroConfig.noProjectDir')}</span>
            )}
          </button>
          {projectDir && (
            <button
              onClick={() => setProjectDir(null)}
              className={`p-1.5 rounded-md hover:bg-muted/50 transition-colors cursor-pointer focus:outline-none focus:ring-2 ${accent.ring}`}
              title={t('kiroConfig.clearProjectDir')}
            >
              <X size={13} className="text-muted-foreground" />
            </button>
          )}
        </div>
      </div>

      {/* Tabs + Content */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <div className="px-5 pt-3 pb-2">
          <TabsList className="glass-card flex h-10 w-full justify-start overflow-x-auto rounded-lg border-none p-0.5 no-scrollbar lg:w-fit">
            {TABS.map(tab => {
              const Icon = tab.icon
              const isDisabled = !!tab.disabled
              return (
                <TabsTrigger
                  key={tab.id}
                  value={tab.id}
                  disabled={isDisabled}
                  title={isDisabled ? t('kiroConfig.selectProjectDir') : ''}
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
              )
            })}
          </TabsList>
        </div>

        <div className="flex-1 min-h-0 overflow-hidden">
          <TabsContent value="mcp" className="h-full m-0">
            <MCPPanel onCountChange={setMcpCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="steering" className="h-full m-0">
            <SteeringPanel onCountChange={setSteeringCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="skills" className="h-full m-0">
            <SkillsPanel onCountChange={setSkillsCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="hooks" className="h-full m-0">
            {projectDir
              ? <HooksPanel onCountChange={setHooksCount} projectDir={projectDir} />
              : (
                <div className="h-full flex items-center justify-center text-sm text-muted-foreground">
                  <div className="text-center">
                    <FolderOpen size={32} className="mx-auto mb-2 opacity-30" />
                    <p>{t('kiroConfig.selectProjectDir')}</p>
                  </div>
                </div>
              )
            }
          </TabsContent>
          <TabsContent value="agents" className="h-full m-0">
            <AgentsPanel onCountChange={setAgentsCount} projectDir={projectDir} />
          </TabsContent>
          <TabsContent value="powers" className="h-full m-0">
            <PowersPanel onCountChange={setPowersCount} />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  )
}

export default KiroConfig
