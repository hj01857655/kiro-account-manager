import { useEffect, useState } from 'react'
import { Brain, Trash2 } from 'lucide-react'
import { deleteMemory, getMemoryMode, listMemories, type MemoryItem } from '../../../api/memoryApi'
import { setKiroAgentSetting } from '../../../api/settingsApi'
import { useDialog } from '../../../contexts/DialogContext'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'

// Agent Memory 管理面板（Kiro 1.4+）：~/.kiro/memories/memories.db 的记忆条目
// 列表/删除 + kiroAgent.memory 模式切换（settings.json 键，四档映射见 memory_cmd.rs）。

// 官方四档 → {mode, reflection}（docs/Kiro memory 系统事实.md）
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

  const load = () => {
    setLoading(true)
    Promise.all([listMemories(), getMemoryMode()])
      .then(([items, mode]) => {
        setMemories(items || [])
        setDbMissing(false)
        setModeId(modeToId(mode.mode, mode.reflection))
      })
      .catch((e: any) => {
        // db 不存在 = Memory 从未启用，按空态处理而不是报错
        if (String(e?.message || e).includes('不存在')) {
          setMemories([])
          setDbMissing(true)
          setModeId('off')
        } else {
          showError(t('common.error'), String(e?.message || e))
        }
      })
      .finally(() => setLoading(false))
  }

  useEffect(load, [])

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
      <div className="glass-card border border-border rounded-xl p-4">
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

      {/* 记忆列表 */}
      {loading ? (
        <div className="text-center py-10 text-muted-foreground text-sm">{t('common.loading')}</div>
      ) : memories.length === 0 ? (
        <div className="text-center py-10 glass-card border border-border rounded-xl">
          <Brain size={36} className="mx-auto mb-3 text-muted-foreground opacity-40" />
          <p className="text-sm text-muted-foreground">
            {dbMissing ? t('agentConfig.memoriesDbMissing') : t('agentConfig.memoriesEmpty')}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {memories.map(m => (
            <div key={m.id} className="glass-card border border-border rounded-xl p-3.5">
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-sm font-medium text-foreground">{m.title || m.id}</span>
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
                </div>
                <button
                  onClick={() => handleDelete(m)}
                  className="p-1.5 rounded-md hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors cursor-pointer shrink-0"
                  title={t('common.delete')}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
