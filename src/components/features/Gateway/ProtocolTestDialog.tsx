import { useState, useMemo, useRef, useCallback } from 'react'
import { Button } from '@/components/ui/button'
import { DialogRoot, DialogContent, DialogHeader, DialogTitle, DialogBody } from '@/components/shared/dialog'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Play, Copy, Square, ClipboardCopy } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { countSseFrames, renderResponseBody, type ResponseView } from './gatewayResponseView'

interface Account {
  id: string
  email?: string
  userId?: string
  label?: string
  status?: string
  enabled?: boolean
  groupId?: string
}

export interface GatewayRoutingConfig {
  accountMode: string
  accountId?: string | null
  groupId?: string | null
  poolAccountIds?: string[]
}

interface ProtocolTestDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  accounts: Account[]
  gatewayBaseUrl: string
  gatewayApiKey: string
  routing?: GatewayRoutingConfig
}

type Protocol = 'anthropic' | 'openai-chat' | 'openai-responses'

const PROTOCOL_LABELS: Record<Protocol, string> = {
  'anthropic': 'Anthropic Messages (/v1/messages)',
  'openai-chat': 'OpenAI Chat (/v1/chat/completions)',
  'openai-responses': 'OpenAI Responses (/v1/responses)'
}

const PROTOCOL_PATHS: Record<Protocol, string> = {
  'anthropic': '/v1/messages',
  'openai-chat': '/v1/chat/completions',
  'openai-responses': '/v1/responses',
}

// 默认请求体里的模型：用池里最便宜、Free 账号也可用的 haiku-4.5。
// 理由只是成本——这个入口真实调用上游、会消耗账号额度，冒烟测试不该
// 默认拿 sonnet 打。旧值 `claude-sonnet-4-5-20250929` 本身能正确解析
// （日期后缀被剥掉、横杠转点号 → claude-sonnet-4.5），不是 bug。
// 三个协议共用同一个模型 id：入口协议不同不影响上游认哪个模型。
const DEFAULT_REQUESTS: Record<Protocol, any> = {
  'anthropic': {
    model: 'claude-haiku-4.5',
    max_tokens: 1024,
    messages: [
      {
        role: 'user',
        content: '你好'
      }
    ],
    stream: true
  },
  'openai-chat': {
    model: 'claude-haiku-4.5',
    messages: [
      {
        role: 'user',
        content: '你好'
      }
    ],
    stream: true,
    temperature: 0.7,
    top_p: 1
  },
  'openai-responses': {
    model: 'claude-haiku-4.5',
    input: [
      {
        role: 'user',
        content: '你好'
      }
    ],
    stream: true,
    temperature: 0.7
  }
}

const QUICK_TEMPLATES: Record<string, { labelKey: string; getRequest: (protocol: Protocol) => any }> = {
  'simple': {
    labelKey: 'gateway.tplSimple',
    getRequest: (protocol: Protocol) => DEFAULT_REQUESTS[protocol]
  },
  'multi-turn': {
    labelKey: 'gateway.tplMultiTurn',
    getRequest: (protocol: Protocol) => {
      const base = DEFAULT_REQUESTS[protocol]
      if (protocol === 'openai-responses') {
        return {
          ...base,
          input: [
            { role: 'user', content: '你好，我是小明' },
            { role: 'assistant', content: '你好小明！很高兴认识你。' },
            { role: 'user', content: '我叫什么名字？' }
          ]
        }
      }
      return {
        ...base,
        messages: [
          { role: 'user', content: '你好，我是小明' },
          { role: 'assistant', content: '你好小明！很高兴认识你。' },
          { role: 'user', content: '我叫什么名字？' }
        ]
      }
    }
  },
  'non-stream': {
    labelKey: 'gateway.tplNonStream',
    getRequest: (protocol: Protocol) => ({
      ...DEFAULT_REQUESTS[protocol],
      stream: false
    })
  }
}

const UNAVAILABLE_STATUSES = new Set([
  'banned', '封禁', '已封禁',
  'invalid', '失效', '已失效', 'Token已失效',
  'expired', '过期', '已过期',
])

function isAccountSelectable(account: Account): boolean {
  if (account.enabled === false) return false
  if (account.status && UNAVAILABLE_STATUSES.has(account.status)) return false
  return true
}

function matchesRouting(account: Account, routing?: GatewayRoutingConfig): boolean {
  if (!routing) return true
  switch (routing.accountMode) {
    case 'single':
      return !routing.accountId || account.id === routing.accountId
    case 'group':
      return !routing.groupId || account.groupId === routing.groupId
    case 'pool':
      return !routing.poolAccountIds?.length || routing.poolAccountIds.includes(account.id)
    default:
      return true
  }
}

