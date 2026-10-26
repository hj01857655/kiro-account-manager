import { useState } from 'react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { useApp } from '../../../hooks/useApp'
import { QuotaPanel } from './QuotaPanel'
import { BillsPanel } from './BillsPanel'
import { RequestsPanel } from './RequestsPanel'
import { SessionsPanel } from './SessionsPanel'
import { ContextPanel } from './ContextPanel'

/** 用量页容器：一页四个子标签（额度概览/请求/会话/上下文），对齐 magpie 的用量结构。 */
export function UsageTabs() {
  const { t } = useApp()
  const [sub, setSub] = useState('overview')

  return (
    <Tabs value={sub} onValueChange={setSub} className="flex flex-col h-full min-h-0">
      <div className="px-4 pt-3">
        <TabsList className="surface-card flex h-9 w-full justify-start rounded-lg border-none p-0.5 lg:w-fit">
          <TabsTrigger value="overview" className="gap-1.5 px-3.5 h-8 text-sm font-medium data-[active]:shadow-sm">
            {t('gatewayUsage.subOverview')}
          </TabsTrigger>
          <TabsTrigger value="requests" className="gap-1.5 px-3.5 h-8 text-sm font-medium data-[active]:shadow-sm">
            {t('gatewayUsage.subRequests')}
          </TabsTrigger>
          <TabsTrigger value="sessions" className="gap-1.5 px-3.5 h-8 text-sm font-medium data-[active]:shadow-sm">
            {t('gatewayUsage.subSessions')}
          </TabsTrigger>
          <TabsTrigger value="context" className="gap-1.5 px-3.5 h-8 text-sm font-medium data-[active]:shadow-sm">
            {t('gatewayUsage.subContext')}
          </TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="overview" className="m-0 flex-1 min-h-0 overflow-auto">
        {/* 概览：额度在上，账单明细（账号 × 模型）接在下方 */}
        <div className="flex flex-col divide-y">
          <div className="min-h-[280px]">
            <QuotaPanel />
          </div>
          <div className="min-h-[280px]">
            <BillsPanel />
          </div>
        </div>
      </TabsContent>

      <TabsContent value="requests" className="m-0 flex-1 min-h-0">
        <RequestsPanel />
      </TabsContent>

      <TabsContent value="sessions" className="m-0 flex-1 min-h-0">
        <SessionsPanel />
      </TabsContent>

      <TabsContent value="context" className="m-0 flex-1 min-h-0">
        <ContextPanel />
      </TabsContent>
    </Tabs>
  )
}
