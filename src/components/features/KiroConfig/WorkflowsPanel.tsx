import { useCallback, useEffect, useMemo, useState } from 'react'
import { FileText, Save, Plus, Trash2, FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useApp } from '../../../hooks/useApp'
import {
  listWorkflows,
  readWorkflow,
  saveWorkflow,
  createWorkflow,
  deleteWorkflow,
  type WorkflowFile,
} from '../../../api/kiroConfigApi'

// Workflows 面板（Kiro 1.1.14）
//
// 逆向结论（详见 docs/Kiro 1.1.14/未覆盖功能与权限预设.md）：
// 文件为 .workflow.json / .workflow.yaml / .workflow.yml，
// 存于 <项目>/.kiro/workflows/ 或 ~/.kiro/workflows/。
// 四个合法根目录中，bundled:// 与 generated:// 属运行时/只读来源，本管理端只编辑用户级与项目级。
// Schema：name + inputs（模板变量）+ steps[] 节点；workflow 级可设 modelId/effortLevel，步骤可覆盖。

const TEMPLATE = `{
  "name": "my-workflow",
  "inputs": [],
  "steps": []
}
`

export default function WorkflowsPanel({
  onCountChange,
  projectDir,
  readOnly,
}: {
  onCountChange?: (n: number) => void
  projectDir: string | null
  readOnly?: boolean
}) {
  const { t } = useApp()
  const [files, setFiles] = useState<WorkflowFile[]>([])
  const [selected, setSelected] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const [baseline, setBaseline] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')

  const scope = projectDir ? 'project' : 'user'

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const list = await listWorkflows(scope, projectDir).catch(() => [] as WorkflowFile[])
      setFiles(list || [])
      const first = list && list.length ? list[0].fileName : null
      if (first) {
        await pickByName(first)
      } else {
        setSelected(null)
        setDraft('')
        setBaseline('')
      }
    } finally {
      setLoading(false)
    }
  }, [scope, projectDir])

  const pickByName = useCallback(
    async (fileName: string) => {
      const data = await readWorkflow(scope, projectDir, fileName).catch(() => null)
      if (!data) return
      setSelected(fileName)
      setDraft(data.content)
      setBaseline(data.content)
    },
    [scope, projectDir],
  )

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    onCountChange?.(files.length)
  }, [files.length, onCountChange])

  const current = useMemo(
    () => files.find(f => f.fileName === selected) || null,
    [files, selected],
  )
  const dirty = !!selected && draft !== baseline

  const handleSave = async () => {
    if (readOnly) return
    if (!selected) return
    setSaving(true)
    try {
      const issues: string[] = await saveWorkflow(scope, projectDir, selected, draft)
      setBaseline(draft)
      setFiles(fs => fs.map(f => (f.fileName === selected ? { ...f, content: draft } : f)))
      if (Array.isArray(issues) && issues.length > 0) {
        // 保存成功但 schema 不合规：Kiro 运行前校验会拒绝该 workflow，必须明确提示
        window.alert(`Workflow 已保存，但存在 schema 问题（Kiro 运行前校验将拒绝）：\n\n• ${issues.join('\n• ')}`)
      }
    } finally {
      setSaving(false)
    }
  }

  const handleCreate = async () => {
    if (readOnly) return
    const name = newName.trim()
    if (!name) return
    const file_name = name.endsWith('.workflow.json') || name.endsWith('.workflow.yaml') || name.endsWith('.workflow.yml')
      ? name
      : `${name}.workflow.json`
    try {
      await createWorkflow(scope, projectDir, file_name)
    } catch {
      // 创建失败（多半是同名文件已存在）：中止后续写入，避免覆盖已有内容
      return
    }
    await saveWorkflow(scope, projectDir, file_name, TEMPLATE).catch(() => {})
    setNewName('')
    setCreating(false)
    await load()
    await pickByName(file_name)
  }

  const handleDelete = async () => {
    if (readOnly) return
    if (!selected) return
    if (!confirm(t('kiroConfig.workflowDeleteConfirm', { name: selected }))) return
    await deleteWorkflow(scope, projectDir, selected).catch(() => {})
    await load()
  }

  const pick = (f: WorkflowFile) => {
    void pickByName(f.fileName)
  }

  if (loading) {
    return <p className="px-5 py-6 text-sm text-muted-foreground">{t('kiroConfig.loading')}</p>
  }

  return (
    <div className="flex h-full min-h-0">
      {/* 左：workflow 文件列表 */}
      <div className="flex w-56 shrink-0 flex-col border-r border-border">
        <div className="flex items-center justify-between gap-1 border-b border-border px-2 py-2">
          <span className="text-xs font-medium text-muted-foreground">
            {scope === 'project' ? t('kiroConfig.scopeProject') : t('kiroConfig.scopeUser')}
          </span>
          <Button size="sm" variant="ghost" onClick={() => setCreating(v => !v)} disabled={readOnly}>
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
              placeholder={t('kiroConfig.workflowNewPlaceholder')}
              className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-2"
            />
            <Button size="sm" onClick={handleCreate} disabled={readOnly}>
              {t('kiroConfig.specCreate')}
            </Button>
          </div>
        )}
        <div className="flex-1 overflow-y-auto p-2">
          {files.length === 0 ? (
            <p className="px-1 py-2 text-xs text-muted-foreground">{t('kiroConfig.workflowEmpty')}</p>
          ) : (
            files.map(f => (
              <button
                key={f.fileName}
                onClick={() => pick(f)}
                className={`mb-1 block w-full truncate rounded-md px-2 py-1.5 text-left text-xs transition-colors cursor-pointer ${
                  f.fileName === selected
                    ? 'bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:bg-muted/50'
                }`}
                title={f.fileName}
              >
                {f.fileName}
              </button>
            ))
          )}
        </div>
      </div>

      {/* 右：编辑区 */}
      <div className="flex min-w-0 flex-1 flex-col">
        {!selected ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
            <FolderOpen size={22} className="text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{t('kiroConfig.workflowNoSelection')}</p>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-2 border-b border-border px-3 py-2">
              <span className="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
                {selected}
              </span>
              {current && (
                <span className="shrink-0 text-[10px] text-muted-foreground">
                  {current.size} B
                </span>
              )}
              {dirty && (
                <span className="shrink-0 text-[10px] text-amber-600 dark:text-amber-400">
                  {t('kiroConfig.unsaved')}
                </span>
              )}
              <Button size="sm" variant="ghost" onClick={handleDelete} className="text-destructive" disabled={readOnly}>
                <Trash2 size={12} />
              </Button>
              <Button size="sm" onClick={handleSave} disabled={!dirty || saving || readOnly}>
                <Save size={12} />
                {saving ? t('kiroConfig.saving') : t('kiroConfig.save')}
              </Button>
            </div>

            <div className="min-h-0 flex-1 p-3">
              <Textarea
                value={draft}
                onChange={e => setDraft(e.target.value)}
                readOnly={readOnly}
                className="h-full resize-none font-mono text-xs"
                placeholder={t('kiroConfig.workflowFilePlaceholder')}
              />
            </div>
          </>
        )}
      </div>
    </div>
  )
}
