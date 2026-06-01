import { useState, useEffect } from 'react'
import { sessionApi } from '@/api/sessionApi'
import { SessionSummary, IdeSession } from '@/types/session'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Checkbox } from '@/components/ui/checkbox'
import { Loader2, Search, Trash2, Download, MessageSquare, ChevronRight, ChevronDown } from 'lucide-react'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useDialog } from '@/contexts/DialogContext'
import { showSuccess, showError, showWarning } from '@/utils/toast'
import { useApp } from '@/hooks/useApp'

export default function SessionManager() {
  const { t } = useApp()
  const { showConfirm } = useDialog()
  const [workspaces, setWorkspaces] = useState<string[]>([])
  const [selectedWorkspace, setSelectedWorkspace] = useState<string | null>(null)
  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Set<string>>(new Set())
  const [workspaceSessions, setWorkspaceSessions] = useState<Map<string, SessionSummary[]>>(new Map())
  const [selectedSession, setSelectedSession] = useState<IdeSession | null>(null)
  const [loading, setLoading] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedWorkspaceHashes, setSelectedWorkspaceHashes] = useState<Set<string>>(new Set())

  // Load workspaces.
  useEffect(() => {
    loadWorkspaces()
  }, [])

  const toggleWorkspace = async (workspaceHash: string) => {
    const newExpanded = new Set(expandedWorkspaces)

    if (newExpanded.has(workspaceHash)) {
      newExpanded.delete(workspaceHash)
    } else {
      // Expand and load this workspace's sessions.
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
      showError(t('session.loadSessionsFailed') + error)
    }
  }

  const decodeWorkspaceName = (hash: string) => {
    try {
      // Remove trailing underscores.
      const cleaned = hash.replace(/_+$/, '')
      // Decode Base64.
      const decoded = atob(cleaned)
      // Use the final path segment as the display name.
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
      showError(t('session.loadWorkspacesFailed') + error)
    } finally {
      setLoading(false)
    }
  }

  const handleSelectSession = async (workspaceHash: string, session: SessionSummary) => {
    // Do not reload the currently selected session.
    if (selectedSession?.sessionId === session.sessionId) {
      return
    }

    try {
      setLoading(true)
      setSelectedSession(null)
      const data = await sessionApi.loadSession(workspaceHash, session.sessionId)
      setSelectedSession(data)
    } catch (error) {
      console.error('Failed to load session:', error)
      showError(t('session.loadFailed') + error)
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteWorkspace = async (workspaceHash: string) => {
    const workspaceName = decodeWorkspaceName(workspaceHash)

    const confirmed = await showConfirm(
      t('session.deleteWorkspace'),
      t('session.deleteWorkspaceConfirm', { name: workspaceName })
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // Delete the whole workspace directory.
      await sessionApi.deleteWorkspace(workspaceHash)

      // Reload workspace list.
      await loadWorkspaces()

      // Clear related state.
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

      showSuccess(t('session.workspaceDeleted', { name: workspaceName }))
    } catch (error) {
      console.error('Failed to delete workspace:', error)
      showError(t('session.deleteWorkspaceFailed') + error)
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteSession = async (workspaceHash: string, session: SessionSummary) => {
    const confirmed = await showConfirm(
      t('session.deleteSession'),
      t('session.deleteSessionConfirm', { title: session.title })
    )

    if (!confirmed) return

    try {
      await sessionApi.deleteSession(session.workspaceHash, session.sessionId)

      // Reload this workspace's sessions.
      await loadSessionsForWorkspace(workspaceHash)

      // Clear details if the selected session was deleted.
      if (selectedSession?.sessionId === session.sessionId) {
        setSelectedSession(null)
      }
      showSuccess(t('session.sessionDeleted'))
    } catch (error) {
      console.error('Failed to delete session:', error)
      showError(t('session.deleteFailed') + error)
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
      showWarning(t('session.selectWorkspacesFirst'))
      return
    }

    const workspaceNames = Array.from(selectedWorkspaceHashes)
      .map(hash => decodeWorkspaceName(hash))
      .join('、')

    const confirmed = await showConfirm(
      t('session.batchDeleteWorkspaces'),
      t('session.batchDeleteConfirm', { count: selectedWorkspaceHashes.size, names: workspaceNames })
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // Delete all selected workspace directories.
      for (const workspaceHash of selectedWorkspaceHashes) {
        await sessionApi.deleteWorkspace(workspaceHash)
      }

      // Reload workspace list.
      await loadWorkspaces()

      // Clear related state.
      setExpandedWorkspaces(new Set())
      setWorkspaceSessions(new Map())
      setSelectedWorkspaceHashes(new Set())
      setSelectedWorkspace(null)
      setSelectedSession(null)

      showSuccess(t('session.workspacesDeleted', { count: selectedWorkspaceHashes.size }))
    } catch (error) {
      console.error('Failed to batch delete workspaces:', error)
      showError(t('session.batchDeleteFailed') + error)
    } finally {
      setLoading(false)
    }
  }

  const handleExportSession = async (format: 'json' | 'markdown') => {
    if (!selectedSession) return

    try {
      // Find the session's workspace hash from workspaceSessions.
      let workspaceHash = ''
      for (const [hash, sessions] of workspaceSessions.entries()) {
        if (sessions.some(s => s.sessionId === selectedSession.sessionId)) {
          workspaceHash = hash
          break
        }
      }

      if (!workspaceHash) {
        showError(t('session.cannotFindWorkspace'))
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
        showSuccess(t('session.exportSuccess'))
      }
    } catch (error) {
      console.error('Failed to export session:', error)
      showError(t('session.exportFailed') + error)
    }
  }

  const filteredSessions = searchQuery
    ? Array.from(workspaceSessions.values())
      .flat()
      .filter(session => session.title.toLowerCase().includes(searchQuery.toLowerCase()))
    : []

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '-'
    return new Date(timestamp * 1000).toLocaleString('zh-CN')
  }

  // Get sessions for a workspace.
  const getWorkspaceSessions = (workspaceHash: string) => {
    return workspaceSessions.get(workspaceHash) || []
  }

  return (
    <div className="flex flex-col h-full glass-main">
      {/* Header */}
      <div className="px-5 py-3 border-b border-border flex items-center gap-2.5">
        <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary/80 to-primary flex items-center justify-center shadow-md ring-1 ring-primary/20">
          <MessageSquare size={20} className="text-primary-foreground" />
        </div>
        <div className="flex flex-col">
          <h1 className="text-lg font-semibold text-foreground leading-tight">{t('session.title')}</h1>
          <p className="text-sm text-muted-foreground leading-tight">{t('session.subtitle')}</p>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* Left Sidebar - Workspaces with expandable sessions */}
        <div className="w-72 border-r border-border flex flex-col">
          <div className="p-3 border-b border-border space-y-2">
            <div className="flex items-center justify-between">
              <h2 className="text-xs font-semibold text-foreground">{t('session.workspacesAndSessions')}</h2>
              {selectedWorkspaceHashes.size > 0 && (
                <Button
                  variant="destructive"
                  size="sm"
                  className="h-6 text-[11px]"
                  onClick={handleBatchDeleteWorkspaces}
                >
                  <Trash2 className="h-3 w-3 mr-1" />
                  {t('session.delete')} ({selectedWorkspaceHashes.size})
                </Button>
              )}
            </div>
            <div className="flex items-center justify-between text-[11px] text-muted-foreground">
              <span>{workspaces.length} {t('session.workspaces')}</span>
              {workspaces.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-5 px-2 text-[11px]"
                  onClick={toggleSelectAllWorkspaces}
                >
                  {selectedWorkspaceHashes.size === workspaces.length ? t('session.deselectAll') : t('session.selectAll')}
                </Button>
              )}
            </div>
            {/* Search */}
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                placeholder={t('session.searchSessions')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="h-8 pl-8 text-xs"
              />
            </div>
          </div>

          <ScrollArea className="flex-1">
            <div className="p-2 space-y-1">
              {/* Search mode: show all matching sessions. */}
              {searchQuery && (
                <div className="space-y-2">
                  {filteredSessions.length === 0 ? (
                    <div className="text-center py-8 text-muted-foreground text-sm">
                      {t('session.noMatchingSessions')}
                    </div>
                  ) : (
                    filteredSessions.map(session => (
                      <Card
                        key={session.sessionId}
                        className={`p-3 cursor-pointer hover:bg-accent transition-colors ${selectedSession?.sessionId === session.sessionId ? 'bg-accent' : ''
                          }`}
                        onClick={() => handleSelectSession(session.workspaceHash, session)}
                      >
                        <div className="space-y-2">
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <h3 className="font-medium text-sm line-clamp-2">
                                {session.title}
                              </h3>
                              <p className="text-xs text-muted-foreground truncate mt-1">
                                {decodeWorkspaceName(session.workspaceHash)}
                              </p>
                            </div>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-6 w-6 shrink-0 hover:bg-destructive hover:text-destructive-foreground"
                              onClick={(e) => {
                                e.stopPropagation()
                                handleDeleteSession(session.workspaceHash, session)
                              }}
                              title={t('session.deleteSession')}
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                          <div className="flex items-center gap-2 flex-wrap">
                            <Badge variant="secondary" className="text-xs">
                              {session.sessionType}
                            </Badge>
                            <span className="text-xs text-muted-foreground flex items-center gap-1">
                              <MessageSquare className="h-3 w-3" />
                              {session.messageCount}
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {formatFileSize(session.fileSize)}
                            </span>
                          </div>
                        </div>
                      </Card>
                    ))
                  )}
                </div>
              )}

              {/* Normal mode: workspace tree. */}
              {!searchQuery && workspaces.map(workspace => {
                const isExpanded = expandedWorkspaces.has(workspace)
                const sessions = getWorkspaceSessions(workspace)

                return (
                  <div key={workspace} className="space-y-1">
                    {/* Workspace Row */}
                    <div
                      className={`group relative rounded-md transition-all ${selectedWorkspace === workspace
                          ? 'bg-primary text-primary-foreground shadow-sm'
                          : ''
                        }`}
                    >
                      <div className="flex items-center gap-2 px-2 py-2">
                        {/* Expand/Collapse Icon */}
                        <button
                          onClick={() => toggleWorkspace(workspace)}
                          className="shrink-0 hover:bg-accent rounded p-1"
                          title={isExpanded ? t('session.collapse') : t('session.expand')}
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
                          className="shrink-0 cursor-pointer"
                        />

                        {/* Workspace Name */}
                        <button
                          onClick={() => {
                            setSelectedWorkspace(workspace)
                            toggleWorkspace(workspace)
                          }}
                          className={`flex-1 text-left text-sm transition-all rounded-md px-2 py-1 ${selectedWorkspace === workspace
                              ? ''
                              : 'hover:bg-accent'
                            }`}
                          title={workspace}
                        >
                          <div className="truncate font-medium">
                            {decodeWorkspaceName(workspace)}
                          </div>
                          {isExpanded && sessions.length > 0 && (
                            <div className="text-xs opacity-70 mt-0.5">
                              {sessions.length} {t('session.sessions')}
                            </div>
                          )}
                        </button>

                        {/* Delete Button */}
                        <Button
                          variant="ghost"
                          size="icon"
                          className={`h-6 w-6 shrink-0 ${selectedWorkspace === workspace
                              ? 'text-primary-foreground hover:bg-primary-foreground/20'
                              : 'hover:bg-destructive hover:text-destructive-foreground'
                            }`}
                          onClick={(e) => {
                            e.stopPropagation()
                            handleDeleteWorkspace(workspace)
                          }}
                          title={t('session.deleteWorkspace')}
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </div>

                    {/* Sessions under this workspace (when expanded) */}
                    {isExpanded && (
                      <div className="ml-6 space-y-1">
                        {loading && sessions.length === 0 ? (
                          <div className="flex items-center justify-center py-4">
                            <Loader2 className="h-4 w-4 animate-spin" />
                          </div>
                        ) : sessions.length === 0 ? (
                          <div className="text-xs text-muted-foreground py-2 px-3">
                            {t('session.noSessions')}
                          </div>
                        ) : (
                          sessions.map(session => (
                            <Card
                              key={session.sessionId}
                              className={`p-2 cursor-pointer hover:bg-accent transition-colors ${selectedSession?.sessionId === session.sessionId ? 'bg-accent' : ''
                                }`}
                              onClick={() => handleSelectSession(workspace, session)}
                            >
                              <div className="space-y-1.5">
                                <div className="flex items-start justify-between gap-2">
                                  <h3 className="font-medium text-xs line-clamp-2 flex-1">
                                    {session.title}
                                  </h3>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-5 w-5 shrink-0 hover:bg-destructive hover:text-destructive-foreground"
                                    onClick={(e) => {
                                      e.stopPropagation()
                                      handleDeleteSession(workspace, session)
                                    }}
                                    title={t('session.deleteSession')}
                                  >
                                    <Trash2 className="h-2.5 w-2.5" />
                                  </Button>
                                </div>
                                <div className="flex items-center gap-2 flex-wrap">
                                  <Badge variant="secondary" className="text-xs h-4 px-1.5">
                                    {session.sessionType}
                                  </Badge>
                                  <span className="text-xs text-muted-foreground flex items-center gap-1">
                                    <MessageSquare className="h-2.5 w-2.5" />
                                    {session.messageCount}
                                  </span>
                                </div>
                              </div>
                            </Card>
                          ))
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
        <div className="flex-1 flex flex-col">
          {loading && selectedSession === null ? (
            <div className="flex-1 flex items-center justify-center">
              <Loader2 className="h-8 w-8 animate-spin" />
            </div>
          ) : selectedSession ? (
            <>
              <div className="px-4 py-2.5 border-b border-border flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <h2 className="text-sm font-semibold text-foreground truncate">{selectedSession.title}</h2>
                  <p className="text-[11px] text-muted-foreground mt-0.5 truncate font-mono">
                    {selectedSession.workspaceDirectory}
                  </p>
                </div>
                <div className="flex gap-1.5 ml-3">
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => handleExportSession('json')}
                  >
                    <Download className="h-3.5 w-3.5 mr-1" />
                    JSON
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => handleExportSession('markdown')}
                  >
                    <Download className="h-3.5 w-3.5 mr-1" />
                    Markdown
                  </Button>
                </div>
              </div>

              <ScrollArea className="flex-1">
                <div className="p-4 space-y-4 max-w-4xl">
                  {/* Conversation summary extracted from the first message. */}
                  {selectedSession.history.length > 0 &&
                    selectedSession.history[0].message.role === 'user' &&
                    selectedSession.history[0].message.content.length > 0 &&
                    (selectedSession.history[0].message.content[0].text.includes('CONTEXT TRANSFER') ||
                      selectedSession.history[0].message.content[0].text.includes('## TASK') ||
                      selectedSession.title.includes('(Continued)')) && (
                      <Card className="p-4 bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800">
                        <div className="flex items-start gap-3">
                          <div className="text-2xl shrink-0">📝</div>
                          <div className="flex-1 min-w-0">
                            <div className="font-medium mb-2 text-blue-900 dark:text-blue-100">
                              {t('session.conversationSummary')}
                            </div>
                            <div className="text-sm text-blue-800 dark:text-blue-200 whitespace-pre-wrap break-words">
                              {selectedSession.history[0].message.content[0].text}
                            </div>
                          </div>
                        </div>
                      </Card>
                    )}

                  {/* Messages */}
                  {selectedSession.history.length === 0 ? (
                    <div className="text-center py-8 text-muted-foreground">
                      {t('session.sessionHasNoMessages')}
                    </div>
                  ) : (
                    selectedSession.history.map((item, index) => {
                      // Skip the first summary message for compressed sessions.
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
                        <Card key={item.message.id} className="p-4">
                          <div className="flex items-start gap-3">
                            <div className="text-2xl shrink-0">
                              {item.message.role === 'user' ? '👤' : '🤖'}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="font-medium mb-2">
                                {item.message.role === 'user' ? 'User' : 'Assistant'}
                              </div>
                              {item.message.content.map((content, i) => (
                                <div key={i} className="whitespace-pre-wrap text-sm break-words">
                                  {content.text}
                                </div>
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
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center">
                <MessageSquare className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                <p className="text-muted-foreground">{t('session.selectSessionToView')}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
