import { Plus, Trash2, Zap } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useApp } from '@/hooks/useApp'

// Kiro internal model format used by the target model dropdown.
const TARGET_MODELS = [
  'claude-opus-4.8',
  'claude-opus-4.8-thinking',
  'claude-opus-4.7',
  'claude-opus-4.7-thinking',
  'claude-opus-4.6',
  'claude-opus-4.6-thinking',
  'claude-sonnet-4.6',
  'claude-sonnet-4.6-thinking',
  'claude-opus-4.5',
  'claude-opus-4.5-thinking',
  'claude-sonnet-4.5',
  'claude-sonnet-4.5-thinking',
  'claude-haiku-4.5',
  'claude-haiku-4.5-thinking',
  'claude-sonnet-4',
  'claude-sonnet-4-thinking',
  'deepseek-3.2',
  'minimax-m2.5',
  'glm-5',
  'qwen3-coder-next',
]

// Common source model names used by the source model dropdown.
const SOURCE_MODELS = [
  // Claude, including thinking variants.
  'claude-opus-4.8',
  'claude-opus-4.8-thinking',
  'claude-opus-4.7',
  'claude-opus-4.7-thinking',
  'claude-opus-4.6',
  'claude-opus-4.6-thinking',
  'claude-sonnet-4.6',
  'claude-sonnet-4.6-thinking',
  'claude-opus-4.5',
  'claude-opus-4.5-thinking',
  'claude-sonnet-4.5',
  'claude-sonnet-4.5-thinking',
  'claude-haiku-4.5',
  'claude-haiku-4.5-thinking',
  'claude-sonnet-4',
  'claude-sonnet-4-thinking',
  // GPT 5.5 series
  'gpt-5.5',
  'gpt-5.5-pro',
  'gpt-5.5-instant',
  // GPT 5.4 series
  'gpt-5.4',
  'gpt-5.4-pro',
  'gpt-5.4-mini',
  // GPT 5.3 series
  'gpt-5.3-codex',
  'gpt-5.3-codex-spark',
  'gpt-5.3-instant',
  // GPT 5.2 series
  'gpt-5.2',
  'gpt-5.2-pro',
  'gpt-5.2-codex',
  // GPT 5.1 series
  'gpt-5.1',
  'gpt-5.1-pro',
  'gpt-5.1-codex',
  'gpt-5.1-codex-max',
  'gpt-5.1-codex-mini',
  'gpt-5.1-instant',
]

// Preset GPT/Codex -> Claude mapping rules.
const PRESET_RULES = [
  // GPT-5.5 series -> Opus 4.7
  { source: 'gpt-5.5', target: 'claude-opus-4.8', name: 'GPT-5.5 → Opus 4.8' },  { source: 'gpt-5.5-pro', target: 'claude-opus-4.7', name: 'GPT-5.5-pro → Opus 4.7' },
  { source: 'gpt-5.5-instant', target: 'claude-sonnet-4.6', name: 'GPT-5.5-instant → Sonnet 4.6' },
  // GPT-5.4 series -> Opus/Sonnet 4.6
  { source: 'gpt-5.4', target: 'claude-opus-4.6', name: 'GPT-5.4 → Opus 4.6' },
  { source: 'gpt-5.4-pro', target: 'claude-opus-4.6', name: 'GPT-5.4-pro → Opus 4.6' },
  { source: 'gpt-5.4-mini', target: 'claude-sonnet-4.6', name: 'GPT-5.4-mini → Sonnet 4.6' },
  // GPT-5.3 series -> Opus 4.5 / Sonnet 4.5
  { source: 'gpt-5.3-codex', target: 'claude-opus-4.5', name: 'GPT-5.3-codex → Opus 4.5' },
  { source: 'gpt-5.3-codex-spark', target: 'claude-sonnet-4.5', name: 'GPT-5.3-codex-spark → Sonnet 4.5' },
  { source: 'gpt-5.3-instant', target: 'claude-sonnet-4.5', name: 'GPT-5.3-instant → Sonnet 4.5' },
  // GPT-5.2 series -> Opus 4.5
  { source: 'gpt-5.2', target: 'claude-opus-4.5', name: 'GPT-5.2 → Opus 4.5' },
  { source: 'gpt-5.2-pro', target: 'claude-opus-4.5', name: 'GPT-5.2-pro → Opus 4.5' },
  { source: 'gpt-5.2-codex', target: 'claude-opus-4.5', name: 'GPT-5.2-codex → Opus 4.5' },
  // GPT-5.1 series -> Sonnet 4.5 / Haiku 4.5
  { source: 'gpt-5.1', target: 'claude-sonnet-4.5', name: 'GPT-5.1 → Sonnet 4.5' },
  { source: 'gpt-5.1-pro', target: 'claude-sonnet-4.5', name: 'GPT-5.1-pro → Sonnet 4.5' },
  { source: 'gpt-5.1-codex', target: 'claude-sonnet-4.5', name: 'GPT-5.1-codex → Sonnet 4.5' },
  { source: 'gpt-5.1-codex-max', target: 'claude-opus-4.5', name: 'GPT-5.1-codex-max → Opus 4.5' },
  { source: 'gpt-5.1-codex-mini', target: 'claude-haiku-4.5', name: 'GPT-5.1-codex-mini → Haiku 4.5' },
  { source: 'gpt-5.1-instant', target: 'claude-haiku-4.5', name: 'GPT-5.1-instant → Haiku 4.5' },
]

interface ModelMappingDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  modelMappings: any[]
  setField: (key: string, value: any) => void
  onSave?: () => void
}

