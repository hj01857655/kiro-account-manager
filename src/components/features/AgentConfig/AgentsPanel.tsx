import { useState, useEffect, useCallback, useMemo } from 'react'
import { getCustomAgents, saveCustomAgent, deleteCustomAgent, createCustomAgent } from '../../../api/agentConfigApi'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Bot, RefreshCw, Trash2, Save, Plus, X, Tag, FolderOpen, Globe } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { MultiSelect } from '@/components/ui/multi-select'
import {
  getThemeAccent,
  getSolidAccentButton,
  getGradientAccentButton,
  getThemeSurfaceStyles} from './themeAccent'
import { handleUiError } from '../../../utils/errorLogger'
import React from 'react'

// 解析 agent 内容，按内容自动判别两种格式：
//   IDE 1.0：整份内容是一个 JSON 对象（name / description / model / tools /
//            allowedTools / resources / includeMcpJson / hooks / prompt）
//   IDE 0.x：Markdown + YAML front-matter（name / description / tools / model /
//            includeMcpJson / includePowers）
// 返回额外的 isJson 与 raw：isJson 决定保存时回写成哪种格式，
// raw 保留原始 JSON 对象，使本工具不认识的字段（resources / allowedTools / hooks 等）不丢失。
const parseAgentFrontMatter = (content: string) => {
  const trimmed = content.trim()

  if (trimmed.startsWith('{')) {
    try {
      const obj = JSON.parse(trimmed)
      if (obj && typeof obj === 'object' && !Array.isArray(obj)) {
        return {
          name: typeof obj.name === 'string' ? obj.name : '',
          description: typeof obj.description === 'string' ? obj.description : '',
          tools: Array.isArray(obj.tools) ? obj.tools.filter((t: unknown) => typeof t === 'string') : [],
          model: typeof obj.model === 'string' ? obj.model : '',
          includeMcpJson: obj.includeMcpJson === true,
          includePowers: obj.includePowers === true,
          // prompt 既可能是内联提示词，也可能是 file:// 路径，原样保留
          body: typeof obj.prompt === 'string' ? obj.prompt : '',
          isJson: true,
          raw: obj,
        }
      }
    } catch {
      // 不是合法 JSON，落到 front-matter 分支
    }
  }

  const match = content.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
  if (!match) {
    return { name: '', description: '', tools: [] as string[], model: '', includeMcpJson: false, includePowers: false, body: content, isJson: false, raw: null }
  }
  const [, fm, body] = match
  // 解析 tools 列表（YAML 数组格式或 "*"）
  let tools: string[] = []
  const toolsWildcard = fm.match(/tools:\s*['"]?\*['"]?/)
  if (toolsWildcard) {
    tools = ['*']
  } else {
    const toolsMatch = fm.match(/tools:\s*\n((?:\s*-\s*.+\n?)*)/)
    if (toolsMatch) {
      tools = toolsMatch[1]
        .split('\n')
        .map(line => line.replace(/^\s*-\s*/, '').replace(/^['"]|['"]$/g, '').trim())
        .filter(Boolean)
    }
  }
  return {
    name: fm.match(/name:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    description: fm.match(/description:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    tools,
    model: fm.match(/model:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    includeMcpJson: /includeMcpJson:\s*true/.test(fm),
    includePowers: /includePowers:\s*true/.test(fm),
    body,
    isJson: false,
    raw: null,
  }
}

// 组装 agent 内容。isJson 为真时输出 IDE 1.0 的 JSON 对象，否则输出 IDE 0.x 的
// Markdown + YAML front-matter（仅供兼容旧环境）。
// JSON 分支以 raw（解析时保留的原始对象）为基础回填，因此 resources / allowedTools /
// hooks 等本工具不编辑的字段不会被覆盖丢失。
const buildAgentContent = ({ name, description, tools, model, includeMcpJson, includePowers, isJson, raw }: any, body: string) => {
  if (isJson) {
    const obj: Record<string, unknown> = raw && typeof raw === 'object' ? { ...raw } : {}
    if (name) obj.name = name
    if (description) obj.description = description
    if (model) obj.model = model
    if (Array.isArray(tools) && tools.length > 0) obj.tools = tools
    obj.includeMcpJson = includeMcpJson === true
    if (includePowers !== undefined) obj.includePowers = includePowers === true
    obj.prompt = body ?? ''
    return JSON.stringify(obj, null, 2) + '\n'
  }

  let fm = '---'
  if (name) fm += `\nname: "${name}"`
  if (description) fm += `\ndescription: "${description}"`
  if (tools.length > 0) {
    if (tools.length === 1 && tools[0] === '*') {
      fm += '\ntools: "*"'
    } else {
      fm += '\ntools:'
      for (const tool of tools) {
        fm += `\n  - ${tool}`
      }
    }
  }
  if (model) fm += `\nmodel: "${model}"`
  if (includeMcpJson) fm += '\nincludeMcpJson: true'
  if (includePowers) fm += '\nincludePowers: true'
  return fm + '\n---\n' + body
}

// 格式化文件大小
const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// scope 徽章
const ScopeBadge = ({ scope, accent }: any) => {
  if (scope === 'project') {
    return (
      <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-500 border border-amber-500/30">
        <FolderOpen size={10} />项目
      </span>
    )
  }
  return (
    <span className={`flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium ${accent.scopeBadge}`}>
      <Globe size={10} />用户
    </span>
  )
}

// 可选工具标签（来自 Kiro IDE 0.x）。
// 注意：IDE 1.0 的工具名体系已更换——实测 1.0.437 的 agent 使用 execute_bash / fs_read /
// fs_write / code / grep / glob / web_fetch / web_search / introspect / session /
// report / tool_search 等，MCP 工具以 @server 形式出现。由于未拿到 1.0 的完整工具名清单，
// 此处暂不替换，待确认后再整体更新。
const AVAILABLE_TOOL_TAGS = [
  '*',
  'read', 'edit', 'browser', 'terminal', 'search', 'mcp',
  'listDir', 'readFile', 'writeFile', 'editFile',
  'executeCommand', 'searchFiles', 'findReferences',
  'semanticRename', 'smartRelocate', 'discloseContext',
]

// Kiro 可用的 Agent 模型
const AVAILABLE_MODELS = [
  { value: 'default', label: '默认（跟随主对话）' },
  { value: 'claude-sonnet-5', label: 'Claude Sonnet 5' },
  { value: 'claude-sonnet-4.6', label: 'Claude Sonnet 4.6' },
  { value: 'claude-sonnet-4.5', label: 'Claude Sonnet 4.5' },
  { value: 'claude-sonnet-4', label: 'Claude Sonnet 4' },
  { value: 'claude-opus-5', label: 'Claude Opus 5' },
  { value: 'claude-opus-4.8', label: 'Claude Opus 4.8' },
  { value: 'claude-opus-4.7', label: 'Claude Opus 4.7' },
  { value: 'claude-opus-4.6', label: 'Claude Opus 4.6' },
  { value: 'claude-opus-4.5', label: 'Claude Opus 4.5' },
  { value: 'claude-haiku-4.5', label: 'Claude Haiku 4.5' },
  { value: 'gpt-5.6-sol', label: 'GPT-5.6 Sol' },
  { value: 'gpt-5.6-terra', label: 'GPT-5.6 Terra' },
  { value: 'gpt-5.6-luna', label: 'GPT-5.6 Luna' },
  { value: 'deepseek-3.2', label: 'DeepSeek 3.2' },
  { value: 'minimax-m2.5', label: 'MiniMax M2.5' },
  { value: 'minimax-m2.1', label: 'MiniMax M2.1' },
  { value: 'glm-5', label: 'GLM-5' },
  { value: 'qwen3-coder-next', label: 'Qwen3 Coder Next' },
]

const normalizeToolTagsSelection = (nextValues: string[], prevValues: string[] = []) => {
  const uniqueValues = [...new Set((nextValues || []).filter(Boolean))]
  if (!uniqueValues.includes('*')) return uniqueValues
  if (uniqueValues.length === 1) return ['*']

  const prevHasWildcard = (prevValues || []).includes('*')
  return prevHasWildcard ? uniqueValues.filter(value => value !== '*') : ['*']
}

function AgentsPanel({ onCountChange, projectDir, readOnly }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const accentGradientButtonClass = getGradientAccentButton(accent)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10 ring-1 ring-primary/15'
  }

  const [agents, setAgents] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedAgent, setSelectedAgent] = useState<any>(null)
  const [editState, setEditState] = useState({
    name: '', description: '', tools: [] as string[], model: '',
    includeMcpJson: false, includePowers: false, body: ''
  })
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)


  const loadAgents = useCallback(async () => {
    setLoading(true)
    try {
      const data = await getCustomAgents(projectDir || null)
      setAgents(data)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('加载 Custom Agents 失败', e, { userMessage: t('agents.loadFailed') || '加载 Custom Agents 失败' })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir, t])

  useEffect(() => {
    setSelectedAgent(null)
    setEditState({ name: '', description: '', tools: [], model: '', includeMcpJson: false, includePowers: false, body: '' })
    setHasChanges(false)
    loadAgents()
  }, [loadAgents])

  const handleSelect = async (agent: any) => {
    if (hasChanges && !await showConfirm(t('agents.unsavedChanges'), t('agents.confirmSwitch'))) return
    setSelectedAgent(agent)
    const parsed = parseAgentFrontMatter(agent.content)
    setEditState(parsed)
    setHasChanges(false)
  }

  const updateEditState = (key: string, value: any) => {
    const normalizedValue = key === 'tools'
      ? normalizeToolTagsSelection(value, editState.tools)
      : value
    const newState = { ...editState, [key]: normalizedValue }
    setEditState(newState)
    if (selectedAgent) {
      const { body, ...fm } = newState
      const newContent = buildAgentContent(fm, body)
      setHasChanges(newContent !== selectedAgent.content)
    }
  }

  const handleSave = async () => {
    if (readOnly) return
    if (!selectedAgent) return
    setSaving(true)
    try {
      const { body, ...fm } = editState
      const fullContent = buildAgentContent(fm, body)
      await saveCustomAgent(selectedAgent.fileName, fullContent, selectedAgent.scope, projectDir || null)
      setAgents(agents.map(a => (a.fileName === selectedAgent.fileName && a.scope === selectedAgent.scope) ? { ...a, content: fullContent } : a))
      setSelectedAgent({ ...selectedAgent, content: fullContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('保存 Custom Agent 失败', e, { userMessage: t('agents.saveFailed') || '保存失败' })
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (agent: any) => {
    if (readOnly) return
    if (!await showConfirm(t('agents.confirmDelete'), t('agents.confirmDeleteAgent', { fileName: agent.fileName }))) return
    try {
      await deleteCustomAgent(agent.fileName, agent.scope, projectDir || null)
      const newAgents = agents.filter(a => !(a.fileName === agent.fileName && a.scope === agent.scope))
      setAgents(newAgents)
      onCountChange?.(newAgents.length)
      if (selectedAgent?.fileName === agent.fileName && selectedAgent?.scope === agent.scope) {
        setSelectedAgent(null)
        setEditState({ name: '', description: '', tools: [], model: '', includeMcpJson: false, includePowers: false, body: '' })
        setHasChanges(false)
      }
    } catch (e) {
      handleUiError('删除 Custom Agent 失败', e, { userMessage: t('agents.deleteFailed') || '删除失败' })
    }
  }

  const handleCreate = async (agentName: string, description: string, tools: string[], model: string, scope: string) => {
    if (readOnly) return
    // IDE 1.0 起 custom agent 为 .json；只有用户显式写了 .md 才按旧格式创建
    const lower = agentName.toLowerCase()
    const isLegacy = lower.endsWith('.md')
    const fileName = isLegacy || lower.endsWith('.json') ? agentName : `${agentName}.json`
    const baseName = isLegacy ? agentName.replace(/\.md$/i, '') : agentName.replace(/\.json$/i, '')
    const body = isLegacy ? '\n<!-- 在此编写 Agent 的系统提示词 -->\n' : '在此编写 Agent 的系统提示词'
    const content = buildAgentContent(
      { name: baseName, description, tools, model, includeMcpJson: false, includePowers: false, isJson: !isLegacy, raw: null },
      body,
    )
    try {
      const newAgent = await createCustomAgent(fileName, content, scope, projectDir || null)
      const newAgents = [...agents, newAgent]
      setAgents(newAgents)
      onCountChange?.(newAgents.length)
      setShowCreateModal(false)
      handleSelect(newAgent)
    } catch (e) {
      handleUiError('创建 Custom Agent 失败', e, { userMessage: t('agents.createFailed') || '创建失败' })
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
      </div>
    )
  }

  return (
    <div className="h-full flex gap-3 p-4 max-w-full overflow-x-hidden">
      {/* 左侧列表 */}
      <div className={`w-72 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        <div className={`p-3 border-b border-border flex items-center justify-between`}>
          <div className="flex items-center gap-2">
            <Bot size={18} className={accent.text} />
            <span className={`text-sm font-semibold text-foreground`}>Custom Agents</span>
            <span className={`text-xs text-muted-foreground`}>({agents.length})</span>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => setShowCreateModal(true)}
              disabled={readOnly}
              className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 disabled:opacity-40 disabled:cursor-not-allowed ${accent.ring}`}
              title={t('agents.create')}
            >
              <Plus size={16} className={accent.text} />
            </button>
              <button
                onClick={loadAgents}
                className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                title={t('common.refresh')}
              >
              <RefreshCw size={16} className={"text-muted-foreground"} />
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4">
          {agents.length === 0 ? (
            <div className={`text-center py-16 text-muted-foreground`}>
              <Bot size={48} className="mx-auto mb-3 opacity-20" />
              <p className="text-sm">{t('agents.noAgents')}</p>
              <p className={`text-xs mt-2 text-muted-foreground`}>{t('agents.noAgentsHint')}</p>
              <button
                onClick={() => setShowCreateModal(true)}
                disabled={readOnly}
                className={`cursor-pointer mt-4 px-4 py-2 rounded-lg text-sm transition-colors duration-200 focus:outline-none focus:ring-2 disabled:opacity-40 disabled:cursor-not-allowed ${accent.ring} ${accentSolidButtonClass}`}
              >
                {t('agents.createFirst')}
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {agents.map(agent => {
                const isSelected = selectedAgent?.fileName === agent.fileName && selectedAgent?.scope === agent.scope
                const parsed = parseAgentFrontMatter(agent.content)
                return (
                  <div
                    key={`${agent.scope}-${agent.fileName}`}
                    onClick={() => handleSelect(agent)}
                    className={`p-4 rounded-xl cursor-pointer group transition-all duration-200 ${
                      isSelected
                        ? `${accent.bg} ring-2 ${accent.ring} shadow-xl border-2 ${accent.border} scale-[1.02]`
                        : `glass-card border border-border hover:bg-muted/50 hover:shadow-lg hover:scale-[1.01]`
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3 mb-2.5">
                      <div className="flex items-center gap-3 flex-1 min-w-0">
                        <div className={`flex items-center justify-center w-8 h-8 rounded-lg transition-colors ${
                          isSelected ? accent.bg : "bg-muted/30"
                        }`}>
                          <Bot
                            size={18}
                            className={`flex-shrink-0 ${isSelected ? accent.text : "text-muted-foreground"}`}
                          />
                        </div>
                        <div className="flex-1 min-w-0">
                          <span className={`font-bold text-sm ${isSelected ? accent.text : "text-foreground"} truncate block`}>
                            {parsed.name || agent.fileName.replace('.md', '')}
                          </span>
                          {parsed.description && (
                            <span className={`text-xs text-muted-foreground truncate block mt-0.5`}>{parsed.description}</span>
                          )}
                        </div>
                        <ScopeBadge scope={agent.scope} accent={accent} />
                      </div>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDelete(agent) }}
                        disabled={readOnly}
                        className="cursor-pointer opacity-0 group-hover:opacity-100 p-2 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60 disabled:opacity-40 disabled:cursor-not-allowed"
                        title={t('common.delete')}
                      >
                        <Trash2 size={16} className="text-red-500" />
                      </button>
                    </div>
                    <div className={`flex items-center gap-2.5 text-xs text-muted-foreground ml-11 flex-wrap`}>
                      <span className={`px-2 py-1 rounded-md bg-muted/30 font-medium`}>
                        {formatSize(agent.size)}
                      </span>
                      {parsed.tools.length > 0 && (
                        <>
                          <span className="opacity-50">•</span>
                          <span className="flex items-center gap-1">
                            <Tag size={12} />
                            {parsed.tools[0] === '*' ? t('agents.allTools') : `${parsed.tools.length} ${t('agents.tools')}`}
                          </span>
                        </>
                      )}
                      {parsed.model && (
                        <>
                          <span className="opacity-50">•</span>
                          <span className={`px-1.5 py-0.5 rounded ${accent.bgSoft} ${accent.textSoft} text-[10px]`}>{parsed.model}</span>
                        </>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>

      {/* 右侧编辑器 */}
      <div className={`flex-1 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        {selectedAgent ? (
          <>
            <div className={`p-3 border-b border-border flex items-center justify-between`}>
              <div className="flex items-center gap-2">
                <h3 className={`font-semibold text-foreground`}>{selectedAgent.fileName}</h3>
                <ScopeBadge scope={selectedAgent.scope} accent={accent} />
                {hasChanges && <span className="text-xs text-orange-500">● {t('agents.unsaved')}</span>}
              </div>
              <button
                onClick={handleSave}
                disabled={!hasChanges || saving || readOnly}
                className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${
                  hasChanges ? accentSolidButtonClass : colors.btnDisabled
                } disabled:opacity-50`}
              >
                <Save size={14} />
                {saving ? t('agents.saving') : t('agents.save')}
              </button>
            </div>
            {/* frontmatter 编辑区 */}
            <div className={`px-4 py-3 border-b border-border space-y-3`}>
              {/* name + description */}
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                  <span className={`text-xs text-muted-foreground`}>{t('agents.fmName')}:</span>
                  <Input
                    value={editState.name}
                    onChange={(e) => updateEditState('name', e.target.value)}
                    placeholder={t('agents.fmNamePlaceholder')}
                    className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                    style={{ width: '140px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
                  />
                </div>
                <div className="flex items-center gap-2 flex-1">
                  <span className={`text-xs text-muted-foreground`}>{t('agents.fmDescription')}:</span>
                  <Input
                    value={editState.description}
                    onChange={(e) => updateEditState('description', e.target.value)}
                    placeholder={t('agents.fmDescriptionPlaceholder')}
                    className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                    style={{ flex: 1, minWidth: '200px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
                  />
                </div>
              </div>
              {/* tools */}
              <div className="flex items-center gap-2">
                <Tag size={14} className={"text-muted-foreground"} />
                <span className={`text-xs text-muted-foreground`}>{t('agents.toolTags')}:</span>
                <div className="flex-1">
                  <MultiSelect
                    value={editState.tools}
                    onChange={(v) => updateEditState('tools', v)}
                    options={AVAILABLE_TOOL_TAGS.map(tag => ({ value: tag, label: tag === '*' ? '* (全部工具)' : tag }))}
                    placeholder={t('agents.selectTools')}
                    searchable
                    clearable
                    className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                  />
                </div>
              </div>
              {/* model + switches */}
              <div className="flex items-center gap-4 flex-wrap">
                <div className="flex items-center gap-2">
                  <span className={`text-xs text-muted-foreground`}>{t('agents.model')}:</span>
                  <Select
                    value={editState.model}
                    onValueChange={(v) => updateEditState('model', v || '')}
                  >
                    <SelectTrigger className={`min-w-[180px] h-6 text-xs ${colors.inputFocus}`}>
                      <SelectValue placeholder={t('agents.model')} />
                    </SelectTrigger>
                    <SelectContent className="">
                      {AVAILABLE_MODELS.map(m => (
                        <SelectItem key={m.value} value={m.value} className="">{m.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={editState.includeMcpJson}
                    onCheckedChange={(checked) => updateEditState('includeMcpJson', checked)}
                  />
                  <span className="text-muted-foreground text-xs">{t('agents.includeMcpJson')}</span>
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={editState.includePowers}
                    onCheckedChange={(checked) => updateEditState('includePowers', checked)}
                  />
                  <span className="text-muted-foreground text-xs">{t('agents.includePowers')}</span>
                </div>
              </div>
            </div>
            <div className="flex-1 p-4 overflow-hidden">
              <Textarea
                value={editState.body}
                onChange={(e) => updateEditState('body', e.target.value)}
                readOnly={readOnly}
                placeholder={t('agents.contentPlaceholder')}
                className={`flex-1 h-full min-h-[400px] p-4 rounded-xl text-sm leading-relaxed font-mono resize-none ${colors.inputFocus}`}
                style={{
                  color: surface.editorText,
                  backgroundColor: surface.editorBg,
                  borderColor: surface.editorBorder
                }}
              />
            </div>
          </>
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <Bot size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('agents.selectToEdit')}</p>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && (
        <CreateAgentModal
          onCreate={handleCreate}
          onClose={() => setShowCreateModal(false)}
          accent={accent}
          surface={surface}
          accentGradientButtonClass={accentGradientButtonClass}
          colors={colors}
          t={t}
          hasProjectDir={!!projectDir}
        />
      )}
    </div>
  )
}

// 创建 Agent 弹窗
function CreateAgentModal({ onCreate, onClose, accent, surface, accentGradientButtonClass, colors, t, hasProjectDir, readOnly }: any) {
  const [agentName, setAgentName] = useState('')
  const [description, setDescription] = useState('')
  const [tools, setTools] = useState<string[]>([])
  const [model, setModel] = useState('')
  const [scope, setScope] = useState('user')


  const handleToolsChange = (values: string[]) => {
    setTools(prev => normalizeToolTagsSelection(values, prev))
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`glass-card rounded-2xl w-full max-w-[460px] shadow-2xl border border-border overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <Bot size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('agents.newAgent')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('agents.agentName')}</label>
            <Input
              placeholder={t('agents.agentNamePlaceholder')}
              value={agentName}
              onChange={(e) => setAgentName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('agents.agentNameHint')}</p>
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('agents.fmDescription')}</label>
            <Input
              placeholder={t('agents.fmDescriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('agents.toolTags')}</label>
            <MultiSelect
              value={tools}
              onChange={handleToolsChange}
              options={AVAILABLE_TOOL_TAGS.map(tag => ({ value: tag, label: tag === '*' ? '* (全部工具)' : tag }))}
              placeholder={t('agents.selectTools')}
              searchable clearable
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('agents.model')}</label>
            <Select
              value={model}
              onValueChange={(v) => setModel(v || '')}
            >
              <SelectTrigger className={`w-full ${colors.inputFocus}`}>
                <SelectValue placeholder={t('agents.model')} />
              </SelectTrigger>
              <SelectContent className="">
                {AVAILABLE_MODELS.map(m => (
                  <SelectItem key={m.value} value={m.value} className="">{m.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {hasProjectDir && (
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('agentConfig.scope')}</label>
              <Select
                value={scope}
                onValueChange={setScope}
              >
                <SelectTrigger className={`w-full ${colors.inputFocus}`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="">
                  <SelectItem value="user" className="">{t('agentConfig.scopeUser')}</SelectItem>
                  <SelectItem value="project" className="">{t('agentConfig.scopeProject')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          <button
            onClick={() => onCreate(agentName.trim(), description.trim(), tools, model, scope)}
            disabled={!agentName.trim() || readOnly}
            className={`cursor-pointer w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default AgentsPanel
