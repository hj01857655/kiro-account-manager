import { Plus, Trash2, Filter } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { useApp } from '@/hooks/useApp'

// Preset filter rules.
const PRESET_RULES = [
  {
    name: 'Filter Git status information',
    ruleType: 'lines-containing',
    matchPattern: 'git status',
    replace: ''
  },
  {
    name: 'Filter recent commit information',
    ruleType: 'lines-containing',
    matchPattern: 'Recent commits:',
    replace: ''
  },
  {
    name: 'Filter assistant knowledge cutoff',
    ruleType: 'lines-containing',
    matchPattern: 'Assistant knowledge cutoff',
    replace: ''
  },
  {
    name: 'Filter billing header information',
    ruleType: 'lines-containing',
    matchPattern: 'x-anthropic-billing-header:',
    replace: ''
  },
  {
    name: 'Filter fast mode tags',
    ruleType: 'regex',
    matchPattern: '<fast_mode_info>.*?</fast_mode_info>',
    replace: ''
  },
  {
    name: 'Filter project path information',
    ruleType: 'lines-containing',
    matchPattern: '.claude/projects/',
    replace: ''
  }
]

interface PromptFilterRulesDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  promptFilterRules: any[]
  setField: (key: string, value: any) => void
  onSave?: () => void
}

