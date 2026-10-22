import { AlertCircle, Copy, Play, RotateCw, ScrollText, Square } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useApp } from '@/hooks/useApp'
import { type ErrorHistoryEntry } from './gatewayPageUtils'
import type { GatewayOverviewStats } from './gatewayPageState'

interface GatewayStatusBarProps {
  running: boolean
  /** 客户端应当连接的地址（已按 localOnly 归一为可连形式） */
  baseUrl: string
  /** 展示与复制同一个值：密钥固定为 kiro 后不再有掩码/真身之分 */
  clientKey: string
  /** LogStore 聚合快照：请求数 / 错误数 / 成功率 / 平均耗时同源，
     不再和运行期计数器混用——同一个「请求数」只能有一个数字 */
  stats: GatewayOverviewStats | null
  /** 最近一条错误，用于错误条；无错误时为 null */
  latestError: ErrorHistoryEntry | null
  /** 需要提示的配置状态（校验失败、需重启生效）；无需提示时为 null */
  hint?: string | null
  /** 任一启停动作进行中 */
  busy: boolean
  /** 自动重启提示：配置变更后正在 stop + start */
  restarting: boolean
  onStart: () => void
  onStop: () => void
  onRestart: () => void
  onOpenLogs: () => void
  onCopy: (text: string, message: string) => void
}

/**
 * 常驻运行状态条（B1）与条件错误条（B2）。
 *
 * 不属于任何 tab：无论用户在看概览还是设置，「它现在活着吗」都是同一个问题。
 * 三段式分工固定为「在哪 / 干了多少 / 怎么控制」，控制动作统一集中在最右段，
 * 不与信息混排。
 */
export function GatewayStatusBar({
  running,
  baseUrl,
  clientKey,
  stats,
  latestError,
  hint = null,
  busy,
  restarting,
  onStart,
  onStop,
  onRestart,
  onOpenLogs,
  onCopy,
}: GatewayStatusBarProps) {
  const { t } = useApp()

  const statusLabel = restarting
    ? t('gateway.restarting')
    : running
      ? t('gateway.running')
      : t('gateway.stopped')

  return (
    <div className="border-b border-border px-5 py-2.5">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        {/* 左段：在哪、怎么连 */}
        <div className="flex min-w-0 items-center gap-2">
          <span className={`h-2 w-2 shrink-0 rounded-full ${running ? 'bg-green-500' : 'bg-muted-foreground/50'}`} />
          <span className="shrink-0 text-xs font-medium text-foreground">{statusLabel}</span>
          <span className="truncate font-mono text-xs text-muted-foreground" title={baseUrl}>{baseUrl}</span>
          <Button
            size="sm"
            variant="ghost"
            className="h-6 w-6 shrink-0 p-0"
            title={t('gateway.copyEntry')}
            onClick={() => onCopy(baseUrl, t('gateway.copiedItem', { label: t('gateway.currentEntry') }))}
          >
            <Copy size={12} className="text-muted-foreground" />
          </Button>
        </div>

        <span className="hidden h-4 w-px bg-border sm:block" />

        {/* 中段：干了多少。计数用等宽数字，避免轮询刷新时宽度跳动 */}
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span className="flex min-w-0 items-center gap-1">
            <span className="truncate">
              <span className="text-muted-foreground">{t('gateway.clientKey')}</span>{' '}
              <span className="font-mono text-foreground">{clientKey}</span>
            </span>
            {clientKey && (
              <Button
                size="sm"
                variant="ghost"
                className="h-5 w-5 shrink-0 p-0"
                title={t('gateway.copyClientKey')}
                onClick={() => onCopy(clientKey, t('gateway.copiedItem', { label: t('gateway.clientKey') }))}
              >
                <Copy size={11} className="text-muted-foreground" />
              </Button>
            )}
          </span>
          {/* 请求类计数只在真有请求时才出现。冷启动时报「请求 0 / 错误 0」，
              读起来像数据坏了而不是还没开始——没有任何信息量，故整体不渲染。

              只留「请求 / 错误」两个绝对数：
              - 成功率是错误的同义重复（错误数与请求数已经能推出百分比），
                且错误为 0 时恒为 100.0%，不携带信息；
              - 平均耗时不能留：`duration_ms` 在流式请求上记的是写日志那一刻的
                elapsed，而流式日志在首包阶段就写了，于是绝大多数请求只计到
                「首包时间」量级，平均出来是几毫秒的假数字。它既不代表用户等了
                多久，也不代表上游多快。真要看快慢用日志里的首包延迟。 */}
          {stats && stats.total > 0 && (
            <>
              <span>
                {t('gateway.requestCountLabel')} <span className="stat-number text-foreground">{stats.total.toLocaleString()}</span>
              </span>
              <span>
                {t('gateway.errorCountLabel')}{' '}
                <span className={`stat-number ${stats.error > 0 ? 'text-red-500' : 'text-foreground'}`}>
                  {stats.error.toLocaleString()}
                </span>
              </span>
            </>
          )}
          {hint && (
            <span className="truncate text-amber-600 dark:text-amber-400" title={hint}>{hint}</span>
          )}
        </div>

        {/* 右段：控制 */}
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          {running ? (
            <Button
              size="sm"
              onClick={onStop}
              disabled={busy}
              className="h-7 bg-red-500 px-2.5 text-xs text-white hover:bg-red-600"
            >
              <Square size={12} className="mr-1" />
              {t('gateway.stop')}
            </Button>
          ) : (
            <Button
              size="sm"
              onClick={onStart}
              disabled={busy}
              className="h-7 bg-green-500 px-2.5 text-xs text-white hover:bg-green-600"
            >
              <Play size={12} className="mr-1" />
              {t('gateway.start')}
            </Button>
          )}
          <Button
            size="sm"
            variant="outline"
            onClick={onRestart}
            disabled={busy}
            className="h-7 px-2.5 text-xs"
          >
            <RotateCw size={12} className={`mr-1 ${restarting ? 'animate-spin' : ''}`} />
            {t('gateway.restart')}
          </Button>
        </div>
      </div>

      {/* B2 条件错误条：无错误时为零高度，不占位 */}
      {latestError && (
        <div className="mt-2 flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-2.5 py-1.5 text-[11px]">
          <AlertCircle size={13} className="mt-0.5 shrink-0 text-amber-600 dark:text-amber-400" />
          <span className="min-w-0 flex-1 break-words text-amber-700 dark:text-amber-400" title={latestError.message}>
            {latestError.message}
          </span>
          {latestError.count > 1 && (
            <span className="shrink-0 text-amber-700/70 dark:text-amber-400/70">×{latestError.count}</span>
          )}
          <span className="shrink-0 text-amber-700/70 dark:text-amber-400/70">{latestError.lastSeenAt}</span>
          {/* 网关停止时后端改读落盘日志，所以这个入口任何时候都可用 */}
          <button
            onClick={onOpenLogs}
            className="flex shrink-0 cursor-pointer items-center gap-1 text-amber-700 underline-offset-2 hover:underline dark:text-amber-400"
          >
            <ScrollText size={11} />
            {t('gateway.requestLogs')}
          </button>
        </div>
      )}
    </div>
  )
}
