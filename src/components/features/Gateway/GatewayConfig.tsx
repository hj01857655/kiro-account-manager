import type React from 'react'
import { useState } from 'react'
import { RotateCw, TrendingUp, Shuffle, Zap } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { GatewaySurfaceCard } from './GatewayShared'
import ModelMappingDialog from './ModelMappingDialog'
import ApiKeysDialog from './ApiKeysDialog'
import PromptFilterRulesDialog from './PromptFilterRulesDialog'
import { useApp } from '@/hooks/useApp'

interface GatewayConfigProps {
  config: any;
  fieldErrors: Record<string, string>;
  setField: (key: string, value: any) => void;
  accountOptions: any[];
  groupOptions: any[];
  setConfig: React.Dispatch<React.SetStateAction<any>>;
  applyGatewayLocalOnlyChange: (config: any, checked: boolean, generator: () => string) => any;
  createGeneratedApiKey: () => string;
  handleSaveConfig: () => Promise<void>;
  handleAutoStartToggle: (checked: boolean) => Promise<void>;
  onShowClientConfig?: () => void;
  hasConfiguredClients?: boolean;
}

function GatewayConfig({
  config,
  fieldErrors,
  setField,
  accountOptions,
  groupOptions,
  setConfig,
  applyGatewayLocalOnlyChange,
  createGeneratedApiKey,
  handleSaveConfig,
  handleAutoStartToggle,
  onShowClientConfig,
  hasConfiguredClients = false,
}: GatewayConfigProps) {
  const { t } = useApp()
  const [showModelMappingDialog, setShowModelMappingDialog] = useState(false)
  const [showApiKeysDialog, setShowApiKeysDialog] = useState(false)
  const [showPromptFilterRulesDialog, setShowPromptFilterRulesDialog] = useState(false)

  return (
    <div className="grid grid-cols-1 gap-3">
      <GatewaySurfaceCard>
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-4">
            {/* Section 1: network and routing */}
            <div className="space-y-3">
              <div className="text-sm font-medium text-foreground flex items-center gap-2">
                <div className="w-1 h-4 bg-primary rounded-full"></div>
                {t('gateway.networkConfig', { defaultValue: 'Network Configuration' })}
              </div>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div className="flex flex-col gap-1.5">
                  <Label>{t('gateway.listenAddress', { defaultValue: 'Listen Address' })}</Label>
                  <Input
                    value={config.host}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('host', e.target.value || '127.0.0.1')}
                    className={fieldErrors.host ? 'border-red-500' : ''}
                  />
                  {fieldErrors.host && <div className="text-xs text-red-500">{fieldErrors.host}</div>}
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>{t('gateway.port', { defaultValue: 'Port' })}</Label>
                  <Input
                    type="number"
                    value={config.port}
                    min={1}
                    max={65535}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('port', Number(e.target.value) || 8765)}
                    className={fieldErrors.port ? 'border-red-500' : ''}
                  />
                  {fieldErrors.port && <div className="text-xs text-red-500">{fieldErrors.port}</div>}
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>Region</Label>
                  <Select value={config.region} onValueChange={(v: string) => setField('region', v || 'us-east-1')}>
                    <SelectTrigger className={fieldErrors.region ? 'border-red-500' : ''}>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="us-east-1">us-east-1</SelectItem>
                      <SelectItem value="us-east-2">us-east-2</SelectItem>
                      <SelectItem value="us-west-1">us-west-1</SelectItem>
                      <SelectItem value="us-west-2">us-west-2</SelectItem>
                      <SelectItem value="eu-central-1">eu-central-1</SelectItem>
                      <SelectItem value="eu-central-2">eu-central-2</SelectItem>
                      <SelectItem value="eu-west-1">eu-west-1</SelectItem>
                      <SelectItem value="eu-west-2">eu-west-2</SelectItem>
                      <SelectItem value="eu-west-3">eu-west-3</SelectItem>
                      <SelectItem value="eu-north-1">eu-north-1</SelectItem>
                      <SelectItem value="eu-south-1">eu-south-1</SelectItem>
                      <SelectItem value="eu-south-2">eu-south-2</SelectItem>
                      <SelectItem value="ap-northeast-1">ap-northeast-1</SelectItem>
                      <SelectItem value="ap-northeast-2">ap-northeast-2</SelectItem>
                      <SelectItem value="ap-northeast-3">ap-northeast-3</SelectItem>
                      <SelectItem value="ap-southeast-1">ap-southeast-1</SelectItem>
                      <SelectItem value="ap-southeast-2">ap-southeast-2</SelectItem>
                      <SelectItem value="ap-southeast-3">ap-southeast-3</SelectItem>
                      <SelectItem value="ap-southeast-4">ap-southeast-4</SelectItem>
                      <SelectItem value="ap-southeast-5">ap-southeast-5</SelectItem>
                      <SelectItem value="ap-southeast-7">ap-southeast-7</SelectItem>
                      <SelectItem value="ap-south-1">ap-south-1</SelectItem>
                      <SelectItem value="ap-south-2">ap-south-2</SelectItem>
                      <SelectItem value="ap-east-1">ap-east-1</SelectItem>
                      <SelectItem value="af-south-1">af-south-1</SelectItem>
                      <SelectItem value="ca-central-1">ca-central-1</SelectItem>
                      <SelectItem value="ca-west-1">ca-west-1</SelectItem>
                      <SelectItem value="sa-east-1">sa-east-1</SelectItem>
                      <SelectItem value="me-south-1">me-south-1</SelectItem>
                      <SelectItem value="me-central-1">me-central-1</SelectItem>
                      <SelectItem value="il-central-1">il-central-1</SelectItem>
                      <SelectItem value="mx-central-1">mx-central-1</SelectItem>
                      <SelectItem value="us-gov-west-1">us-gov-west-1</SelectItem>
                      <SelectItem value="us-gov-east-1">us-gov-east-1</SelectItem>
                      <SelectItem value="cn-north-1">cn-north-1</SelectItem>
                      <SelectItem value="cn-northwest-1">cn-northwest-1</SelectItem>
                    </SelectContent>
                  </Select>
                  {fieldErrors.region && <div className="text-xs text-red-500">{fieldErrors.region}</div>}
                </div>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div className="flex items-center justify-between p-3 rounded-lg border border-border bg-muted/30">
                  <div className="flex flex-col gap-0.5">
                    <Label className="text-sm">{t('gateway.accountPool', { defaultValue: 'Account Pool' })}</Label>
                    <span className="text-xs text-muted-foreground">
                      {config.accountMode === 'pool'
                        ? t('gateway.useAllAvailableAccounts', { defaultValue: 'Use all available accounts' })
                        : config.accountMode === 'group'
                          ? t('gateway.useSpecifiedGroupAccounts', { defaultValue: 'Use accounts from the selected group' })
                          : t('gateway.fixedUseSingleAccount', { defaultValue: 'Use a fixed single account' })}
                    </span>
                  </div>
                  <Switch
                    checked={config.accountMode === 'pool' || config.accountMode === 'group'}
                    onCheckedChange={(checked: boolean) => setField('accountMode', checked ? 'pool' : 'single')}
                  />
                </div>
                {(config.accountMode === 'pool' || config.accountMode === 'group') ? (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.routingStrategy', { defaultValue: 'Routing Strategy' })}</Label>
                    <Select value={config.strategy} onValueChange={(v: string) => setField('strategy', v || 'round_robin')}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="round_robin"><div className="flex items-center gap-2"><RotateCw size={14} /><span>{t('gateway.roundRobin', { defaultValue: 'Round Robin' })}</span></div></SelectItem>
                        <SelectItem value="most_quota"><div className="flex items-center gap-2"><TrendingUp size={14} /><span>{t('gateway.priorityRemainingQuota', { defaultValue: 'Priority Remaining Quota' })}</span></div></SelectItem>
                        <SelectItem value="random"><div className="flex items-center gap-2"><Shuffle size={14} /><span>{t('gateway.random', { defaultValue: 'Random' })}</span></div></SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5">
                    <Label>{t('gateway.specifyAccount', { defaultValue: 'Specify Account' })}</Label>
                    <Select value={config.accountId} onValueChange={(v: string) => setField('accountId', v)}>
                      <SelectTrigger className={fieldErrors.accountId ? 'border-red-500' : ''}>
                        <SelectValue placeholder={t('gateway.selectAnAccount', { defaultValue: 'Select an account' })} />
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
            </div>

            {/* Section 2: client authentication and models */}
            <div className="space-y-3">
              <div className="text-sm font-medium text-foreground flex items-center gap-2">
                <div className="w-1 h-4 bg-primary rounded-full"></div>
                {t('gateway.clientAuth', { defaultValue: 'Client Authentication' })} & {t('gateway.model', { defaultValue: 'Model' })}
              </div>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div className="flex items-center justify-between p-3 border rounded-lg bg-muted/20">
                  <div className="text-sm text-muted-foreground">
                    {(() => {
                      const rawKeys = (config.clientApiKeysText || '').split(/[\n,]+/).map((k: string) => k.trim()).filter(Boolean)
                      const enabledCount = rawKeys.filter((k: string) => !k.startsWith('#disabled#')).length
                      return rawKeys.length > 0
                        ? `${rawKeys.length} keys, ${enabledCount} enabled`
                        : t('gateway.noApiKeys', { defaultValue: 'No API Keys yet' })
                    })()}
                  </div>
                  <Button size="sm" variant="outline" className="h-7 text-sm" onClick={() => setShowApiKeysDialog(true)}>
                    {t('gateway.clientApiKeys', { defaultValue: 'Client API Keys' })}
                  </Button>
                </div>
                <div className="flex items-center justify-between p-3 border rounded-lg bg-muted/20">
                  <div className="text-sm text-muted-foreground">
                    {config.modelMappings?.length > 0
                      ? `${config.modelMappings.length} mapping rules, ${config.modelMappings.filter((r: any) => r.enabled).length} enabled`
                      : t('gateway.noModelMappings', { defaultValue: 'No mapping rules yet' })}
                  </div>
                  <Button size="sm" variant="outline" className="h-7 text-sm" onClick={() => setShowModelMappingDialog(true)}>
                    <Shuffle size={12} className="mr-1" />
                    {t('gateway.modelMappingRules', { defaultValue: 'Mapping Rules' })}
                  </Button>
                </div>
                {onShowClientConfig && (
                  <div className="flex items-center justify-between p-3 border rounded-lg bg-muted/20">
                    <div className="text-sm text-muted-foreground">
                      {hasConfiguredClients
                        ? t('gateway.clientsConfigured', { defaultValue: 'Clients configured' })
                        : t('gateway.writeClientConfig', { defaultValue: 'Write client config' })}
                    </div>
                    <Button
                      size="sm"
                      variant={hasConfiguredClients ? "default" : "outline"}
                      className="h-7 text-sm"
                      onClick={onShowClientConfig}
                    >
                      <Zap size={12} className="mr-1" />
                      {hasConfiguredClients
                        ? t('gateway.reconfigure', { defaultValue: 'Reconfigure' })
                        : t('gateway.configureClients', { defaultValue: 'Configure Clients' })}
                    </Button>
                  </div>
                )}
              </div>
              {fieldErrors.clientApiKeysText && <div className="text-xs text-red-500">{fieldErrors.clientApiKeysText}</div>}
            </div>

            {/* Section 3: prompt filtering */}
            <div className="space-y-3">
              <div className="text-sm font-medium text-foreground flex items-center gap-2">
                <div className="w-1 h-4 bg-primary rounded-full"></div>
                {t('gateway.promptFiltering', { defaultValue: 'Prompt Filtering' })}
              </div>
              <div className="grid grid-cols-3 gap-2">
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.filterClaudeCode', { defaultValue: 'Trim Claude Code Prompt' })}</Label>
                  <Switch checked={!!config.filterClaudeCode} onCheckedChange={(checked: boolean) => setField('filterClaudeCode', checked)} />
                </div>
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.stripBoundaries', { defaultValue: 'Strip Boundary Markers' })}</Label>
                  <Switch checked={!!config.filterStripBoundaries} onCheckedChange={(checked: boolean) => setField('filterStripBoundaries', checked)} />
                </div>
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.removeEnvNoise', { defaultValue: 'Remove Environment Noise' })}</Label>
                  <Switch checked={!!config.filterEnvNoise} onCheckedChange={(checked: boolean) => setField('filterEnvNoise', checked)} />
                </div>
              </div>
              <div className="flex items-center justify-between p-3 border rounded-lg bg-muted/20">
                <div className="text-sm text-muted-foreground">
                  {config.promptFilterRules?.length > 0
                    ? `${config.promptFilterRules.length} custom rules, ${config.promptFilterRules.filter((r: any) => r.enabled).length} enabled`
                    : t('gateway.noCustomRules', { defaultValue: 'No custom rules yet' })}
                </div>
                <Button size="sm" variant="outline" className="h-7 text-sm" onClick={() => setShowPromptFilterRulesDialog(true)}>
                  {t('gateway.manageRules', { defaultValue: 'Manage Rules' })}
                </Button>
              </div>
            </div>

            {/* Section 4: security and advanced */}
            <div className="space-y-3">
              <div className="text-sm font-medium text-foreground flex items-center gap-2">
                <div className="w-1 h-4 bg-primary rounded-full"></div>
                {t('gateway.securityAndAccess', { defaultValue: 'Security and Access' })}
              </div>
              <div className="grid grid-cols-3 md:grid-cols-4 gap-2">
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.localOnly', { defaultValue: 'Local only' })}</Label>
                  <Switch
                    checked={!!config.localOnly}
                    onCheckedChange={(checked: boolean) => {
                      setConfig((prev: any) => applyGatewayLocalOnlyChange(prev, checked, createGeneratedApiKey))
                    }}
                  />
                </div>
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.autoStart', { defaultValue: 'Auto Start' })}</Label>
                  <Switch checked={!!config.enabled} onCheckedChange={handleAutoStartToggle} />
                </div>
                <div className="flex items-center justify-between p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-sm">{t('gateway.responseCache', { defaultValue: 'Response Cache' })}</Label>
                  <Switch checked={!!config.responseCacheEnabled} onCheckedChange={(checked: boolean) => setField('responseCacheEnabled', checked)} />
                </div>
                <div className="flex flex-col gap-0.5 p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-xs text-muted-foreground">{t('gateway.cacheTtlSeconds', { defaultValue: 'Cache TTL (seconds)' })}</Label>
                  <Input
                    type="number"
                    value={config.responseCacheTtl}
                    min={30}
                    max={3600}
                    className="h-6 text-sm px-1.5"
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('responseCacheTtl', Number(e.target.value) || 180)}
                    disabled={!config.responseCacheEnabled}
                  />
                </div>
                <div className="flex flex-col gap-0.5 p-2.5 rounded-lg border border-border bg-muted/30">
                  <Label className="text-xs text-muted-foreground">{t('gateway.switchThreshold', { defaultValue: 'Switch Threshold (%)' })}</Label>
                  <Input
                    type="number"
                    value={config.threshold}
                    min={1}
                    max={100}
                    className="h-6 text-sm px-1.5"
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setField('threshold', Number(e.target.value) || 90)}
                  />
                </div>
              </div>

              {!config.localOnly && (
                <div className="flex flex-col gap-1.5">
                  <Label>{t('gateway.ipWhitelistAllowRemoteAccess', { defaultValue: 'IP Whitelist' })}</Label>
                  <Textarea
                    placeholder={'192.168.1.10\n10.0.0.0/24'}
                    rows={2}
                    value={config.allowedIpsText}
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setField('allowedIpsText', e.target.value)}
                    className={fieldErrors.allowedIpsText ? 'border-red-500' : ''}
                  />
                  {fieldErrors.allowedIpsText && <div className="text-xs text-red-500">{fieldErrors.allowedIpsText}</div>}
                </div>
              )}
            </div>
          </div>
        </div>
      </GatewaySurfaceCard>

      {/* ModelMappingDialog */}
      <ModelMappingDialog
        open={showModelMappingDialog}
        onOpenChange={setShowModelMappingDialog}
        modelMappings={config.modelMappings}
        setField={setField}
        onSave={handleSaveConfig}
      />

      {/* ApiKeysDialog */}
      <ApiKeysDialog
        open={showApiKeysDialog}
        onOpenChange={setShowApiKeysDialog}
        clientApiKeysText={config.clientApiKeysText}
        setConfig={setConfig}
        onSave={handleSaveConfig}
      />

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
