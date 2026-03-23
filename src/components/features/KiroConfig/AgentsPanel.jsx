import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Bot, RefreshCw, Trash2, Save, Plus, X, Tag, FolderOpen, Globe } from 'lucide-react'
import { TextInput, Select, Textarea, MultiSelect, Switch } from '@mantine/core'
import {
  getThemeAccent,
  getSolidAccentButton,
  getGradientAccentButton,
  getThemeSurfaceStyles,
} from './themeAccent'
import { handleUiError } from '../../../utils/errorLogger'

// 解析 agent front-matter（v0.10.32 完整 schema: name, description, tools, model, includeMcpJson, includePowers）
const parseAgentFrontMatter = (content) => {
  const match = content.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
  if (!match) return { name: '', description: '', tools: [], model: '', includeMcpJson: false, includePowers: false, body: content }
  const [, fm, body] = match
  // 解析 tools 列表（YAML 数组格式或 "*"）
  let tools = []
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
    body
  }
}

// 组装 agent front-matter（v0.10.32 完整 schema）
const buildAgentContent = ({ name, description, tools, model, includeMcpJson, includePowers }, body) => {
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
const formatSize = (bytes) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// scope 徽章
const ScopeBadge = ({ scope, accent }) => {
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

// Kiro v0.10.32 可用的工具标签
const AVAILABLE_TOOL_TAGS = [
  '*',
  'read', 'edit', 'browser', 'terminal', 'search', 'mcp',
  'listDir', 'readFile', 'writeFile', 'editFile',
  'executeCommand', 'searchFiles', 'findReferences',
  'semanticRename', 'smartRelocate', 'discloseContext',
]

// Kiro v0.10.32 可用的模型
const AVAILABLE_MODELS = [
  { value: '', label: '默认（跟随主对话）' },
  { value: 'claude-sonnet-4.5', label: 'Claude Sonnet 4.5' },
  { value: 'claude-sonnet-4', label: 'Claude Sonnet 4' },
  { value: 'claude-haiku-4.5', label: 'Claude Haiku 4.5' },
]

const normalizeToolTagsSelection = (nextValues, prevValues = []) => {
  const uniqueValues = [...new Set((nextValues || []).filter(Boolean))]
  if (!uniqueValues.includes('*')) return uniqueValues
  if (uniqueValues.length === 1) return ['*']

  const prevHasWildcard = (prevValues || []).includes('*')
  return prevHasWildcard ? uniqueValues.filter(value => value !== '*') : ['*']
}

function AgentsPanel({ onCountChange, projectDir }) {
  const { t, theme, colors } = useApp()
  const { showConfirm } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  const accent = getThemeAccent(theme)
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const accentGradientButtonClass = getGradientAccentButton(accent)

  const [agents, setAgents] = useState([])
  const [loading, setLoading] = useState(true)
  const [selectedAgent, setSelectedAgent] = useState(null)
  const [editState, setEditState] = useState({
    name: '', description: '', tools: [], model: '',
    includeMcpJson: false, includePowers: false, body: ''
  })
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [editToolsDropdownOpened, setEditToolsDropdownOpened] = useState(false)
  const [editModelDropdownOpened, setEditModelDropdownOpened] = useState(false)

  const loadAgents = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke('get_custom_agents', { projectDir: projectDir || null })
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

  const handleSelect = async (agent) => {
    if (hasChanges && !await showConfirm(t('agents.unsavedChanges'), t('agents.confirmSwitch'))) return
    setSelectedAgent(agent)
    const parsed = parseAgentFrontMatter(agent.content)
    setEditState(parsed)
    setHasChanges(false)
  }

  const updateEditState = (key, value) => {
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
    if (!selectedAgent) return
    setSaving(true)
    try {
      const { body, ...fm } = editState
      const fullContent = buildAgentContent(fm, body)
      await invoke('save_custom_agent', {
        fileName: selectedAgent.fileName,
        content: fullContent,
        scope: selectedAgent.scope,
        projectDir: projectDir || null
      })
      setAgents(agents.map(a => (a.fileName === selectedAgent.fileName && a.scope === selectedAgent.scope) ? { ...a, content: fullContent } : a))
      setSelectedAgent({ ...selectedAgent, content: fullContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('保存 Custom Agent 失败', e, { userMessage: t('agents.saveFailed') || '保存失败' })
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (agent) => {
    if (!await showConfirm(t('agents.confirmDelete'), t('agents.confirmDeleteAgent', { fileName: agent.fileName }))) return
    try {
      await invoke('delete_custom_agent', {
        fileName: agent.fileName,
        scope: agent.scope,
        projectDir: projectDir || null
      })
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

  const handleCreate = async (agentName, description, tools, model, scope) => {
    const fileName = agentName.endsWith('.md') ? agentName : `${agentName}.md`
    const body = '\n<!-- 在此编写 Agent 的系统提示词 -->\n'
    const content = buildAgentContent({ name: agentName.replace('.md', ''), description, tools, model, includeMcpJson: false, includePowers: false }, body)
    try {
      const newAgent = await invoke('create_custom_agent', {
        fileName,
        content,
        scope,
        projectDir: projectDir || null
      })
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
    <div className="h-full flex gap-4 p-4 max-w-full overflow-x-hidden">
      {/* 左侧列表 */}
      <div className={`w-80 flex flex-col ${colors.card} border ${colors.cardBorder} rounded-2xl overflow-hidden shadow-lg`}>
        <div className={`p-4 border-b ${colors.cardBorder} flex items-center justify-between`}>
          <div className="flex items-center gap-2">
            <Bot size={18} className={accent.text} />
            <span className={`text-sm font-semibold ${colors.text}`}>Custom Agents</span>
            <span className={`text-xs ${colors.textMuted}`}>({agents.length})</span>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => setShowCreateModal(true)}
              className={`cursor-pointer p-2 rounded-lg ${colors.cardHover} transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
              title={t('agents.create')}
            >
              <Plus size={16} className={accent.text} />
            </button>
              <button
                onClick={loadAgents}
                className={`cursor-pointer p-2 rounded-lg ${colors.cardHover} transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                title={t('common.refresh')}
              >
              <RefreshCw size={16} className={colors.textMuted} />
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4">
          {agents.length === 0 ? (
            <div className={`text-center py-16 ${colors.textMuted}`}>
              <Bot size={48} className="mx-auto mb-3 opacity-20" />
              <p className="text-sm">{t('agents.noAgents')}</p>
              <p className={`text-xs mt-2 ${colors.textMuted}`}>{t('agents.noAgentsHint')}</p>
              <button
                onClick={() => setShowCreateModal(true)}
                className={`cursor-pointer mt-4 px-4 py-2 rounded-lg text-sm transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${accentSolidButtonClass}`}
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
                        : `${colors.card} border ${colors.cardBorder} ${colors.cardHover} hover:shadow-lg hover:scale-[1.01]`
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3 mb-2.5">
                      <div className="flex items-center gap-3 flex-1 min-w-0">
                        <div className={`flex items-center justify-center w-8 h-8 rounded-lg transition-colors ${
                          isSelected ? accent.bg : colors.cardSecondary
                        }`}>
                          <Bot
                            size={18}
                            className={`flex-shrink-0 ${isSelected ? accent.text : colors.textMuted}`}
                          />
                        </div>
                        <div className="flex-1 min-w-0">
                          <span className={`font-bold text-sm ${isSelected ? accent.text : colors.text} truncate block`}>
                            {parsed.name || agent.fileName.replace('.md', '')}
                          </span>
                          {parsed.description && (
                            <span className={`text-xs ${colors.textMuted} truncate block mt-0.5`}>{parsed.description}</span>
                          )}
                        </div>
                        <ScopeBadge scope={agent.scope} accent={accent} />
                      </div>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDelete(agent) }}
                        className="cursor-pointer opacity-0 group-hover:opacity-100 p-2 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                        title={t('common.delete')}
                      >
                        <Trash2 size={16} className="text-red-500" />
                      </button>
                    </div>
                    <div className={`flex items-center gap-2.5 text-xs ${colors.textMuted} ml-11 flex-wrap`}>
                      <span className={`px-2 py-1 rounded-md ${colors.cardSecondary} font-medium`}>
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
      <div className={`flex-1 flex flex-col ${colors.card} border ${colors.cardBorder} rounded-2xl overflow-hidden shadow-lg`}>
        {selectedAgent ? (
          <>
            <div className={`p-4 border-b ${colors.cardBorder} flex items-center justify-between`}>
              <div className="flex items-center gap-2">
                <h3 className={`font-semibold ${colors.text}`}>{selectedAgent.fileName}</h3>
                <ScopeBadge scope={selectedAgent.scope} accent={accent} />
                {hasChanges && <span className="text-xs text-orange-500">● {t('agents.unsaved')}</span>}
              </div>
              <button
                onClick={handleSave}
                disabled={!hasChanges || saving}
                className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${
                  hasChanges ? accentSolidButtonClass : colors.btnDisabled
                } disabled:opacity-50`}
              >
                <Save size={14} />
                {saving ? t('agents.saving') : t('agents.save')}
              </button>
            </div>
            {/* frontmatter 编辑区 */}
            <div className={`px-4 py-3 border-b ${colors.cardBorder} space-y-3`}>
              {/* name + description */}
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                  <span className={`text-xs ${colors.textMuted}`}>{t('agents.fmName')}:</span>
                  <TextInput
                    value={editState.name}
                    onChange={(e) => updateEditState('name', e.target.value)}
                    placeholder={t('agents.fmNamePlaceholder')}
                    size="xs"
                    classNames={{ input: `${colors.text} ${colors.input} ${colors.inputFocus}` }}
                    styles={{ input: { width: '140px', borderRadius: '0.5rem' } }}
                  />
                </div>
                <div className="flex items-center gap-2 flex-1">
                  <span className={`text-xs ${colors.textMuted}`}>{t('agents.fmDescription')}:</span>
                  <TextInput
                    value={editState.description}
                    onChange={(e) => updateEditState('description', e.target.value)}
                    placeholder={t('agents.fmDescriptionPlaceholder')}
                    size="xs"
                    classNames={{ input: `${colors.text} ${colors.input} ${colors.inputFocus}` }}
                    styles={{ input: { flex: 1, minWidth: '200px', borderRadius: '0.5rem' } }}
                  />
                </div>
              </div>
              {/* tools */}
              <div className="flex items-center gap-2">
                <Tag size={14} className={colors.textMuted} />
                <span className={`text-xs ${colors.textMuted}`}>{t('agents.toolTags')}:</span>
                <div className="flex-1">
                  <MultiSelect
                    value={editState.tools}
                    onChange={(v) => updateEditState('tools', v)}
                    dropdownOpened={editToolsDropdownOpened}
                    onDropdownOpen={() => {
                      setEditToolsDropdownOpened(true)
                      setEditModelDropdownOpened(false)
                    }}
                    onDropdownClose={() => setEditToolsDropdownOpened(false)}
                    data={AVAILABLE_TOOL_TAGS.map(tag => ({ value: tag, label: tag === '*' ? '* (全部工具)' : tag }))}
                    placeholder={t('agents.selectTools')}
                    searchable
                    clearable
                    size="xs"
                    classNames={{
                      input: `${colors.text} ${colors.input} ${colors.inputFocus}`,
                      dropdown: `${colors.card} border ${colors.cardBorder}`,
                      option: `${colors.text}`,
                      pillsList: 'gap-1'
                    }}
                    styles={{
                      input: {
                        borderRadius: '0.5rem',
                        minHeight: '32px',
                        backgroundColor: surface.inputBg,
                        borderColor: surface.inputBorder,
                        color: surface.inputText,
                      },
                      inputField: {
                        color: surface.inputText,
                        '&::placeholder': {
                          color: surface.placeholder,
                          opacity: 1
                        }
                      },
                      dropdown: {
                        backgroundColor: surface.dropdownBg,
                        borderColor: surface.dropdownBorder,
                      },
                      option: {
                        color: surface.inputText,
                        backgroundColor: 'transparent',
                      },
                      pill: {
                        backgroundColor: surface.pillBg,
                        color: surface.pillText,
                        border: surface.pillBorder,
                        borderRadius: '0.375rem'
                      }
                    }}
                  />
                </div>
              </div>
              {/* model + switches */}
              <div className="flex items-center gap-4 flex-wrap">
                <div className="flex items-center gap-2">
                  <span className={`text-xs ${colors.textMuted}`}>{t('agents.model')}:</span>
                  <Select
                    value={editState.model}
                    onChange={(v) => updateEditState('model', v || '')}
                    dropdownOpened={editModelDropdownOpened}
                    onDropdownOpen={() => {
                      setEditModelDropdownOpened(true)
                      setEditToolsDropdownOpened(false)
                    }}
                    onDropdownClose={() => setEditModelDropdownOpened(false)}
                    data={AVAILABLE_MODELS}
                    size="xs"
                    clearable
                    classNames={{
                      input: `${colors.text} ${colors.input} ${colors.inputFocus}`,
                      dropdown: `${colors.card} border ${colors.cardBorder}`,
                      option: `${colors.text}`,
                      pill: `${colors.cardSecondary} ${colors.text} border ${colors.cardBorder}`

                    }}
                    styles={{
                      input: {
                        minWidth: '180px',
                        borderRadius: '0.5rem',
                        backgroundColor: surface.inputBg,
                        borderColor: surface.inputBorder,
                        color: surface.inputText,
                      },
                      dropdown: {
                        backgroundColor: surface.dropdownBg,
                        borderColor: surface.dropdownBorder,
                      },
                      option: {
                        color: surface.inputText,
                        backgroundColor: 'transparent',
                      }
                    }}
                  />
                </div>
                <Switch
                  label={t('agents.includeMcpJson')}
                  checked={editState.includeMcpJson}
                  onChange={(e) => updateEditState('includeMcpJson', e.currentTarget.checked)}
                  size="xs"
                  classNames={{ label: `${colors.textMuted} text-xs` }}
                />
                <Switch
                  label={t('agents.includePowers')}
                  checked={editState.includePowers}
                  onChange={(e) => updateEditState('includePowers', e.currentTarget.checked)}
                  size="xs"
                  classNames={{ label: `${colors.textMuted} text-xs` }}
                />
              </div>
            </div>
            <div className="flex-1 p-4 overflow-hidden">
              <Textarea
                value={editState.body}
                onChange={(e) => updateEditState('body', e.target.value)}
                placeholder={t('agents.contentPlaceholder')}
                classNames={{
                  input: `${colors.inputFocus}`
                }}
                styles={{
                  root: { height: '100%', display: 'flex', flexDirection: 'column' },
                  wrapper: { flex: 1, display: 'flex' },
                  input: {
                    flex: 1,
                    height: '100%',
                    minHeight: '400px',
                    padding: '1rem',
                    borderRadius: '0.75rem',
                    fontSize: '0.875rem',
                    lineHeight: '1.5',
                    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                    resize: 'none',
                    color: surface.editorText,
                    backgroundColor: surface.editorBg,
                    borderColor: surface.editorBorder,
                  }
                }}
              />
            </div>
          </>
        ) : (
          <div className={`flex-1 flex items-center justify-center ${colors.textMuted}`}>
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
function CreateAgentModal({ onCreate, onClose, accent, surface, accentGradientButtonClass, colors, t, hasProjectDir }) {
  const [agentName, setAgentName] = useState('')
  const [description, setDescription] = useState('')
  const [tools, setTools] = useState([])
  const [model, setModel] = useState('')
  const [scope, setScope] = useState('user')
  const [toolsDropdownOpened, setToolsDropdownOpened] = useState(false)
  const [modelDropdownOpened, setModelDropdownOpened] = useState(false)

  const handleToolsChange = (values) => {
    setTools(prev => normalizeToolTagsSelection(values, prev))
  }

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`${colors.card} rounded-2xl w-full max-w-[460px] shadow-2xl border ${colors.cardBorder} overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <Bot size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold ${colors.text}`}>{t('agents.newAgent')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors ${colors.cardHover}`}>
            <X size={18} className={colors.textMuted} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('agents.agentName')}</label>
            <TextInput
              placeholder={t('agents.agentNamePlaceholder')}
              value={agentName}
              onChange={(e) => setAgentName(e.target.value)}
              size="md"
              classNames={{ input: `${colors.text} ${colors.input} ${colors.inputFocus}` }}
              styles={{ input: { borderRadius: '0.5rem' } }}
            />
            <p className={`text-xs ${colors.textMuted} mt-1`}>{t('agents.agentNameHint')}</p>
          </div>

          <div>
            <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('agents.fmDescription')}</label>
            <TextInput
              placeholder={t('agents.fmDescriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              size="md"
              classNames={{ input: `${colors.text} ${colors.input} ${colors.inputFocus}` }}
              styles={{ input: { borderRadius: '0.5rem' } }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('agents.toolTags')}</label>
            <MultiSelect
              value={tools}
              onChange={handleToolsChange}
              dropdownOpened={toolsDropdownOpened}
              onDropdownOpen={() => {
                setToolsDropdownOpened(true)
                setModelDropdownOpened(false)
              }}
              onDropdownClose={() => setToolsDropdownOpened(false)}
              data={AVAILABLE_TOOL_TAGS.map(tag => ({ value: tag, label: tag === '*' ? '* (全部工具)' : tag }))}
              placeholder={t('agents.selectTools')}
              searchable clearable size="md"
              classNames={{
                input: `${colors.text} ${colors.input} ${colors.inputFocus}`,
                dropdown: `${colors.card} border ${colors.cardBorder}`,
                option: `${colors.text}`,
                pillsList: 'gap-1'
              }}
              styles={{
                input: {
                  borderRadius: '0.5rem',
                  backgroundColor: surface.inputBg,
                  borderColor: surface.inputBorder,
                  color: surface.inputText,
                },
                inputField: {
                  color: surface.inputText,
                  '&::placeholder': {
                    color: surface.placeholder,
                    opacity: 1
                  }
                },
                dropdown: {
                  backgroundColor: surface.dropdownBg,
                  borderColor: surface.dropdownBorder,
                },
                option: {
                  color: surface.inputText,
                  backgroundColor: 'transparent',
                },
                pill: {
                  backgroundColor: surface.pillBg,
                  color: surface.pillText,
                  border: surface.pillBorder,
                  borderRadius: '0.375rem'
                }
              }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('agents.model')}</label>
            <Select
              value={model}
              onChange={(v) => setModel(v || '')}
              dropdownOpened={modelDropdownOpened}
              onDropdownOpen={() => {
                setModelDropdownOpened(true)
                setToolsDropdownOpened(false)
              }}
              onDropdownClose={() => setModelDropdownOpened(false)}
              data={AVAILABLE_MODELS}
              size="md" clearable
              classNames={{
                input: `${colors.text} ${colors.input} ${colors.inputFocus}`,
                dropdown: `${colors.card} border ${colors.cardBorder}`,
                option: `${colors.text}`
              }}
              styles={{
                input: {
                  borderRadius: '0.5rem',
                  backgroundColor: surface.inputBg,
                  borderColor: surface.inputBorder,
                  color: surface.inputText,
                },
                dropdown: {
                  backgroundColor: surface.dropdownBg,
                  borderColor: surface.dropdownBorder,
                },
                option: {
                  color: surface.inputText,
                  backgroundColor: 'transparent',
                }
              }}
            />
          </div>

          {hasProjectDir && (
            <div>
              <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('kiroConfig.scope')}</label>
              <Select
                value={scope}
                onChange={setScope}
                data={[
                  { value: 'user', label: t('kiroConfig.scopeUser') },
                  { value: 'project', label: t('kiroConfig.scopeProject') },
                ]}
                size="md"
                classNames={{
                  input: `${colors.text} ${colors.input} ${colors.inputFocus}`,
                  dropdown: `${colors.card} border ${colors.cardBorder}`,
                  option: `${colors.text}`
                }}
                styles={{
                  input: {
                    borderRadius: '0.5rem',
                    backgroundColor: surface.inputBg,
                    borderColor: surface.inputBorder,
                    color: surface.inputText,
                  },
                  dropdown: {
                    backgroundColor: surface.dropdownBg,
                    borderColor: surface.dropdownBorder,
                  },
                  option: {
                    color: surface.inputText,
                    backgroundColor: 'transparent',
                  }
                }}
              />
            </div>
          )}

          <button
            onClick={() => onCreate(agentName.trim(), description.trim(), tools, model, scope)}
            disabled={!agentName.trim()}
            className={`w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default AgentsPanel
