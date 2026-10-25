import { useState, Fragment } from 'react'
import { AlertCircle, AlertTriangle, Copy, KeyRound, TestTube2, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '@/hooks/useApp'
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion'
import { formatGatewayRequestDuration, shortClientLabel, type ClientRecipe } from './gatewayPageUtils'
import { renderResponseBody, type ResponseView } from './gatewayResponseView'

/// 概览「最近请求」表的一行：与日志 tab 同源，只取少量字段
/// （完整明细在日志 tab，这里不重复展开请求体/响应体）。
export interface RecentRequestLog {
  /** 后端 request_id（UUID）：唯一键 */
  requestId?: string
  occurredAt: string
  endpoint: string
  statusCode: number
  durationMs: number
  model?: string
  error?: string
  /** 客户端 User-Agent：概览里就能看出是哪个客户端在打 */
  clientUserAgent?: string
  inputTokens?: number
  outputTokens?: number
  /** Prompt Caching：缓存读取（节省成本）/ 缓存写入（首次 +25% 成本），概览合并一列显示 */
  cacheReadInputTokens?: number
  cacheCreationInputTokens?: number
  /** 上游 meteringEvent 的真实计费消耗；非流式请求为 undefined */
  credits?: number
  /** 原始请求体 / 响应体（后端已截断到 64KB），供行内展开查看 */
  requestBody?: string
  responseBody?: string
}

interface GatewayOverviewProps {
  recipes: ClientRecipe[]
  onCopy: (text: string, message: string) => void
  /** 最近请求（后端 getGatewayRequestLogs 的 5 条切片，与 10s 轮询同频）——概览用一张表呈现
    请求与错误，错误只是其中一行的状态，不再单设「最近错误」卡片 */
  recentLogs: RecentRequestLog[]
  /** 账号池里是否真能取到号（只看启用账号），与概览顶部那条告警直接挂钩 */
  hasUsableAccountPool: boolean
  /** 账号库是否为空。与 hasUsableAccountPool 分开：空库要说「先导入账号」，
     而不是让人去配置账号池——两者该做的事完全不同。 */
  hasNoAccountsAtAll: boolean
  onOpenLogs: () => void
  onOpenProtocolTest: () => void
}

// 概览表格只显示时分秒，完整时间在日志 tab
const formatLogTime = (value: string): string => {
  if (!value) return '-'
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleTimeString()
}

/**
 * 概览面板：接入客户端（B3）→ 最近请求（B5）→ 工具入口（B6）。
 *
 * 原「近期统计」块已删除：它的四个绝对数与日志弹窗是同一份 LogStore 聚合，
 * 而状态条当时的计数是运行期口径——同一个「请求数」在页面上曾出现两个数字。
 * 唯二不重复且有用的比率（成功率、平均耗时）已上提到常驻状态条，
 * 与请求数、错误数同源。概览因此只留下「怎么接」与「最近发生了什么」。
 */
export function GatewayOverview({
  recipes,
  onCopy,
  recentLogs,
  hasUsableAccountPool,
  hasNoAccountsAtAll,
  onOpenLogs,
  onOpenProtocolTest,
}: GatewayOverviewProps) {
  const { t } = useApp()

  // 接入区改为「一次只看一个客户端」：顶部切换，下方紧凑行。
  // 三张并排大卡在窄屏会纵向堆叠成三倍高度，且多数用户只接一个客户端。
  const [activeRecipeId, setActiveRecipeId] = useState('')
  const activeRecipe = recipes.find(r => r.id === activeRecipeId) || recipes[0]
  // 最近请求表展开的行：点击一行就地展开请求/响应体，不用跳走。
  const [expandedLogId, setExpandedLogId] = useState<string | null>(null)
  // 响应体查看格式：原文 / 格式化 JSON / 原始文本
  const [responseView, setResponseView] = useState<ResponseView>('raw')

  return (
    <div className="flex flex-col gap-3">
      {/* 两种空池要分开说：一个账号都没有时让人去配账号池是误导，
          该做的是先导入账号。 */}
      {!hasUsableAccountPool && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-700 dark:text-amber-400">
          <AlertTriangle size={13} className="mt-0.5 shrink-0" />
          <span>{t(hasNoAccountsAtAll ? 'gateway.noAccountsAtAll' : 'gateway.noRunningAccountPool')}</span>
        </div>
      )}

      {/* 概览各区块均可折叠：接入引导与统计是日常要看的内容，
          但请求表格与工具入口体积大，收起来能让首屏只留下关键信息。
          multiple = 可同时展开多块；collapsible = 允许全部收起。 */}
      <Accordion type="multiple" className="w-full">

      {/* B3 接入客户端：三张等宽卡，纵向四项（Base URL / 鉴权头 / 端点 / 动作）
          Base URL 与鉴权头必须相邻——这两项无法互相推导，填错就是 401。 */}
      <AccordionItem value="connect">
        <AccordionTrigger className="py-2 hover:no-underline">
          <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
            <span className="shrink-0 text-sm font-semibold text-foreground">{t('gateway.connectClients')}</span>
          </span>
        </AccordionTrigger>
        <AccordionContent>

        {/* 切换轴是协议，不是客户端：Base URL / 鉴权头 / 端点这三项由协议决定，
            客户端只是同一协议下的一个例子。按客户端切会让用户以为「换客户端就要换地址」，
            实际上 Claude Code 与其他 Anthropic 客户端填的完全一样。 */}
        <div className="mb-1.5 flex items-center gap-2">
          <span className="shrink-0 text-[11px] font-medium text-muted-foreground">{t('gateway.chooseProtocol')}</span>
          <span className="text-[10px] text-muted-foreground/70">{t('gateway.chooseProtocolHint')}</span>
        </div>
        <div className="mb-2.5 flex flex-wrap items-center gap-1.5">
          {recipes.map(recipe => (
            <button
              key={recipe.id}
              type="button"
              onClick={() => setActiveRecipeId(recipe.id)}
              className={`h-7 rounded-full px-2.5 text-[11px] font-medium transition-colors ${
 recipe.id === activeRecipe.id
 ? 'bg-primary/15 text-primary ring-1 ring-primary/25'
 : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground'
 }`}
              title={recipe.label}
            >
              {recipe.protocol}
            </button>
          ))}
        </div>

        {activeRecipe && (
          <div className="flex flex-col gap-1.5 text-[11px] ">
            {/* 客户端只作「适用于」提示，不再是主轴 */}
            <div className="mb-0.5 flex items-center gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t('gateway.appliesTo')}</span>
              <span className="min-w-0 flex-1 truncate text-foreground">{activeRecipe.label}</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t('gateway.baseUrlLabel')}</span>
              <code className="min-w-0 flex-1 truncate font-mono text-foreground" title={activeRecipe.baseUrl}>{activeRecipe.baseUrl}</code>
              <button
                type="button"
                onClick={() => onCopy(activeRecipe.baseUrl, t('gateway.copiedItem', { label: t('gateway.baseUrlLabel') }))}
                className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                title={t('gateway.copy')}
              >
                <Copy size={12} />
              </button>
            </div>
            <div className="flex items-center gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t('gateway.authHeaderLabel')}</span>
              <code className="min-w-0 flex-1 truncate font-mono text-foreground" title={activeRecipe.authHeader}>{activeRecipe.authHeader}</code>
              <button
                type="button"
                onClick={() => onCopy(activeRecipe.authHeader, t('gateway.copiedItem', { label: t('gateway.authHeaderLabel') }))}
                className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                title={t('gateway.copy')}
              >
                <Copy size={12} />
              </button>
            </div>
            <div className="flex items-center gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t('gateway.endpointLabel')}</span>
              <code className="min-w-0 flex-1 truncate font-mono text-muted-foreground" title={activeRecipe.endpoint}>{activeRecipe.endpoint}</code>
            </div>
            {/* 密钥固定为 kiro 后恒有值，不再需要空值条件渲染 */}
            <div className="flex items-center gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t('gateway.clientKey')}</span>
              <code className="min-w-0 flex-1 truncate font-mono text-foreground" title={activeRecipe.keyMasked}>{activeRecipe.keyMasked}</code>
              <button
                type="button"
                onClick={() => onCopy(activeRecipe.key, t('gateway.copiedItem', { label: t('gateway.clientKey') }))}
                className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                title={t('gateway.copyKey')}
              >
                <KeyRound size={12} />
              </button>
            </div>

            {/* 协议面板只给参数，不给写入按钮：
                协议是「怎么说」，客户端是「谁在说」，两者是独立维度。
                Anthropic 协议可能是 Cursor、Continue、Cline、Claude Code 中的任何一个，
                把「写入 Claude Code」挂在协议下面等于偷换了概念。
                这里复制的是连接配置本身（Base URL + Key 的环境变量写法），用户拿它贴进
                自己那个客户端的配置里；curl 那种「发一次请求看通不通」属于协议实测，
                不占这个按钮。 */}
            <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
              <Button
                size="sm"
                variant="outline"
                className="h-7 text-xs"
                onClick={() => onCopy(activeRecipe.copySample, t('gateway.copiedItem', { label: activeRecipe.label }))}
              >
                <Copy size={11} className="mr-1" />
                {t('gateway.copyConfig')}
              </Button>
            </div>
          </div>
        )}

        </AccordionContent>
      </AccordionItem>

      {/* B4 近期统计已删：请求总数 / 错误数 / Tokens / 缓存命中在日志弹窗里是同一份
          LogStore 聚合，而状态条的计数是运行期口径，同一个「请求数」曾出现两个数字。
          唯一只有它有、且真在状态条里用得上的两个比率（成功率、平均耗时）已上提到状态条。 */}

      {/* B5 最近请求：明细的切片。错误只是表格里一行的状态，
          不再单设「最近错误」卡片——同粒度不两处呈现。
          整行可点，落到日志 tab 看完整字段。
          数据本身按运行会话隔离（后端 sessionOnly）：停止/重启后归零，
          这里不需要再按运行状态做条件渲染。 */}
      <AccordionItem value="recent">
        <AccordionTrigger className="py-2 hover:no-underline">
          <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
            <span className="shrink-0 text-sm font-semibold text-foreground">{t('gateway.recentRequests')}</span>
          </span>
        </AccordionTrigger>
        <AccordionContent>
          <div className="flex justify-end pb-1.5">
            <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={onOpenLogs}>
              {t('gateway.viewAllLogs')}
            </Button>
          </div>
        {recentLogs.length === 0 ? (
          <p className="py-2 text-[11px] text-muted-foreground">{t('gateway.noRecentRequests')}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-[11px]">
              <thead>
                <tr className="text-left text-muted-foreground">
                  <th className="py-1 pr-2 font-medium">{t('gateway.time')}</th>
                  <th className="py-1 pr-2 font-medium">{t('gateway.endpoint')}</th>
                  <th className="py-1 pr-2 font-medium">{t('gatewayLogs.colClient')}</th>
                  <th className="py-1 pr-2 font-medium">{t('gateway.model')}</th>
                  <th className="py-1 pr-2 font-medium">{t('gateway.status')}</th>
                  <th className="py-1 pr-2 text-right font-medium">{t('gateway.tokens')}</th>
                  <th className="py-1 pr-2 text-right font-medium" title={t('gateway.cacheReadWriteTitle')}>{t('gateway.cacheReadWrite')}</th>
                  <th className="py-1 pr-2 text-right font-medium" title={t('gateway.creditsTitle')}>{t('gateway.credits')}</th>
                  <th className="py-1 pr-2 text-right font-medium">{t('gateway.duration')}</th>
                </tr>
              </thead>
              <tbody>
                {recentLogs.map((log) => {
                  const failed = log.statusCode >= 400
                  const rowKey = log.requestId || `${log.occurredAt}-${log.endpoint}`
                  const expanded = expandedLogId === rowKey
                  const hasBody = !!(log.requestBody || log.responseBody)
                  return (
                    <Fragment key={rowKey}>
                      <tr
                        onClick={() => setExpandedLogId(expanded ? null : rowKey)}
                        title={hasBody ? t('gateway.rowToggleDetail') : t('gateway.metricJumpToLogs')}
                        className={`cursor-pointer border-t border-border/50 transition-colors hover:bg-primary/5 ${expanded ? 'bg-primary/5' : ''}`}
                      >
                        <td className="py-1 pr-2 font-mono text-muted-foreground">
                          <span className="inline-flex items-center gap-1">
                            {hasBody && (
                              <ChevronRight size={10} className={`shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
                            )}
                            {formatLogTime(log.occurredAt)}
                          </span>
                        </td>
                        <td className="py-1 pr-2 text-foreground">{log.endpoint}</td>
                        <td className="max-w-[100px] truncate py-1 pr-2 text-muted-foreground" title={log.clientUserAgent || '-'}>
                          {shortClientLabel(log.clientUserAgent)}
                        </td>
                        <td className="max-w-[120px] truncate py-1 pr-2 text-foreground" title={log.model}>{log.model || '-'}</td>
                        <td className={`py-1 pr-2 font-medium ${failed ? 'text-red-500' : 'text-green-600'}`}>
                          {log.statusCode}
                        </td>
                        <td className="py-1 pr-2 text-right text-muted-foreground">
                          {log.inputTokens || log.outputTokens
                            ? `${log.inputTokens ?? 0}/${log.outputTokens ?? 0}`
                            : '-'}
                        </td>
                        <td className="py-1 pr-2 text-right font-mono text-emerald-600 dark:text-emerald-400" title={t('gateway.cacheReadWriteTitle')}>
                          {log.cacheReadInputTokens || log.cacheCreationInputTokens
                            ? `${log.cacheReadInputTokens ?? 0}/${log.cacheCreationInputTokens ?? 0}`
                            : '-'}
                        </td>
                        <td className="py-1 pr-2 text-right font-mono text-foreground" title={t('gateway.creditsTitle')}>
                          {log.credits != null ? log.credits.toFixed(4) : '-'}
                        </td>
                        <td className="py-1 pr-2 text-right text-muted-foreground">{formatGatewayRequestDuration(log.durationMs)}</td>
                      </tr>
                      {expanded && (
                        <tr className="border-t border-border/50">
                          <td colSpan={9} className="p-0">
                            <div className="flex flex-col gap-2 bg-muted/20 px-3 py-2">
                              {log.error && (
                                <div className="break-all text-[11px] text-red-500">{log.error}</div>
                              )}
                              {log.requestBody && (
                                <div className="flex flex-col gap-1">
                                  <span className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">{t('gateway.requestBody')}</span>
                                  <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-border/50 bg-background/60 p-2 font-mono text-[10px] text-foreground">{log.requestBody}</pre>
                                </div>
                              )}
                              {log.responseBody && (
                                <div className="flex flex-col gap-1">
                                  <div className="flex items-center gap-2">
                                    <span className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">{t('gateway.responseBody')}</span>
                                    <div className="ml-auto flex items-center gap-0.5 rounded-full border border-border/50 p-0.5">
                                      {([
                                        { id: 'raw', label: t('gateway.viewRaw') },
                                        { id: 'json', label: t('gateway.viewJson') },
                                        { id: 'text', label: t('gateway.viewText') },
                                      ] as const).map(option => (
                                        <button
                                          key={option.id}
                                          type="button"
                                          onClick={(e) => { e.stopPropagation(); setResponseView(option.id) }}
                                          className={`rounded-full px-2 py-0.5 text-[10px] transition-colors ${
 responseView === option.id ? 'bg-primary/15 text-primary' : 'text-muted-foreground hover:text-foreground'
 }`}
                                        >
                                          {option.label}
                                        </button>
                                      ))}
                                    </div>
                                  </div>
                                  <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-border/50 bg-background/60 p-2 font-mono text-[10px] text-foreground">
                                    {renderResponseBody(log.responseBody, responseView)}
                                  </pre>
                                </div>
                              )}
                              <div className="flex justify-end">
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-6 text-[10px]"
                                  onClick={(e) => { e.stopPropagation(); onOpenLogs() }}
                                >
                                  {t('gateway.viewAllLogs')}
                                </Button>
                              </div>
                            </div>
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
        </AccordionContent>
      </AccordionItem>

      {/* B6 工具入口：只剩协议实测。
          路由测试原与它高度重叠（同为「选协议→选账号→发请求→看响应」），
          且其账号指定能力这里早已具备（x-account-id），已并入此处。
          两者现在按成本分名：路由测试 = 不花钱的选号预演，
          协议实测 = 真实打上游、消耗账号额度。 */}
      <AccordionItem value="tools">
        <AccordionTrigger className="py-2 hover:no-underline">
          <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
            <span className="shrink-0 text-sm font-semibold text-foreground">{t('gateway.tools')}</span>
          </span>
        </AccordionTrigger>
        <AccordionContent>
      <div className="flex flex-col gap-2">
        {[
          { id: 'protocol-test', label: t('gateway.protocolTest'), desc: t('gateway.protocolTestDesc'), icon: TestTube2, onClick: onOpenProtocolTest },
        ].map(entry => {
          const Icon = entry.icon
          return (
            <button
              key={entry.id}
              onClick={entry.onClick}
              className="flex cursor-pointer flex-col gap-0.5 rounded-xl border border-border bg-muted/20 p-3 text-left transition-colors hover:border-primary/40 hover:bg-primary/5"
            >
              <span className="flex items-center gap-1.5 text-sm font-medium text-foreground">
                <Icon size={13} />
                {entry.label}
              </span>
              <span className="text-[11px] text-muted-foreground">{entry.desc}</span>
            </button>
          )
        })}
      </div>
        </AccordionContent>
      </AccordionItem>
      </Accordion>
    </div>
  )
}

export default GatewayOverview
