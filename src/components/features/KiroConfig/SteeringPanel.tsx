import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { FileText, RefreshCw, Trash2, Save, Plus, X, Globe, FolderOpen, Wand2, Sparkles } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import {
  getThemeAccent,
  getSolidAccentButton,
  getGradientAccentButton,
  getThemeSurfaceStyles} from './themeAccent'
import { handleUiError } from '../../../utils/errorLogger'
import React from 'react'

// 解析 front-matter（v0.10.32: inclusion + name + description + fileMatchPattern）
const parseFrontMatter = (content: string) => {
  const match = content.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
  if (!match) return { inclusion: 'always', filePattern: '', name: '', description: '', body: content }
  const [, fm, body] = match
  return {
    inclusion: fm.match(/inclusion:\s*(\w+)/)?.[1] || 'always',
    filePattern: fm.match(/fileMatchPattern:\s*['"]?([^'"\n]+)['"]?/)?.[1] || '',
    name: fm.match(/name:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    description: fm.match(/description:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    body
  }
}

// 组装 front-matter（v0.10.32: inclusion + name + description + fileMatchPattern）
const buildContent = (inclusion: string, filePattern: string, body: string, name: string, description: string) => {
  let fm = `---\ninclusion: ${inclusion}`
  if (name?.trim()) fm += `\nname: "${name.trim()}"`
  if (description?.trim()) fm += `\ndescription: "${description.trim()}"`
  if (inclusion === 'fileMatch' && filePattern.trim()) fm += `\nfileMatchPattern: '${filePattern.trim()}'`
  return fm + '\n---\n' + body
}

// 格式化文件大小
const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// scope 徽章
const ScopeBadge = ({ scope, accent, t }: any) => {
  if (scope === 'project') {
    return (
      <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-500 border border-amber-500/30">
        <FolderOpen size={10} />{t('common.project')}
      </span>
    )
  }
  return (
    <span className={`flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium ${accent.scopeBadge}`}>
      <Globe size={10} />{t('common.user')}
    </span>
  )
}

function SteeringPanel({ onCountChange, projectDir }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm, showSuccess } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  
  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10'
  }
  
  const [files, setFiles] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedFile, setSelectedFile] = useState<any>(null)
  const [editState, setEditState] = useState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [refining, setRefining] = useState(false)
  const [creatingDefault, setCreatingDefault] = useState(false)
  const [initializingProject, setInitializingProject] = useState(false)

  const loadFiles = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<any[]>('get_steering_files', { projectDir: projectDir || null })
      setFiles(data)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('load Steering files failed', e, { userMessage: t('steering.loadFailed') })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir, t])

  useEffect(() => {
    setSelectedFile(null)
    setEditState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
    setHasChanges(false)
    loadFiles()
  }, [loadFiles])

  const handleSelect = async (file: any) => {
    if (hasChanges && !await showConfirm(t('steering.unsavedChanges'), t('steering.confirmSwitch'))) return
    setSelectedFile(file)
    const parsed = parseFrontMatter(file.content)
    setEditState({ content: parsed.body, inclusion: parsed.inclusion, filePattern: parsed.filePattern, name: parsed.name, description: parsed.description })
    setHasChanges(false)
  }

  const updateEditState = (key: string, value: any) => {
    const newState = { ...editState, [key]: value }
    setEditState(newState)
    if (selectedFile) {
      const newContent = buildContent(newState.inclusion, newState.filePattern, newState.content, newState.name, newState.description)
      setHasChanges(newContent !== selectedFile.content)
    }
  }

  const handleSave = async () => {
    if (!selectedFile) return
    setSaving(true)
    try {
      const fullContent = buildContent(editState.inclusion, editState.filePattern, editState.content, editState.name, editState.description)
      await invoke('save_steering_file', {
        fileName: selectedFile.fileName,
        content: fullContent,
        scope: selectedFile.scope,
        projectDir: projectDir || null
      })
      setFiles(files.map(f => (f.fileName === selectedFile.fileName && f.scope === selectedFile.scope) ? { ...f, content: fullContent } : f))
      setSelectedFile({ ...selectedFile, content: fullContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('save Steering file failed', e, { userMessage: t('steering.saveFailed') })
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (file: any) => {
    if (!await showConfirm(t('steering.confirmDelete'), t('steering.confirmDeleteFile', { fileName: file.fileName }))) return
    try {
      await invoke('delete_steering_file', {
        fileName: file.fileName,
        scope: file.scope,
        projectDir: projectDir || null
      })
      const newFiles = files.filter(f => !(f.fileName === file.fileName && f.scope === file.scope))
      setFiles(newFiles)
      onCountChange?.(newFiles.length)
      if (selectedFile?.fileName === file.fileName && selectedFile?.scope === file.scope) {
        setSelectedFile(null)
        setEditState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
        setHasChanges(false)
      }
    } catch (e) {
      handleUiError('delete Steering file failed', e, { userMessage: t('steering.deleteFailed') })
    }
  }

  const handleCreate = async (fileName: string, inclusion: string, filePattern: string, scope: string, name: string, description: string) => {
    const fName = fileName.endsWith('.md') ? fileName : `${fileName}.md`
    const content = buildContent(inclusion, filePattern, `\n${t('steering.rulePlaceholder')}\n`, name, description)
    try {
      const newFile = await invoke<any>('create_steering_file', {
        fileName: fName,
        content,
        scope,
        projectDir: projectDir || null
      })
      const newFiles = [...files, newFile]
      setFiles(newFiles)
      onCountChange?.(newFiles.length)
      setShowCreateModal(false)
      handleSelect(newFile)
    } catch (e) {
      handleUiError('create Steering file failed', e, { userMessage: t('steering.createFailed') })
    }
  }

  const resolveScope = async () => {
    if (!projectDir) return 'user'
    const useProjectScope = await showConfirm(
      t('steering.scopeTitle'),
      t('steering.scopeMessage'),
      {
        confirmText: t('kiroConfig.scopeProject'),
        cancelText: t('kiroConfig.scopeUser')}
    )
    return useProjectScope ? 'project' : 'user'
  }

  const upsertFile = (nextFile: any) => {
    const nextFiles = [
      ...files.filter(file => !(file.fileName === nextFile.fileName && file.scope === nextFile.scope)),
      nextFile
    ]
    setFiles(nextFiles)
    onCountChange?.(nextFiles.length)
    handleSelect(nextFile)
  }

  const handleCreateDefault = async () => {
    setCreatingDefault(true)
    try {
      const scope = await resolveScope()
      const created = await invoke<any>('create_default_steering_file', {
        scope,
        projectDir: projectDir || null})
      upsertFile(created)
      showSuccess(t('steering.defaultCreated'), created.fileName)
    } catch (e) {
      handleUiError('create default Steering template failed', e, { userMessage: t('steering.createDefaultFailed') })
    } finally {
      setCreatingDefault(false)
    }
  }

  const handleCreateInitial = async () => {
    if (!projectDir) return
    setInitializingProject(true)
    try {
      const created = await invoke<any>('create_initial_project_steering', { projectDir })
      const createdFiles = Array.isArray(created) ? created : []
      if (createdFiles.length > 0) {
        const merged = [...files]
        for (const file of createdFiles) {
          const index = merged.findIndex(item => item.fileName === file.fileName && item.scope === file.scope)
          if (index >= 0) {
            merged[index] = file
          } else {
            merged.push(file)
          }
        }
        setFiles(merged)
        onCountChange?.(merged.length)
        handleSelect(createdFiles[0])
      }
      showSuccess(t('steering.initialCreated'), projectDir)
    } catch (e) {
      handleUiError('initialize project Steering failed', e, { userMessage: t('steering.initializeFailed') })
    } finally {
      setInitializingProject(false)
    }
  }

  const handleRefine = async () => {
    if (!selectedFile) return
    setRefining(true)
    try {
      const refined = await invoke<any>('refine_steering_file', {
        fileName: selectedFile.fileName,
        scope: selectedFile.scope,
        projectDir: projectDir || null})
      upsertFile(refined)
      showSuccess(t('steering.refineSuccess'), refined.fileName)
    } catch (e) {
      handleUiError('refine Steering file failed', e, { userMessage: t('steering.refineFailed') })
    } finally {
      setRefining(false)
    }
  }

  const inclusionOptions = [
    { value: 'always', label: t('steering.inclusionAlways'), desc: t('steering.inclusionAlwaysDesc') },
    { value: 'auto', label: t('steering.inclusionAuto'), desc: t('steering.inclusionAutoDesc') },
    { value: 'fileMatch', label: t('steering.inclusionFileMatch'), desc: t('steering.inclusionFileMatchDesc') },
    { value: 'manual', label: t('steering.inclusionManual'), desc: t('steering.inclusionManualDesc') },
  ]

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
      </div>
    )
  }

  return (
    <div className="h-full flex gap-3 p-4">
      {/* 左侧列表 */}
      <FileList
        files={files}
        selectedFile={selectedFile}
        onSelect={handleSelect}
        onDelete={handleDelete}
        onRefresh={loadFiles}
        onCreate={() => setShowCreateModal(true)}
        onCreateDefault={handleCreateDefault}
        onCreateInitial={handleCreateInitial}
        creatingDefault={creatingDefault}
        initializingProject={initializingProject}
        hasProjectDir={!!projectDir}
        accent={accent}
        colors={colors}
        t={t}
      />

      {/* 右侧编辑器 */}
      <div className={`flex-1 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        {selectedFile ? (
          <Editor
            file={selectedFile}
            editState={editState}
            hasChanges={hasChanges}
            saving={saving}
            inclusionOptions={inclusionOptions}
            onContentChange={(v: string) => updateEditState('content', v)}
            onInclusionChange={(v: string) => updateEditState('inclusion', v)}
            onFilePatternChange={(v: string) => updateEditState('filePattern', v)}
            onNameChange={(v: string) => updateEditState('name', v)}
            onDescriptionChange={(v: string) => updateEditState('description', v)}
            onSave={handleSave}
            onRefine={handleRefine}
            refining={refining}
            surface={surface}
            accent={accent}
            colors={colors}
            t={t}
          />
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <FileText size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('steering.selectToEdit')}</p>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && (
        <CreateModal
          inclusionOptions={inclusionOptions}
          onCreate={handleCreate}
          onClose={() => setShowCreateModal(false)}
          accent={accent}
          colors={colors}
          t={t}
          hasProjectDir={!!projectDir}
        />
      )}
    </div>
  )
}

// inclusion 模式配色映射
const getInclusionStyles = (accent: any, t: any): any => ({
  always:    { color: 'text-green-500',  bg: 'bg-green-500/15', border: 'border-green-500/30', dot: 'bg-green-500', label: t('steering.inclusionAlways') },
  auto:      { color: accent.text, bg: accent.bgSoft, border: accent.borderSoft, dot: accent.solidBg, label: t('steering.inclusionAuto') },
  fileMatch: { color: accent.text, bg: accent.bgSoft, border: accent.borderSoft, dot: accent.solidBg, label: t('steering.inclusionFileMatch') },
  manual:    { color: 'text-orange-500', bg: 'bg-orange-500/15', border: 'border-orange-500/30', dot: 'bg-orange-500', label: t('steering.inclusionManual') }})

// inclusion 徽章
const InclusionBadge = ({ inclusion, accent, t }: any) => {
  const styles = getInclusionStyles(accent, t)
  const s = styles[inclusion] || styles.always
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium ${s.bg} ${s.color} border ${s.border}`}>
      <span className={`w-1.5 h-1.5 rounded-full ${s.dot}`} />
      {s.label}
    </span>
  )
}

// 文件列表组件
function FileList({ files, selectedFile, onSelect, onDelete, onRefresh, onCreate, onCreateDefault, onCreateInitial, creatingDefault, initializingProject, hasProjectDir, accent, colors, t }: any) {
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const inclusionStyles = getInclusionStyles(accent, t)
  // 按 inclusion 分组（保持顺序）
  const groups = [
    { key: 'always',    label: t('steering.groupAlways') },
    { key: 'auto',      label: t('steering.groupAuto') },
    { key: 'fileMatch', label: t('steering.groupFileMatch') },
    { key: 'manual',    label: t('steering.groupManual') },
  ].map(g => ({
    ...g,
    files: files.filter((f: any) => parseFrontMatter(f.content).inclusion === g.key),
    style: inclusionStyles[g.key]})).filter(g => g.files.length > 0)

  return (
    <div className={`w-72 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
      <div className={`p-3 border-b border-border flex items-center justify-between`}>
        <div className="flex items-center gap-2">
          <FileText size={18} className={accent.text} />
          <span className={`text-sm font-semibold text-foreground`}>Steering</span>
          <span className={`text-xs text-muted-foreground`}>({files.length})</span>
        </div>
        <div className="flex gap-2">
          <button
            onClick={onCreateDefault}
            disabled={creatingDefault}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50 cursor-pointer`}
            title={t('steering.defaultTemplate')}
          >
            <Wand2 size={16} className={accent.text} />
          </button>
          {hasProjectDir && (
            <button
              onClick={onCreateInitial}
              disabled={initializingProject}
              className={`p-2 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50 cursor-pointer`}
              title={t('steering.initializeProject')}
            >
              <Sparkles size={16} className={accent.text} />
            </button>
          )}
          <button
            onClick={onCreate}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer`}
            title={t('steering.newSteering')}
          >
              <Plus size={16} className={accent.text} />
          </button>
          <button
            onClick={onRefresh}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer`}
            title={t('common.refresh')}
          >
            <RefreshCw size={16} className={"text-muted-foreground"} />
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-auto p-4">
        {files.length === 0 ? (
          <div className={`text-center py-16 text-muted-foreground`}>
            <FileText size={48} className="mx-auto mb-3 opacity-20" />
            <p className="text-sm">{t('steering.noFiles')}</p>
            <button
              onClick={onCreate}
              className={`mt-4 px-4 py-2 rounded-lg text-sm transition-colors cursor-pointer ${accentSolidButtonClass}`}
            >
              {t('steering.newSteering')}
            </button>
          </div>
        ) : (
          <div className="space-y-5">
            {groups.map(group => (
              <div key={group.key}>
                {/* 分组标题 - 紧凑风格 */}
                <div className={`flex items-center gap-2 mb-2 px-1`}>
                  <span className={`w-2 h-2 rounded-full ${group.style.dot}`} />
                  <span className={`text-xs font-semibold text-muted-foreground uppercase tracking-wider`}>{group.label}</span>
                  <span className={`text-[10px] text-muted-foreground opacity-60`}>{group.files.length}</span>
                  <div className={`flex-1 h-px border-border opacity-50`} />
                </div>

                {/* 文件卡片 */}
                <div className="space-y-2">
                  {group.files.map((file: any) => {
                    const parsed = parseFrontMatter(file.content)
                    const isSelected = selectedFile?.fileName === file.fileName && selectedFile?.scope === file.scope
                    return (
                      <div
                        key={`${file.scope}-${file.fileName}`}
                        onClick={() => onSelect(file)}
                        className={`p-3 rounded-xl cursor-pointer group transition-all duration-200 ${
                          isSelected
                            ? `${accent.bg} ring-2 ${accent.ring} shadow-lg border ${accent.border}`
                            : `glass-card border border-border hover:bg-muted/50 hover:shadow-md`
                        }`}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center gap-2.5 flex-1 min-w-0">
                            <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 transition-colors ${
                              isSelected ? accent.bg : "bg-muted/30"
                            }`}>
                              <FileText size={15} className={isSelected ? accent.text : "text-muted-foreground"} />
                            </div>
                            <div className="flex-1 min-w-0">
                              <span className={`font-semibold text-sm ${isSelected ? accent.text : "text-foreground"} truncate block leading-tight`}>
                                {parsed.name || file.fileName.replace('.md', '')}
                              </span>
                              {parsed.description && (
                                <span className={`text-xs text-muted-foreground truncate block mt-0.5 leading-tight`}>{parsed.description}</span>
                              )}
                            </div>
                          </div>
                          <button
                            onClick={(e) => { e.stopPropagation(); onDelete(file) }}
                            className="cursor-pointer opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                            title={t('common.delete')}
                          >
                            <Trash2 size={14} className="text-red-500" />
                          </button>
                        </div>
                        <div className={`flex items-center gap-2 text-xs text-muted-foreground mt-2 flex-wrap`} style={{ marginLeft: '2.375rem' }}>
                          <ScopeBadge scope={file.scope} accent={accent} t={t} />
                          <span className={`px-1.5 py-0.5 rounded bg-muted/30 text-[10px] font-medium`}>
                            {formatSize(file.size)}
                          </span>
                          {parsed.filePattern && (
                            <code className={`px-1.5 py-0.5 rounded ${accent.bgSoft} border ${accent.borderSoft} font-mono text-[10px] ${accent.textSoft} truncate max-w-[120px]`}>
                              {parsed.filePattern}
                            </code>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// 编辑器组件
function Editor({ file, editState, hasChanges, saving, refining, inclusionOptions, onContentChange, onInclusionChange, onFilePatternChange, onNameChange, onDescriptionChange, onSave, onRefine, surface, accent, colors, t }: any) {
  const accentSolidButtonClass = getSolidAccentButton(accent)
  return (
    <>
      <div className={`p-3 border-b border-border flex items-center justify-between`}>
        <div className="flex items-center gap-2">
          <h3 className={`font-semibold text-foreground`}>{file.fileName}</h3>
          <ScopeBadge scope={file.scope} accent={accent} t={t} />
          {hasChanges && <span className="text-xs text-orange-500">● {t('steering.save')}</span>}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={onRefine}
            disabled={refining}
            className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} bg-muted/30 disabled:opacity-50`}
          >
            <Wand2 size={14} />
            {refining ? t('steering.refining') : t('steering.refine')}
          </button>
          <button
            onClick={onSave}
            disabled={!hasChanges || saving}
            className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${
              hasChanges ? accentSolidButtonClass : colors.btnDisabled
            } disabled:opacity-50`}
          >
            <Save size={14} />
            {saving ? t('steering.saving') : t('steering.save')}
          </button>
        </div>
      </div>
      {/* Frontmatter editor */}
      <div className={`px-4 py-3 border-b border-border space-y-2`}>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <span className={`text-xs text-muted-foreground`}>{t('steering.inclusionMode')}:</span>
            <Select value={editState.inclusion} onValueChange={onInclusionChange}>
              <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ minWidth: '120px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}>
                <SelectValue placeholder={t('steering.selectMode', { defaultValue: 'Select mode...' })} />
              </SelectTrigger>
              <SelectContent className={`glass-card border border-border`}>
                {inclusionOptions.map((opt: any) => (
                  <SelectItem key={opt.value} value={opt.value} className={"text-foreground"}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {editState.inclusion === 'fileMatch' && (
            <div className="flex items-center gap-2">
              <span className={`text-xs text-muted-foreground`}>{t('steering.filePattern')}:</span>
              <Input
                value={editState.filePattern}
                onChange={(e) => onFilePatternChange(e.target.value)}
                placeholder="**/*.jsx"
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ width: '128px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
              />
            </div>
          )}
        </div>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <span className={`text-xs text-muted-foreground`}>{t('steering.fmName')}:</span>
            <Input
              value={editState.name}
              onChange={(e) => onNameChange(e.target.value)}
              placeholder={t('steering.fmNamePlaceholder')}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ width: '140px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
            />
          </div>
          <div className="flex items-center gap-2 flex-1">
            <span className={`text-xs text-muted-foreground`}>{t('steering.fmDescription')}:</span>
            <Input
              value={editState.description}
              onChange={(e) => onDescriptionChange(e.target.value)}
              placeholder={t('steering.fmDescriptionPlaceholder')}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ flex: 1, minWidth: '200px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
            />
          </div>
        </div>
      </div>
      <div className="flex-1 p-4 overflow-hidden">
        <Textarea
          value={editState.content}
          onChange={(e) => onContentChange(e.target.value)}
          placeholder={t('steering.contentPlaceholder')}
          className={`flex-1 w-full h-full min-h-[400px] p-4 rounded-xl text-sm leading-relaxed font-mono resize-none border border-input ${colors.inputFocus}`}
          style={{ 
            color: surface.editorText,
            backgroundColor: surface.editorBg,
            borderColor: surface.editorBorder
          }}
        />
      </div>
    </>
  )
}

// 创建弹窗组件
function CreateModal({ inclusionOptions, onCreate, onClose, accent, colors, t, hasProjectDir }: any) {
  const accentGradientButtonClass = getGradientAccentButton(accent)
  const [fileName, setFileName] = useState('')
  const [inclusion, setInclusion] = useState('always')
  const [filePattern, setFilePattern] = useState('')
  const [scope, setScope] = useState('user')
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`glass-card rounded-2xl w-full max-w-[380px] shadow-2xl border border-border overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <FileText size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('steering.newSteering')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fileName')}</label>
            <Input
              placeholder={t('steering.fileNamePlaceholder')}
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('steering.fileNameHint')}</p>
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fmName')}</label>
            <Input
              placeholder={t('steering.fmNamePlaceholder')}
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fmDescription')}</label>
            <Input
              placeholder={t('steering.fmDescriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          {hasProjectDir && (
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('kiroConfig.scope')}</label>
              <Select value={scope} onValueChange={setScope}>
                <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ borderRadius: '0.5rem' }}>
                  <SelectValue placeholder={t('kiroConfig.scopeUser')} />
                </SelectTrigger>
                <SelectContent className={`glass-card border border-border`}>
                  <SelectItem value="user" className={"text-foreground"}>{t('kiroConfig.scopeUser')}</SelectItem>
                  <SelectItem value="project" className={"text-foreground"}>{t('kiroConfig.scopeProject')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.inclusionMode')}</label>
            <Select value={inclusion} onValueChange={setInclusion}>
              <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ borderRadius: '0.5rem' }}>
                <SelectValue placeholder={t('steering.selectMode', { defaultValue: 'Select mode' })} />
              </SelectTrigger>
              <SelectContent className={`glass-card border border-border`}>
                {inclusionOptions.map((opt: any) => (
                  <SelectItem key={opt.value} value={opt.value} className={"text-foreground"}>
                    {opt.label} - {opt.desc}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {inclusion === 'fileMatch' && (
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.filePattern')}</label>
              <Input
                placeholder={t('steering.filePatternPlaceholder')}
                value={filePattern}
                onChange={(e) => setFilePattern(e.target.value)}
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ borderRadius: '0.5rem' }}
              />
            </div>
          )}

          <button
            onClick={() => onCreate(fileName, inclusion, filePattern, scope, name.trim(), description.trim())}
            disabled={!fileName.trim()}
            className={`cursor-pointer w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default SteeringPanel
