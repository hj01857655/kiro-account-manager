import { useCallback, useEffect, useMemo, useState } from 'react'
import { FileText, Save, Plus, Trash2, FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useApp } from '../../../hooks/useApp'
import {
  listSpecs,
  readSpec,
  saveSpecFile,
  createSpec,
  deleteSpec,
  type SpecInfo,
  type SpecSummary,
  type SpecFile,
} from '../../../api/kiroConfigApi'

// Specs 面板（Kiro 1.1.14）
//
// 结论（详见 docs/Kiro 1.1.14/未覆盖功能与权限预设.md）：
// 每个 spec 是 .kiro/specs/<name>/ 下一个子目录，内含三个约定命名的 Markdown
// —— requirements.md / design.md / tasks.md，无 schema，纯文本。
// 用户级在 ~/.kiro/specs，项目级在 <项目>/.kiro/specs。

const FILE_KINDS: { kind: 'requirements' | 'design' | 'tasks'; labelKey: string }[] = [
  { kind: 'requirements', labelKey: 'kiroConfig.specRequirements' },
  { kind: 'design', labelKey: 'kiroConfig.specDesign' },
  { kind: 'tasks', labelKey: 'kiroConfig.specTasks' },
]

export default function SpecsPanel({
  onCountChange,
  projectDir,
}: {
  onCountChange?: (n: number) => void
  projectDir: string | null
}) {
  const { t } = useApp()
  const [specs, setSpecs] = useState<SpecSummary[]>([])
  const [selected, setSelected] = useState<string | null>(null)
  const [info, setInfo] = useState<SpecInfo | null>(null)
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const [baselines, setBaselines] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')

  // 与 Steering/Skills 等面板一致：有项目目录走项目级，否则用户级。
  // 后端对 user scope 会忽略 projectDir，故两种 scope 都直接传 projectDir 即可。
  const scope = projectDir ? 'project' : 'user'

  const loadList = useCallback(async () => {
    setLoading(true)
    try {
      const list = await listSpecs(scope, projectDir).catch(() => [] as SpecSummary[])
      setSpecs(list || [])
      const first = list && list.length ? list[0].name : null
      setSelected(first)
      if (first) {
        await loadSpec(first)
      } else {
        setInfo(null)
        setDrafts({})
        setBaselines({})
      }
    } finally {
      setLoading(false)
    }
  }, [scope, projectDir])

  const loadSpec = useCallback(
    async (name: string) => {
      const data = await readSpec(scope, projectDir, name).catch(() => null)
      if (!data) return
      setInfo(data)
      const d: Record<string, string> = {}
      const b: Record<string, string> = {}
      for (const k of ['requirements', 'design', 'tasks'] as const) {
        d[k] = data[k].content
        b[k] = data[k].content
      }
      setDrafts(d)
      setBaselines(b)
    },
    [scope, projectDir],
  )

  useEffect(() => {
    void loadList()
  }, [loadList])

  useEffect(() => {
    onCountChange?.(specs.length)
  }, [specs.length, onCountChange])

  const dirtyOf = (kind: string) => drafts[kind] !== baselines[kind]

  const anyDirty = useMemo(
    () => FILE_KINDS.some(f => dirtyOf(f.kind)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [drafts, baselines],
  )

  const handleSave = async () => {
    if (!selected) return
    setSaving(true)
    try {
      for (const f of FILE_KINDS) {
        if (dirtyOf(f.kind)) {
          await saveSpecFile(scope, projectDir, selected, f.kind, drafts[f.kind])
        }
      }
      const b: Record<string, string> = { ...drafts }
      setBaselines(b)
      // 回写列表里的 exists/size（这里只更新草稿基线，刷新列表时再取）
      void loadSpec(selected)
    } finally {
      setSaving(false)
    }
  }

  const handleCreate = async () => {
    const name = newName.trim()
    if (!name) return
    await createSpec(scope, projectDir, name).catch(() => {})
    setNewName('')
    setCreating(false)
    await loadList()
    setSelected(name)
    await loadSpec(name)
  }

  const handleDelete = async () => {
    if (!selected) return
    if (!confirm(t('kiroConfig.specDeleteConfirm', { name: selected }))) return
    await deleteSpec(scope, projectDir, selected).catch(() => {})
    await loadList()
  }

  const pick = (s: SpecSummary) => {
    setSelected(s.name)
    void loadSpec(s.name)
  }

  if (loading) {
    return <p className="px-5 py-6 text-sm text-muted-foreground">{t('kiroConfig.loading')}</p>
  }

  return (
    <div className="flex h-full min-h-0">
      {/* 左：spec 列表 */}
      <div className="flex w-56 shrink-0 flex-col border-r border-border">
        <div className="flex items-center justify-between gap-1 border-b border-border px-2 py-2">
          <span className="text-xs font-medium text-muted-foreground">
            {scope === 'project' ? t('kiroConfig.scopeProject') : t('kiroConfig.scopeUser')}
          </span>
          <Button size="sm" variant="ghost" onClick={() => setCreating(v => !v)}>
            <Plus size={12} />
          </Button>
        </div>
        {creating && (
          <div className="flex gap-1 border-b border-border p-2">
            <input
              autoFocus
              value={newName}
              onChange={e => setNewName(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleCreate()}
              placeholder={t('kiroConfig.specNewPlaceholder')}
              className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-2"
            />
            <Button size="sm" onClick={handleCreate}>
              {t('kiroConfig.specCreate')}
            </Button>
          </div>
        )}
        <div className="flex-1 overflow-y-auto p-2">
          {specs.length === 0 ? (
            <p className="px-1 py-2 text-xs text-muted-foreground">{t('kiroConfig.specEmpty')}</p>
          ) : (
            specs.map(s => (
              <button
                key={s.name}
                onClick={() => pick(s)}
                className={`mb-1 block w-full truncate rounded-md px-2 py-1.5 text-left text-xs transition-colors cursor-pointer ${
                  s.name === selected
                    ? 'bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:bg-muted/50'
                }`}
                title={s.name}
              >
                {s.name}
              </button>
            ))
          )}
        </div>
      </div>

      {/* 右：三文件编辑区 */}
      <div className="flex min-w-0 flex-1 flex-col">
        {!selected || !info ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
            <FolderOpen size={22} className="text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{t('kiroConfig.specNoSelection')}</p>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-2 border-b border-border px-3 py-2">
              <span className="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
                {selected}
              </span>
              {anyDirty && (
                <span className="shrink-0 text-[10px] text-amber-600 dark:text-amber-400">
                  {t('kiroConfig.unsaved')}
                </span>
              )}
              <Button size="sm" variant="ghost" onClick={handleDelete} className="text-destructive">
                <Trash2 size={12} />
              </Button>
              <Button size="sm" onClick={handleSave} disabled={!anyDirty || saving}>
                <Save size={12} />
                {saving ? t('kiroConfig.saving') : t('kiroConfig.save')}
              </Button>
            </div>

            <div className="flex-1 space-y-3 overflow-y-auto p-3">
              {FILE_KINDS.map(f => {
                const file: SpecFile | undefined = info[f.kind]
                return (
                  <div key={f.kind} className="flex flex-col">
                    <div className="mb-1 flex items-center gap-2">
                      <span className="text-xs font-medium text-foreground">
                        {t(f.labelKey)}
                      </span>
                      <span className="text-[10px] text-muted-foreground">{f.kind}.md</span>
                      {!file?.exists && (
                        <span className="text-[10px] text-muted-foreground">
                          （{t('kiroConfig.specNotCreated')}）
                        </span>
                      )}
                    </div>
                    <Textarea
                      value={drafts[f.kind] ?? ''}
                      onChange={e =>
                        setDrafts(d => ({ ...d, [f.kind]: e.target.value }))
                      }
                      className="h-40 resize-y font-mono text-xs"
                      placeholder={t('kiroConfig.specFilePlaceholder')}
                    />
                  </div>
                )
              })}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
