import type React from 'react'
import { useState, useMemo, useEffect } from 'react'
import { RotateCw, TrendingUp, Shuffle, Users, CheckCircle2, Search, AlertCircle } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Checkbox } from '@/components/ui/checkbox'
import { Badge } from '@/components/ui/badge'
import { formatUsage, getQuota, getUsed } from '@/utils/accountStats'

const POOL_STATUS_COLORS: Record<string, string> = {
  active: 'bg-green-500',
  overage: 'bg-purple-500',
  capped: 'bg-yellow-500',
  banned: 'bg-red-500',
  invalid: 'bg-gray-500',
}
const POOL_STATUS_ORDER: Record<string, number> = { active: 0, overage: 1, '': 0, capped: 2, invalid: 3, banned: 4 }
const POOL_STATUS_KEYS: Record<string, string> = {
  active: 'gateway.poolStatusActive',
  overage: 'gateway.poolStatusOverage',
  capped: 'gateway.poolStatusCapped',
  banned: 'gateway.poolStatusBanned',
  invalid: 'gateway.poolStatusInvalid',
}

const STRATEGY_KEYS: Record<string, string> = {
  round_robin: 'gateway.roundRobin',
  random: 'gateway.random',
  balanced: 'gateway.strategyBalanced',
  most_quota: 'gateway.strategyMostQuota',
  weighted_random: 'gateway.weightedRandom',
  least_connections: 'gateway.leastConnections',
}

const ACCOUNT_MODE_KEYS: Record<string, string> = {
  single: 'gateway.singleAccount',
  group: 'gateway.byGroup',
  pool: 'gateway.globalPool',
}

// 只暴露语义能自解释的三种策略；后端另支持 balanced / weighted_random /
// least_connections，但界面无法解释其差异，故不给出会误导的选项。
type StrategyOption = { value: string; icon: LucideIcon }

const STRATEGY_OPTIONS: StrategyOption[] = [
  { value: 'round_robin', icon: RotateCw },
  { value: 'most_quota', icon: TrendingUp },
  { value: 'random', icon: Shuffle },
]
import { Textarea } from '@/components/ui/textarea'
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion'
import { GatewaySurfaceCard } from './GatewayShared'
import { ALLOWED_REGIONS } from './gatewayPageUtils'
import PromptFilterRulesDialog from './PromptFilterRulesDialog'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter
} from '@/components/shared/dialog'
import { useApp } from '../../../hooks/useApp'

interface GatewayConfigProps {
  config: any;
  fieldErrors: Record<string, string>;
  setField: (key: string, value: any) => void;
  accountOptions: any[];
  groupOptions: any[];
  setConfig: React.Dispatch<React.SetStateAction<any>>;
  applyGatewayLocalOnlyChange: (config: any, checked: boolean) => any;
  handleSaveConfig: () => Promise<void>;
  handleAutoStartToggle: (checked: boolean) => Promise<void>;
}

