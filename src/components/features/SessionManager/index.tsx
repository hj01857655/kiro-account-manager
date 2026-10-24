import { useState, useEffect, useRef, useMemo } from 'react'
import { sessionApi } from '@/api/sessionApi'
import { SessionSummary, IdeSession } from '@/types/session'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Loader2,
  Search,
  Trash2,
  Download,
  MessageSquare,
  ChevronRight,
  ChevronDown,
  RefreshCw,
  Clock,
  Copy,
  Check,
  FileText,
  X
} from 'lucide-react'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useDialog } from '@/contexts/DialogContext'
import { useProjectScope } from '@/contexts/ProjectScopeContext'
import { useApp } from '@/hooks/useApp'
import { showSuccess, showError, showWarning } from '@/utils/toast'

interface SessionCounts {
  workspaces: number
  loadedSessions: number
  selected: number
}

// 按 ``` 围栏切分消息文本：代码段深底等宽 + 复制，普通段保持 pre-wrap
function splitCodeBlocks(text: string): { code: boolean; content: string }[] {
  const parts: { code: boolean; content: string }[] = []
  let buf: string[] = []
  let inCode = false
  for (const line of text.split('\n')) {
    if (line.trimStart().startsWith('```')) {
      parts.push({ code: inCode, content: buf.join('\n') })
      buf = []
      inCode = !inCode
      continue
    }
    buf.push(line)
  }
  parts.push({ code: inCode, content: buf.join('\n') })
  return parts.filter(p => p.content.length > 0)
}

function CopyBtn({ text, t, className }: { text: string; t: (k: string) => string; className?: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      onClick={(e) => {
        e.stopPropagation()
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true)
          setTimeout(() => setCopied(false), 1500)
        })
      }}
      className={className}
      title={copied ? t('sessions.copied') : t('sessions.copy')}
    >
      {copied ? <Check size={12} /> : <Copy size={12} />}
    </button>
  )
}

function MessageBody({ text, t }: { text: string; t: (k: string) => string }) {
  return (
    <>
      {splitCodeBlocks(text).map((p, i) =>
        p.code ? (
          <div key={i} className="relative my-1 rounded-xl bg-zinc-900 ring-1 ring-zinc-700/50">
            <button
              onClick={() => navigator.clipboard.writeText(p.content)}
              title={t('sessions.copy')}
              className="absolute right-2 top-2 rounded-md p-1.5 text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100 cursor-pointer"
            >
              <Copy size={12} />
            </button>
            <pre className="overflow-x-auto p-3 pr-9 font-mono text-xs leading-5 text-zinc-100">{p.content}</pre>
          </div>
        ) : (
          <div key={i} className="whitespace-pre-wrap break-words rounded-xl bg-background/70 p-3 text-sm leading-6 text-foreground/90 ring-1 ring-border/50">
            {p.content}
          </div>
        )
      )}
    </>
  )
}

