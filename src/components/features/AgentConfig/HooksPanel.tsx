import { useCallback, useEffect, useState, useMemo } from 'react'
import { getHooks, saveHook, deleteHook, createHook } from '../../../api/agentConfigApi'
import { Link2, Plus, RefreshCw, Save, Trash2, X } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { handleUiError } from '../../../utils/errorLogger'
import { getThemeAccent, getSolidAccentButton, getGradientAccentButton, getThemeSurfaceStyles } from './themeAccent'
import React from 'react'

const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// Kiro IDE 1.0 起 hook 文件改为 .json（单文件可含多个 hook，字段 trigger / action）；
// IDE 0.x 的 .kiro.hook（when / then）仍可识别，故两种后缀都接受，新建默认用 .json。
const HOOK_SUFFIX = '.json'
const HOOK_SUFFIX_LEGACY = '.kiro.hook'
const HOOK_BASE_NAME_RE = /^[A-Za-z0-9._-]+$/

// 去掉已知后缀，返回文件名主体
const stripHookSuffix = (name: string): string => {
  const lower = name.toLowerCase()
  if (lower.endsWith(HOOK_SUFFIX_LEGACY)) return name.slice(0, -HOOK_SUFFIX_LEGACY.length)
  if (lower.endsWith(HOOK_SUFFIX)) return name.slice(0, -HOOK_SUFFIX.length)
  return name
}

// 已带 .json / .kiro.hook 的原样返回，否则补 .json
const normalizeHookFileName = (raw: string): string => {
  const name = raw.trim()
  if (!name) return ''
  const lower = name.toLowerCase()
  if (lower.endsWith(HOOK_SUFFIX_LEGACY) || lower.endsWith(HOOK_SUFFIX)) return name
  return `${name}${HOOK_SUFFIX}`
}

// 去掉已知后缀后的主体必须只含 [A-Za-z0-9._-]
const isValidHookFileName = (raw: string): boolean => {
  const name = raw.trim()
  if (!name) return false
  return HOOK_BASE_NAME_RE.test(stripHookSuffix(name))
}