function GatewayConfig({
  config,
  fieldErrors,
  setField,
  accountOptions,
  groupOptions,
  setConfig,
  applyGatewayLocalOnlyChange,
  handleSaveConfig,
  handleAutoStartToggle,
}: GatewayConfigProps) {
  const { t } = useApp()
  const [showPromptFilterRulesDialog, setShowPromptFilterRulesDialog] = useState(false)
  const [showAccountPoolDialog, setShowAccountPoolDialog] = useState(false)
  const [poolSearchQuery, setPoolSearchQuery] = useState('')
  const [poolStatusFilter, setPoolStatusFilter] = useState<string>('all')

  const getStrategyLabel = (strategy: string) =>
    STRATEGY_KEYS[strategy] ? t(STRATEGY_KEYS[strategy]) : strategy

  const getAccountModeLabel = (mode: string) =>
    ACCOUNT_MODE_KEYS[mode] ? t(ACCOUNT_MODE_KEYS[mode]) : (mode || 'single')

  const selectedPoolAccountIds = Array.isArray(config.poolAccountIds) ? config.poolAccountIds : []
  // 配置里的 id 可能指向已删除的账号。分母是「现存可选账号」，故分子也只该数
  // 真正能解析到账号的 id，否则会出现「已选 23 / 共 0」这种分母小于分子的昏话。
  const availablePoolIdSet = new Set(accountOptions.map((o: any) => o.value))
  const effectiveSelectedPoolIds = selectedPoolAccountIds.filter((id: string) =>
    availablePoolIdSet.has(id)
  )
  const selectedPoolAccounts = accountOptions.filter((account: any) => selectedPoolAccountIds.includes(account.value))
  const effectiveStrategy = config.strategy || 'round_robin'
  const promptRules = config.promptFilterRules || []
  const promptSwitchesOn = [config.filterClaudeCode, config.filterStripBoundaries, config.filterEnvNoise]
    .filter(Boolean).length

  // 各折叠组标题右侧的当前值摘要：收起状态下仍能一眼读全「现在是怎样」
  // 「请求处理」组合并提示词清洗与响应缓存：两者都是改请求的旋钮，
  // 摘要因此要同时说出两边各自的状态
  const requestHandlingSummary = [
    `${promptSwitchesOn} ${t('gateway.switchesOn')}`,
    config.responseCacheEnabled
      ? `${t('gateway.enabled')} · TTL ${config.responseCacheTtl}s`
      : t('gateway.disabled'),
    config.webSearchEnabled ? t('gateway.webSearchOn') : t('gateway.webSearchOff'),
    config.toolSearchEnabled ? t('gateway.toolSearchOn') : t('gateway.toolSearchOff')
  ].join(' · ')
  const accessSummary = `${config.localOnly ? t('gateway.localOnly') : t('gateway.allowRemote')} · ${config.enabled ? t('gateway.autoStartOn') : t('gateway.autoStartOff')}`

  const routingSummary = (() => {
    const mode = getAccountModeLabel(config.accountMode)
    if (config.accountMode === 'pool') {
      const selected = effectiveSelectedPoolIds.length > 0
        ? `${t('gateway.selected')} ${effectiveSelectedPoolIds.length} ${t('gateway.accounts')}`
        : t('gateway.noPoolSelected')
      return `${mode} · ${selected} · ${getStrategyLabel(effectiveStrategy)}`
    }
    if (config.accountMode === 'group') {
      const groupLabel = groupOptions.find((option: any) => option.value === config.groupId)?.label
      return `${mode} · ${groupLabel || t('gateway.notSelected')} · ${getStrategyLabel(effectiveStrategy)}`
    }
    const accountLabel = accountOptions.find((option: any) => option.value === config.accountId)?.label
    return `${mode} · ${accountLabel || t('gateway.notSelected')}`
  })()

  const filteredPoolAccounts = useMemo(() => {
    return accountOptions
      .filter((opt: any) => {
        const acct = opt.account
        const status = acct?.status || 'active'
        if (poolStatusFilter !== 'all' && status !== poolStatusFilter) return false
        if (poolSearchQuery.trim()) {
          const q = poolSearchQuery.toLowerCase()
          const email = (acct?.email || '').toLowerCase()
          const userId = (acct?.userId || '').toLowerCase()
          const provider = (acct?.provider || '').toLowerCase()
          return email.includes(q) || userId.includes(q) || provider.includes(q)
        }
        return true
      })
      .sort((a: any, b: any) => {
        const sa = POOL_STATUS_ORDER[a.account?.status || ''] ?? 5
        const sb = POOL_STATUS_ORDER[b.account?.status || ''] ?? 5
        if (sa !== sb) return sa - sb
        const ca = selectedPoolAccountIds.includes(a.value) ? 0 : 1
        const cb = selectedPoolAccountIds.includes(b.value) ? 0 : 1
        return ca - cb
      })
  }, [accountOptions, poolSearchQuery, poolStatusFilter, selectedPoolAccountIds])

  const togglePoolAccount = (accountId: string) => {
    const next = selectedPoolAccountIds.includes(accountId)
      ? selectedPoolAccountIds.filter((id: string) => id !== accountId)
      : [...selectedPoolAccountIds, accountId]
    setField('poolAccountIds', next)
  }

  const toggleAllPoolAccounts = () => {
    // 用「现存账号是否都已在池中」判定，而非直接比较长度：
    // 池里可能残留已删除账号的 id，长度相等不代表选满
    if (accountOptions.length > 0 && effectiveSelectedPoolIds.length === accountOptions.length) {
      setField('poolAccountIds', [])
    } else {
      setField('poolAccountIds', accountOptions.map((account: any) => account.value))
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <GatewaySurfaceCard>
        {/* 设置 tab 的全部字段按使用频率收进折叠组，默认全部收起 */}
        <Accordion type="multiple" className="w-full">
          <AccordionItem value="routing">
            <AccordionTrigger>
              <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
                <span className="shrink-0 text-sm font-medium text-foreground">{t('gateway.groupRouting')}</span>
                {(!!fieldErrors.accountId || !!fieldErrors.groupId) && (
                  <AlertCircle size={13} className="shrink-0 text-red-500" />
                )}
                <span className="ml-auto min-w-0 truncate text-[11px] font-normal text-muted-foreground" title={routingSummary}>
                  {routingSummary}
                </span>
              </span>
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-3 pb-1">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div className="flex flex-col gap-1.5">
                  <Label>{t('gateway.accountMode')}</Label>
                  <Select value={config.accountMode} onValueChange={(v: string) => setField('accountMode', v)}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="single">{t('gateway.singleAccount')}</SelectItem>
                      <SelectItem value="group">{t('gateway.byGroup')}</SelectItem>
                      <SelectItem value="pool">{t('gateway.globalPool')}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                {config.accountMode === 'pool' ? (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.accountPool')}</Label>
                    <Button
                      variant="outline"
                      className="h-10 justify-start"
                      onClick={() => setShowAccountPoolDialog(true)}
                    >
                      <Users size={16} className="mr-2" />
                      {effectiveSelectedPoolIds.length > 0
                        ? `${t('gateway.selected')} ${effectiveSelectedPoolIds.length} ${t('gateway.accounts')}`
                        : t('gateway.configureAccountPool')}
                    </Button>
                  </div>
                ) : config.accountMode === 'group' ? (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.selectGroup')}</Label>
                    <Select value={config.groupId} onValueChange={(v: string) => setField('groupId', v)}>
                      <SelectTrigger className={fieldErrors.groupId ? 'border-red-500' : ''}>
                        <SelectValue placeholder={t('gateway.selectGroupPlaceholder')} />
                      </SelectTrigger>
                      <SelectContent>
                        {groupOptions.map((opt: any) => (
                          <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {fieldErrors.groupId && <div className="text-xs text-red-500">{fieldErrors.groupId}</div>}
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.specifySingleAccount')}</Label>
                    <Select value={config.accountId} onValueChange={(v: string) => setField('accountId', v)}>
                      <SelectTrigger className={fieldErrors.accountId ? 'border-red-500' : ''}>
                        <SelectValue placeholder={t('gateway.selectAGroup')} />
                      </SelectTrigger>
                      <SelectContent>
                        {accountOptions.map((opt: any) => (
                          <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {fieldErrors.accountId && <div className="text-xs text-red-500">{fieldErrors.accountId}</div>}
                  </div>
                )}
              </div>
              {/* 单账号模式固定使用所选账号，策略与阈值不生效，故在该模式下不展示 */}
              {config.accountMode !== 'single' && (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.routingStrategy')}</Label>
                    <Select value={effectiveStrategy} onValueChange={(v: string) => setField('strategy', v || 'round_robin')}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {STRATEGY_OPTIONS.map(option => {
                          const Icon = option.icon
                          return (
                            <SelectItem key={option.value} value={option.value}>
                              <div className="flex items-center gap-2">
                                <Icon size={14} />
                                <span>{getStrategyLabel(option.value)}</span>
                              </div>
                            </SelectItem>
                          )
                        })}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.thresholdPercent')}</Label>
                    <Input
                      type="number"
                      value={config.threshold}
                      min={1}
                      max={100}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('threshold', Number(e.target.value) || 90)}
                    />
                    <span className="text-[11px] text-muted-foreground">{t('gateway.thresholdHint')}</span>
                  </div>
                </div>
              )}

              {/* Region 属于上游连接参数（网关调 Kiro 时用），不是本地监听参数。
                  原先放在「网络与服务」组里会让人以为它影响本地服务。 */}
              <div className="flex flex-col gap-1.5">
                <Label>{t('gateway.upstreamRegion')}</Label>
                <Select value={config.region} onValueChange={(v: string) => setField('region', v || 'us-east-1')}>
                  <SelectTrigger className={fieldErrors.region ? 'border-red-500' : ''}>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ALLOWED_REGIONS.map(region => (
                      <SelectItem key={region} value={region}>{region}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <span className="text-[11px] text-muted-foreground">{t('gateway.upstreamRegionHint')}</span>
                {fieldErrors.region && <div className="text-xs text-red-500">{fieldErrors.region}</div>}
              </div>
              </div>
            </AccordionContent>
          </AccordionItem>

          {/* 客户端密钥与模型映射改为下方平铺入口卡，不再各占一个折叠组 */}

          {/* 提示词清洗与响应缓存合并为一组「请求处理」：两者都是改请求的旋钮。
              原先「客户端密钥」「模型映射」各占一个折叠组，展开后只有一个开弹窗的按钮，
              没有表单可收，且组标题摘要与内容摘要重复，已改为下方平铺入口卡。 */}
          <AccordionItem value="requestHandling">
            <AccordionTrigger>
              <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
                <span className="shrink-0 text-sm font-medium text-foreground">{t('gateway.groupRequestHandling')}</span>
                <span className="ml-auto min-w-0 truncate text-[11px] font-normal text-muted-foreground" title={requestHandlingSummary}>
                  {requestHandlingSummary}
                </span>
              </span>
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-3 pb-1">
              <div className="grid grid-cols-1 gap-2 md:grid-cols-3">
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.simplifyCCPrompt')}</Label>
                  <Switch checked={!!config.filterClaudeCode} onCheckedChange={(checked: boolean) => setField('filterClaudeCode', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.removeBoundaryMarkers')}</Label>
                  <Switch checked={!!config.filterStripBoundaries} onCheckedChange={(checked: boolean) => setField('filterStripBoundaries', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.removeEnvNoise')}</Label>
                  <Switch checked={!!config.filterEnvNoise} onCheckedChange={(checked: boolean) => setField('filterEnvNoise', checked)} />
                </div>
              </div>
              <div className="flex items-center justify-between gap-3 rounded-lg border border-border bg-muted/20 p-3">
                <span className="text-xs text-muted-foreground">
                  {promptRules.length > 0
                    ? `${promptRules.length} ${t('gateway.customRules')}, ${promptRules.filter((r: any) => r.enabled).length} ${t('gateway.enabled')}`
                    : t('gateway.noCustomRules')}
                </span>
                <Button size="sm" variant="outline" className="h-7 text-sm" onClick={() => setShowPromptFilterRulesDialog(true)}>
                  {t('gateway.manageRules')}
                </Button>
              </div>
              <div className="grid grid-cols-1 gap-3 pb-1 md:grid-cols-2">
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.responseCache')}</Label>
                  <Switch checked={!!config.responseCacheEnabled} onCheckedChange={(checked: boolean) => setField('responseCacheEnabled', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.webSearchToggle')}</Label>
                  <Switch checked={!!config.webSearchEnabled} onCheckedChange={(checked: boolean) => setField('webSearchEnabled', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm">{t('gateway.toolSearchToggle')}</Label>
                  <Switch checked={!!config.toolSearchEnabled} onCheckedChange={(checked: boolean) => setField('toolSearchEnabled', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm" title={t('gateway.sessionAffinityHint')}>{t('gateway.sessionAffinityToggle')}</Label>
                  <Switch checked={!!config.sessionAffinity} onCheckedChange={(checked: boolean) => setField('sessionAffinity', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm" title={t('gateway.logStreamEventsHint')}>{t('gateway.logStreamEventsToggle')}</Label>
                  <Switch checked={!!config.logStreamEvents} onCheckedChange={(checked: boolean) => setField('logStreamEvents', checked)} />
                </div>
                <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                  <Label className="text-sm" title={t('gateway.privacyMaskHint')}>{t('gateway.privacyMaskToggle')}</Label>
                  <Switch checked={!!config.privacyMaskEnabled} onCheckedChange={(checked: boolean) => setField('privacyMaskEnabled', checked)} />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label className="text-xs text-muted-foreground">{t('gateway.cacheTTLSeconds')}</Label>
                  <Input
                    type="number"
                    value={config.responseCacheTtl}
                    min={30}
                    max={3600}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('responseCacheTtl', Number(e.target.value) || 180)}
                    disabled={!config.responseCacheEnabled}
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label className="text-xs text-muted-foreground">{t('gateway.maxToolDescLabel')}</Label>
                  <Input
                    type="number"
                    value={config.maxToolDescriptionLength}
                    min={0}
                    max={100000}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('maxToolDescriptionLength', Number(e.target.value) || 0)}
                  />
                  <span className="text-[11px] text-muted-foreground">{t('gateway.maxToolDescHint')}</span>
                </div>
              </div>
              </div>
            </AccordionContent>
          </AccordionItem>

          <AccordionItem value="access">
            <AccordionTrigger>
              <span className="flex min-w-0 flex-1 items-center gap-2 pr-2">
                <span className="shrink-0 text-sm font-medium text-foreground">{t('gateway.groupAccess')}</span>
                {!!fieldErrors.allowedIpsText && <AlertCircle size={13} className="shrink-0 text-red-500" />}
                <span className="ml-auto min-w-0 truncate text-[11px] font-normal text-muted-foreground" title={accessSummary}>
                  {accessSummary}
                </span>
              </span>
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-3 pb-1">
                <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                  <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                    <Label className="text-sm">{t('gateway.localOnly')}</Label>
                    <Switch
                      checked={!!config.localOnly}
                      onCheckedChange={(checked: boolean) => {
                        setConfig((prev: any) => applyGatewayLocalOnlyChange(prev, checked))
                      }}
                    />
                  </div>
                  <div className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 p-2.5">
                    <Label className="text-sm">{t('gateway.autoStart')}</Label>
                    <Switch checked={!!config.enabled} onCheckedChange={handleAutoStartToggle} />
                  </div>
                </div>

                {!config.localOnly && (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.ipWhitelist')}</Label>
                    <Textarea
                      placeholder={'192.168.1.10\n10.0.0.0/24'}
                      rows={2}
                      value={config.allowedIpsText}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setField('allowedIpsText', e.target.value)}
                      className={fieldErrors.allowedIpsText ? 'border-red-500' : ''}
                    />
                    {fieldErrors.allowedIpsText
                      ? <div className="text-xs text-red-500">{fieldErrors.allowedIpsText}</div>
                      : <span className="text-[11px] text-muted-foreground">{t('gateway.ipWhitelistHint')}</span>}
                  </div>
                )}

                <div className="flex flex-col gap-1.5">
                  <Label>{t('gateway.upstreamProxyLabel')}</Label>
                  <input
                    type="text"
                    value={config.upstreamProxy ?? ''}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('upstreamProxy', e.target.value)}
                    placeholder="http://127.0.0.1:7890"
                    className={
                      'h-9 w-full rounded-md border border-border bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 ' +
                      (fieldErrors.upstreamProxy ? 'border-red-500' : '')
                    }
                  />
                  {fieldErrors.upstreamProxy
                    ? <div className="text-xs text-red-500">{fieldErrors.upstreamProxy}</div>
                    : <span className="text-[11px] text-muted-foreground">{t('gateway.upstreamProxyHint')}</span>}
                </div>
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      </GatewaySurfaceCard>

      {/* AccountPoolDialog */}
      <DialogRoot open={showAccountPoolDialog} onOpenChange={setShowAccountPoolDialog}>
        <DialogContent maxWidth="800px">
          <DialogHeader icon={Users}>
            <DialogTitle>{t('gateway.configureAccountPool')}</DialogTitle>
            <DialogDescription>
              {t('gateway.accountPoolDescription')}
            </DialogDescription>
          </DialogHeader>

          <DialogBody className="space-y-4">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <div className="rounded-xl border border-border bg-muted/20 p-3">
                <div className="text-xs text-muted-foreground">{t('gateway.selectedAccounts')}</div>
                <div className="mt-1 text-2xl font-semibold text-foreground">{effectiveSelectedPoolIds.length}<span className="text-sm text-muted-foreground font-normal ml-1">/ {accountOptions.length}</span></div>
              </div>
              <div className="rounded-xl border border-border bg-muted/20 p-3">
                <div className="flex items-center justify-between gap-2">
                  <div className="text-xs text-muted-foreground">{t('gateway.routingStrategy')}</div>
                  {!config.strategy && <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px]">{t('gateway.defaultRoundRobin')}</Badge>}
                </div>
                <Select value={effectiveStrategy} onValueChange={(v: string) => setField('strategy', v || 'round_robin')}>
                  <SelectTrigger className="mt-1 h-9">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="round_robin"><div className="flex items-center gap-2"><RotateCw size={14} /><span>{t('gateway.roundRobin')}</span></div></SelectItem>
                    <SelectItem value="most_quota"><div className="flex items-center gap-2"><TrendingUp size={14} /><span>{t('gateway.priorityQuota')}</span></div></SelectItem>
                    <SelectItem value="random"><div className="flex items-center gap-2"><Shuffle size={14} /><span>{t('gateway.random')}</span></div></SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            {/* 搜索 + 过滤 + 操作栏 */}
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <Search className="absolute left-2.5 top-2 h-4 w-4 text-muted-foreground" />
                <Input
                  value={poolSearchQuery}
                  onChange={e => setPoolSearchQuery(e.target.value)}
                  placeholder={t('gateway.searchAccounts')}
                  className="h-8 pl-8 text-xs"
                />
              </div>
              <Select value={poolStatusFilter} onValueChange={setPoolStatusFilter}>
                <SelectTrigger className="h-8 w-[100px] text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{t('gateway.statusAll')}</SelectItem>
                  <SelectItem value="active">{t('gateway.poolStatusActive')}</SelectItem>
                  <SelectItem value="overage">{t('gateway.poolStatusOverage')}</SelectItem>
                  <SelectItem value="capped">{t('gateway.poolStatusCapped')}</SelectItem>
                  <SelectItem value="banned">{t('gateway.poolStatusBanned')}</SelectItem>
                  <SelectItem value="invalid">{t('gateway.poolStatusInvalid')}</SelectItem>
                </SelectContent>
              </Select>
              <Button type="button" variant="outline" size="sm" className="h-8 text-xs" onClick={toggleAllPoolAccounts} disabled={accountOptions.length === 0}>
                {effectiveSelectedPoolIds.length === accountOptions.length && accountOptions.length > 0 ? t('gateway.deselectAll') : t('gateway.selectAll')}
              </Button>
              <Button type="button" variant="outline" size="sm" className="h-8 text-xs" onClick={() => {
                const activeIds = accountOptions
                  .filter((opt: any) => {
                    const acct = opt.account
                    if (!acct) return false
                    if (acct.enabled === false) return false
                    const status = (acct.status || '').toLowerCase()
                    return status === 'active' || status === 'overage' || status === ''
                  })
                  .map((opt: any) => opt.value)
                setField('poolAccountIds', activeIds)
              }}>
                {t('gateway.selectAvailable')}
              </Button>
            </div>

            {/* 账号列表 */}
            <div className="max-h-[380px] space-y-1.5 overflow-y-auto pr-1">
              {accountOptions.length === 0 ? (
                <div className="rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
                  {t('gateway.noAvailableAccounts')}
                </div>
              ) : (
                filteredPoolAccounts.map((opt: any) => {
                    const checked = selectedPoolAccountIds.includes(opt.value)
                    const acct = opt.account || {}
                    const status = acct.status || 'active'
                    const breakdown = acct.usageData?.usageBreakdownList?.[0]
                    const used = getUsed(acct)
                    const limit = getQuota(acct)
                    const currentOverages = breakdown?.currentOverages ?? 0
                    const overageCap = breakdown?.overageCap ?? 0
                    const percent = limit > 0 ? Math.min(100, (used / limit) * 100) : 0
                    const email = acct.email || acct.userId || t('gateway.unknownAccount')
                    const provider = acct.provider || ''
                    const subPlan = acct.usageData?.subscriptionInfo?.subscriptionTitle || ''
                    const hasData = !!acct.usageData

                    return (
                      <div
                        key={opt.value}
                        role="button"
                        tabIndex={0}
                        className={`flex w-full items-center gap-3 rounded-lg border p-2.5 text-left transition-all cursor-pointer ${checked ? 'border-primary bg-primary/5 ring-1 ring-primary/20' : 'border-border bg-background/70 hover:border-primary/40 hover:bg-primary/5'} ${status === 'banned' || status === 'invalid' ? 'opacity-60' : ''}`}
                        onClick={() => togglePoolAccount(opt.value)}
                        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') togglePoolAccount(opt.value) }}
                      >
                        <Checkbox checked={checked} onCheckedChange={() => togglePoolAccount(opt.value)} onClick={(e) => e.stopPropagation()} />
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-1.5 flex-wrap">
                            <span className="truncate text-xs font-medium text-foreground">{email}</span>
                            {provider && <Badge variant="outline" className="text-[9px] px-1 py-0 h-4 shrink-0">{provider}</Badge>}
                            {subPlan && <Badge variant="secondary" className="text-[9px] px-1 py-0 h-4 shrink-0">{subPlan}</Badge>}
                            <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${POOL_STATUS_COLORS[status] || 'bg-gray-400'}`} />
                            <span className="text-[9px] text-muted-foreground shrink-0">{POOL_STATUS_KEYS[status] ? t(POOL_STATUS_KEYS[status]) : (status || '')}</span>
                          </div>
                          {/* 额度信息 */}
                          {hasData ? (
                            <div className="mt-1.5 flex items-center gap-2">
                              <div className="flex-1 h-1 rounded-full bg-muted overflow-hidden">
                                <div
                                  className={`h-full rounded-full transition-all ${currentOverages > 0 ? 'bg-purple-500' : percent >= 90 ? 'bg-red-500' : percent >= 70 ? 'bg-yellow-500' : 'bg-green-500'}`}
                                  style={{ width: `${Math.min(100, percent)}%` }}
                                />
                              </div>
                              <span className="text-[9px] text-muted-foreground whitespace-nowrap">
                                {currentOverages > 0
                                  ? `⚡${formatUsage(currentOverages)}${overageCap > 0 ? '/' + formatUsage(overageCap) : ''}`
                                  : limit > 0
                                    ? `${formatUsage(Math.max(0, limit - used))}/${formatUsage(limit)} (${Math.round(100 - percent)}%)`
                                    : t('gateway.noQuota')
                                }
                              </span>
                            </div>
                          ) : (
                            <div className="mt-1 text-[9px] text-muted-foreground/60 italic">{t('gateway.notRefreshed')}</div>
                          )}
                        </div>
                        {checked && <CheckCircle2 size={14} className="text-primary shrink-0" />}
                      </div>
                    )
                  })
              )}
            </div>
          </DialogBody>

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAccountPoolDialog(false)}>{t('gateway.close')}</Button>
            <Button onClick={() => { setShowAccountPoolDialog(false); handleSaveConfig() }}>{t('gateway.saveConfig')}</Button>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>

      {/* PromptFilterRulesDialog */}
      <PromptFilterRulesDialog
        open={showPromptFilterRulesDialog}
        onOpenChange={setShowPromptFilterRulesDialog}
        promptFilterRules={config.promptFilterRules}
        setField={setField}
        onSave={handleSaveConfig}
      />
    </div>
  )
}

export default GatewayConfig
