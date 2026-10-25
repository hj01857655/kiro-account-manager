import { useState, useEffect, useCallback, useMemo } from 'react'
import { getMcpConfig, getMcpSources, getMcpToolStats, toggleMcpServer, deleteMcpServer } from '../../../api/mcpApi'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Server, Plus, Edit2, Trash2, Terminal, Search, X, Wrench } from 'lucide-react'
import MCPSettingsCard from './MCPSettingsCard'
import { Input } from '@/components/ui/input'
import AddMCPModal from './AddMCPModal'
import EditMCPModal from './EditMCPModal'
import { handleUiError } from '../../../utils/errorLogger'
import { getThemeAccent, getGradientAccentButton } from './themeAccent'
import React from 'react'

// 搜索框组件
function SearchInput({ value, onChange, placeholder, colors, t, accent }: any) {
  return (
    <div className="flex-1 max-w-xs relative">
      <Search size={16} className={`absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none`} />
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={`pl-9 pr-8 h-8 text-foreground bg-background border-input ${colors.inputFocus} focus:ring-1 transition-colors duration-200`}
      />
      {value && (
        <button
          type="button"
          aria-label={t('common.clear')}
          onClick={() => onChange('')}
          className={`absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted/50 cursor-pointer transition-colors duration-200 focus:ring-2 ${accent.ring}`}
        >
          <X size={14} className={"text-muted-foreground"} />
        </button>
      )}
    </div>
  )
}

