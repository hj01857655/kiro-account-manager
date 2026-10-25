import { useState, useEffect, useMemo } from 'react'
import { X, Terminal, AlertCircle, Wand2, ClipboardPaste, Check, AlertTriangle } from 'lucide-react'
import { getMcpConfig, saveMcpServer } from '../../../api/mcpApi'
import { useApp } from '../../../hooks/useApp'
import { MCP_TEMPLATES } from './MCPTemplates'
import { showSuccess } from '../../../utils/toast'
import { getThemeAccent, getGradientAccentButton } from './themeAccent'
import React from 'react'

const DUPLICATE_STRATEGY_KEY = 'mcpDuplicateStrategy'
const SCOPE_KEY = 'mcpTargetScope'

function AddMCPModal({ onClose, onSuccess, projectDir, readOnly }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const accentGradientButtonClass = getGradientAccentButton(accent)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    info: 'bg-primary/10 ring-1 ring-primary/15'
  }

  const [jsonConfig, setJsonConfig] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [parseResult, setParseResult] = useState<any>(null)
  const [existingServers, setExistingServers] = useState<string[]>([])
  const [duplicates, setDuplicates] = useState<string[]>([])
  const [duplicateStrategy, setDuplicateStrategy] = useState(() => localStorage.getItem(DUPLICATE_STRATEGY_KEY) || 'skip')
  // 保存层级：有项目目录时可选，默认跟 projectDir；持久化用户上次的选择
  const [targetScope, setTargetScope] = useState<'user' | 'project'>(
    () => (localStorage.getItem(SCOPE_KEY) as 'user' | 'project') || (projectDir ? 'project' : 'user')
  )
  const saveDir = targetScope === 'project' && projectDir ? projectDir : null

  const getUniqueServerName = (baseName: string, occupiedNames: Set<string>) => {
    let i = 1
    let candidate = `${baseName}-${i}`
    while (occupiedNames.has(candidate)) {
      i += 1
      candidate = `${baseName}-${i}`
    }
    return candidate
  }

  // 加载现有服务列表
  useEffect(() => {
    getMcpConfig(projectDir || null).then(config => {
      setExistingServers(Object.keys(config.mcpServers || {}))
    }).catch(() => {})
  }, [projectDir])

  // 持久化冲突处理策略
  useEffect(() => {
    localStorage.setItem(DUPLICATE_STRATEGY_KEY, duplicateStrategy)
  }, [duplicateStrategy])

  // 持久化保存层级选择
  useEffect(() => {
    localStorage.setItem(SCOPE_KEY, targetScope)
  }, [targetScope])

  // 初始化示例
  useEffect(() => {
    const example = { 'server-name': { command: 'uvx', args: ['package-name'] } }
    setJsonConfig(JSON.stringify(example, null, 2))
  }, [])

  // 实时解析 JSON
  useEffect(() => {
    if (!jsonConfig.trim()) {
      setParseResult(null)
      setDuplicates([])
      return
    }
    try {
      const parsed = JSON.parse(jsonConfig)
      const servers: any[] = []

      // 格式1: { mcpServers: { name: config, ... } }
      if (parsed.mcpServers && typeof parsed.mcpServers === 'object') {
        for (const [name, config] of Object.entries(parsed.mcpServers)) {
          if ((config as any).command) servers.push({ name, config })
        }
      }
      // 格式2: { name: config, ... }
      else if (typeof parsed === 'object' && !parsed.command) {
        for (const [name, config] of Object.entries(parsed)) {
          if (config && typeof config === 'object' && (config as any).command) {
            servers.push({ name, config })
          }
        }
      }
      // 格式3: { command: ... } - 单个配置
      else if (parsed.command) {
        setParseResult({ servers: [], error: t('mcp.invalidFormat') })
        setDuplicates([])
        return
      }

      if (servers.length === 0) {
        setParseResult({ servers: [], error: t('mcp.noValidConfig') })
        setDuplicates([])
      } else {
        setParseResult({ servers, error: null })
        // 检测重名
        const dups = servers.filter(s => existingServers.includes(s.name)).map(s => s.name)
        setDuplicates(dups)
      }
    } catch (e) {
      setParseResult({ servers: [], error: t('mcp.errJsonFormat') })
      setDuplicates([])
    }
  }, [jsonConfig, existingServers, t])

  // 应用模板
  const applyTemplate = (templateName: string) => {
    const config = { [templateName]: (MCP_TEMPLATES as any)[templateName] }
    setJsonConfig(JSON.stringify(config, null, 2))
    setError('')
  }

  // 格式化 JSON
  const formatJson = () => {
    try {
      const parsed = JSON.parse(jsonConfig)
      setJsonConfig(JSON.stringify(parsed, null, 2))
    } catch {}
  }

  // 从剪贴板粘贴
  const pasteFromClipboard = async () => {
    try {
      const text = await navigator.clipboard.readText()
      const parsed = JSON.parse(text)
      setJsonConfig(JSON.stringify(parsed, null, 2))
      setError('')
    } catch {
      setError(t('mcp.errClipboardJson'))
    }
  }

  // 保存
  const handleSave = async () => {
    if (readOnly) return
    if (!parseResult || parseResult.servers.length === 0) {
      setError(parseResult?.error || t('mcp.msgNoValidConfig'))
      return
    }

    setSaving(true)
    setError('')

    let latestServers = existingServers
    try {
      const latestConfig = await getMcpConfig(projectDir || null)
      latestServers = Object.keys(latestConfig.mcpServers || {})
      setExistingServers(latestServers)
    } catch {
      // 读取失败时回退到已缓存列表
    }

    const occupiedNames = new Set(latestServers)
    const results: any = { success: [], failed: [], skipped: [], renamed: [] }

    for (const { name, config } of parseResult.servers) {
      let targetName = name

      if (occupiedNames.has(name)) {
        if (duplicateStrategy === 'skip') {
          results.skipped.push(name)
          continue
        }
        if (duplicateStrategy === 'rename') {
          targetName = getUniqueServerName(name, occupiedNames)
          results.renamed.push({ from: name, to: targetName })
        }
      }

      try {
        const finalConfig = {
          command: config.command,
          args: config.args || [],
          env: config.env || {},
          disabled: config.disabled ?? false,
          autoApprove: config.autoApprove || []
        }
        await saveMcpServer(targetName, finalConfig, saveDir)
        results.success.push(targetName)
        occupiedNames.add(targetName)
      } catch (e) {
        results.failed.push({ name: targetName, error: String(e) })
      }
    }

    setSaving(false)

    if (results.failed.length > 0) {
      const failedNames = results.failed.map((f: any) => f.name).join(', ')
      const summary = [t('mcp.resPartialFailed', { names: failedNames })]
      if (results.success.length > 0) summary.push(t('mcp.resSucceeded', { names: results.success.join(', ') }))
      if (results.skipped.length > 0) summary.push(t('mcp.resSkipped', { names: results.skipped.join(', ') }))
      if (results.renamed.length > 0) summary.push(t('mcp.resRenamed', { names: results.renamed.map((r: any) => `${r.from}→${r.to}`).join(', ') }))
      setError(summary.join(t('mcp.listSep')))
      return
    }

    if (results.success.length === 0 && results.skipped.length > 0) {
      setError(t('mcp.resNoneAdded', { names: results.skipped.join(', ') }))
      return
    }

    const summaryParts = []
    if (results.success.length > 0) summaryParts.push(t('mcp.resAdded', { count: results.success.length }))
    if (results.skipped.length > 0) summaryParts.push(t('mcp.resSkippedCount', { count: results.skipped.length }))
    if (results.renamed.length > 0) summaryParts.push(t('mcp.resRenamedCount', { count: results.renamed.length }))
    if (summaryParts.length > 0) {
      showSuccess(t('mcp.resProcessed', { parts: summaryParts.join(t('mcp.partsSep')) }))
    }

    onSuccess()
  }

  const serverCount = parseResult?.servers?.length || 0
  const serverNames = parseResult?.servers?.map((s: any) => s.name) || []
  // 限制显示的名称数量
  const displayNames = serverNames.length > 3 
    ? serverNames.slice(0, 3).join(', ') + ' ' + t('mcp.namesEtc', { count: serverNames.length })
    : serverNames.join(', ')

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div 
        className={`relative overflow-hidden surface-card border border-border rounded-lg shadow-2xl w-[560px] max-w-full max-h-[85vh] flex flex-col`}
        onClick={e => e.stopPropagation()}
      >
        {/* 顶部渐变装饰 */}
        <div className={`absolute top-0 left-0 right-0 h-24 ${accent.bgSoft} pointer-events-none`} />
        
        {/* 标题 */}
        <div className={`relative flex items-center justify-between px-6 py-4 border-b border-border`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-lg bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-lg ${accent.shadow}`}>
              <Terminal size={20} className="text-white" />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('mcpManager.addMCPServer')}</h2>
          </div>
          <button
            onClick={onClose}
            className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
          >
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        {/* 内容 */}
        <div className="relative flex-1 overflow-auto p-6 space-y-4">
          {/* 快速模板 */}
          <div>
            <label className={`block text-xs text-muted-foreground mb-1.5`}>{t('mcp.quickFill')}</label>
            <div className="flex flex-wrap gap-1.5">
              {Object.keys(MCP_TEMPLATES).map(key => (
                <button
                  key={key}
                  onClick={() => applyTemplate(key)}
                  className={`cursor-pointer px-2.5 py-1 text-xs rounded-lg border border-border bg-background border-input ${accent.hoverBorder} text-foreground transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                >
                  {key}
                </button>
              ))}
            </div>
          </div>

          {/* JSON 配置 */}
          <div className="flex-1">
            <div className="flex items-center justify-between mb-1.5">
              <div className="flex items-center gap-2 flex-1 min-w-0">
                <label className={`text-xs text-muted-foreground shrink-0`}>{t('mcp.configLabel')}</label>
                {serverCount > 0 && !parseResult?.error && (
                  <span className="text-xs text-green-500 flex items-center gap-1 truncate">
                    <Check size={12} className="shrink-0" />
                    <span className="truncate">{displayNames}</span>
                  </span>
                )}
                {parseResult?.error && (
                  <span className="text-xs text-red-500 flex items-center gap-1">
                    <AlertCircle size={12} />
                    {parseResult.error}
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={pasteFromClipboard}
                  className={`cursor-pointer text-xs text-muted-foreground ${accent.textHover} flex items-center gap-1 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                >
                  <ClipboardPaste size={12} />
                  {t('common.paste')}
                </button>
                <button
                  onClick={formatJson}
                  className={`cursor-pointer text-xs text-muted-foreground ${accent.textHover} flex items-center gap-1 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                >
                  <Wand2 size={12} />
                  {t('common.format')}
                </button>
              </div>
            </div>
            <textarea
              value={jsonConfig}
              onChange={e => setJsonConfig(e.target.value)}
              rows={12}
              spellCheck={false}
              className={`w-full px-3 py-2 text-xs border rounded-lg font-mono resize-none bg-background border-input text-foreground ${colors.inputFocus} focus:ring-2`}
            />
            <div className={`text-xs text-muted-foreground mt-1.5`}>
              {t('mcp.jsonFormatsHint')}
            </div>
          </div>

          {/* 保存层级：仅选择了项目目录时可选 */}
          {projectDir && (
            <div className="flex items-center gap-2 text-xs">
              <span className="text-muted-foreground">{t('mcp.saveToLabel')}</span>
              <button
                onClick={() => setTargetScope('user')}
                className={`cursor-pointer px-2 py-1 rounded border text-xs transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${targetScope === 'user' ? `${accent.border} ${accent.bgSoft}` : `border-border hover:bg-muted/50`}`}
              >
                {t('mcp.saveToGlobal')}
              </button>
              <button
                onClick={() => setTargetScope('project')}
                className={`cursor-pointer px-2 py-1 rounded border text-xs transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${targetScope === 'project' ? `${accent.border} ${accent.bgSoft}` : `border-border hover:bg-muted/50`}`}
              >
                {t('mcp.saveToProject')}
              </button>
            </div>
          )}

          {/* 重名处理 */}
          {duplicates.length > 0 && (
            <div className="text-xs text-amber-500 bg-amber-500/10 px-3 py-2 rounded-lg flex flex-col gap-2">
              <div className="flex items-start gap-2">
                <AlertTriangle size={14} className="shrink-0 mt-0.5" />
                <span>
                  {t('mcp.dupExistsPrefix')} {duplicates.join(', ')}
                  {duplicateStrategy === 'overwrite' && t('mcp.dupOverwriteNote')}
                  {duplicateStrategy === 'skip' && t('mcp.dupSkipNote')}
                  {duplicateStrategy === 'rename' && t('mcp.dupRenameNote')}
                </span>
              </div>
              <div className="flex items-center gap-1.5 pl-6">
                <span className={`text-muted-foreground`}>{t('mcp.conflictLabel')}</span>
                <button
                  onClick={() => setDuplicateStrategy('skip')}
                  className={`cursor-pointer px-2 py-1 rounded border text-xs transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${duplicateStrategy === 'skip' ? `${accent.border} ${accent.bgSoft}` : `border-border hover:bg-muted/50`}`}
                >
                  {t('mcp.conflictSkip')}
                </button>
                <button
                  onClick={() => setDuplicateStrategy('overwrite')}
                  className={`cursor-pointer px-2 py-1 rounded border text-xs transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${duplicateStrategy === 'overwrite' ? `${accent.border} ${accent.bgSoft}` : `border-border hover:bg-muted/50`}`}
                >
                  {t('mcp.conflictOverwrite')}
                </button>
                <button
                  onClick={() => setDuplicateStrategy('rename')}
                  className={`cursor-pointer px-2 py-1 rounded border text-xs transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${duplicateStrategy === 'rename' ? `${accent.border} ${accent.bgSoft}` : `border-border hover:bg-muted/50`}`}
                >
                  {t('mcp.conflictRename')}
                </button>
              </div>
            </div>
          )}

          {/* 错误提示 */}
          {error && (
            <div className="text-red-500 text-xs bg-red-500/10 px-3 py-2 rounded-lg">{error}</div>
          )}
        </div>

        {/* 底部按钮 */}
        <div className={`relative flex justify-end gap-3 px-6 py-4 border-t border-border`}>
          <button
            onClick={onClose}
            className={`cursor-pointer px-5 py-2.5 rounded-lg text-sm text-foreground hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
          >
            {t('common.cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={saving || serverCount === 0 || readOnly}
            className={`cursor-pointer px-6 py-2.5 ${accentGradientButtonClass} rounded-lg text-sm font-medium disabled:opacity-50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
          >
            {saving ? t('common.saving') : serverCount > 1 ? t('mcp.addCount', { count: serverCount }) : t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default AddMCPModal
