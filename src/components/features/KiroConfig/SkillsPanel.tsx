import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Puzzle, RefreshCw, Trash2, Save, Plus, X, FolderOpen, Globe, Download } from 'lucide-react'
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

// 格式化文件大小
const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// 解析 SKILL.md frontmatter（name + description 必填）
const parseSkillFrontMatter = (content: string) => {
  const match = content.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
  if (!match) return { name: '', description: '', body: content }
  const [, fm, body] = match
  return {
    name: fm.match(/name:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    description: fm.match(/description:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    body
  }
}

// 组装 SKILL.md frontmatter
const buildSkillContent = (name: string, description: string, body: string) => {
  let fm = '---'
  if (name) fm += `\nname: "${name}"`
  if (description) fm += `\ndescription: "${description}"`
  return fm + '\n---\n' + body
}

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

function SkillsPanel({ onCountChange, projectDir }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm, showSuccess } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const accentGradientButtonClass = getGradientAccentButton(accent)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10'
  }

  const [skills, setSkills] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedSkill, setSelectedSkill] = useState<any>(null)
  const [editState, setEditState] = useState({ name: '', description: '', body: '' })
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showGithubImportModal, setShowGithubImportModal] = useState(false)

  const loadSkills = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<any[]>('get_skills', { projectDir: projectDir || null })
      setSkills(data)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('load Skills failed', e, { userMessage: t('skills.loadFailed') })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir, t])

  useEffect(() => {
    setSelectedSkill(null)
    setEditState({ name: '', description: '', body: '' })
    setHasChanges(false)
    loadSkills()
  }, [loadSkills])

  const handleSelect = async (skill: any) => {
    if (hasChanges && !await showConfirm(t('skills.unsavedChanges'), t('skills.confirmSwitch'))) return
    setSelectedSkill(skill)
    const parsed = parseSkillFrontMatter(skill.content)
    setEditState({ name: parsed.name, description: parsed.description, body: parsed.body })
    setHasChanges(false)
  }

  const updateEditState = (key: string, value: any) => {
    const newState = { ...editState, [key]: value }
    setEditState(newState)
    if (selectedSkill) {
      const newContent = buildSkillContent(newState.name, newState.description, newState.body)
      setHasChanges(newContent !== selectedSkill.content)
    }
  }

  const handleSave = async () => {
    if (!selectedSkill) return
    setSaving(true)
    try {
      const fullContent = buildSkillContent(editState.name, editState.description, editState.body)
      await invoke('save_skill', {
        name: selectedSkill.name,
        content: fullContent,
        scope: selectedSkill.scope,
        projectDir: projectDir || null
      })
      setSkills(skills.map(s => (s.name === selectedSkill.name && s.scope === selectedSkill.scope) ? { ...s, content: fullContent } : s))
      setSelectedSkill({ ...selectedSkill, content: fullContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('save Skill failed', e, { userMessage: t('skills.saveFailed') })
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (skill: any) => {
    if (!await showConfirm(t('skills.confirmDelete'), t('skills.confirmDeleteSkill', { name: skill.name }))) return
    try {
      await invoke('delete_skill', {
        name: skill.name,
        scope: skill.scope,
        projectDir: projectDir || null
      })
      const newSkills = skills.filter(s => !(s.name === skill.name && s.scope === skill.scope))
      setSkills(newSkills)
      onCountChange?.(newSkills.length)
      if (selectedSkill?.name === skill.name && selectedSkill?.scope === skill.scope) {
        setSelectedSkill(null)
        setEditState({ name: '', description: '', body: '' })
        setHasChanges(false)
      }
    } catch (e) {
      handleUiError('delete Skill failed', e, { userMessage: t('skills.deleteFailed') })
    }
  }

  const handleCreate = async (skillName: string, description: string, scope: string) => {
    const body = `\n${t('skills.instructionPlaceholder')}\n`
    const content = buildSkillContent(skillName, description, body)
    try {
      const newSkill = await invoke<any>('create_skill', {
        name: skillName,
        content,
        scope,
        projectDir: projectDir || null
      })
      const newSkills = [...skills, newSkill]
      setSkills(newSkills)
      onCountChange?.(newSkills.length)
      setShowCreateModal(false)
      handleSelect(newSkill)
    } catch (e) {
      handleUiError('create Skill failed', e, { userMessage: t('skills.createFailed') })
    }
  }

  const resolveImportScope = async () => {
    if (!projectDir) return 'user'
    const useProjectScope = await showConfirm(
      t('skills.importScopeTitle'),
      t('skills.importScopeMessage'),
      {
        confirmText: t('kiroConfig.scopeProject'),
        cancelText: t('kiroConfig.scopeUser')}
    )
    return useProjectScope ? 'project' : 'user'
  }

  const upsertImportedSkill = (imported: any) => {
    const nextSkills = [
      ...skills.filter(skill => !(skill.name === imported.name && skill.scope === imported.scope)),
      imported
    ].sort((a, b) => a.name.localeCompare(b.name))
    setSkills(nextSkills)
    onCountChange?.(nextSkills.length)
    handleSelect(imported)
  }

  const handleImportLocal = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('skills.importLocal')})
      if (!selected) return

      const scope = await resolveImportScope()
      const imported = await invoke<any>('import_skill_local', {
        sourcePath: selected as string,
        scope,
        projectDir: projectDir || null,
        overwrite: false})
      upsertImportedSkill(imported)
      showSuccess(t('skills.importSuccess'), `${imported.name}`)
    } catch (e) {
      handleUiError('import local Skill failed', e, { userMessage: t('skills.importFailed') })
    }
  }

  const handleImportGithub = async ({ repoUrl, pathInRepo, branch, targetName }: any) => {
    try {
      const scope = await resolveImportScope()
      const imported = await invoke<any>('import_skill_from_github', {
        repoUrl: repoUrl.trim(),
        pathInRepo: pathInRepo.trim() || null,
        branch: branch.trim() || null,
        targetName: targetName.trim() || null,
        scope,
        projectDir: projectDir || null,
        overwrite: false})
      upsertImportedSkill(imported)
      setShowGithubImportModal(false)
      showSuccess(t('skills.importSuccess'), `${imported.name}`)
    } catch (e) {
      handleUiError('import Skill from GitHub failed', e, { userMessage: t('skills.importGithubFailed') })
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
            <Puzzle size={18} className={accent.text} />
            <span className={`text-sm font-semibold text-foreground`}>Skills</span>
            <span className={`text-xs text-muted-foreground`}>({skills.length})</span>
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleImportLocal}
              className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
              title={t('skills.importLocal')}
            >
              <FolderOpen size={16} className={accent.text} />
            </button>
            <button
              onClick={() => setShowGithubImportModal(true)}
              className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
              title={t('skills.importGithub')}
            >
              <Globe size={16} className={accent.text} />
            </button>
            <button
              onClick={() => setShowCreateModal(true)}
              className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
              title={t('skills.create')}
            >
              <Plus size={16} className={accent.text} />
            </button>
              <button
                onClick={loadSkills}
                className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
                title={t('common.refresh')}
              >
              <RefreshCw size={16} className={"text-muted-foreground"} />
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4">
          {skills.length === 0 ? (
            <div className={`text-center py-16 text-muted-foreground`}>
              <Puzzle size={48} className="mx-auto mb-3 opacity-20" />
              <p className="text-sm">{t('skills.noSkills')}</p>
              <p className={`text-xs mt-2 text-muted-foreground`}>{t('skills.noSkillsHint')}</p>
              <button
                onClick={() => setShowCreateModal(true)}
                className={`cursor-pointer mt-4 px-4 py-2 rounded-lg text-sm transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${accentSolidButtonClass}`}
              >
                {t('skills.createFirst')}
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {skills.map(skill => {
                const isSelected = selectedSkill?.name === skill.name && selectedSkill?.scope === skill.scope
                const parsed = parseSkillFrontMatter(skill.content)
                return (
                  <div
                    key={`${skill.scope}-${skill.name}`}
                    onClick={() => handleSelect(skill)}
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
                          <Puzzle
                            size={18}
                            className={`flex-shrink-0 ${isSelected ? accent.text : "text-muted-foreground"}`}
                          />
                        </div>
                        <div className="flex-1 min-w-0">
                          <span className={`font-bold text-sm ${isSelected ? accent.text : "text-foreground"} truncate block`}>
                            {skill.name}
                          </span>
                          {parsed.description && (
                            <span className={`text-xs text-muted-foreground truncate block mt-0.5`}>{parsed.description}</span>
                          )}
                        </div>
                        <ScopeBadge scope={skill.scope} accent={accent} t={t} />
                      </div>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDelete(skill) }}
                        className="cursor-pointer opacity-0 group-hover:opacity-100 p-2 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                        title={t('common.delete')}
                      >
                        <Trash2 size={16} className="text-red-500" />
                      </button>
                    </div>
                    <div className={`flex items-center gap-2.5 text-xs text-muted-foreground ml-11`}>
                      <span className={`px-2 py-1 rounded-md bg-muted/30 font-medium`}>
                        {formatSize(skill.size)}
                      </span>
                      {skill.extraFiles.length > 0 && (
                        <>
                          <span className="opacity-50">•</span>
                          <span className="flex items-center gap-1">
                            <FolderOpen size={12} />
                            +{skill.extraFiles.length} {t('skills.files')}
                          </span>
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
        {selectedSkill ? (
          <>
            <div className={`p-3 border-b border-border flex items-center justify-between`}>
              <div className="flex items-center gap-2">
                <h3 className={`font-semibold text-foreground`}>{selectedSkill.name}/SKILL.md</h3>
                <ScopeBadge scope={selectedSkill.scope} accent={accent} t={t} />
                {hasChanges && <span className="text-xs text-orange-500">● {t('skills.unsaved')}</span>}
              </div>
              <button
                onClick={handleSave}
                disabled={!hasChanges || saving}
                className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${
                  hasChanges ? accentSolidButtonClass : colors.btnDisabled
                } disabled:opacity-50`}
              >
                <Save size={14} />
                {saving ? t('skills.saving') : t('skills.save')}
              </button>
            </div>
            {/* frontmatter 编辑区 */}
            <div className={`px-4 py-3 border-b border-border flex items-center gap-4 flex-wrap`}>
              <div className="flex items-center gap-2">
                <span className={`text-xs text-muted-foreground`}>{t('skills.fmName')}:</span>
                <Input
                  value={editState.name}
                  onChange={(e) => updateEditState('name', e.target.value)}
                  placeholder={t('skills.fmNamePlaceholder')}
                  className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                  style={{ width: '140px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
                />
              </div>
              <div className="flex items-center gap-2 flex-1">
                <span className={`text-xs text-muted-foreground`}>{t('skills.fmDescription')}:</span>
                <Input
                  value={editState.description}
                  onChange={(e) => updateEditState('description', e.target.value)}
                  placeholder={t('skills.fmDescriptionPlaceholder')}
                  className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                  style={{ flex: 1, minWidth: '200px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
                />
              </div>
            </div>
            {selectedSkill.extraFiles.length > 0 && (
              <div className={`px-4 py-2 border-b border-border flex items-center gap-2 text-xs text-muted-foreground`}>
                <FolderOpen size={14} />
                <span>{t('skills.extraFiles')}:</span>
                {selectedSkill.extraFiles.map((f: string) => (
                  <code key={f} className={`px-2 py-0.5 rounded-md bg-muted/30 font-mono`}>{f}</code>
                ))}
              </div>
            )}
            <div className="flex-1 p-4 overflow-hidden">
              <Textarea
                value={editState.body}
                onChange={(e) => updateEditState('body', e.target.value)}
                placeholder={t('skills.contentPlaceholder')}
                className={`${colors.inputFocus}`}
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
                    borderColor: surface.editorBorder}
                }}
              />
            </div>
          </>
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <Puzzle size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('skills.selectToEdit')}</p>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && (
        <CreateSkillModal
          onCreate={handleCreate}
          onClose={() => setShowCreateModal(false)}
          accent={accent}
          accentGradientButtonClass={accentGradientButtonClass}
          colors={colors}
          t={t}
          hasProjectDir={!!projectDir}
        />
      )}
      {showGithubImportModal && (
        <ImportGithubSkillModal
          onImport={handleImportGithub}
          onClose={() => setShowGithubImportModal(false)}
          accent={accent}
          accentGradientButtonClass={accentGradientButtonClass}
          colors={colors}
          t={t}
        />
      )}
    </div>
  )
}

// 创建 Skill 弹窗
function CreateSkillModal({ onCreate, onClose, accent, accentGradientButtonClass, colors, t, hasProjectDir }: any) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [scope, setScope] = useState('user')

  const isValidName = name.trim() && /^[a-zA-Z0-9_-]+$/.test(name.trim())

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`glass-card rounded-2xl w-full max-w-[420px] shadow-2xl border border-border overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <Puzzle size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('skills.newSkill')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.skillName')}</label>
            <Input
              placeholder={t('skills.skillNamePlaceholder')}
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('skills.skillNameHint')}</p>
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.fmDescription')}</label>
            <Input
              placeholder={t('skills.fmDescriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('skills.fmDescriptionHint')}</p>
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

          <button
            onClick={() => onCreate(name.trim(), description.trim(), scope)}
            disabled={!isValidName}
            className={`cursor-pointer w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}

function ImportGithubSkillModal({ onImport, onClose, accent, accentGradientButtonClass, colors, t }: any) {
  const [repoUrl, setRepoUrl] = useState('')
  const [pathInRepo, setPathInRepo] = useState('')
  const [branch, setBranch] = useState('main')
  const [targetName, setTargetName] = useState('')

  const canSubmit = repoUrl.trim().startsWith('https://github.com/')

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`glass-card rounded-2xl w-full max-w-[460px] shadow-2xl border border-border overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <Download size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('skills.importGithub')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.githubRepoUrl')}</label>
            <Input
              placeholder="https://github.com/owner/repo"
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.githubPath')}</label>
            <Input
              placeholder={t('skills.githubPathPlaceholder')}
              value={pathInRepo}
              onChange={(e) => setPathInRepo(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.githubBranch')}</label>
              <Input
                placeholder="main"
                value={branch}
                onChange={(e) => setBranch(e.target.value)}
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ borderRadius: '0.5rem' }}
              />
            </div>
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('skills.importTargetName')}</label>
              <Input
                placeholder={t('skills.importTargetNamePlaceholder')}
                value={targetName}
                onChange={(e) => setTargetName(e.target.value)}
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ borderRadius: '0.5rem' }}
              />
            </div>
          </div>

          <button
            onClick={() => onImport({ repoUrl, pathInRepo, branch, targetName })}
            disabled={!canSubmit}
            className={`cursor-pointer w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('skills.importGithub')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default SkillsPanel
