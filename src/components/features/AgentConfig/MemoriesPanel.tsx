import { useEffect, useState } from 'react'
import { AlertTriangle, Brain, ChevronDown, ChevronRight, Pencil, Plus, Trash2 } from 'lucide-react'
import { deleteMemory, getMemoryMode, listMemoriesPage, previewMemoryInjection, saveMemory, type MemoryInjectionPreview, type MemoryItem } from '../../../api/memoryApi'
import { setKiroAgentSetting } from '../../../api/settingsApi'
import { useDialog } from '../../../contexts/DialogContext'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'

// Agent Memory 管理面板（Kiro 1.4+）：~/.kiro/memories/memories.db 的记忆条目
// 列表/删除 + kiroAgent.memory 模式切换（settings.json 键，四档映射见 memory_cmd.rs）。

// 官方四档 → {mode, reflection}（docs/Kiro memory 系统.md）
const MEMORY_MODES = [
  { id: 'off', mode: 'disabled', reflection: false },
  { id: 'useOnly', mode: 'read_only', reflection: false },
  { id: 'useWithoutLearning', mode: 'read_write', reflection: false },
  { id: 'useAndLearn', mode: 'read_write', reflection: true },
] as const

function modeToId(mode: string, reflection: boolean): string {
  return MEMORY_MODES.find(m => m.mode === mode && m.reflection === reflection)?.id ?? 'off'
}