function MCPPanel({ onCountChange, projectDir, readOnly }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const accentGradientButtonClass = getGradientAccentButton(accent)
  const { showConfirm } = useDialog()

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    badgeActive: 'bg-primary/20 text-primary border border-primary/30',
    badgeDisabled: 'bg-muted/50 text-muted-foreground border border-border/50',
    toggleOn: 'bg-primary',
    toggleOff: 'bg-muted',
    toggleThumb: 'bg-white',
    warning: 'bg-amber-500/10',
    warningBorder: 'border-amber-500/20'
  }

  const [servers, setServers] = useState<any>({})
  const [loading, setLoading] = useState(true)
  const [showAddModal, setShowAddModal] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [editingServer, setEditingServer] = useState<any>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [toolCount, setToolCount] = useState(0)
  const [sources, setSources] = useState<Record<string, string>>({})

  const loadConfig = useCallback(async () => {
    try {
      const config = await getMcpConfig(projectDir || null)
      const mcpServers = config.mcpServers || {}
      setServers(mcpServers)
      onCountChange?.(Object.keys(mcpServers).length)

      // 加载工具统计
      const stats = await getMcpToolStats(projectDir || null)
      setToolCount(stats.estimatedTools)

      // 加载各 server 的配置来源（用户级/项目级/覆盖/冗余/插件）
      getMcpSources(projectDir || null).then(setSources).catch(() => setSources({}))
    } catch (e) {
      handleUiError(t('mcp.loadFailedInternal'), e, { userMessage: t('mcp.loadFailed') })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir])

  useEffect(() => { loadConfig() }, [loadConfig])

  // 写入层级按来源决定：user/power 来源的配置写在用户级（传 null），
  // project/overridden/redundant 写在项目级——否则编辑一个用户级 server
  // 会把它复制到项目级，批量制造「冗余覆盖」
  const dirFor = (name: string) => {
    const s = sources[name]
    return s === 'user' || s === 'power' ? null : (projectDir || null)
  }

  const handleToggle = async (name: string, disabled: boolean) => {
    if (readOnly) return
    const oldDisabled = servers[name]?.disabled
    setServers((prev: any) => ({ ...prev, [name]: { ...prev[name], disabled } }))
    try {
      await toggleMcpServer(name, disabled, dirFor(name))
    } catch (e) {
      setServers((prev: any) => ({ ...prev, [name]: { ...prev[name], disabled: oldDisabled } }))
      handleUiError(t('mcp.toggleFailedInternal'), e, { userMessage: t('mcp.toggleFailed') })
    }
  }

  const handleDelete = async (name: string) => {
    if (readOnly) return
    if (!await showConfirm(t('mcp.confirmDelete'), `${t('common.confirm')} ${name}?`)) return
    try {
      await deleteMcpServer(name, dirFor(name))
      setServers((prev: any) => { const next = { ...prev }; delete next[name]; return next })
    } catch (e) {
      handleUiError(t('mcp.deleteFailedInternal'), e, { userMessage: t('mcp.deleteFailed') })
    }
  }

  const serverList = Object.entries(servers)
  const filteredServers = useMemo(() => {
    if (!searchQuery.trim()) return serverList
    const q = searchQuery.toLowerCase()
    return serverList.filter(([name, cfg]: [string, any]) => 
      name.toLowerCase().includes(q) || [cfg.command, ...(cfg.args || [])].join(' ').toLowerCase().includes(q)
    )
  }, [serverList, searchQuery])

  return (
      <div className="h-full flex flex-col max-w-full overflow-x-hidden">
      {/* 警告横幅 */}
      {toolCount > 50 && (
        <div className={`mx-3 mt-3 mb-1 px-3 py-2.5 rounded-lg border ${colors.warning} ${colors.warningBorder} flex items-start gap-2`}>
          <div className="mt-0.5 text-foreground text-sm">
            ⚠️
          </div>
          <div className="flex-1">
            <div className="text-xs font-semibold text-foreground mb-0.5">
              {t('mcp.toolCountTitle')}
            </div>
            <div className="text-[11px] text-muted-foreground leading-relaxed">
              {t('mcp.toolCountDesc', { tools: toolCount, servers: serverList.length })}
            </div>
          </div>
        </div>
      )}

      {/* 工具栏 */}
      <div className="px-3 py-2.5 border-b border-border flex items-center justify-between gap-3 max-w-full overflow-x-hidden">
        <SearchInput
          value={searchQuery}
          onChange={setSearchQuery}
          placeholder={t('common.search')}
          colors={colors}
          t={t}
          accent={accent}
        />
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">{filteredServers.length}/{serverList.length}</span>
          <button
            onClick={() => setShowSettings(true)}
            className={`cursor-pointer px-2.5 h-8 rounded-md text-xs font-medium flex items-center gap-1 border border-border text-foreground hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
          >
            <Wrench size={13} />{t('settings.configureMCP')}
          </button>
          <button
            onClick={() => setShowAddModal(true)}
            className={`cursor-pointer px-2.5 h-8 rounded-md text-xs font-medium flex items-center gap-1 ${accentGradientButtonClass} transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
          >
            <Plus size={13} />{t('mcp.add')}
          </button>
        </div>
      </div>

      {/* 列表 */}
      <div className="flex-1 overflow-auto p-3">
        {loading ? (
          <div className="text-center py-12 text-muted-foreground text-sm">{t('common.loading')}</div>
        ) : serverList.length === 0 ? (
          <div className="text-center py-12">
            <Server size={40} className="mx-auto mb-3 text-muted-foreground opacity-40" />
            <p className="text-sm text-muted-foreground">{t('mcp.noServers')}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {filteredServers.map(([name, config]: [string, any]) => (
              <MCPServerItem
                key={name}
                name={name}
                config={config}
                source={sources[name]}
                accent={accent}
                colors={colors}
                t={t}
                onToggle={(disabled: boolean) => handleToggle(name, disabled)}
                onEdit={() => setEditingServer({ name, config })}
                onDelete={() => handleDelete(name)}
              />
            ))}
          </div>
        )}
      </div>

      {showAddModal && (
        <AddMCPModal
          onClose={() => setShowAddModal(false)}
          onSuccess={() => { setShowAddModal(false); loadConfig() }}
          projectDir={projectDir}
          readOnly={readOnly}
        />
      )}
      {editingServer && (
        <EditMCPModal
          name={editingServer.name}
          config={editingServer.config}
          source={sources[editingServer.name]}
          onClose={() => setEditingServer(null)}
          onSuccess={() => { setEditingServer(null); loadConfig() }}
          projectDir={projectDir}
          readOnly={readOnly}
        />
      )}
      {/* MCP 配置弹窗：settings.json 键（自动注册开关 + env 白名单），不占列表首屏 */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={() => setShowSettings(false)}>
          <div
            className="relative surface-card border border-border rounded-lg shadow-2xl w-[520px] max-w-full max-h-[85vh] flex flex-col"
            onClick={e => e.stopPropagation()}
          >
            <button
              onClick={() => setShowSettings(false)}
              className="absolute top-2.5 right-2.5 p-1.5 rounded-md hover:bg-muted/50 text-muted-foreground transition-colors"
            >
              <X size={14} />
            </button>
            <div className="overflow-y-auto p-4">
              <MCPSettingsCard t={t} />
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

// MCP 服务器卡片
function MCPServerItem({ name, config, source, accent, colors, onToggle, onEdit, onDelete, t, readOnly }: any) {
  const isDisabled = config.disabled
  const commandStr = [config.command, ...(config.args || [])].join(' ')
  const envCount = Object.keys(config.env || {}).length
  const autoApproveCount = config.autoApprove?.length || 0

  return (
    <div className={`surface-card border border-border rounded-xl p-4 flex items-center gap-4`}>
      <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${
        isDisabled ? "bg-muted/30" : colors.badgeActive
      }`}>
        <Server size={20} className={isDisabled ? "text-muted-foreground" : accent.text} />
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h3 className={`font-semibold text-foreground ${isDisabled ? 'opacity-50' : ''}`}>{name}</h3>
          {source && (
            <span
              title={t(`agentConfig.mcpSource_${source}Desc`)}
              className={`text-[10px] px-1.5 py-0.5 rounded ${
                source === 'redundant' ? 'bg-red-500/15 text-red-600 dark:text-red-400'
                : source === 'overridden' ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400'
                : source === 'project' ? 'bg-primary/10 text-primary'
                : source === 'power' ? 'bg-blue-500/15 text-blue-600 dark:text-blue-400'
                : 'bg-muted/60 text-muted-foreground'
              }`}
            >
              {t(`agentConfig.mcpSource_${source}`)}
            </span>
          )}
          {isDisabled && (
            <span className={`text-xs px-2 py-0.5 rounded ${colors.badgeDisabled}`}>{t('mcp.disabled')}</span>
          )}
        </div>
        <div className={`flex items-center gap-1.5 text-sm text-muted-foreground ${isDisabled ? 'opacity-50' : ''}`}>
          <Terminal size={12} />
          <code className="truncate">{commandStr}</code>
        </div>
        {(envCount > 0 || autoApproveCount > 0) && (
          <div className={`flex items-center gap-3 mt-1.5 text-xs text-muted-foreground ${isDisabled ? 'opacity-50' : ''}`}>
            {envCount > 0 && (
              <span className={`px-1.5 py-0.5 rounded info-badge`}>
                {envCount} {t('mcpManager.envVars')}
              </span>
            )}
            {autoApproveCount > 0 && (
              <span className={`px-1.5 py-0.5 rounded success-badge`}>
                {config.autoApprove?.includes('*') ? `${t('mcpManager.autoApprove')} *` : `${t('mcpManager.autoApprove')} ${autoApproveCount} ${t('mcpManager.tools')}`}
              </span>
            )}
          </div>
        )}
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={() => onToggle(!isDisabled)}
          disabled={readOnly}
          className={`cursor-pointer relative w-11 h-6 min-h-[24px] rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${isDisabled ? colors.toggleOff : colors.toggleOn}`}
        >
          <div className={`absolute top-0.5 w-5 h-5 ${colors.toggleThumb} rounded-full transition-transform ${isDisabled ? 'left-0.5' : 'left-5'}`} />
        </button>
        <button
          onClick={onEdit}
          disabled={readOnly}
          className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
        >
          <Edit2 size={16} className={"text-muted-foreground"} />
        </button>
        <button
          onClick={onDelete}
          disabled={readOnly}
          className="cursor-pointer p-2 rounded-lg hover:bg-red-500/10 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
        >
          <Trash2 size={16} className="text-red-500" />
        </button>
      </div>
    </div>
  )
}

export default MCPPanel
