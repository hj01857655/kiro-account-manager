import { useState, useEffect, useMemo } from 'react'
import { cliSessionApi } from '@/api/cliSessionApi'
import { CliSessionSummary, CliSession } from '@/types/cliSession'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Loader2,
  Search,
  Trash2,
  Download,
  MessageSquare,
  Terminal,
  RefreshCw,
  X,
  Cpu,
  Clock,
  Coins,
} from 'lucide-react'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useDialog } from '@/contexts/DialogContext'
import { useApp } from '@/hooks/useApp'
import { showSuccess, showError } from '@/utils/toast'

export default function CliSessionManager() {
  const { t } = useApp()
  const { showConfirm } = useDialog()
  const [sessions, setSessions] = useState<CliSessionSummary[]>([])
  const [selectedSession, setSelectedSession] = useState<CliSession | null>(null)
  const [loading, setLoading] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')

  useEffect(() => {
    loadSessions()
  }, [])

  const loadSessions = async () => {
    try {
      setLoading(true)
      const data = await cliSessionApi.listSessions()
      setSessions(data)
    } catch (error) {
      console.error('Failed to load CLI sessions:', error)
      showError(t('cliSessions.loadListFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  // 搜索匹配多个字段（标题 / 工作目录 / 会话 id / 模型名），标题命中排前。
  const filteredSessions = useMemo(() => {
    const q = searchQuery.trim().toLowerCase()
    if (!q) return sessions
    const titleHits: CliSessionSummary[] = []
    const otherHits: CliSessionSummary[] = []
    for (const s of sessions) {
      if (s.title.toLowerCase().includes(q)) titleHits.push(s)
      else if (
        s.cwd.toLowerCase().includes(q) ||
        s.sessionId.toLowerCase().includes(q) ||
        (s.modelName || '').toLowerCase().includes(q)
      )
        otherHits.push(s)
    }
    return [...titleHits, ...otherHits]
  }, [sessions, searchQuery])

  const handleSelectSession = async (summary: CliSessionSummary) => {
    if (selectedSession?.sessionId === summary.sessionId) return
    try {
      setLoading(true)
      const full = await cliSessionApi.loadSession(summary.sessionId)
      setSelectedSession(full)
    } catch (error) {
      showError(t('cliSessions.loadDetailFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  const handleDelete = async (sessionId: string) => {
    const confirmed = await showConfirm(
      t('cliSessions.deleteTitle'),
      t('cliSessions.deleteConfirm')
    )
    if (!confirmed) return
    try {
      await cliSessionApi.deleteSession(sessionId)
      setSessions(prev => prev.filter(s => s.sessionId !== sessionId))
      if (selectedSession?.sessionId === sessionId) {
        setSelectedSession(null)
      }
      showSuccess(t('cliSessions.deleted'))
    } catch (error) {
      showError(t('cliSessions.deleteFailed', { error }))
    }
  }

  const handleExport = async (sessionId: string, format: 'json' | 'markdown') => {
    try {
      const content = await cliSessionApi.exportSession(sessionId, format)
      const ext = format === 'json' ? 'json' : 'md'
      const path = await save({
        defaultPath: `cli-session-${sessionId.slice(0, 8)}.${ext}`,
        filters: [{ name: format.toUpperCase(), extensions: [ext] }],
      })
      if (path) {
        await writeTextFile(path, content)
        showSuccess(t('cliSessions.exported', { path }))
      }
    } catch (error) {
      showError(t('cliSessions.exportFailed', { error }))
    }
  }

  const formatTime = (isoStr: string | null) => {
    if (!isoStr) return ''
    try {
      const date = new Date(isoStr)
      return !isNaN(date.getTime()) ? date.toLocaleString(undefined, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' }) : isoStr
    } catch {
      return isoStr
    }
  }

  const formatCredits = (credits: number) => {
    if (credits === 0) return '0'
    return credits.toFixed(2)
  }

  return (
    <div className="flex h-full min-h-0 gap-3 overflow-hidden p-4">
      {/* 左侧：会话列表 */}
      <div className="w-[340px] flex flex-col gap-2 shrink-0 min-h-0">
        {/* 搜索 */}
        <div className="relative">
          <Search className="absolute left-2.5 top-2 h-4 w-4 text-muted-foreground" />
          <Input
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Escape' && searchQuery) {
                e.stopPropagation()
                setSearchQuery('')
              }
            }}
            placeholder={t('cliSessions.searchPlaceholder')}
            className="h-8 pl-8 pr-8 text-xs"
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery('')}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
              aria-label={t('sessions.clearSearch')}
              title={t('sessions.clearSearch')}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>

        {/* 统计 */}
        <div className="flex items-center gap-2 px-1 text-[11px] text-muted-foreground">
          <Terminal size={12} />
          <span>{t('cliSessions.count', { count: filteredSessions.length })}</span>
          <Button
            variant="ghost"
            size="sm"
            className="ml-auto h-6 rounded-full px-2 text-[11px] hover:bg-primary/10 hover:text-primary"
            onClick={loadSessions}
            disabled={loading}
            title={t('sessions.refresh')}
          >
            <RefreshCw size={12} className={loading ? 'animate-spin' : ''} />
          </Button>
          {loading && <Loader2 size={12} className="animate-spin" />}
        </div>

        {/* 列表 */}
        <ScrollArea className="flex-1">
          <div className="space-y-1.5 pr-2">
            {filteredSessions.map(session => (
              <Card
                key={session.sessionId}
                className={`p-2.5 cursor-pointer transition-colors hover:bg-muted/50 ${
                  selectedSession?.sessionId === session.sessionId
                    ? 'border-primary bg-primary/5'
                    : ''
                }`}
                onClick={() => handleSelectSession(session)}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <p className="text-xs font-medium truncate">{session.title}</p>
                    <p className="text-[10px] text-muted-foreground truncate mt-0.5">
                      {session.cwd}
                    </p>
                    <div className="flex items-center gap-2 mt-1 flex-wrap">
                      {session.modelName && (
                        <Badge variant="secondary" className="text-[9px] px-1 py-0 h-4">
                          <Cpu size={8} className="mr-0.5" />
                          {session.modelName}
                        </Badge>
                      )}
                      <span className="text-[9px] text-muted-foreground flex items-center gap-0.5">
                        <MessageSquare size={8} />
                        {session.messageCount}
                      </span>
                      {session.totalCredits > 0 && (
                        <span className="text-[9px] text-muted-foreground flex items-center gap-0.5">
                          <Coins size={8} />
                          {formatCredits(session.totalCredits)}
                        </span>
                      )}
                    </div>
                  </div>
                  <div className="flex flex-col items-end gap-1">
                    <span className="text-[9px] text-muted-foreground whitespace-nowrap">
                      {formatTime(session.updatedAt)}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5"
                      onClick={e => {
                        e.stopPropagation()
                        handleDelete(session.sessionId)
                      }}
                    >
                      <Trash2 size={10} className="text-destructive" />
                    </Button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        </ScrollArea>
      </div>

      {/* 右侧：会话详情 */}
      <div className="flex-1 flex flex-col min-w-0 min-h-0">
        {selectedSession ? (
          <>
            {/* 头部 */}
            <div className="flex items-center justify-between pb-2 border-b mb-2">
              <div>
                <h3 className="text-sm font-medium">{selectedSession.title}</h3>
                <div className="flex items-center gap-2 mt-0.5">
                  <span className="text-[10px] text-muted-foreground">{selectedSession.cwd}</span>
                  {selectedSession.modelName && (
                    <Badge variant="outline" className="text-[9px] px-1 py-0">
                      {selectedSession.modelName}
                    </Badge>
                  )}
                  {selectedSession.contextUsage && (
                    <span className="text-[9px] text-muted-foreground">
                      {t('cliSessions.context', { pct: selectedSession.contextUsage.toFixed(1) })}
                    </span>
                  )}
                </div>
              </div>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs"
                  onClick={() => handleExport(selectedSession.sessionId, 'markdown')}
                >
                  <Download size={12} className="mr-1" />
                  MD
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs"
                  onClick={() => handleExport(selectedSession.sessionId, 'json')}
                >
                  <Download size={12} className="mr-1" />
                  JSON
                </Button>
              </div>
            </div>

            {/* 消息列表 */}
            <ScrollArea className="flex-1">
              <div className="space-y-3 pr-2">
                {selectedSession.messages.map((msg, idx) => (
                  <div
                    key={idx}
                    className={`rounded-lg p-3 text-xs ${
                      msg.kind === 'Prompt'
                        ? 'bg-primary/5 border border-primary/20'
                        : 'bg-muted/30 border border-border'
                    }`}
                  >
                    <div className="flex items-center gap-1.5 mb-1.5 text-[10px] text-muted-foreground">
                      {msg.kind === 'Prompt' ? (
                        <>
                          <span className="font-medium text-primary">👤 {t('cliSessions.user')}</span>
                        </>
                      ) : (
                        <>
                          <span className="font-medium text-emerald-600">🤖 {t('cliSessions.assistant')}</span>
                        </>
                      )}
                      {msg.data.meta?.timestamp && (
                        <span className="ml-auto flex items-center gap-0.5">
                          <Clock size={8} />
                          {new Date(msg.data.meta.timestamp * 1000).toLocaleTimeString()}
                        </span>
                      )}
                    </div>
                    <div className="whitespace-pre-wrap break-words leading-relaxed">
                      {msg.data.content.map((c, i) => (
                        <span key={i}>{c.data}</span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
            <div className="text-center">
              <Terminal size={32} className="mx-auto mb-2 opacity-30" />
              <p>{t('cliSessions.selectToView')}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
