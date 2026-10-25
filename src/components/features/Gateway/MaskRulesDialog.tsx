import React, { useState, useEffect } from 'react'
import { Plus, Trash2, ShieldAlert, Regex } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter
} from '@/components/shared/dialog'
import { toast } from 'sonner'
import { useApp } from '../../../hooks/useApp'
import { MASK_KIND_LABELS, MaskCustomRule, MaskKindOverride, MaskNeverMask } from './gatewayPageState'

type Action = 'mask' | 'warn' | 'block'

const ACTION_LABEL_KEYS: Record<Action, string> = {
  mask: 'gatewayMaskRules.actionMask',
  warn: 'gatewayMaskRules.actionWarn',
  block: 'gatewayMaskRules.actionBlock',
}

/** 动作三选一的下拉（三项，Select 比三个 radio 紧凑） */
function ActionSelect({ value, onChange }: { value: Action; onChange: (a: Action) => void }) {
  const { t } = useApp()
  return (
    <Select value={value} onValueChange={(v) => onChange(v as Action)}>
      <SelectTrigger className="h-7 w-[92px] text-[11px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {(['mask', 'warn', 'block'] as Action[]).map(a => (
          <SelectItem key={a} value={a} className="text-[11px]">
            {t(ACTION_LABEL_KEYS[a])}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

/**
 * 遮蔽规则编辑器：三类配置一块管。
 *
 * 1. **类别覆盖**——内置七类各一行：开关（停用该类）+ 动作三选一
 * 2. **自定义规则**——自己的正则：名字 + 模式 + 动作 + 开关
 * 3. **永不遮蔽**——白名单：值 + 备注（精确匹配优先于一切规则）
 *
 * 保存走网关配置的常规保存链路（setField + 防抖自动保存），
 * 规则在网关下次启动时装配进引擎。
 */
export function MaskRulesDialog({
  open,
  onOpenChange,
  kindOverrides,
  customRules,
  neverMask,
  onChange
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  kindOverrides: MaskKindOverride[]
  customRules: MaskCustomRule[]
  neverMask: MaskNeverMask[]
  onChange: (next: {
    maskKindOverrides: MaskKindOverride[]
    maskCustomRules: MaskCustomRule[]
    maskNeverMask: MaskNeverMask[]
  }) => void
}) {
  const { t } = useApp()
  const [localKinds, setLocalKinds] = useState<MaskKindOverride[]>([])
  const [localCustom, setLocalCustom] = useState<MaskCustomRule[]>([])
  const [localNever, setLocalNever] = useState<MaskNeverMask[]>([])

  // 打开时从仓库配置初始化本地副本（取消即丢弃）
  useEffect(() => {
    if (!open) return
    setLocalKinds(kindOverrides)
    setLocalCustom(customRules)
    setLocalNever(neverMask)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open])

  const overrideFor = (kind: string): MaskKindOverride =>
    localKinds.find(o => o.kind === kind) ?? { kind, enabled: true, action: 'mask' }

  const setOverride = (next: MaskKindOverride) => {
    setLocalKinds(prev => {
      const idx = prev.findIndex(o => o.kind === next.kind)
      if (idx >= 0) {
        const copy = [...prev]
        copy[idx] = next
        return copy
      }
      return [...prev, next]
    })
  }

  const handleSave = () => {
    // 自定义规则的 id 去重校验：空 id 或重复 id 会让后端覆盖语义错乱
    const ids = new Set<string>()
    for (const r of localCustom) {
      const id = r.id.trim()
      if (!id) {
        toast.error(t('gatewayMaskRules.idRequired'))
        return
      }
      if (ids.has(id)) {
        toast.error(t('gatewayMaskRules.idDuplicate', { id }))
        return
      }
      ids.add(id)
    }
    // 正则先在前端试编译，坏模式直接拦住（后端虽会跳过，但不Save更好）
    for (const r of localCustom) {
      try {
        new RegExp(r.pattern)
      } catch (e) {
        toast.error(t('gatewayMaskRules.badPattern', { name: r.name, error: String(e) }))
        return
      }
    }
    onChange({
      maskKindOverrides: localKinds,
      maskCustomRules: localCustom.map(r => ({ ...r, id: r.id.trim() })),
      maskNeverMask: localNever.filter(n => n.value.trim().length > 0)
    })
    onOpenChange(false)
    toast.success(t('gatewayMaskRules.saved'))
  }

  return (
    <DialogRoot open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldAlert size={16} className="text-primary" />
            {t('gatewayMaskRules.title')}
          </DialogTitle>
          <DialogDescription>{t('gatewayMaskRules.desc')}</DialogDescription>
        </DialogHeader>

        <DialogBody className="space-y-5">
          {/* 1. 类别覆盖 */}
          <section className="space-y-2">
            <h4 className="text-xs font-semibold">{t('gatewayMaskRules.kindsTitle')}</h4>
            <div className="rounded-lg border divide-y divide-border/40">
              {MASK_KIND_LABELS.map(({ kind, labelKey }) => {
                const o = overrideFor(kind)
                return (
                  <div key={kind} className="flex items-center gap-3 px-3 py-2">
                    <Switch
                      size="sm"
                      checked={o.enabled}
                      onCheckedChange={(checked) => setOverride({ ...o, enabled: checked })}
                    />
                    <span className="flex-1 text-xs">{t(labelKey)}</span>
                    <ActionSelect
                      value={o.action}
                      onChange={(action) => setOverride({ ...o, action })}
                    />
                  </div>
                )
              })}
            </div>
            <p className="text-[10px] text-muted-foreground">{t('gatewayMaskRules.kindsHint')}</p>
          </section>

          {/* 2. 自定义规则 */}
          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold">{t('gatewayMaskRules.customTitle')}</h4>
              <Button
                variant="outline"
                size="sm"
                className="h-6 gap-1 text-[10px]"
                onClick={() =>
                  setLocalCustom(prev => [
                    ...prev,
                    {
                      id: `rule-${Date.now().toString(36)}`,
                      name: '',
                      pattern: '',
                      action: 'mask',
                      enabled: true
                    }
                  ])
                }
              >
                <Plus size={10} />
                {t('gatewayMaskRules.addRule')}
              </Button>
            </div>
            {localCustom.length === 0 ? (
              <p className="rounded-lg border px-3 py-2 text-[10px] text-muted-foreground">
                {t('gatewayMaskRules.noCustom')}
              </p>
            ) : (
              <div className="space-y-2">
                {localCustom.map((rule, idx) => (
                  <div key={idx} className="space-y-1.5 rounded-lg border p-2.5">
                    <div className="flex items-center gap-2">
                      <Switch
                        size="sm"
                        checked={rule.enabled}
                        onCheckedChange={(checked) => {
                          const copy = [...localCustom]
                          copy[idx] = { ...rule, enabled: checked }
                          setLocalCustom(copy)
                        }}
                      />
                      <Input
                        value={rule.name}
                        onChange={(e) => {
                          const copy = [...localCustom]
                          copy[idx] = { ...rule, name: e.target.value }
                          setLocalCustom(copy)
                        }}
                        placeholder={t('gatewayMaskRules.ruleName')}
                        className="h-7 flex-1 text-[11px]"
                      />
                      <ActionSelect
                        value={rule.action}
                        onChange={(action) => {
                          const copy = [...localCustom]
                          copy[idx] = { ...rule, action }
                          setLocalCustom(copy)
                        }}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
                        onClick={() => setLocalCustom(prev => prev.filter((_, i) => i !== idx))}
                      >
                        <Trash2 size={11} />
                      </Button>
                    </div>
                    <div className="flex items-center gap-2">
                      <Regex size={11} className="shrink-0 text-muted-foreground" />
                      <Input
                        value={rule.pattern}
                        onChange={(e) => {
                          const copy = [...localCustom]
                          copy[idx] = { ...rule, pattern: e.target.value }
                          setLocalCustom(copy)
                        }}
                        placeholder={t('gatewayMaskRules.patternPlaceholder')}
                        className="h-7 flex-1 font-mono text-[11px]"
                      />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>

          {/* 3. 永不遮蔽 */}
          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold">{t('gatewayMaskRules.neverTitle')}</h4>
              <Button
                variant="outline"
                size="sm"
                className="h-6 gap-1 text-[10px]"
                onClick={() => setLocalNever(prev => [...prev, { value: '', note: '' }])}
              >
                <Plus size={10} />
                {t('gatewayMaskRules.addNever')}
              </Button>
            </div>
            <p className="text-[10px] text-muted-foreground">{t('gatewayMaskRules.neverHint')}</p>
            {localNever.length > 0 && (
              <div className="space-y-2">
                {localNever.map((entry, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <Textarea
                      value={entry.value}
                      onChange={(e) => {
                        const copy = [...localNever]
                        copy[idx] = { ...entry, value: e.target.value }
                        setLocalNever(copy)
                      }}
                      placeholder={t('gatewayMaskRules.neverValue')}
                      rows={1}
                      className="h-7 min-h-7 flex-1 font-mono text-[11px]"
                    />
                    <Input
                      value={entry.note}
                      onChange={(e) => {
                        const copy = [...localNever]
                        copy[idx] = { ...entry, note: e.target.value }
                        setLocalNever(copy)
                      }}
                      placeholder={t('gatewayMaskRules.neverNote')}
                      className="h-7 w-40 text-[11px]"
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 shrink-0 p-0 text-muted-foreground hover:text-destructive"
                      onClick={() => setLocalNever(prev => prev.filter((_, i) => i !== idx))}
                    >
                      <Trash2 size={11} />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </section>
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button size="sm" onClick={handleSave}>
            {t('common.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}