function IdeSessionManager({ onCountsChange }: { onCountsChange?: (counts: SessionCounts) => void }) {
  const { t } = useApp()
  const { showConfirm } = useDialog()
  // 项目作用域：选中项目时会话列表只显示该 workspace
  const { projectDir } = useProjectScope()
  const [workspaces, setWorkspaces] = useState<string[]>([])
  const [selectedWorkspace, setSelectedWorkspace] = useState<string | null>(null)
  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Set<string>>(new Set())
  const [workspaceSessions, setWorkspaceSessions] = useState<Map<string, SessionSummary[]>>(new Map())
  const [selectedSession, setSelectedSession] = useState<IdeSession | null>(null)
  const [loading, setLoading] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedWorkspaceHashes, setSelectedWorkspaceHashes] = useState<Set<string>>(new Set())

  // 加载 workspaces：选中项目时只显示该项目的 workspace（未命中则空）；未选项目展示全部
  useEffect(() => {
    loadForScope()
  }, [projectDir])

  const toggleWorkspace = async (workspaceHash: string) => {
    const newExpanded = new Set(expandedWorkspaces)

    if (newExpanded.has(workspaceHash)) {
      // 折叠 - 清空该工作区的选中状态
      newExpanded.delete(workspaceHash)
      if (selectedWorkspace === workspaceHash) {
        setSelectedWorkspace(null)
      }
    } else {
      // 展开 - 加载该工作区的 sessions
      newExpanded.add(workspaceHash)
      if (!workspaceSessions.has(workspaceHash)) {
        await loadSessionsForWorkspace(workspaceHash)
      }
    }

    setExpandedWorkspaces(newExpanded)
  }

  const loadSessionsForWorkspace = async (workspaceHash: string) => {
    try {
      const data = await sessionApi.listSessions(workspaceHash)
      setWorkspaceSessions(prev => new Map(prev).set(workspaceHash, data))
    } catch (error) {
      console.error('Failed to load sessions:', error)
      showError(t('sessions.loadSessionListFailed', { error }))
    }
  }

  /// 刷新：工作区列表与已展开的会话都重拉。
  /// 后端摘要按 (大小, 修改时间) 缓存，未变的文件不会重复解析，所以全量重拉代价很小。
  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      await loadForScope()
      const expanded = Array.from(expandedWorkspaces)
      await Promise.all(expanded.map(loadSessionsForWorkspace))
    } finally {
      setRefreshing(false)
    }
  }

  const decodeWorkspaceName = (hash: string) => {
    try {
      // 移除末尾的 __ 或 _
      const cleaned = hash.replace(/_+$/, '')
      // Base64 解码
      const decoded = atob(cleaned)
      // 提取最后一个路径段作为显示名称
      const parts = decoded.split(/[/\\]/)
      const name = parts[parts.length - 1] || parts[parts.length - 2] || decoded
      return name
    } catch {
      return hash
    }
  }

  const loadWorkspaces = async () => {
    try {
      setLoading(true)
      const data = await sessionApi.listWorkspaces()
      setWorkspaces(data)
    } catch (error) {
      console.error('Failed to load workspaces:', error)
      showError(t('sessions.loadWorkspaceFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  // 按 projectDir 作用域加载：未选项目 → 全部 workspace；选中 → 只加载匹配该目录的 workspace
  const loadForScope = async () => {
    if (!projectDir) {
      await loadWorkspaces()
      return
    }
    setLoading(true)
    try {
      const hash = await sessionApi.findWorkspaceByDir(projectDir)
      if (hash) {
        setWorkspaces([hash])
        setSelectedWorkspace(hash)
        setExpandedWorkspaces(new Set([hash]))
        await loadSessionsForWorkspace(hash)
      } else {
        setWorkspaces([])
        setWorkspaceSessions(new Map())
        setExpandedWorkspaces(new Set())
        setSelectedWorkspace(null)
      }
    } catch (error) {
      console.error('Failed to find workspace for project:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleSelectSession = async (workspaceHash: string, session: SessionSummary) => {
    // 如果点击的是当前已选中的 session，不重复加载
    if (selectedSession?.sessionId === session.sessionId) {
      return
    }

    try {
      setLoading(true)
      setSelectedSession(null) // 先清空，避免显示旧数据
      setSelectedWorkspace(workspaceHash)
      const data = await sessionApi.loadSession(workspaceHash, session.sessionId)
      setSelectedSession(data)
    } catch (error) {
      console.error('Failed to load session:', error)
      showError(t('sessions.loadFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteWorkspace = async (workspaceHash: string) => {
    const workspaceName = decodeWorkspaceName(workspaceHash)

    const confirmed = await showConfirm(
      t('sessions.deleteWorkspaceTitle'),
      t('sessions.deleteWorkspaceConfirm', { name: workspaceName })
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // 直接删除整个工作区目录
      await sessionApi.deleteWorkspace(workspaceHash)

      // 重新加载工作区列表
      await loadWorkspaces()

      // 清空相关状态
      setExpandedWorkspaces(prev => {
        const newSet = new Set(prev)
        newSet.delete(workspaceHash)
        return newSet
      })
      setWorkspaceSessions(prev => {
        const newMap = new Map(prev)
        newMap.delete(workspaceHash)
        return newMap
      })
      if (selectedWorkspace === workspaceHash) {
        setSelectedWorkspace(null)
        setSelectedSession(null)
      }

      showSuccess(t('sessions.workspaceDeleted', { name: workspaceName }))
    } catch (error) {
      console.error('Failed to delete workspace:', error)
      showError(t('sessions.deleteWorkspaceFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteSession = async (workspaceHash: string, session: SessionSummary) => {
    const confirmed = await showConfirm(
      t('sessions.deleteSessionTitle'),
      t('sessions.deleteSessionConfirm', { title: session.title })
    )

    if (!confirmed) return

    try {
      await sessionApi.deleteSession(session.workspaceHash, session.sessionId)

      // 重新加载该工作区的会话列表
      await loadSessionsForWorkspace(workspaceHash)

      // 如果删除的是当前选中的 session，清空详情
      if (selectedSession?.sessionId === session.sessionId) {
        setSelectedSession(null)
      }
      showSuccess(t('sessions.sessionDeleted'))
    } catch (error) {
      console.error('Failed to delete session:', error)
      showError(t('sessions.deleteFailed', { error }))
    }
  }

  const toggleWorkspaceSelection = (workspaceHash: string) => {
    const newSelected = new Set(selectedWorkspaceHashes)
    if (newSelected.has(workspaceHash)) {
      newSelected.delete(workspaceHash)
    } else {
      newSelected.add(workspaceHash)
    }
    setSelectedWorkspaceHashes(newSelected)
  }

  const toggleSelectAllWorkspaces = () => {
    if (selectedWorkspaceHashes.size === workspaces.length) {
      setSelectedWorkspaceHashes(new Set())
    } else {
      setSelectedWorkspaceHashes(new Set(workspaces))
    }
  }

  const handleBatchDeleteWorkspaces = async () => {
    if (selectedWorkspaceHashes.size === 0) {
      showWarning(t('sessions.pleaseSelectWorkspace'))
      return
    }

    const workspaceNames = Array.from(selectedWorkspaceHashes)
      .map(hash => decodeWorkspaceName(hash))
      .join('、')

    const confirmed = await showConfirm(
      t('sessions.batchDeleteWorkspaceTitle'),
      t('sessions.batchDeleteWorkspaceConfirm', {
        count: selectedWorkspaceHashes.size,
        names: workspaceNames
      })
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // 直接删除所有选中的工作区目录
      for (const workspaceHash of selectedWorkspaceHashes) {
        await sessionApi.deleteWorkspace(workspaceHash)
      }

      // 重新加载工作区列表
      await loadWorkspaces()

      // 清空相关状态
      setExpandedWorkspaces(new Set())
      setWorkspaceSessions(new Map())
      setSelectedWorkspaceHashes(new Set())
      setSelectedWorkspace(null)
      setSelectedSession(null)

      showSuccess(t('sessions.batchDeleteSuccess', { count: selectedWorkspaceHashes.size }))
    } catch (error) {
      console.error('Failed to batch delete workspaces:', error)
      showError(t('sessions.batchDeleteFailed', { error }))
    } finally {
      setLoading(false)
    }
  }

  const handleExportSession = async (format: 'json' | 'markdown') => {
    if (!selectedSession) return

    try {
      // 从 workspaceSessions 中找到对应的 session 获取 workspaceHash
      let workspaceHash = ''
      for (const [hash, sessions] of workspaceSessions.entries()) {
        if (sessions.some(s => s.sessionId === selectedSession.sessionId)) {
          workspaceHash = hash
          break
        }
      }

      if (!workspaceHash) {
        showError(t('sessions.sessionNotFound'))
        return
      }

      const content = await sessionApi.exportSession(
        workspaceHash,
        selectedSession.sessionId,
        format
      )

      const ext = format === 'json' ? 'json' : 'md'
      const defaultPath = `${selectedSession.title}.${ext}`

      const filePath = await save({
        defaultPath,
        filters: [{
          name: format === 'json' ? 'JSON' : 'Markdown',
          extensions: [ext]
        }]
      })

      if (filePath) {
        await writeTextFile(filePath, content)
        showSuccess(t('sessions.exportSuccess'))
      }
    } catch (error) {
      console.error('Failed to export session:', error)
      showError(t('sessions.exportFailed', { error }))
    }
  }

  // 搜索匹配多个字段，与 magpie 的 shown() 一致：标题、路径、id、模型名。
  // 全文搜索：后端 search_sessions 遍历所有 workspace 的会话内容，
  // 前端只做 300ms 防抖，结果集直接来自后端（不再本地过滤标题）
  const [searchResults, setSearchResults] = useState<SessionSummary[]>([])
  const [searching, setSearching] = useState(false)

  useEffect(() => {
    const needle = searchQuery.trim()
    if (!needle) {
      setSearchResults([])
      setSearching(false)
      return
    }
    setSearching(true)
    const timer = setTimeout(() => {
      sessionApi.searchSessions(needle)
        .then(r => setSearchResults(r))
        .catch(() => setSearchResults([]))
        .finally(() => setSearching(false))
    }, 300)
    return () => clearTimeout(timer)
  }, [searchQuery])

  const filteredSessions = searchResults

  // 相对时间：30 天内用「N 前」，超过回退绝对时间
  const formatRelativeTime = (ms: number) => {
    const diff = Date.now() - ms
    const minutes = Math.floor(diff / 60000)
    if (minutes < 1) return t('sessions.timeJustNow')
    if (minutes < 60) return t('sessions.timeMinutesAgo', { n: minutes })
    const hours = Math.floor(minutes / 60)
    if (hours < 24) return t('sessions.timeHoursAgo', { n: hours })
    const days = Math.floor(hours / 24)
    if (days < 30) return t('sessions.timeDaysAgo', { n: days })
    return new Date(ms).toLocaleString()
  }

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '-'
    return new Date(timestamp * 1000).toLocaleString()
  }

  // 获取工作区的会话列表
  const getWorkspaceSessions = (workspaceHash: string) => {
    return workspaceSessions.get(workspaceHash) || []
  }

  const loadedSessionCount = Array.from(workspaceSessions.values()).reduce(
    (total, sessions) => total + sessions.length,
    0
  )

  // 统计口径上抛给页头；页头位于 Tabs 外层，切到 CLI 标签也常驻
  useEffect(() => {
    onCountsChange?.({
      workspaces: workspaces.length,
      loadedSessions: loadedSessionCount,
      selected: selectedWorkspaceHashes.size,
    })
  }, [workspaces.length, loadedSessionCount, selectedWorkspaceHashes.size, onCountsChange])

  const selectedSessionId = selectedSession?.sessionId

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex min-h-0 flex-1 overflow-hidden p-4 gap-4">
        {/* Left Sidebar - Workspaces with expandable sessions */}
        <div className="w-80 shrink-0 overflow-hidden rounded-2xl border border-border/70 bg-card/80 shadow-sm flex flex-col">
          <div className="border-b border-border/70 bg-muted/20 p-4 space-y-3">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold text-foreground">{t('sessions.workspacesAndSessions')}</h2>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 rounded-full px-2 text-[11px] hover:bg-primary/10 hover:text-primary"
                  onClick={handleRefresh}
                  disabled={refreshing}
                  title={t('sessions.refresh')}
                >
                  <RefreshCw className={`h-3 w-3 ${refreshing ? 'animate-spin' : ''}`} />
                </Button>
                {selectedWorkspaceHashes.size > 0 && (
                  <Button
                    variant="destructive"
                    size="sm"
                    className="h-7 rounded-full px-3 text-[11px]"
                    onClick={handleBatchDeleteWorkspaces}
                  >
                    <Trash2 className="h-3 w-3 mr-1" />
                    {t('sessions.deleteCount', { count: selectedWorkspaceHashes.size })}
                  </Button>
                )}
              </div>
            </div>
            <div className="flex items-center justify-between rounded-xl bg-background/70 px-3 py-2 text-[11px] text-muted-foreground ring-1 ring-border/50">
              <span>{t('sessions.workspaceCount', { count: workspaces.length })}</span>
              {workspaces.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 rounded-full px-2 text-[11px] hover:bg-primary/10 hover:text-primary"
                  onClick={toggleSelectAllWorkspaces}
                >
                  {selectedWorkspaceHashes.size === workspaces.length ? t('sessions.deselectAll') : t('sessions.selectAll')}
                </Button>
              )}
            </div>
            {/* Search */}
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                placeholder={t('sessions.searchPlaceholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => {
                  // Esc 清空搜索，回到工作区树（同 magpie）
                  if (e.key === 'Escape' && searchQuery) {
                    e.stopPropagation()
                    setSearchQuery('')
                  }
                }}
                className="h-9 rounded-xl border-border/70 bg-background/80 pl-8 pr-8 text-xs shadow-inner focus-visible:ring-primary/30"
              />
              {searchQuery && (
                <button
                  type="button"
                  onClick={() => setSearchQuery('')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                  aria-label={t('sessions.clearSearch')}
                  title={t('sessions.clearSearch')}
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          </div>

          <ScrollArea className="flex-1">
            <div className="p-3 space-y-2">
              {/* 搜索模式：显示所有匹配的会话 */}
              {searchQuery && (
                <div className="space-y-2">
                  <div className="px-1 text-[11px] text-muted-foreground flex items-center gap-1.5" role="status">
                    {searching && <Loader2 className="h-3 w-3 animate-spin" />}
                    {t('sessions.searchResultCount', { count: filteredSessions.length })}
                  </div>
                  {filteredSessions.length === 0 ? (
                    <div className="rounded-2xl border border-dashed border-border/70 bg-muted/20 px-4 py-10 text-center text-sm text-muted-foreground">
                      {t('sessions.noMatch')}
                    </div>
                  ) : (
                    filteredSessions.map(session => {
                      const isSelected = selectedSessionId === session.sessionId

                      return (
                      <Card
                        key={session.sessionId}
                        className={`group relative cursor-pointer overflow-hidden rounded-2xl p-3 shadow-sm transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:bg-primary/5 hover:shadow-md ${isSelected ? 'border-primary/60 bg-primary/10 shadow-sm ring-1 ring-primary/20' : 'border-border/70 bg-card'}
                        `}
                        onClick={() => handleSelectSession(session.workspaceHash, session)}
                      >
                        {isSelected && (
                          <>
                            <div className="absolute inset-y-2 left-0 w-1 rounded-r-full bg-primary" />
                          </>
                        )}
                        <div className="space-y-2 pl-1">
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <h3 className={`line-clamp-2 text-sm font-semibold leading-snug ${isSelected ? 'text-primary' : 'text-foreground'}`}>
                                {session.title}
                              </h3>
                              <p className="mt-1 truncate text-xs text-muted-foreground">
                                {decodeWorkspaceName(session.workspaceHash)}
                              </p>
                            </div>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-7 w-7 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-opacity hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                              onClick={(e) => {
                                e.stopPropagation()
                                handleDeleteSession(session.workspaceHash, session)
                              }}
                              title={t('sessions.deleteSessionTitle')}
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                          <div className="flex items-center gap-2 flex-wrap">
                            <Badge variant="secondary" className="h-5 rounded-full px-2 text-[10px] font-normal">
                              {session.sessionType}
                            </Badge>
                            <span className="flex items-center gap-1 text-xs text-muted-foreground">
                              <MessageSquare className="h-3 w-3" />
                              {session.messageCount}
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {formatFileSize(session.fileSize)}
                            </span>
                            {typeof session.modifiedAt === 'number' && (
                              <span
                                className="flex items-center gap-1 text-xs text-muted-foreground"
                                title={new Date(session.modifiedAt).toLocaleString()}
                              >
                                <Clock className="h-3 w-3" />
                                {formatRelativeTime(session.modifiedAt)}
                              </span>
                            )}
                          </div>
                        </div>
                      </Card>
                      )
                    })
                  )}
                </div>
              )}

              {/* 空态：无会话时给出明确反馈（选中项目未命中时尤其重要） */}
              {!searchQuery && !loading && workspaces.length === 0 && (
                <div className="py-12 text-center">
                  <MessageSquare className="mx-auto h-8 w-8 mb-2 opacity-40" />
                  <p className="text-sm text-muted-foreground">
                    {projectDir ? t('sessions.noSessionsForProject') : t('sessions.noSessions')}
                  </p>
                </div>
              )}

              {/* 正常模式：显示工作区树 */}
              {!searchQuery && workspaces.map(workspace => {
                const isExpanded = expandedWorkspaces.has(workspace)
                const sessions = getWorkspaceSessions(workspace)

                return (
                  <div key={workspace} className="space-y-1 min-w-0">
                    {/* Workspace Row */}
                    <div
                      className={`group relative rounded-xl transition-all duration-200 cursor-pointer ${selectedWorkspace === workspace
                          ? 'border-2 border-primary/70 bg-gradient-to-br from-primary/12 via-primary/8 to-primary/4 shadow-lg ring-1 ring-primary/30 scale-[1.02]'
                          : isExpanded
                          ? 'border border-border/60 bg-muted/40 hover:border-primary/50 hover:bg-muted/50 hover:shadow-sm'
                          : 'border border-border/40 bg-card hover:border-border/70 hover:bg-muted/40'
                        }`}
                      onClick={() => toggleWorkspace(workspace)}
                    >
                      <div className="flex items-center gap-2 px-2.5 py-2.5">
                        {/* Expand/Collapse Icon */}
                        <button
                          onClick={() => toggleWorkspace(workspace)}
                          className={`shrink-0 rounded-lg p-1 transition-all ${selectedWorkspace === workspace ? 'text-primary hover:bg-primary/15 hover:text-primary' : 'text-muted-foreground hover:bg-background hover:text-foreground'}`}
                          title={isExpanded ? t('sessions.collapse') : t('sessions.expand')}
                        >
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>

                        {/* Checkbox */}
                        <Checkbox
                          checked={selectedWorkspaceHashes.has(workspace)}
                          onCheckedChange={(checked) => {
                            toggleWorkspaceSelection(workspace)
                          }}
                          onClick={(e) => e.stopPropagation()}
                          className="shrink-0 cursor-pointer border-foreground/40 data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground"
                        />

                        {/* Workspace Name */}
                        <div
                          className="flex-1 min-w-0 rounded-lg px-2 py-1 text-left text-sm"
                          title={workspace}
                        >
                          <div className={`truncate font-medium transition-colors ${selectedWorkspace === workspace ? 'text-primary font-semibold' : 'text-foreground'}`}>
                            {decodeWorkspaceName(workspace)}
                          </div>
                          {isExpanded && sessions.length > 0 && (
                            <div className="mt-0.5 text-xs text-muted-foreground truncate">
                              {t('sessions.sessionCount', { count: sessions.length })}
                            </div>
                          )}
                        </div>

                        {/* Delete Button */}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-all hover:opacity-100 hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleDeleteWorkspace(workspace)
                          }}
                          title={t('sessions.deleteWorkspaceTitle')}
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </div>

                    {/* Sessions under this workspace (when expanded) */}
                    {isExpanded && (
                      <div className="ml-7 space-y-1.5 border-l-2 border-border/50 pl-3 py-1.5 min-w-0 overflow-hidden">
                        {loading && sessions.length === 0 ? (
                          <div className="flex items-center justify-center py-4">
                            <Loader2 className="h-4 w-4 animate-spin text-primary" />
                          </div>
                        ) : sessions.length === 0 ? (
                          <div className="rounded-xl border border-dashed border-border/60 bg-muted/20 px-3 py-3 text-xs text-muted-foreground">
                            {t('sessions.noSessions')}
                          </div>
                        ) : (
                          sessions.map(session => {
                            const isSelected = selectedSessionId === session.sessionId

                            return (
                            <Card
                              key={session.sessionId}
                              className={`group relative cursor-pointer overflow-hidden rounded-xl p-3 shadow-sm transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:bg-primary/5 hover:shadow-md min-w-0 ${isSelected ? 'border-primary/60 bg-primary/10 ring-1 ring-primary/20' : 'border-border/60 bg-card hover:border-border'}
                              `}
                              onClick={() => handleSelectSession(workspace, session)}
                            >
                              {isSelected && (
                                <div className="absolute inset-y-2 left-0 w-1 rounded-r-full bg-primary" />
                              )}
                              <div className="space-y-2 pl-1 min-w-0">
                                <div className="flex items-start justify-between gap-2 min-w-0">
                                  <h3 className={`line-clamp-2 flex-1 min-w-0 text-xs font-semibold leading-snug break-all overflow-hidden ${isSelected ? 'text-primary' : 'text-foreground'}`}>
                                    {session.title}
                                  </h3>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-6 w-6 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-opacity hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                                    onClick={(e) => {
                                      e.stopPropagation()
                                      handleDeleteSession(workspace, session)
                                    }}
                                    title={t('sessions.deleteSessionTitle')}
                                  >
                                    <Trash2 className="h-3 w-3" />
                                  </Button>
                                </div>
                                <div className="flex items-center gap-2 flex-wrap">
                                  <Badge variant="secondary" className="h-5 rounded-full px-2 text-[10px] font-normal">
                                    {session.sessionType}
                                  </Badge>
                                  <span className="flex items-center gap-1 text-xs text-muted-foreground">
                                    <MessageSquare className="h-3 w-3" />
                                    {session.messageCount}
                                  </span>
                                </div>
                              </div>
                            </Card>
                            )
                          })
                        )}
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          </ScrollArea>
        </div>

        {/* Right Panel - Session Detail */}
        <div className="min-w-0 flex-1 overflow-hidden rounded-2xl border border-border/70 bg-card/70 shadow-sm flex flex-col">
          {loading && selectedSession === null ? (
            <div className="flex-1 flex items-center justify-center text-muted-foreground">
              <div className="flex items-center gap-3 rounded-2xl border border-border/70 bg-background/80 px-5 py-4 shadow-sm">
                <Loader2 className="h-5 w-5 animate-spin text-primary" />
                <span className="text-sm">{t('sessions.loading')}</span>
              </div>
            </div>
          ) : selectedSession ? (
            <>
              <div className="border-b border-border/70 bg-background/75 px-5 py-3.5 shadow-sm flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <h2 className="truncate text-base font-semibold tracking-tight text-foreground">{selectedSession.title}</h2>
                  <p className="text-[11px] text-muted-foreground mt-0.5 truncate font-mono">
                    {selectedSession.workspaceDirectory}
                  </p>
                  <div className="mt-1.5 flex items-center gap-2 flex-wrap">
                    {(selectedSession.modelId || selectedSession.selectedModel) && (
                      <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px] font-normal">
                        🤖 {selectedSession.modelId || selectedSession.selectedModel}
                      </Badge>
                    )}
                    {selectedSession.autonomyMode && (
                      <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px] font-normal">
                        {selectedSession.autonomyMode === 'Autopilot' ? '🚀' : '👁️'} {selectedSession.autonomyMode}
                      </Badge>
                    )}
                  </div>
                </div>
                <div className="flex gap-1.5 ml-3">
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-8 rounded-full px-3 text-xs"
                    onClick={() => handleExportSession('json')}
                  >
                    <Download className="h-3.5 w-3.5 mr-1" />
                    JSON
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-8 rounded-full px-3 text-xs"
                    onClick={() => handleExportSession('markdown')}
                  >
                    <Download className="h-3.5 w-3.5 mr-1" />
                    Markdown
                  </Button>
                </div>
              </div>

              <ScrollArea className="flex-1">
                <div className="mx-auto max-w-5xl space-y-4 p-5">
                  {/* Conversation Summary - 从第一条消息中提取 */}
                  {selectedSession.history.length > 0 &&
                    selectedSession.history[0].message.role === 'user' &&
                    selectedSession.history[0].message.content.length > 0 &&
                    (selectedSession.history[0].message.content[0].text.includes('CONTEXT TRANSFER') ||
                      selectedSession.history[0].message.content[0].text.includes('## TASK') ||
                      selectedSession.title.includes('(Continued)')) && (
                      <Card className="overflow-hidden rounded-2xl border-blue-200/80 bg-gradient-to-br from-blue-50 to-cyan-50 p-0 shadow-sm dark:border-blue-800/70 dark:from-blue-950/70 dark:to-cyan-950/40">
                        <div className="flex items-start gap-3 p-4">
                          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-blue-500/10 text-xl ring-1 ring-blue-500/20">📝</div>
                          <div className="min-w-0 flex-1">
                            <div className="mb-2 font-semibold text-blue-900 dark:text-blue-100">
                              {t('sessions.conversationSummary')}
                            </div>
                            <div className="max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-xl bg-white/55 p-3 text-sm leading-6 text-blue-900 ring-1 ring-blue-200/60 dark:bg-black/20 dark:text-blue-100 dark:ring-blue-800/50">
                              {selectedSession.history[0].message.content[0].text}
                            </div>
                          </div>
                        </div>
                      </Card>
                    )}

                  {/* Messages */}
                  {selectedSession.history.length === 0 ? (
                    <div className="rounded-2xl border border-dashed border-border/70 bg-muted/20 px-6 py-14 text-center text-sm text-muted-foreground">
                      {t('sessions.noMessages')}
                    </div>
                  ) : (
                    selectedSession.history.map((item, index) => {
                      // 跳过第一条摘要消息（如果是压缩会话）
                      const isSummaryMessage = index === 0 &&
                        item.message.role === 'user' &&
                        item.message.content.length > 0 &&
                        (item.message.content[0].text.includes('CONTEXT TRANSFER') ||
                          item.message.content[0].text.includes('## TASK') ||
                          selectedSession.title.includes('(Continued)'))

                      if (isSummaryMessage) {
                        return null
                      }

                      return (
                        <Card key={item.message.id} className={`overflow-hidden rounded-2xl border-border/70 p-0 shadow-sm ${item.message.role === 'user' ? 'bg-background' : 'bg-muted/25'}`}>
                          <div className="flex items-start gap-3 p-4">
                            <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl text-lg ring-1 ${item.message.role === 'user' ? 'bg-primary/10 ring-primary/20' : 'bg-emerald-500/10 ring-emerald-500/20'}`}>
                              {item.message.role === 'user' ? '👤' : '🤖'}
                            </div>
                            <div className="min-w-0 flex-1">
                              <div className="mb-2 flex items-center gap-2">
                                <span className="text-sm font-semibold text-foreground">
                                  {item.message.role === 'user' ? t('sessions.user') : t('sessions.assistant')}
                                </span>
                                <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px] font-normal">
                                  #{index + 1}
                                </Badge>
                                {item.executionId && (
                                  <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px] font-mono font-normal text-muted-foreground">
                                    {item.executionId.slice(0, 8)}
                                  </Badge>
                                )}
                                <CopyBtn
                                  text={item.message.content.map((c: any) => c.text).join('\n')}
                                  t={t}
                                  className="ml-auto rounded-md p-1.5 text-muted-foreground opacity-0 transition-opacity hover:bg-muted/60 hover:text-foreground group-hover:opacity-100 cursor-pointer"
                                />
                              </div>
                              {item.contextItems && item.contextItems.length > 0 && (
                                <div className="mb-2 flex flex-wrap gap-1.5">
                                  {item.contextItems.map((ci: any, ciIdx: number) =>
                                    ci?.relativePath ? (
                                      <span
                                        key={ciIdx}
                                        title={ci.relativePath}
                                        className="inline-flex max-w-[220px] items-center gap-1 rounded-md bg-muted/50 px-1.5 py-0.5 text-[10px] text-muted-foreground ring-1 ring-border/40"
                                      >
                                        <FileText size={10} className="shrink-0" />
                                        <span className="truncate">{ci.relativePath.split(/[/\\]/).pop()}</span>
                                      </span>
                                    ) : null
                                  )}
                                </div>
                              )}
                              {item.message.content.map((content, i) => (
                                <MessageBody key={i} text={content.text} t={t} />
                              ))}
                            </div>
                          </div>
                        </Card>
                      )
                    })
                  )}
                </div>
              </ScrollArea>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center p-8">
              <div className="max-w-sm rounded-3xl border border-dashed border-border/80 bg-background/70 px-8 py-10 text-center shadow-sm">
                <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 ring-1 ring-primary/20">
                  <MessageSquare className="h-7 w-7 text-primary" />
                </div>
                <p className="font-medium text-foreground">{t('sessions.selectSessionToView')}</p>
                <p className="mt-2 text-sm text-muted-foreground">{t('sessions.selectFromTree')}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}


// ===== Source Switcher =====
import { lazy, Suspense } from 'react'
import { createPortal } from 'react-dom'
import { Terminal, MonitorSmartphone, ChevronDown as ChevronDownIcon, Check } from 'lucide-react'

const CliSessionManager = lazy(() => import('../CliSessionManager/index'))

/// 会话来源。目前只有 Kiro IDE 与 Kiro CLI；以后新增来源（如网关记录的会话）
/// 只需往这里加一项，下拉与页头都会自动跟上。
type SessionSource = {
  id: string
  label: string
  icon: typeof MonitorSmartphone
}

const SESSION_SOURCES: SessionSource[] = [
  { id: 'ide', label: 'Kiro IDE', icon: MonitorSmartphone },
  { id: 'cli', label: 'Kiro CLI', icon: Terminal },
]

/// 来源下拉：取代原先并列的两个 tab。
/// 用 Portal 渲染到 body，避开父容器 overflow-hidden 的裁剪（同 FilterDropdown）。
function SourcePicker({
  value,
  onChange,
}: {
  value: string
  onChange: (id: string) => void
}) {
  const [open, setOpen] = useState(false)
  const [rect, setRect] = useState<DOMRect | null>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const current = SESSION_SOURCES.find((s) => s.id === value) ?? SESSION_SOURCES[0]
  const CurrentIcon = current.icon

  useEffect(() => {
    if (!open) return
    const onPointerDown = (e: MouseEvent) => {
      const target = e.target as Node
      if (buttonRef.current?.contains(target) || panelRef.current?.contains(target)) return
      setOpen(false)
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('mousedown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={(e) => {
          setRect(e.currentTarget.getBoundingClientRect())
          setOpen((prev) => !prev)
        }}
        className="glass-card inline-flex h-9 items-center gap-2 rounded-lg border border-border/70 px-3 text-xs font-medium text-foreground transition-colors hover:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/40"
      >
        <CurrentIcon size={13} className="text-primary" />
        <span>{current.label}</span>
        <ChevronDownIcon size={13} className={`text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && rect &&
        createPortal(
          <div
            ref={panelRef}
            role="menu"
            className="fixed z-[9999] min-w-[168px] overflow-hidden rounded-xl border border-border glass-card shadow-xl"
            style={{
              top: `${rect.bottom + 6}px`,
              left: `${rect.left}px`,
              animation: 'slideDown 0.15s ease-out',
            }}
          >
            <div className="p-1">
              {SESSION_SOURCES.map((source) => {
                const Icon = source.icon
                const active = source.id === value
                return (
                  <button
                    key={source.id}
                    type="button"
                    role="menuitemradio"
                    aria-checked={active}
                    onClick={() => {
                      onChange(source.id)
                      setOpen(false)
                    }}
                    className={`flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-xs transition-colors ${
                      active ? 'bg-primary/10 text-primary' : 'text-foreground hover:bg-muted/60'
                    }`}
                  >
                    <Icon size={13} />
                    <span className="flex-1">{source.label}</span>
                    {active && <Check size={12} />}
                  </button>
                )
              })}
            </div>
          </div>,
          document.body
        )}
    </>
  )
}

export default function SessionManager() {
  const { t } = useApp()
  const [source, setSource] = useState('ide')
  const [counts, setCounts] = useState<SessionCounts>({ workspaces: 0, loadedSessions: 0, selected: 0 })
  return (
    <div className="flex h-full flex-col overflow-hidden bg-gradient-to-br from-background via-background to-muted/40">
      {/* 页头常驻：切换来源时标题与图标不消失 */}
      <div className="shrink-0 border-b border-border/70 bg-card/70 px-6 py-4 shadow-sm">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="relative flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-br from-primary via-primary/90 to-primary/70 shadow-lg shadow-primary/20 ring-1 ring-primary/25">
              <div className="absolute inset-0 rounded-2xl bg-white/10" />
              <MessageSquare size={21} className="relative text-primary-foreground" />
            </div>
            <div className="flex flex-col">
              <h1 className="text-xl font-semibold tracking-tight text-foreground">{t('sessions.title')}</h1>
              <p className="text-sm text-muted-foreground">{t('sessions.subtitle')}</p>
            </div>
          </div>

          {/* 统计口径来自 IDE 来源，切到 CLI 时不展示，避免误导 */}
          {source === 'ide' && (
            <div className="hidden items-center gap-2 md:flex">
              <Badge variant="secondary" className="h-8 rounded-full px-3 font-normal">
                {t('sessions.workspaceCount', { count: counts.workspaces })}
              </Badge>
              <Badge variant="outline" className="h-8 rounded-full px-3 font-normal bg-background/70">
                {t('sessions.loadedSessions', { count: counts.loadedSessions })}
              </Badge>
              {counts.selected > 0 && (
                <Badge variant="destructive" className="h-8 rounded-full px-3 font-normal">
                  {t('sessions.selectedCount', { count: counts.selected })}
                </Badge>
              )}
            </div>
          )}
        </div>
      </div>

      {/* 来源选择与页头左对齐 */}
      <div className="shrink-0 px-6 pt-3">
        <SourcePicker value={source} onChange={setSource} />
      </div>

      <div className="flex-1 min-h-0">
        {source === 'ide' ? (
          <IdeSessionManager onCountsChange={setCounts} />
        ) : (
          <Suspense
            fallback={
              <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                {t('sessions.loadingCli')}
              </div>
            }
          >
            <CliSessionManager />
          </Suspense>
        )}
      </div>
    </div>
  )
}