function accountLabel(account: Account): string {
  return account.email || account.userId || account.label || account.id
}

const formatDuration = (ms: number): string =>
  ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(2)} s`

/**
 * 协议实测：用某个入口协议的格式真发一个请求，看网关转出来什么。
 *
 * 与「路由测试」的分界写在名字里——这个会真实调用上游、消耗账号额度；
 * 路由测试只模拟选号、不花钱。两者不能混成一个按钮。
 *
 * 版面按「标准协议测试页」排：顶部是请求条件（协议 / 账号 / 模板），
 * 中间左右分栏放请求体与响应体，响应区自带状态行（HTTP 码 / 耗时 / 帧数）
 * 与三种查看方式，底部固定动作区。
 */
export function ProtocolTestDialog({
  open,
  onOpenChange,
  accounts,
  gatewayBaseUrl,
  gatewayApiKey,
  routing,
}: ProtocolTestDialogProps) {
  const { t } = useApp()
  const [protocol, setProtocol] = useState<Protocol>('anthropic')
  const [selectedAccount, setSelectedAccount] = useState<string>('')
  const [requestBody, setRequestBody] = useState<string>(JSON.stringify(DEFAULT_REQUESTS['anthropic'], null, 2))
  // 响应统一存原始文本：三视图（原文 / JSON / 文本）都从它派生，
  // 流式累积也写进同一个字段，避免「流式只给原文、非流式能切视图」的不对称。
  const [rawResponse, setRawResponse] = useState<string>('')
  const [status, setStatus] = useState<number | null>(null)
  const [durationMs, setDurationMs] = useState<number | null>(null)
  const [contentType, setContentType] = useState<string>('')
  const [responseView, setResponseView] = useState<ResponseView>('raw')
  const [error, setError] = useState<string>('')
  const [loading, setLoading] = useState(false)
  const [activeTemplate, setActiveTemplate] = useState('simple')
  const abortRef = useRef<AbortController | null>(null)

  // 仅展示当前网关路由配置下可用的账号
  const availableAccounts = useMemo(() => {
    return accounts.filter(acc => isAccountSelectable(acc) && matchesRouting(acc, routing))
  }, [accounts, routing])

  const routingHint = useMemo(() => {
    if (!routing) return t('gateway.routingAuto')
    switch (routing.accountMode) {
      case 'single':
        return t('gateway.routingSingle')
      case 'group':
        return t('gateway.routingGroup')
      case 'pool':
        return t('gateway.routingPool', { count: routing.poolAccountIds?.length || 0 })
      default:
        return routing.accountMode
    }
  }, [routing])

  const endpointPath = PROTOCOL_PATHS[protocol]
  const fullEndpoint = `${gatewayBaseUrl}${endpointPath}`

  const resetResponse = () => {
    setRawResponse('')
    setStatus(null)
    setDurationMs(null)
    setContentType('')
    setError('')
  }

  const handleProtocolChange = (value: Protocol) => {
    setProtocol(value)
    setRequestBody(JSON.stringify(DEFAULT_REQUESTS[value], null, 2))
    resetResponse()
    setActiveTemplate('simple')
  }

  const handleTemplateSelect = (templateKey: string) => {
    const template = QUICK_TEMPLATES[templateKey]
    if (template) {
      setRequestBody(JSON.stringify(template.getRequest(protocol), null, 2))
      setActiveTemplate(templateKey)
    }
  }

  const handleFormatJson = () => {
    try {
      const parsed = JSON.parse(requestBody)
      setRequestBody(JSON.stringify(parsed, null, 2))
      setError('')
    } catch (e: any) {
      setError(t('gateway.jsonFormatError', { message: e.message }))
    }
  }

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }

  // 复制成可直接跑的 curl：排障时把这段贴给别人，比描述「我发了什么」准确得多。
  const handleCopyCurl = () => {
    const headers = [
      `-H "Content-Type: application/json"`,
      `-H "Authorization: Bearer ${gatewayApiKey}"`,
      ...(protocol === 'anthropic' ? [`-H "anthropic-version: 2023-06-01"`] : []),
      ...(selectedAccount ? [`-H "x-account-id: ${selectedAccount}"`] : []),
    ]
    const oneLine = requestBody.replace(/\s*\n\s*/g, '')
    const curl = [
      `curl -X POST ${fullEndpoint} \\`,
      ...headers.map(h => `  ${h} \\`),
      `  -d '${oneLine}'`,
    ].join('\n')
    copyText(curl)
  }

  const handleAbort = useCallback(() => {
    abortRef.current?.abort()
  }, [])

  const handleSendRequest = async () => {
    let body: any
    try {
      body = JSON.parse(requestBody)
    } catch (e: any) {
      setError(t('gateway.jsonFormatError', { message: e.message }))
      return
    }

    setLoading(true)
    resetResponse()

    const controller = new AbortController()
    abortRef.current = controller
    const startedAt = performance.now()

    try {
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${gatewayApiKey}`,
      }
      // Anthropic 入口要求版本头，缺了上游会按旧行为处理
      if (protocol === 'anthropic') {
        headers['anthropic-version'] = '2023-06-01'
      }
      // 指定账号：后端会在当前路由可用集合内强制使用该账号
      if (selectedAccount) {
        headers['x-account-id'] = selectedAccount
      }

      const res = await fetch(fullEndpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
        signal: controller.signal,
      })

      setStatus(res.status)
      const ct = res.headers.get('content-type') || ''
      setContentType(ct)

      // 错误响应也要原样展示：网关会把上游错误体透传回来，
      // 只显示一句「HTTP 4xx」等于把最有用的排障信息丢掉。
      if (!res.ok) {
        setRawResponse(await res.text())
        setDurationMs(performance.now() - startedAt)
        return
      }

      if (body.stream && ct.includes('text/event-stream')) {
        const reader = res.body?.getReader()
        const decoder = new TextDecoder()
        let accumulated = ''
        if (reader) {
          for (;;) {
            const { value, done } = await reader.read()
            if (done) break
            if (value) {
              accumulated += decoder.decode(value, { stream: true })
              setRawResponse(accumulated)
            }
          }
        }
      } else {
        setRawResponse(JSON.stringify(await res.json(), null, 2))
      }
      setDurationMs(performance.now() - startedAt)
    } catch (e: any) {
      // 主动中止不是错误，不弹红条
      if (e?.name !== 'AbortError') {
        setError(t('gateway.requestFailed', { message: e.message }))
      }
      setDurationMs(performance.now() - startedAt)
    } finally {
      setLoading(false)
      abortRef.current = null
    }
  }

  const renderedResponse = useMemo(
    () => renderResponseBody(rawResponse, responseView),
    [rawResponse, responseView]
  )

  const frameCount = useMemo(
    () => (contentType.includes('text/event-stream') ? countSseFrames(rawResponse) : 0),
    [rawResponse, contentType]
  )

  const statusTone =
    status === null
      ? 'text-muted-foreground'
      : status < 400
        ? 'text-green-600'
        : 'text-red-500'

  const viewOptions: { id: ResponseView; labelKey: string }[] = [
    { id: 'raw', labelKey: 'gateway.viewRaw' },
    { id: 'json', labelKey: 'gateway.viewJson' },
    { id: 'text', labelKey: 'gateway.viewText' },
  ]

  return (
    <DialogRoot open={open} onOpenChange={onOpenChange}>
      <DialogContent maxWidth="min(94vw, 1280px)" className="h-[85vh] flex flex-col surface-card">
        <DialogHeader className="pb-4 border-b border-border/40">
          <DialogTitle className="flex items-center gap-2.5 text-base font-semibold">
            <div className="flex items-center justify-center h-7 w-7 rounded-lg bg-primary/10">
              <Play className="w-3.5 h-3.5 text-primary" />
            </div>
            {t('gateway.protocolTest')}
            {/* 徽章必须给关闭按钮让位：DialogContent 的关闭按钮绝对定位在右上角，
                而 ml-auto 会把徽章推到同一个位置。用 pr 预留出关闭按钮的宽度。 */}
            <span className="ml-auto mr-9 inline-flex items-center gap-1.5 rounded-full bg-muted/60 border border-border/50 px-2.5 py-0.5 text-[10px] font-mono text-muted-foreground">
              <span className={`h-1.5 w-1.5 rounded-full ${loading ? 'bg-primary animate-pulse' : 'bg-green-500/80'}`}></span>
              POST {endpointPath}
            </span>
          </DialogTitle>
        </DialogHeader>

        {/* DialogBody 自带 overflow-y-auto，这里再叠一个 overflow-hidden 会与它打架：
            谁生效取决于 CSS 生成顺序而非书写顺序，结果是可能出现内外两层滚动条。
            改用 noPadding 关掉默认内边距，并让本层不滚动——
            滚动交给下方双面板各自的 textarea。 */}
        <DialogBody noPadding className="flex-1 min-h-0 overflow-hidden flex flex-col gap-3 px-6 py-4">
          {/* 请求条件：协议 / 账号 / 模板 */}
          <div className="rounded-xl border border-border/40 bg-muted/20 p-3">
            <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.4fr)] gap-3 items-end">
              <div className="space-y-1.5">
                <Label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">{t('gateway.protocol')}</Label>
                <Select value={protocol} onValueChange={(value) => handleProtocolChange(value as Protocol)}>
                  <SelectTrigger className="h-9 text-sm bg-background/80 border-border/50">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {Object.entries(PROTOCOL_LABELS).map(([key, label]) => (
                      <SelectItem key={key} value={key}>
                        {label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-1.5">
                <Label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">
                  {t('gateway.specifiedAccount')}
                  <span className="ml-1.5 normal-case tracking-normal font-normal opacity-70">
                    · {routingHint}
                  </span>
                </Label>
                <Select
                  value={selectedAccount || '__default__'}
                  onValueChange={(value) => setSelectedAccount(value === '__default__' ? '' : value)}
                >
                  <SelectTrigger className="h-9 text-sm bg-background/80 border-border/50">
                    <SelectValue placeholder={t('gateway.autoSelect')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="__default__">{t('gateway.autoSelectLb')}</SelectItem>
                    {availableAccounts.map(acc => (
                      <SelectItem key={acc.id} value={acc.id}>
                        {accountLabel(acc)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                {availableAccounts.length === 0 && (
                  <p className="text-[10px] text-amber-600 dark:text-amber-400">
                    {t('gateway.noAvailableAccountsForRouting')}
                  </p>
                )}
              </div>

              <div className="space-y-1.5">
                <Label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">{t('gateway.templates')}</Label>
                <div className="flex flex-wrap gap-1.5">
                  {Object.entries(QUICK_TEMPLATES).map(([key, { labelKey }]) => (
                    <Button
                      key={key}
                      variant="outline"
                      size="sm"
                      className={`h-9 text-xs px-2.5 border-border/50 transition-colors ${activeTemplate === key
 ? 'bg-primary/10 text-primary border-primary/40 shadow-sm'
 : 'bg-background/80 hover:bg-primary/5 hover:border-primary/40 hover:text-primary'
 }`}
                      onClick={() => handleTemplateSelect(key)}
                      disabled={loading}
                    >
                      {t(labelKey)}
                    </Button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* 窄屏时两个面板会堆叠；只给它们 flex-1 会让两者平分有限高度，
              响应面板被压成 98px（textarea 只剩 64px）几乎不可用。
              改成：宽屏并排时 flex-1 平分；窄屏堆叠时各自给一个最小高度并由
              外层滚动，保证两边都能看清。 */}
          <div className="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-3 min-h-0 overflow-y-auto lg:overflow-visible no-scrollbar">
            {/* 左：请求体 */}
            <div className="flex flex-col min-h-[220px] lg:min-h-0 lg:flex-1 rounded-xl border border-border/50 overflow-hidden shadow-sm">
              <div className="flex items-center justify-between px-3 py-2 bg-gradient-to-r from-muted/40 to-muted/20 border-b border-border/40">
                <div className="flex items-center gap-2">
                  <span className="h-2 w-2 rounded-full bg-blue-500/70"></span>
                  <span className="text-xs font-medium text-foreground">{t('gateway.requestBody')}</span>
                  <span className="text-[10px] text-muted-foreground">JSON</span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 text-[11px] px-2 hover:text-primary"
                    onClick={handleCopyCurl}
                    disabled={loading}
                    title={t('gateway.copyAsCurl')}
                  >
                    <ClipboardCopy className="h-3 w-3 mr-1" />
                    curl
                  </Button>
                  <Button variant="ghost" size="sm" className="h-6 text-[11px] px-2 hover:text-primary" onClick={handleFormatJson} disabled={loading}>
                    {t('gateway.format')}
                  </Button>
                </div>
              </div>
              <Textarea
                value={requestBody}
                onChange={(e) => {
                  setRequestBody(e.target.value)
                  setActiveTemplate('custom')
                }}
                className="flex-1 font-mono text-[11px] leading-relaxed resize-none border-0 rounded-none focus-visible:ring-0 bg-background/60"
                placeholder={t('gateway.inputJsonPlaceholder')}
                disabled={loading}
              />
            </div>

            {/* 右：响应 */}
            <div className="flex flex-col min-h-[220px] lg:min-h-0 lg:flex-1 rounded-xl border border-border/50 overflow-hidden shadow-sm">
              <div className="flex items-center justify-between px-3 py-2 bg-gradient-to-r from-muted/40 to-muted/20 border-b border-border/40">
                <div className="flex items-center gap-2 min-w-0">
                  {loading ? (
                    <span className="h-2 w-2 shrink-0 rounded-full bg-primary animate-pulse"></span>
                  ) : rawResponse ? (
                    <span className={`h-2 w-2 shrink-0 rounded-full ${status !== null && status >= 400 ? 'bg-red-500/70' : 'bg-green-500/70'}`}></span>
                  ) : (
                    <span className="h-2 w-2 shrink-0 rounded-full bg-muted-foreground/30"></span>
                  )}
                  <span className="text-xs font-medium text-foreground shrink-0">
                    {t('gateway.responseResult')}
                  </span>
                  {/* 状态行：HTTP 码 / 耗时 / 帧数——判断「快不快、是不是流」全靠它 */}
                  <span className="flex items-center gap-2 text-[10px] font-mono text-muted-foreground truncate">
                    {status !== null && <span className={statusTone}>{status}</span>}
                    {durationMs !== null && <span>{formatDuration(durationMs)}</span>}
                    {frameCount > 0 && <span>{frameCount} frames</span>}
                  </span>
                  {loading && <span className="text-[10px] text-primary animate-pulse shrink-0">{t('gateway.receiving')}</span>}
                </div>
                <div className="flex items-center gap-1 shrink-0">
                  {rawResponse && (
                    <>
                      <div className="flex items-center gap-0.5 rounded-full border border-border/50 p-0.5">
                        {viewOptions.map(option => (
                          <button
                            key={option.id}
                            type="button"
                            onClick={() => setResponseView(option.id)}
                            className={`rounded-full px-2 py-0.5 text-[10px] transition-colors ${responseView === option.id ? 'bg-primary/15 text-primary' : 'text-muted-foreground hover:text-foreground'
 }`}
                          >
                            {t(option.labelKey)}
                          </button>
                        ))}
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 text-[11px] px-2 hover:text-primary"
                        onClick={() => copyText(renderedResponse)}
                      >
                        <Copy className="h-3 w-3 mr-1" />
                        {t('gateway.copy')}
                      </Button>
                    </>
                  )}
                </div>
              </div>
              <Textarea
                value={renderedResponse}
                readOnly
                className="flex-1 font-mono text-[11px] leading-relaxed resize-none border-0 rounded-none focus-visible:ring-0 bg-muted/10"
                placeholder={t('gateway.responsePlaceholder')}
              />
            </div>
          </div>

          {error && (
            <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-3.5 py-2.5 flex items-start gap-2">
              <span className="h-4 w-4 rounded-full bg-destructive/20 flex items-center justify-center shrink-0 mt-0.5">
                <span className="h-1.5 w-1.5 rounded-full bg-destructive"></span>
              </span>
              <p className="text-xs text-destructive leading-relaxed break-all">{error}</p>
            </div>
          )}

          {/* 动作区 */}
          <div className="flex items-center justify-between gap-3 pt-3 border-t border-border/40">
            <div className="flex items-center gap-2 text-[11px] text-muted-foreground min-w-0">
              <span className="font-mono truncate" title={fullEndpoint}>{fullEndpoint}</span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 shrink-0"
                onClick={() => copyText(fullEndpoint)}
                title={t('gateway.copyEntry')}
              >
                <Copy size={11} />
              </Button>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <Button variant="outline" size="sm" className="h-8 px-3" onClick={() => onOpenChange(false)} disabled={loading}>
                {t('gateway.close')}
              </Button>
              {loading ? (
                <Button variant="outline" size="sm" className="h-8 px-3" onClick={handleAbort}>
                  <Square className="h-3 w-3 mr-1.5" />
                  {t('gateway.abort')}
                </Button>
              ) : (
                <Button
                  size="sm"
                  className="h-8 px-4 bg-primary hover:bg-primary/90 text-primary-foreground shadow-sm"
                  onClick={handleSendRequest}
                  disabled={!requestBody.trim()}
                >
                  <Play className="h-3.5 w-3.5 mr-1.5" />
                  {t('gateway.sendRequest')}
                </Button>
              )}
            </div>
          </div>
        </DialogBody>
      </DialogContent>
    </DialogRoot>
  )
}
