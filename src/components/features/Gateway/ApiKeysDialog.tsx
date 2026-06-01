import { useState } from 'react'
import { Plus, Dice6, Copy, Trash2, Pencil, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { useApp } from '@/hooks/useApp'

interface ApiKeysDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  clientApiKeysText: string
  setConfig: React.Dispatch<React.SetStateAction<any>>
  onSave?: () => void
}

function ApiKeysDialog({ open, onOpenChange, clientApiKeysText, setConfig, onSave }: ApiKeysDialogProps) {
  const { t } = useApp()
  const [editingIdx, setEditingIdx] = useState<number | null>(null)
  const [editingKey, setEditingKey] = useState('')

  const rawKeys = (clientApiKeysText || '').split(/[\n,]+/).map(k => k.trim()).filter(Boolean)

  const keys = rawKeys.map(rawKey => {
    const isDisabled = rawKey.startsWith('#disabled#')
    const rest = isDisabled ? rawKey.substring(10) : rawKey
    const colonIdx = rest.indexOf(':')
    const hasName = colonIdx > 0 && !rest.startsWith('sk-') && !rest.startsWith('PROXY_KEY:')
    const name = hasName ? rest.substring(0, colonIdx) : ''
    const key = hasName ? rest.substring(colonIdx + 1) : rest
    return { name, key, enabled: !isDisabled }
  })

  const updateKeys = (newKeys: typeof keys) => {
    const newRawKeys = newKeys.map(k => {
      const prefix = k.enabled ? '' : '#disabled#'
      const namePrefix = k.name ? `${k.name}:` : ''
      return `${prefix}${namePrefix}${k.key}`
    })
    setConfig((prev: any) => ({
      ...prev,
      clientApiKeysText: newRawKeys.join('\n'),
      apiKey: newKeys.find(k => k.enabled)?.key || newKeys[0]?.key || ''
    }))
  }

  const generateKey = () => {
    const random = crypto?.randomUUID?.().replace(/-/g, '') || `${Date.now()}${Math.random().toString(36).slice(2)}`
    const newKey = `sk-${random}`
    setConfig((prev: any) => ({
      ...prev,
      clientApiKeysText: prev.clientApiKeysText ? `${prev.clientApiKeysText}\n${newKey}` : newKey,
      apiKey: newKey
    }))
  }

  const addKey = () => {
    const newKey = `sk-${Date.now()}`
    setConfig((prev: any) => ({
      ...prev,
      clientApiKeysText: prev.clientApiKeysText ? `${prev.clientApiKeysText}\n${newKey}` : newKey
    }))
  }

  const startEdit = (idx: number) => {
    setEditingIdx(idx)
    setEditingKey(keys[idx].key)
  }

  const confirmEdit = () => {
    if (editingIdx === null || !editingKey.trim()) return
    const newKeys = [...keys]
    newKeys[editingIdx] = { ...newKeys[editingIdx], key: editingKey.trim() }
    updateKeys(newKeys)
    setEditingIdx(null)
    setEditingKey('')
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) { setEditingIdx(null); onSave?.() } }}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{t('gateway.clientApiKeys', { defaultValue: 'Client API Keys' })}</DialogTitle>
          <DialogDescription>
            {t('gateway.clientAuthDescription', { defaultValue: 'Manage client authentication keys. Disabled keys will not be used.' })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3 mt-2">
          <div className="flex gap-2 justify-end">
            <Button size="sm" variant="outline" onClick={generateKey} className="h-7 text-xs gap-1">
              <Dice6 size={12} /> {t('gateway.generate', { defaultValue: 'Generate' })}
            </Button>
            <Button size="sm" variant="outline" onClick={addKey} className="h-7 text-xs gap-1">
              <Plus size={12} /> {t('gateway.add', { defaultValue: 'Add' })}
            </Button>
          </div>

          <div className="border rounded-lg overflow-hidden max-h-[350px] overflow-y-auto">
            {keys.length === 0 ? (
              <div className="p-8 text-center text-sm text-muted-foreground">
                {t('gateway.noApiKeys', { defaultValue: 'No API Keys yet, click Generate to create one' })}
              </div>
            ) : (
              keys.map((item, idx) => (
                <div key={idx} className={`flex items-center gap-2 p-2.5 border-b last:border-b-0 hover:bg-muted/30 ${!item.enabled ? 'opacity-50' : ''}`}>
                  <Switch
                    size="sm"
                    checked={item.enabled}
                    onCheckedChange={(checked) => {
                      const newKeys = [...keys]
                      newKeys[idx] = { ...newKeys[idx], enabled: checked }
                      updateKeys(newKeys)
                    }}
                  />
                  <Input
                    value={item.name}
                    onChange={(e) => {
                      const newKeys = [...keys]
                      newKeys[idx] = { ...newKeys[idx], name: e.target.value }
                      updateKeys(newKeys)
                    }}
                    className="h-7 text-xs w-[70px]"
                    placeholder={t('gateway.keyNamePlaceholder', { defaultValue: 'Name' })}
                  />
                  {editingIdx === idx ? (
                    <>
                      <Input
                        value={editingKey}
                        onChange={(e) => setEditingKey(e.target.value)}
                        className="h-7 text-xs flex-1 font-mono"
                        autoFocus
                        onKeyDown={(e) => { if (e.key === 'Enter') confirmEdit() }}
                      />
                      <Button size="sm" variant="ghost" className="h-6 w-6 p-0 text-green-600" onClick={confirmEdit}>
                        <Check size={12} />
                      </Button>
                    </>
                  ) : (
                    <>
                      <code className="flex-1 text-xs font-mono bg-muted/50 px-2 py-1 rounded truncate">
                        {item.key.length > 24 ? `${item.key.substring(0, 10)}...${item.key.slice(-4)}` : item.key}
                      </code>
                      <Button size="sm" variant="ghost" className="h-6 w-6 p-0" onClick={() => startEdit(idx)}>
                        <Pencil size={12} className="text-muted-foreground" />
                      </Button>
                      <Button size="sm" variant="ghost" className="h-6 w-6 p-0" onClick={() => navigator.clipboard.writeText(item.key)}>
                        <Copy size={12} className="text-muted-foreground" />
                      </Button>
                    </>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 w-6 p-0 text-red-500 hover:text-red-600"
                    onClick={() => updateKeys(keys.filter((_, i) => i !== idx))}
                  >
                    <Trash2 size={12} />
                  </Button>
                </div>
              ))
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default ApiKeysDialog
