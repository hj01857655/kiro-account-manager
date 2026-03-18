import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { CopyButton } from '@mantine/core'
import { Copy, Check, Folder, Plus, X } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { setAccountTags, setAccountGroup, getGroups, addGroup } from '../../../api/groupTag'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { TagSelector } from './GroupTagManager'
import { TokenJsonView } from './TokenJsonView'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter,
} from '../../ui/dialog'
import { Button } from '../../ui/button'

const PRESET_COLORS = [
  '#3b82f6', '#10b981', '#f59e0b', '#ef4444', 
  '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'
]

function GroupSelector({ groups, value, onChange, onGroupsChange }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const [newGroupName, setNewGroupName] = useState('')
  const [showInput, setShowInput] = useState(false)

  const handleAddGroup = async () => {
    const trimmed = newGroupName.trim().slice(0, 20)
    if (!trimmed) return
    if (groups.some(g => g.name === trimmed)) {
      setNewGroupName('')
      return
    }
    const color = PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)]
    try {
      const newGroup = await addGroup(trimmed, color)
      onGroupsChange([...groups, newGroup])
      onChange(newGroup.id)
      setNewGroupName('')
      setShowInput(false)
    } catch (e) {
      console.error('创建分组失败:', e)
    }
  }

  if (showInput) {
    return (
      <div className="flex gap-2">
        <input
          type="text"
          value={newGroupName}
          onChange={(e) => setNewGroupName(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleAddGroup()}
          placeholder={t('groups.newGroupPlaceholder') || '输入新分组名...'}
          className={`flex-1 px-4 py-2.5 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2`}
        />
        <button
          onClick={handleAddGroup}
          disabled={!newGroupName.trim()}
          className={`p-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} disabled:opacity-50`}
        >
          <Check size={16} />
        </button>
        <button
          onClick={() => { setShowInput(false); setNewGroupName('') }}
          className={`p-2.5 rounded-xl ${colors.cardHover}`}
        >
          <X size={16} />
        </button>
      </div>
    )
  }

  return (
    <div className="flex gap-2">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`flex-1 px-4 py-2.5 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2`}
      >
        <option value="">{t('groups.noGroup') || '无分组'}</option>
        {groups.map(g => (
          <option key={g.id} value={g.id}>{g.name}</option>
        ))}
      </select>
      <button
        onClick={() => setShowInput(true)}
        className={`p-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg}`}
      >
        <Plus size={16} />
      </button>
    </div>
  )
}

function EditAccountModal({ account, onClose, onSuccess }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const { showError } = useDialog()
  
  const [form, setForm] = useState({
    label: account.label || '',
    accessToken: account.accessToken || '',
    refreshToken: account.refreshToken || '',
    clientId: account.clientId || '',
    clientSecret: account.clientSecret || '',
    machineId: account.machineId || '',
  })
  const [selectedTagIds, setSelectedTagIds] = useState((account.tagLinks || []).map(link => link.tagId))
  const [selectedGroupId, setSelectedGroupId] = useState(account.groupId || '')
  const [groups, setGroups] = useState([])
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    getGroups().then(setGroups).catch(() => {})
  }, [])

  const handleSave = async () => {
    setSaving(true)
    try {
      const params = {
        id: account.id,
        label: form.label || null,
        accessToken: form.accessToken || null,
        refreshToken: form.refreshToken || null,
        machineId: form.machineId || null,
      }
      if (account.provider === 'BuilderId') {
        params.clientId = form.clientId || null
        params.clientSecret = form.clientSecret || null
      }
      await invoke('update_account', params)
      await setAccountGroup(account.id, selectedGroupId || null)
      await setAccountTags(account.id, selectedTagIds)
      onSuccess?.()
      onClose()
    } catch (e) {
      await showError(t('editAccount.saveFailed'), e.toString())
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="480px">
        <DialogHeader icon={Folder} iconColor={accent.text} iconBg={accent.iconBadgeBg}>
          <DialogTitle>{t('editAccount.title')}</DialogTitle>
          <DialogDescription>{getAccountDisplayName(account)}</DialogDescription>
        </DialogHeader>

        <DialogBody>
          <div>
            <label className={`block text-sm font-medium ${colors.text} mb-2`}>
              {t('accounts.remark')}
            </label>
            <input
              type="text"
              placeholder={t('editAccount.labelPlaceholder')}
              value={form.label}
              onChange={(e) => setForm({ ...form, label: e.target.value })}
              className={`w-full px-4 py-3 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2`}
            />
          </div>

          <div>
            <label className={`block text-sm font-medium ${colors.text} mb-2`}>
              {t('addAccount.machineId')}
            </label>
            <div className="relative">
              <input
                type="text"
                placeholder={t('addAccount.machineIdPlaceholder')}
                value={form.machineId}
                onChange={(e) => setForm({ ...form, machineId: e.target.value })}
                className={`w-full px-4 py-3 pr-10 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2`}
              />
              <CopyButton value={form.machineId}>
                {({ copied, copy }) => (
                  <button
                    onClick={copy}
                    className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg ${colors.cardHover}`}
                    title={copied ? '已复制' : '复制'}
                  >
                    {copied ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={colors.textMuted} />}
                  </button>
                )}
              </CopyButton>
            </div>
          </div>

          {account.provider === 'BuilderId' && (
            <>
              <div>
                <label className={`block text-sm font-medium ${colors.text} mb-2`}>
                  Client ID
                </label>
                <div className="relative">
                  <input
                    type="text"
                    placeholder="刷新 Token 需要"
                    value={form.clientId}
                    onChange={(e) => setForm({ ...form, clientId: e.target.value })}
                    className={`w-full px-4 py-3 pr-10 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2`}
                  />
                  <CopyButton value={form.clientId}>
                    {({ copied, copy }) => (
                      <button
                        onClick={copy}
                        className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg ${colors.cardHover}`}
                        title={copied ? '已复制' : '复制'}
                      >
                        {copied ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={colors.textMuted} />}
                      </button>
                    )}
                  </CopyButton>
                </div>
              </div>
              <div>
                <label className={`block text-sm font-medium ${colors.text} mb-2`}>
                  Client Secret
                </label>
                <div className="relative">
                  <textarea
                    placeholder="刷新 Token 需要"
                    value={form.clientSecret}
                    onChange={(e) => setForm({ ...form, clientSecret: e.target.value })}
                    rows={2}
                    className={`w-full px-4 py-3 pr-10 border rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2 resize-none`}
                  />
                  <CopyButton value={form.clientSecret}>
                    {({ copied, copy }) => (
                      <button
                        onClick={copy}
                        className={`absolute right-3 top-3 p-1.5 rounded-lg ${colors.cardHover}`}
                        title={copied ? '已复制' : '复制'}
                      >
                        {copied ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={colors.textMuted} />}
                      </button>
                    )}
                  </CopyButton>
                </div>
              </div>
            </>
          )}

          <div>
            <div className={`text-sm font-medium mb-2 flex items-center gap-1.5 ${colors.text}`}>
              <Folder size={14} />
              {t('groups.title') || '分组'}
            </div>
            <GroupSelector
              groups={groups}
              value={selectedGroupId}
              onChange={setSelectedGroupId}
              onGroupsChange={setGroups}
            />
          </div>

          <TagSelector 
            selectedTagIds={selectedTagIds} 
            onChange={setSelectedTagIds} 
          />

          <TokenJsonView account={account} defaultExpanded={false} />
        </DialogBody>

        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="success"
            onClick={handleSave}
            disabled={saving}
            loading={saving}
          >
            {t('common.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}

export default EditAccountModal