function PromptFilterRulesDialog({ open, onOpenChange, promptFilterRules, setField, onSave }: PromptFilterRulesDialogProps) {
  const { t } = useApp()
  const rules = promptFilterRules || []

  const handleToggle = (idx: number, checked: boolean) => {
    const updated = [...rules]
    updated[idx] = { ...updated[idx], enabled: checked }
    setField('promptFilterRules', updated)
  }

  const handleDelete = (idx: number) => {
    setField('promptFilterRules', rules.filter((_: any, i: number) => i !== idx))
  }

  const handleAdd = () => {
    const nameEl = document.getElementById('dialog-filter-name') as HTMLInputElement
    const typeEl = document.getElementById('dialog-filter-type') as HTMLInputElement
    const patternEl = document.getElementById('dialog-filter-pattern') as HTMLTextAreaElement
    const replaceEl = document.getElementById('dialog-filter-replace') as HTMLTextAreaElement

    if (!nameEl?.value?.trim() || !patternEl?.value?.trim()) return

    const newRule = {
      id: crypto.randomUUID(),
      name: nameEl.value.trim(),
      enabled: true,
      ruleType: typeEl?.value || 'lines-containing',
      matchPattern: patternEl.value.trim(),
      replace: replaceEl?.value || ''
    }
    setField('promptFilterRules', [...rules, newRule])
    nameEl.value = ''
    patternEl.value = ''
    replaceEl.value = ''
  }

  const handlePreset = () => {
    const existingPatterns = new Set(rules.map((r: any) => r.matchPattern))
    const newRules = PRESET_RULES
      .filter(p => !existingPatterns.has(p.matchPattern))
      .map(p => ({
        id: crypto.randomUUID(),
        name: p.name,
        enabled: true,
        ruleType: p.ruleType,
        matchPattern: p.matchPattern,
        replace: p.replace
      }))
    if (newRules.length > 0) {
      setField('promptFilterRules', [...rules, ...newRules])
    }
  }

  const handleSave = async () => {
    if (onSave) {
      await onSave()
    }
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t('gateway.promptFilterRules', { defaultValue: 'Prompt Filter Rules' })}</DialogTitle>
          <DialogDescription>
            {t('gateway.promptFilterRulesDesc', { defaultValue: 'Configure regular expression or keyword filters to remove noisy content from system prompts.' })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Existing rules */}
          {rules.length > 0 && (
            <div className="space-y-2">
              <Label className="text-sm font-medium">
                {t('gateway.configuredRules', { count: rules.length, defaultValue: `Configured Rules (${rules.length})` })}
              </Label>
              <div className="space-y-2 max-h-64 overflow-y-auto border rounded-lg p-3 bg-muted/20">
                {rules.map((rule: any, idx: number) => (
                  <div key={rule.id || idx} className="flex items-start gap-3 p-3 rounded-lg border bg-background">
                    <Switch
                      checked={rule.enabled}
                      onCheckedChange={(checked: boolean) => handleToggle(idx, checked)}
                      className="mt-1"
                    />
                    <div className="flex-1 min-w-0 space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-sm">{rule.name}</span>
                        <Badge variant="outline" className="text-xs">
                          {rule.ruleType === 'regex'
                            ? t('gateway.regex', { defaultValue: 'Regex' })
                            : t('gateway.containsKeyword', { defaultValue: 'Contains keyword' })}
                        </Badge>
                      </div>
                      <div className="text-xs text-muted-foreground font-mono break-all">
                        {t('gateway.match', { defaultValue: 'Match' })}: {rule.matchPattern}
                      </div>
                      {rule.ruleType === 'regex' && rule.replace && (
                        <div className="text-xs text-muted-foreground font-mono break-all">
                          {t('gateway.replace', { defaultValue: 'Replace' })}: {rule.replace}
                        </div>
                      )}
                    </div>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-8 w-8 p-0 text-destructive hover:text-destructive"
                      onClick={() => handleDelete(idx)}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Add a new rule */}
          <div className="space-y-3 border rounded-lg p-4 bg-muted/10">
            <Label className="text-sm font-medium">{t('gateway.addNewRule', { defaultValue: 'Add New Rule' })}</Label>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label className="text-xs text-muted-foreground">{t('gateway.ruleName', { defaultValue: 'Rule Name' })}</Label>
                <Input id="dialog-filter-name" placeholder={t('gateway.ruleNameExample', { defaultValue: 'Example: filter Git status' })} />
              </div>
              <div className="space-y-1.5">
                <Label className="text-xs text-muted-foreground">{t('gateway.ruleType', { defaultValue: 'Rule Type' })}</Label>
                <Select defaultValue="lines-containing">
                  <SelectTrigger id="dialog-filter-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="lines-containing">{t('gateway.linesContaining', { defaultValue: 'Contains keyword (delete matching lines)' })}</SelectItem>
                    <SelectItem value="regex">{t('gateway.regexReplace', { defaultValue: 'Regular expression (replace matches)' })}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">{t('gateway.matchPattern', { defaultValue: 'Match Pattern' })}</Label>
              <Textarea
                id="dialog-filter-pattern"
                placeholder={t('gateway.matchPatternPlaceholder', { defaultValue: 'Keyword mode: git status\nRegex mode: <fast_mode_info>.*?</fast_mode_info>' })}
                rows={2}
                className="font-mono text-xs"
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">{t('gateway.replacementContent', { defaultValue: 'Replacement Content (regex only, leave empty to delete)' })}</Label>
              <Input
                id="dialog-filter-replace"
                placeholder={t('gateway.leaveEmptyToDelete', { defaultValue: 'Leave empty to delete matches' })}
                className="font-mono text-xs"
              />
            </div>
            <div className="flex gap-2">
              <Button size="sm" onClick={handleAdd} className="flex-1">
                <Plus size={14} className="mr-1" />
                {t('gateway.addRule', { defaultValue: 'Add Rule' })}
              </Button>
              <Button size="sm" variant="outline" onClick={handlePreset}>
                <Filter size={14} className="mr-1" />
                {t('gateway.addPresetRules', { defaultValue: 'Add Preset Rules' })}
              </Button>
            </div>
          </div>

          {/* Footer actions */}
          <div className="flex justify-end gap-2 pt-2 border-t">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t('common.cancel', { defaultValue: 'Cancel' })}
            </Button>
            <Button onClick={handleSave}>
              {t('gateway.saveConfig', { defaultValue: 'Save Config' })}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default PromptFilterRulesDialog
