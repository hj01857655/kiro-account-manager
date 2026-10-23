import { useCallback, useEffect, useMemo, useState } from 'react'
import { FileText, Save, FolderOpen, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useApp } from '../../../hooks/useApp'
import {
  scanAgentsMd,
  saveAgentsMd,
  listAgentsMdIgnoreFiles,
  type AgentsMdFile,
  type IgnoreFileInfo,
} from '../../../api/kiroConfigApi'

const formatSize = (bytes: number) =>
  bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// 嵌套 AGENTS.md 面板（Kiro 1.1.14）
//
// 结论（详见 docs/Kiro 1.1.14/嵌套AGENTS.md加载机制.md）：
// Kiro 会递归找出工作区里**每一层目录**的 AGENTS.md，与 .kiro/steering/*.md
// 合并成同一份清单，且顺序为：根 AGENTS.md → 嵌套 AGENTS.md → steering → 全局。
// 后端返回的列表已按此顺序排好，这里直接按序展示即可。
//
// 因 IDE 只在**工作区**内扫描嵌套 AGENTS.md，本面板必须依赖 projectDir；
// 未选择项目时不加载（而不是退化成全局，避免误导）。
export default function AgentsMdPanel({
  onCountChange,
  projectDir,
}: {
  onCountChange?: (n: number) => void
  projectDir: string | null
}) {
  const { t } = useApp()
  const [files, setFiles] = useState<AgentsMdFile[]>([])
  const [selected, setSelected] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const [baseline, setBaseline] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  // 影响本次扫描的忽略文件；非空时在列表顶部提示，避免用户对「少了几个 AGENTS.md」困惑
  const [ignoreFiles, setIgnoreFiles] = useState<IgnoreFileInfo[]>([])

  const load = useCallback(async () => {
    if (!projectDir) {
      setFiles([])
      setSelected(null)
      return
    }
    setLoading(true)
    try {
      const list = await scanAgentsMd(projectDir).catch(() => [] as AgentsMdFile[])
      setFiles(list || [])
      // 忽略文件单独查询：它影响本次扫描结果，但失败不影响主列表展示
      listAgentsMdIgnoreFiles(projectDir)
        .then(v => setIgnoreFiles(v || []))
        .catch(() => setIgnoreFiles([]))
      // 默认选中第一个（项目根的那个，与 IDE 的优先级一致）
      const first = list && list.length ? list[0] : null
      if (first) {
        setSelected(first.relPath)
        setDraft(first.content)
        setBaseline(first.content)
      } else {
        setSelected(null)
        setDraft('')
        setBaseline('')
      }
    } finally {
      setLoading(false)
    }
  }, [projectDir])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    onCountChange?.(files.length)
  }, [files.length, onCountChange])

  // 切换文件时重置草稿：避免把 A 的改动误存到 B
  const pick = (f: AgentsMdFile) => {
    setSelected(f.relPath)
    setDraft(f.content)
    setBaseline(f.content)
  }

  const current = useMemo(
    () => files.find(f => f.relPath === selected) || null,
    [files, selected],
  )
  const dirty = !!current && draft !== baseline

  const handleSave = async () => {
    if (!projectDir || !selected) return
    setSaving(true)
    try {
      await saveAgentsMd(projectDir, selected, draft)
      setBaseline(draft)
      // 回写列表里的 content，避免重新扫描前读到旧内容
      setFiles(fs => fs.map(f => (f.relPath === selected ? { ...f, content: draft } : f)))
    } finally {
      setSaving(false)
    }
  }

  if (!projectDir) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
        <FolderOpen size={22} className="text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{t('kiroConfig.agentsMdNoProject')}</p>
      </div>
    )
  }

  if (loading) {
    return <p className="px-5 py-6 text-sm text-muted-foreground">{t('kiroConfig.loading')}</p>
  }

  // 忽略文件提示条：有 .kiroignore / .gitignore 生效时告知用户，
  // 否则「某些目录的 AGENTS.md 没出现」会被误判为 bug
  const ignoreHint = ignoreFiles.length > 0 && (
    <div className="flex items-start gap-2 border-b border-border bg-muted/30 px-3 py-2">
      <Info size={13} className="mt-0.5 shrink-0 text-muted-foreground" />
      <div className="min-w-0 text-[11px] leading-relaxed text-muted-foreground">
        <span>{t('kiroConfig.agentsMdIgnoreActive')}</span>{' '}
        <span className="break-all font-mono">
          {ignoreFiles.map(f => `${f.relPath}(${f.ruleCount})`).join('、')}
        </span>
      </div>
    </div>
  )

  if (files.length === 0) {
    return (
      <div className="flex h-full flex-col">
        {ignoreHint}
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
          <FileText size={22} className="text-muted-foreground" />
          <p className="text-sm text-muted-foreground">{t('kiroConfig.agentsMdEmpty')}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {ignoreHint}
      <div className="flex min-h-0 flex-1">
        {/* 左：按层级列出的 AGENTS.md */}
        <div className="w-56 shrink-0 overflow-y-auto border-r border-border p-2">
          {files.map(f => (
            <button
              key={f.relPath}
              onClick={() => pick(f)}
              className={`mb-1 block w-full rounded-md px-2 py-1.5 text-left text-xs transition-colors cursor-pointer ${
                f.relPath === selected
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:bg-muted/50'
              }`}
              title={f.relPath}
            >
              {/* 用缩进体现层级，根目录的左边界与子目录对齐 */}
              <span style={{ paddingLeft: `${Math.min(f.depth, 6) * 8}px` }} className="block truncate">
                {f.depth === 0 ? 'AGENTS.md' : f.dirRel.split('/').pop() || f.relPath}
              </span>
              <span className="mt-0.5 block truncate text-[10px] opacity-60">
                {f.depth === 0 ? t('kiroConfig.agentsMdRoot') : f.dirRel}
              </span>
            </button>
          ))}
        </div>

        {/* 右：编辑区 */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="flex items-center gap-2 border-b border-border px-3 py-2">
            <span className="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
              {current?.relPath}
            </span>
            {current && (
              <span className="shrink-0 text-[10px] text-muted-foreground">
                {formatSize(current.size)}
              </span>
            )}
            {dirty && (
              <span className="shrink-0 text-[10px] text-amber-600 dark:text-amber-400">
                {t('kiroConfig.unsaved')}
              </span>
            )}
            <Button size="sm" onClick={handleSave} disabled={!dirty || saving}>
              <Save size={12} />
              {saving ? t('kiroConfig.saving') : t('kiroConfig.save')}
            </Button>
          </div>

          <div className="min-h-0 flex-1 p-3">
            <Textarea
              value={draft}
              onChange={e => setDraft(e.target.value)}
              className="h-full resize-none font-mono text-xs"
              placeholder={t('kiroConfig.agentsMdPlaceholder')}
            />
          </div>
        </div>
      </div>
    </div>
  )
}