function HooksPanel({ onCountChange, projectDir, readOnly }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm, showError } = useDialog()
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

  const [hooks, setHooks] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedHook, setSelectedHook] = useState<any>(null)
  const [editContent, setEditContent] = useState('')
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)

  const loadHooks = useCallback(async () => {
    // 无项目目录时仍要加载：用户级 hooks（~/.kiro/hooks）不依赖项目。
    // 后端 get_hooks 的 project_dir 本就是 Option，缺省时只扫用户级目录。
    setLoading(true)
    try {
      const data = await getHooks(projectDir)
      setHooks(data)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('加载 Hooks 失败', e, { userMessage: t('hooks.loadFailed') || '加载 Hooks 失败' })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir, t])

  useEffect(() => {
    setSelectedHook(null)
    setEditContent('')
    setHasChanges(false)
    loadHooks()
  }, [loadHooks])

  const handleSelect = async (hookFile: any) => {
    if (hasChanges && !await showConfirm(t('hooks.unsavedChanges'), t('hooks.confirmSwitch'))) return
    setSelectedHook(hookFile)
    setEditContent(hookFile.content || '')
    setHasChanges(false)
  }

  const handleSave = async () => {
    if (readOnly || !selectedHook) return
    // 用户级 hook（~/.kiro/hooks）不依赖项目目录，只有项目级才要求
    if ((selectedHook.scope || 'project') !== 'user' && !projectDir) return

    setSaving(true)
    try {
      await saveHook(selectedHook.fileName, editContent, selectedHook.scope || 'project', projectDir)
      const newList = hooks.map(h => (h.fileName === selectedHook.fileName)
        ? { ...h, content: editContent }
        : h)
      setHooks(newList)
      setSelectedHook({ ...selectedHook, content: editContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('保存 Hook 失败', e, { userMessage: t('hooks.saveFailed') || '保存失败' })
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (hookFile: any) => {
    if (readOnly) return
    // 用户级 hook（~/.kiro/hooks）不依赖项目目录，只有项目级才要求
    if ((hookFile.scope || 'project') !== 'user' && !projectDir) return
    if (!await showConfirm(t('hooks.confirmDelete'), t('hooks.confirmDeleteFile', { fileName: hookFile.fileName }))) return
    try {
      await deleteHook(hookFile.fileName, hookFile.scope || 'project', projectDir)
      const next = hooks.filter(h => h.fileName !== hookFile.fileName)
      setHooks(next)
      onCountChange?.(next.length)
      if (selectedHook?.fileName === hookFile.fileName) {
        setSelectedHook(null)
        setEditContent('')
        setHasChanges(false)
      }
    } catch (e) {
      handleUiError('删除 Hook 失败', e, { userMessage: t('hooks.deleteFailed') || '删除失败' })
    }
  }

  const handleCreate = async (fileName: string, scope: string = 'project') => {
    if (readOnly) return false
    // 用户级（~/.kiro/hooks）不依赖项目目录
    if (scope !== 'user' && !projectDir) return false

    const raw = fileName.trim()
    if (!raw) {
      showError(t('hooks.createFailed'), t('hooks.fileNameRequired'))
      return false
    }

    const normalized = normalizeHookFileName(raw)
    if (!isValidHookFileName(raw)) {
      showError(t('hooks.createFailed'), t('hooks.fileNameInvalid'))
      return false
    }

    const exists = hooks.some(h => h.fileName.toLowerCase() === normalized.toLowerCase())
    if (exists) {
      showError(t('hooks.createFailed'), t('hooks.fileNameDuplicate'))
      return false
    }

    const baseName = stripHookSuffix(normalized)
    const isLegacy = normalized.toLowerCase().endsWith(HOOK_SUFFIX_LEGACY)

    // 新建模板按后缀给出对应格式：
    // .json -> IDE 1.0（version + hooks[] + trigger + action）
    // .kiro.hook -> IDE 0.x（when + then），仅供兼容旧环境
    const template = isLegacy
      ? `{
  "enabled": true,
  "name": "${baseName}",
  "description": "",
  "version": "1",
  "when": {
    "type": "userTriggered",
    "filePattern": null
  },
  "then": {
    "type": "askAgent",
    "prompt": "请在这里填写执行说明"
  },
  "workspaceFolderName": "",
  "shortName": "${baseName}",
  "fileName": "${normalized}"
}
`
      : `{
  "version": "v1",
  "hooks": [
    {
      "name": "${baseName}",
      "description": "",
      "trigger": "PostFileSave",
      "action": {
        "type": "command",
        "command": "echo TODO: 替换为实际命令"
      },
      "enabled": true
    }
  ]
}
`
    try {
      const newHook = await createHook(normalized, template, scope, projectDir)
      const next = [...hooks, newHook]
      setHooks(next)
      onCountChange?.(next.length)
      setShowCreateModal(false)
      handleSelect(newHook)
      return true
    } catch (e) {
      handleUiError('创建 Hook 失败', e, { userMessage: t('hooks.createFailed') || '创建失败' })
      return false
    }
  }

  if (loading) {
    return <div className="flex items-center justify-center h-full"><RefreshCw className={`animate-spin ${accent.text}`} size={24} /></div>
  }

  return (
    <div className="flex h-full min-h-0 gap-3 p-4 overflow-hidden">
      <div className={`w-72 min-h-0 flex flex-col glass-card border border-border rounded-xl overflow-hidden max-w-full`}>
        <div className={`p-4 border-b border-border`}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
            <Link2 size={18} className={accent.text} />
            <span className={`text-sm font-semibold text-foreground`}>{t('hooks.title')}</span>
            <span className={`text-xs text-muted-foreground`}>({hooks.length})</span>
          </div>
            <div className="flex gap-2">
              <button
                onClick={() => setShowCreateModal(true)}
                disabled={readOnly}
                className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
              >
                <Plus size={16} className={accent.text} />
              </button>
              <button onClick={loadHooks} className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`} title={t('common.refresh')}>
                <RefreshCw size={16} className="text-muted-foreground" />
              </button>
            </div>
          </div>
          <div className={`mt-2 text-[11px] text-muted-foreground leading-relaxed`}>{t('hooks.scopeHint')}</div>
        </div>

        <div className="flex-1 overflow-auto p-4">
          {hooks.length === 0 ? (
            <div className={`text-center py-16 text-muted-foreground`}>
              <Link2 size={48} className="mx-auto mb-3 opacity-20" />
              <p className="text-sm">{t('hooks.noHooks')}</p>
              <button
                onClick={() => setShowCreateModal(true)}
                disabled={readOnly}
                className={`cursor-pointer mt-4 px-4 py-2 rounded-lg text-sm transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${accentSolidButtonClass}`}
              >
                {t('common.add')}
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {hooks.map(h => {
                const isSelected = selectedHook?.fileName === h.fileName
                return (
                  <div
                    key={h.fileName}
                    onClick={() => handleSelect(h)}
                    className={`p-4 rounded-xl cursor-pointer group transition-all duration-200 ${
                      isSelected
                        ? `${accent.bg} ring-2 ${accent.ring} shadow-xl border-2 ${accent.border}`
                        : `glass-card border border-border hover:bg-muted/50 hover:shadow-lg`
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3 mb-2.5">
                      <div className="flex items-center gap-3 flex-1 min-w-0">
                        <div className={`flex items-center justify-center w-8 h-8 rounded-lg ${isSelected ? accent.bg : "bg-muted/30"}`}>
                          <Link2 size={16} className={isSelected ? accent.text : "text-muted-foreground"} />
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className={`font-semibold text-sm truncate ${isSelected ? accent.text : "text-foreground"}`}>{h.fileName}</div>
                        </div>
                      </div>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDelete(h) }}
                        disabled={readOnly}
                        className="cursor-pointer opacity-0 group-hover:opacity-100 p-2 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                      >
                        <Trash2 size={16} className="text-red-500" />
                      </button>
                    </div>
                    <div className={`flex items-center gap-2.5 text-xs text-muted-foreground ml-11`}>
                      <span className={`px-2 py-1 rounded-md bg-muted/30 font-medium`}>{formatSize(h.size)}</span>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>

      <div className={`flex-1 min-h-0 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        {selectedHook ? (
          <>
            <div className={`p-3 border-b border-border flex items-center justify-between`}>
              <div className="flex items-center gap-2">
                <h3 className={`font-semibold text-foreground`}>{selectedHook.fileName}</h3>
                {hasChanges && <span className="text-xs text-orange-500">● {t('hooks.unsaved')}</span>}
              </div>
              <button
                onClick={handleSave}
                disabled={readOnly || !hasChanges || saving}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all cursor-pointer ${hasChanges ? accentSolidButtonClass : colors.btnDisabled} disabled:opacity-50 disabled:cursor-not-allowed`}
              >
                <Save size={14} />
                {saving ? t('hooks.saving') : t('hooks.save')}
              </button>
            </div>
            <div className="flex-1 p-4 overflow-hidden">
              <textarea
                value={editContent}
                onChange={(e) => {
                  const next = e.target.value
                  setEditContent(next)
                  setHasChanges(next !== (selectedHook.content || ''))
                }}
                readOnly={readOnly}
                placeholder={t('hooks.contentPlaceholder')}
                className={`w-full h-full min-h-[400px] p-4 rounded-xl text-sm leading-relaxed font-mono resize-none ${colors.inputFocus}`}
                style={{
                  color: surface.editorText,
                  backgroundColor: surface.editorBg,
                  borderColor: surface.editorBorder}}
              />
            </div>
          </>
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <Link2 size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('hooks.selectToEdit')}</p>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && (
        <CreateHookModal
          onCreate={handleCreate}
          onClose={() => setShowCreateModal(false)}
          colors={colors}
          t={t}
          accent={accent}
          accentGradientButtonClass={accentGradientButtonClass}
          existingFileNames={hooks.map(h => h.fileName)}
          hasProjectDir={!!projectDir}
          readOnly={readOnly}
        />
      )}
    </div>
  )
}

function CreateHookModal({ onCreate, onClose, colors, t, accent, accentGradientButtonClass, existingFileNames, hasProjectDir, readOnly }: any) {
  const [fileName, setFileName] = useState('')
  const [creating, setCreating] = useState(false)
  // user = ~/.kiro/hooks（Kiro IDE 1.0.182+，对所有项目生效）；project = <project>/.kiro/hooks
  const [scope, setScope] = useState(hasProjectDir ? 'project' : 'user')

  const raw = fileName.trim()
  const normalized = normalizeHookFileName(raw)
  const invalidName = !!raw && !isValidHookFileName(raw)
  const duplicateName = normalized && existingFileNames.some((name: string) => name.toLowerCase() === normalized.toLowerCase())
  const canSubmit = !!raw && !invalidName && !duplicateName && !creating

  const handleSubmit = async () => {
    if (readOnly) return
    if (!canSubmit) return
    setCreating(true)
    try {
      await onCreate(raw, scope)
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div className={`glass-card rounded-2xl w-full max-w-[420px] shadow-2xl border border-border overflow-hidden`} onClick={(e) => e.stopPropagation()}>
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <Link2 size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('hooks.newHook')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('hooks.fileName')}</label>
            <input
              type="text"
              placeholder={t('hooks.fileNamePlaceholder')}
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  handleSubmit()
                }
              }}
              className={`w-full px-3 py-2 text-sm border rounded-lg text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2`}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('hooks.fileNameHint')}</p>
            {!!normalized && <p className={`text-xs mt-1 text-muted-foreground`}>{t('hooks.fileNamePreview')}: {normalized}</p>}
            {invalidName && <p className="text-xs mt-1 text-red-500">{t('hooks.fileNameInvalid')}</p>}
            {duplicateName && <p className="text-xs mt-1 text-red-500">{t('hooks.fileNameDuplicate')}</p>}
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('hooks.scope')}</label>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setScope('project')}
                disabled={!hasProjectDir}
                className={`flex-1 px-3 py-2 text-sm rounded-lg border transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${scope === 'project' ? 'border-primary bg-primary/10 text-foreground' : 'border-input text-muted-foreground hover:bg-muted/50'}`}
              >
                {t('hooks.scopeProject')}
              </button>
              <button
                type="button"
                onClick={() => setScope('user')}
                className={`flex-1 px-3 py-2 text-sm rounded-lg border transition-colors cursor-pointer ${scope === 'user' ? 'border-primary bg-primary/10 text-foreground' : 'border-input text-muted-foreground hover:bg-muted/50'}`}
              >
                {t('hooks.scopeUser')}
              </button>
            </div>
            <p className={`text-xs text-muted-foreground mt-1`}>
              {scope === 'user'
                ? t('hooks.scopeUserHint')
                : t('hooks.scopeProjectHint')}
            </p>
          </div>

          <button
            onClick={handleSubmit}
            disabled={readOnly || !canSubmit}
            className={`w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] cursor-pointer ${accentGradientButtonClass}`}
          >
            {creating ? t('hooks.saving') : t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default HooksPanel