function ModelMappingDialog({ open, onOpenChange, modelMappings, setField, onSave }: ModelMappingDialogProps) {
  const { t } = useApp()
  const rules = modelMappings || []

  const handleToggle = (idx: number, checked: boolean) => {
    const updated = [...rules]
    updated[idx] = { ...updated[idx], enabled: checked }
    setField('modelMappings', updated)
  }

  const handleDelete = (idx: number) => {
    setField('modelMappings', rules.filter((_: any, i: number) => i !== idx))
  }

  const handleAdd = () => {
    const sourceEl = document.getElementById('dialog-mapping-source') as HTMLInputElement
    const targetEl = document.getElementById('dialog-mapping-target') as HTMLInputElement
    if (!sourceEl?.value?.trim() || !targetEl?.value?.trim()) return

    const newRule = {
      id: crypto.randomUUID(),
      name: `${sourceEl.value.trim()} → ${targetEl.value.trim()}`,
      enabled: true,
      ruleType: 'replace',
      sourceModel: sourceEl.value.trim(),
      targetModels: [targetEl.value.trim()],
      weights: []
    }
    setField('modelMappings', [...rules, newRule])
    sourceEl.value = ''
    targetEl.value = ''
  }

  const handlePreset = () => {
    const existingSources = new Set(rules.map((r: any) => r.sourceModel))
    const newRules = PRESET_RULES
      .filter(p => !existingSources.has(p.source))
      .map(p => ({
        id: crypto.randomUUID(),
        name: p.name,
        enabled: true,
        ruleType: 'replace',
        sourceModel: p.source,
        targetModels: [p.target],
        weights: []
      }))
    if (newRules.length > 0) {
      setField('modelMappings', [...rules, ...newRules])
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v && onSave) onSave() }}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{t('gateway.modelMappingRules', { defaultValue: 'Model Mapping Rules' })}</DialogTitle>
          <DialogDescription>
            {t('gateway.modelMappingDesc', { defaultValue: 'Client-requested model names are mapped to Kiro internal models according to these rules.' })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 mt-2">
          {/* Rule list */}
          <div className="border rounded-lg overflow-hidden max-h-[300px] overflow-y-auto">
            {rules.length === 0 ? (
              <div className="p-6 text-center text-sm text-muted-foreground">
                {t('gateway.noModelMappings', { defaultValue: 'No mapping rules yet' })}
              </div>
            ) : (
              rules.map((rule: any, idx: number) => (
                <div key={rule.id} className={`flex items-center gap-2 p-3 border-b last:border-b-0 hover:bg-muted/30 ${!rule.enabled ? 'opacity-50' : ''}`}>
                  <Switch
                    size="sm"
                    checked={rule.enabled}
                    onCheckedChange={(checked) => handleToggle(idx, checked)}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span className="text-sm font-medium truncate">{rule.name || rule.sourceModel}</span>
                      <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                        {rule.ruleType === 'replace'
                          ? t('gateway.replace', { defaultValue: 'Replace' })
                          : rule.ruleType === 'alias'
                            ? t('gateway.alias', { defaultValue: 'Alias' })
                            : t('gateway.loadBalancing', { defaultValue: 'Load Balancing' })}
                      </Badge>
                    </div>
                    <div className="text-xs text-muted-foreground font-mono truncate mt-0.5">
                      {rule.sourceModel} → {rule.targetModels.join(', ')}
                    </div>
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 w-7 p-0 text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-950/20"
                    onClick={() => handleDelete(idx)}
                  >
                    <Trash2 size={14} />
                  </Button>
                </div>
              ))
            )}
          </div>

          {/* Add a new rule */}
          <div className="space-y-2 p-3 border rounded-lg bg-muted/10">
            <div className="text-xs font-medium text-muted-foreground">
              {t('gateway.addNewRule', { defaultValue: 'Add New Rule' })}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div className="relative">
                <Input placeholder={t('gateway.sourceModelName', { defaultValue: 'Source model name' })} className="text-xs" id="dialog-mapping-source" list="model-list-source" />
                <datalist id="model-list-source">
                  {SOURCE_MODELS.map(m => <option key={m} value={m} />)}
                </datalist>
              </div>
              <div className="relative">
                <Input placeholder={t('gateway.targetModelName', { defaultValue: 'Target model name' })} className="text-xs" id="dialog-mapping-target" list="model-list-target" />
                <datalist id="model-list-target">
                  {TARGET_MODELS.map(m => <option key={m} value={m} />)}
                </datalist>
              </div>
            </div>
            <div className="flex gap-2">
              <Select defaultValue="replace">
                <SelectTrigger className="text-xs flex-1" id="dialog-mapping-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="replace">{t('gateway.replace', { defaultValue: 'Replace' })} (replace)</SelectItem>
                  <SelectItem value="alias">{t('gateway.alias', { defaultValue: 'Alias' })} (alias)</SelectItem>
                </SelectContent>
              </Select>
              <Button size="sm" className="text-xs" onClick={handleAdd}>
                <Plus size={14} className="mr-1" />
                {t('gateway.add', { defaultValue: 'Add' })}
              </Button>
            </div>
          </div>

          {/* Preset rules */}
          <div className="flex items-center justify-between pt-1">
            <div className="text-xs text-muted-foreground">
              {t('gateway.quickAddOpenAICodexMappings', { defaultValue: 'Quickly add OpenAI/Codex-compatible mappings' })}
            </div>
            <Button size="sm" variant="outline" className="h-7 text-xs" onClick={handlePreset}>
              <Zap size={12} className="mr-1" />
              {t('gateway.presetGptMappings', { defaultValue: 'Preset GPT Mappings' })}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default ModelMappingDialog