export default function MemoriesPanel({ t }: { t: (key: string) => string }) {
  const { showError, showConfirm } = useDialog()
  const [memories, setMemories] = useState<MemoryItem[]>([])
  const [loading, setLoading] = useState(true)
  const [dbMissing, setDbMissing] = useState(false)
  const [modeId, setModeId] = useState('off')
  const [preview, setPreview] = useState<MemoryInjectionPreview | null>(null)

  // 注入预览：记忆或模式变化后重算（模拟 buildMemoryBlock 的分层与预算）
  useEffect(() => {
    previewMemoryInjection().then(setPreview).catch(() => setPreview(null))
  }, [memories, modeId])
  // 分页状态（口径对齐 IDE：limit 默认 100、上限 1000）
  const [nextCursor, setNextCursor] = useState<string | undefined>(undefined)
  const [loadingMore, setLoadingMore] = useState(false)
  // scope 过滤（IDE 的 scopeFilter）：空数组 = 不过滤
  const [scopeFilter, setScopeFilter] = useState<string[]>([])
  const [scopeLabel, setScopeLabel] = useState<string | undefined>(undefined)
  // 展开的记忆 id。列表只展示摘要，content/why/howToApply 需要展开才看——
  // 这三个字段后端已一并查出，不展开就白白浪费了。
  const [expanded, setExpanded] = useState<Set<string>>(new Set())

  // 编辑草稿：null = 未在编辑；id 为空 = 新建
  const [draft, setDraft] = useState<MemoryItem | null>(null)

  const startCreate = () => {
    setDraft({
      id: '',
      memoryType: 'USER_PREFERENCE',
      scopes: ['Global'],
      title: '',
      summary: '',
      content: '',
      why: '',
      howToApply: '',
      sessionId: '',
      createdAt: '',
      updatedAt: '',
    })
  }

  const startEdit = (m: MemoryItem) => setDraft({ ...m })

  const saveDraft = async () => {
    if (!draft) return
    try {
      await saveMemory({
        id: draft.id || undefined,
        memoryType: draft.memoryType,
        scopes: draft.scopes,
        title: draft.title,
        summary: draft.summary,
        content: draft.content,
        why: draft.why || undefined,
        howToApply: draft.howToApply || undefined,
      })
      setDraft(null)
      load()
    } catch (err: any) {
      // 校验错误来自后端（与 IDE 同口径），直接展示原文
      showError(t('settings.saveFailed'), String(err?.message || err))
    }
  }

  const toggleExpanded = (id: string) => {
    setExpanded(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const load = (scopes: string[] = scopeFilter) => {
    setLoading(true)
    Promise.all([listMemoriesPage(undefined, undefined, scopes.length ? scopes : undefined), getMemoryMode()])
      .then(([page, mode]) => {
        setMemories(page.memories || [])
        setNextCursor(page.nextCursor)
        setScopeLabel(page.scopeLabel)
        setDbMissing(false)
        setModeId(modeToId(mode.mode, mode.reflection))
      })
      .catch((e: any) => {
        // db 不存在 = Memory 从未启用，按空态处理而不是报错
        if (String(e?.message || e).includes('不存在')) {
          setMemories([])
          setNextCursor(undefined)
          setScopeLabel(undefined)
          setDbMissing(true)
          setModeId('off')
        } else {
          showError(t('common.error'), String(e?.message || e))
        }
      })
      .finally(() => setLoading(false))
  }

  // 切换 scope 过滤：换过滤条件必须丢掉旧 cursor（后端会拒绝不一致的 cursor）
  const applyScopeFilter = (scopes: string[]) => {
    setScopeFilter(scopes)
    setExpanded(new Set())
    load(scopes)
  }

  // 追加下一页：cursor 由后端签发，前端不解释内容
  const loadMore = async () => {
    if (!nextCursor || loadingMore) return
    setLoadingMore(true)
    try {
      const page = await listMemoriesPage(undefined, nextCursor, scopeFilter.length ? scopeFilter : undefined)
      setMemories(prev => [...prev, ...(page.memories || [])])
      setNextCursor(page.nextCursor)
    } catch (err: any) {
      showError(t('common.error'), String(err?.message || err))
    } finally {
      setLoadingMore(false)
    }
  }

  useEffect(() => { load([]) }, [])

  const handleModeChange = async (id: string) => {
    setModeId(id)
    const m = MEMORY_MODES.find(x => x.id === id)
    if (!m) return
    try {
      await setKiroAgentSetting('kiroAgent.memory', { mode: m.mode, reflection: m.reflection })
    } catch (err: any) {
      showError(t('settings.saveFailed'), String(err?.message || err))
    }
  }

  const handleDelete = async (m: MemoryItem) => {
    const ok = await showConfirm(
      t('agentConfig.memoryDeleteConfirmTitle'),
      t('agentConfig.memoryDeleteConfirm'),
      { confirmText: t('common.delete') },
    )
    if (!ok) return
    try {
      await deleteMemory(m.id)
      setMemories(prev => prev.filter(x => x.id !== m.id))
    } catch (err: any) {
      showError(t('common.error'), String(err?.message || err))
    }
  }

  return (
    <div className="space-y-3">
      {/* 模式开关 */}
      <div className="surface-card border border-border rounded-xl p-4">
        <div className="flex items-center justify-between gap-3 flex-wrap">
          <div className="min-w-0">
            <div className="text-sm font-medium text-foreground flex items-center gap-2">
              <Brain size={15} className="text-primary" />
              {t('agentConfig.memoryMode')}
            </div>
            <p className="text-[11px] text-muted-foreground mt-0.5">{t('agentConfig.memoryModeDesc')}</p>
          </div>
          <Select value={modeId} onValueChange={handleModeChange}>
            <SelectTrigger className="h-8 text-xs w-44"><SelectValue /></SelectTrigger>
            <SelectContent>
              {MEMORY_MODES.map(m => (
                <SelectItem key={m.id} value={m.id}>{t(`agentConfig.memoryMode_${m.id}`)}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* mode 联动：关闭时提示——会话侧不读不学，此处编辑仍落库 */}
      {modeId === 'off' && (
        <div className="px-3 py-2.5 rounded-lg border border-amber-500/30 bg-amber-500/10 flex items-start gap-2.5">
          <AlertTriangle size={16} className="mt-0.5 text-amber-500 shrink-0" />
          <div className="text-xs leading-relaxed text-foreground">{t('agentConfig.memoryOffBanner')}</div>
        </div>
      )}

      {/* 注入预览：模拟 Kiro buildMemoryBlock 的分层与预算（§7.1.11） */}
      {preview && (
        <div className="surface-card border border-border rounded-xl p-4 space-y-2.5">
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-foreground">{t('agentConfig.memoryPreviewTitle')}</h3>
            {preview.enabled ? (
              <span className="text-xs text-muted-foreground font-mono">
                {preview.renderedChars} / {preview.totalBudget}
              </span>
            ) : (
              <span className="text-xs text-amber-500">{t('agentConfig.memoryPreviewDisabled')}</span>
            )}
          </div>
          {preview.enabled && (
            <>
              <div className="flex items-center gap-2 flex-wrap text-[11px]">
                <span className="px-2 py-0.5 rounded-full bg-primary/10 text-primary">
                  {t('agentConfig.memoryPreviewResident')} {preview.resident.length}
                </span>
                <span className="px-2 py-0.5 rounded-full bg-muted/60 text-muted-foreground">
                  {t('agentConfig.memoryPreviewManifest')} {preview.manifest.length}
                </span>
                {preview.droppedTooLong.length > 0 && (
                  <span
                    className="px-2 py-0.5 rounded-full bg-red-500/15 text-red-600 dark:text-red-400"
                    title={preview.droppedTooLong.join('\n')}
                  >
                    {t('agentConfig.memoryPreviewDropped')} {preview.droppedTooLong.length}
                  </span>
                )}
                <span className="px-2 py-0.5 rounded-full bg-muted/60 text-muted-foreground">
                  {t('agentConfig.memoryPreviewPrefQuota')} {preview.prefQuota}
                </span>
              </div>
              <div className="max-h-48 overflow-y-auto space-y-1 font-mono text-[11px] leading-relaxed">
                {preview.resident.map(r => (
                  <div key={`r-${r.title}`} className="truncate text-foreground" title={r.line}>{r.line}</div>
                ))}
                {preview.manifest.map(r => (
                  <div key={`m-${r.title}`} className="truncate text-muted-foreground" title={r.line}>{r.line}</div>
                ))}
              </div>
            </>
          )}
        </div>
      )}

      {/* scope 过滤：从当前已加载的记忆里汇总可选 scope */}
      {(() => {
        const allScopes = Array.from(new Set(memories.flatMap(m => m.scopes || []))).sort()
        if (allScopes.length <= 1) return null
        return (
          <div className="flex items-center gap-1.5 flex-wrap">
            <button
              onClick={() => applyScopeFilter([])}
              className={`h-6 px-2 rounded-full text-[11px] transition-colors cursor-pointer ${
                scopeFilter.length === 0
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted/60 text-muted-foreground hover:text-foreground'
              }`}
            >
              {t('agentConfig.memoryScopeAll')}
            </button>
            {allScopes.map(s => {
              const active = scopeFilter.includes(s)
              return (
                <button
                  key={s}
                  onClick={() => applyScopeFilter(active ? scopeFilter.filter(x => x !== s) : [...scopeFilter, s])}
                  className={`h-6 px-2 rounded-full text-[11px] font-mono transition-colors cursor-pointer ${
                    active
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted/60 text-muted-foreground hover:text-foreground'
                  }`}
                >
                  {s}
                </button>
              )
            })}
            {scopeLabel && (
              <span className="text-[10px] text-muted-foreground/70 ml-1">{scopeLabel}</span>
            )}
          </div>
        )
      })()}

      {/* 新建/编辑表单 */}
      {draft && (
        <div className="surface-card border border-primary/40 rounded-xl p-4 space-y-2.5">
          <div className="text-sm font-medium text-foreground">
            {draft.id ? t('agentConfig.memoryEdit') : t('agentConfig.memoryCreate')}
          </div>

          <div className="grid grid-cols-2 gap-2.5">
            <div>
              <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryFieldTitle')}</Label>
              <input
                value={draft.title}
                onChange={e => setDraft({ ...draft, title: e.target.value })}
                placeholder={t('agentConfig.memoryFieldTitleHint')}
                className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground"
              />
            </div>
            <div>
              <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryFieldType')}</Label>
              <Select value={draft.memoryType} onValueChange={v => setDraft({ ...draft, memoryType: v })}>
                <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="USER_PREFERENCE">USER_PREFERENCE</SelectItem>
                  <SelectItem value="SEMANTIC">SEMANTIC</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryFieldScopes')}</Label>
            <input
              value={draft.scopes.join(', ')}
              onChange={e => setDraft({ ...draft, scopes: e.target.value.split(',').map(s => s.trim()).filter(Boolean) })}
              placeholder="Global, workspace:name, repo:github/owner/repo"
              className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground font-mono"
            />
          </div>

          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryFieldSummary')}</Label>
            <input
              value={draft.summary}
              onChange={e => setDraft({ ...draft, summary: e.target.value })}
              className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground"
            />
          </div>

          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryContent')}</Label>
            <textarea
              value={draft.content}
              onChange={e => setDraft({ ...draft, content: e.target.value })}
              rows={4}
              className="w-full px-2 py-1.5 rounded-md bg-background border border-border text-xs text-foreground font-mono resize-y"
            />
          </div>

          <div className="grid grid-cols-2 gap-2.5">
            <div>
              <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryWhy')}</Label>
              <input
                value={draft.why || ''}
                onChange={e => setDraft({ ...draft, why: e.target.value })}
                className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground"
              />
            </div>
            <div>
              <Label className="block text-[11px] text-muted-foreground mb-1">{t('agentConfig.memoryHowToApply')}</Label>
              <input
                value={draft.howToApply || ''}
                onChange={e => setDraft({ ...draft, howToApply: e.target.value })}
                className="w-full h-8 px-2 rounded-md bg-background border border-border text-xs text-foreground"
              />
            </div>
          </div>

          <div className="flex items-center gap-2 pt-1">
            <button
              onClick={saveDraft}
              className="h-8 px-3 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:opacity-90 cursor-pointer"
            >
              {t('common.save')}
            </button>
            <button
              onClick={() => setDraft(null)}
              className="h-8 px-3 rounded-md border border-border text-xs text-muted-foreground hover:text-foreground cursor-pointer"
            >
              {t('common.cancel')}
            </button>
          </div>
        </div>
      )}

      {/* 记忆列表 */}
      {loading ? (
        <div className="text-center py-10 text-muted-foreground text-sm">{t('common.loading')}</div>
      ) : memories.length === 0 ? (
        <div className="text-center py-10 surface-card border border-border rounded-xl">
          <Brain size={36} className="mx-auto mb-3 text-muted-foreground opacity-40" />
          <p className="text-sm text-muted-foreground">
            {dbMissing ? t('agentConfig.memoriesDbMissing') : t('agentConfig.memoriesEmpty')}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          <button
            onClick={startCreate}
            className="w-full h-9 rounded-xl border border-dashed border-border text-xs text-muted-foreground hover:text-foreground hover:border-primary/50 transition-colors cursor-pointer flex items-center justify-center gap-1.5"
          >
            <Plus size={13} />
            {t('agentConfig.memoryCreate')}
          </button>
          {memories.map(m => {
            const isOpen = expanded.has(m.id)
            return (
            <div key={m.id} className="surface-card border border-border rounded-xl p-3.5">
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <button
                      onClick={() => toggleExpanded(m.id)}
                      className="flex items-center gap-1 text-sm font-medium text-foreground hover:text-primary transition-colors cursor-pointer text-left"
                    >
                      {isOpen ? <ChevronDown size={13} className="shrink-0" /> : <ChevronRight size={13} className="shrink-0" />}
                      <span>{m.title || m.id}</span>
                    </button>
                    <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-muted/60 text-muted-foreground font-mono">
                      {m.memoryType}
                    </span>
                    {(m.scopes || []).slice(0, 3).map(s => (
                      <span key={s} className="text-[10px] px-1.5 py-0.5 rounded-full bg-primary/10 text-primary">
                        {s}
                      </span>
                    ))}
                  </div>
                  {m.summary && <p className="text-xs text-muted-foreground mt-1 leading-relaxed">{m.summary}</p>}
                  <div className="text-[10px] text-muted-foreground/70 mt-1.5">{m.updatedAt}</div>

                  {isOpen && (
                    <div className="mt-2.5 pt-2.5 border-t border-border/60 space-y-2">
                      {m.content && (
                        <div>
                          <div className="text-[10px] uppercase tracking-wide text-muted-foreground/70 mb-0.5">
                            {t('agentConfig.memoryContent')}
                          </div>
                          <pre className="text-xs text-foreground whitespace-pre-wrap break-words font-sans leading-relaxed m-0">
                            {m.content}
                          </pre>
                        </div>
                      )}
                      {m.why && (
                        <div>
                          <div className="text-[10px] uppercase tracking-wide text-muted-foreground/70 mb-0.5">
                            {t('agentConfig.memoryWhy')}
                          </div>
                          <p className="text-xs text-muted-foreground leading-relaxed m-0">{m.why}</p>
                        </div>
                      )}
                      {m.howToApply && (
                        <div>
                          <div className="text-[10px] uppercase tracking-wide text-muted-foreground/70 mb-0.5">
                            {t('agentConfig.memoryHowToApply')}
                          </div>
                          <p className="text-xs text-muted-foreground leading-relaxed m-0">{m.howToApply}</p>
                        </div>
                      )}
                      {m.sessionId && (
                        <div className="text-[10px] text-muted-foreground/60 font-mono break-all">
                          {t('agentConfig.memorySession')}: {m.sessionId}
                        </div>
                      )}
                    </div>
                  )}
                </div>
                <div className="flex items-center gap-1 shrink-0">
                  <button
                    onClick={() => startEdit(m)}
                    className="p-1.5 rounded-md hover:bg-primary/10 text-muted-foreground hover:text-primary transition-colors cursor-pointer"
                    title={t('common.edit')}
                  >
                    <Pencil size={14} />
                  </button>
                  <button
                    onClick={() => handleDelete(m)}
                    className="p-1.5 rounded-md hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors cursor-pointer"
                    title={t('common.delete')}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            </div>
            )
          })}
          {nextCursor && (
            <button
              onClick={loadMore}
              disabled={loadingMore}
              className="w-full h-9 rounded-xl border border-border text-xs text-muted-foreground hover:text-foreground hover:border-primary/50 transition-colors cursor-pointer disabled:opacity-50"
            >
              {loadingMore ? t('common.loading') : t('agentConfig.memoryLoadMore')}
            </button>
          )}
        </div>
      )}
    </div>
  )
}
