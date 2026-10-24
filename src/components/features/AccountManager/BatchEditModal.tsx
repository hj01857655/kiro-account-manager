import { useState, useEffect, useRef, useMemo } from 'react'
import { X, Tag, Plus, Folder, Check, Edit, Power } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { getTags, getGroups, setAccountTags, setAccountGroup, addTag, addGroup } from '../../../api/groupTag'
import { updateAccount } from '../../../api/accountApi'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter} from '../../shared/dialog'
import { Button } from '../../shared/button'
import { getThemeAccent } from '../AgentConfig/themeAccent'
import { Account, TagDefinition, GroupDefinition } from '../../../types/account'
import React from 'react'

const PRESET_COLORS = [
  '#8b5cf6', '#3b82f6', '#10b981', '#f59e0b', 
  '#ef4444', '#ec4899', '#06b6d4', '#84cc16'
]

interface BatchEditModalProps {
  accountIds: string[];
  accounts?: Account[];
  onClose: () => void;
  onSuccess?: (data: { accountIds: string[]; selectedTagIds: string[]; selectedGroupId: string | null; enabledChange: boolean | null }) => void;
}

function BatchEditModal({ accountIds, accounts = [], onClose, onSuccess }: BatchEditModalProps) {
  const { t, theme } = useApp()
  const { showError } = useDialog()

  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])
  
  const [tags, setTags] = useState<TagDefinition[]>([])
  const [groups, setGroups] = useState<GroupDefinition[]>([])
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([])
  const [selectedGroupId, setSelectedGroupId] = useState<string>('')
  const [enabledAction, setEnabledAction] = useState<'keep' | 'enable' | 'disable'>('keep')
  const [newTagName, setNewTagName] = useState('')
  const [newGroupName, setNewGroupName] = useState('')
  const [loading, setLoading] = useState(false)
  const [showTagDropdown, setShowTagDropdown] = useState(false)
  const [showNewGroupInput, setShowNewGroupInput] = useState(false)
  const tagInputContainerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    Promise.all([getTags(), getGroups()])
      .then(([tagsData, groupsData]) => {
        setTags(tagsData as TagDefinition[])
        setGroups(groupsData as GroupDefinition[])
      })
      .catch(() => {})
  }, [])

  // 点击外部关闭标签下拉
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (tagInputContainerRef.current && !tagInputContainerRef.current.contains(e.target as Node)) {
        setShowTagDropdown(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // 计算选中账号的共同标签和分组
  useEffect(() => {
    if (accounts.length === 0 || accountIds.length === 0) return
    
    const selectedAccounts = accounts.filter(a => accountIds.includes(a.id))
    if (selectedAccounts.length === 0) return
    
    // 标签交集
    const firstTags = new Set((selectedAccounts[0]?.tagLinks || []).map(link => link.tagId))
    const commonTags = selectedAccounts.slice(1).reduce((common, account) => {
      const accountTags = new Set((account.tagLinks || []).map(link => link.tagId))
      return new Set([...common].filter(t => accountTags.has(t)))
    }, firstTags)
    setSelectedTagIds([...commonTags])
    
    // 分组（如果所有账号都有相同分组）
    const firstGroupId = selectedAccounts[0]?.groupId || ''
    const allSameGroup = selectedAccounts.every(a => (a.groupId || '') === firstGroupId)
    if (allSameGroup) {
      setSelectedGroupId(firstGroupId)
    }
  }, [accounts, accountIds])

  const handleToggleTag = (tagId: string) => {
    setSelectedTagIds(prev => 
      prev.includes(tagId) ? prev.filter(id => id !== tagId) : [...prev, tagId]
    )
  }

  const handleAddTag = async () => {
    const trimmed = newTagName.trim()
    if (!trimmed) return
    const safeName = trimmed.slice(0, 20)
    
    const existing = tags.find(t => t.name === safeName)
    if (existing) {
      if (!selectedTagIds.includes(existing.id)) {
        setSelectedTagIds([...selectedTagIds, existing.id])
      }
      setNewTagName('')
      return
    }
    
    const color = PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)]
    try {
      const newTag = await addTag(safeName, color) as TagDefinition
      setTags([...tags, newTag])
      setSelectedTagIds([...selectedTagIds, newTag.id])
      setNewTagName('')
    } catch (e) {
      await showError(t('common.error'), String(e))
    }
  }

  const handleAddGroup = async () => {
    const trimmed = newGroupName.trim()
    if (!trimmed) return
    const safeName = trimmed.slice(0, 20)
    
    const existing = groups.find(g => g.name === safeName)
    if (existing) {
      setSelectedGroupId(existing.id)
      setNewGroupName('')
      setShowNewGroupInput(false)
      return
    }
    
    const color = PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)]
    try {
      const newGroup = await addGroup(safeName, color) as GroupDefinition
      setGroups([...groups, newGroup])
      setSelectedGroupId(newGroup.id)
      setNewGroupName('')
      setShowNewGroupInput(false)
    } catch (e) {
      await showError(t('common.error'), String(e))
    }
  }

  const handleSubmit = async () => {
    setLoading(true)
    const enabledChange = enabledAction === 'keep' ? null : enabledAction === 'enable'
    try {
      // 批量设置标签和分组
      await Promise.all([
        ...accountIds.map(id => setAccountTags(id, selectedTagIds)),
        ...accountIds.map(id => setAccountGroup(id, selectedGroupId || null)),
        ...(enabledChange !== null
          ? accountIds.map(id => updateAccount({ id, enabled: enabledChange }))
          : [])
      ])
      onSuccess?.({ accountIds, selectedTagIds, selectedGroupId: selectedGroupId || null, enabledChange })
      onClose()
    } catch (e) {
      await showError(t('common.error'), String(e))
    } finally {
      setLoading(false)
    }
  }

  const availableTags = tags.filter(t => !selectedTagIds.includes(t.id))
  const filteredTags = newTagName.trim()
    ? availableTags.filter(t => t.name.toLowerCase().includes(newTagName.toLowerCase()))
    : availableTags

  const selectedGroup = groups.find(g => g.id === selectedGroupId)

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="560px">
        <DialogHeader icon={Edit} iconColor={accent.text} iconBg={accent.iconBadgeBg}>
          <DialogTitle>批量编辑</DialogTitle>
          <DialogDescription>{accountIds.length} 个账号</DialogDescription>
        </DialogHeader>

        <DialogBody gap="lg">
          {/* 账号状态设置 */}
          <div>
            <label className={`block text-sm font-semibold text-foreground mb-3 flex items-center gap-2`}>
              <Power size={16} className={accent.text} />
              账号状态
            </label>
            <div className="flex gap-2">
              {([
                { key: 'keep', label: '保持不变' },
                { key: 'enable', label: '启用' },
                { key: 'disable', label: '禁用' },
              ] as const).map(opt => (
                <button
                  key={opt.key}
                  type="button"
                  onClick={() => setEnabledAction(opt.key)}
                  className={`flex-1 px-4 py-2.5 rounded-xl border-2 text-sm transition-all cursor-pointer ${
                    enabledAction === opt.key
                      ? `${accent.solidBg} text-white border-transparent`
                      : 'bg-background border-input text-foreground hover:bg-muted/50'
                  }`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>

          {/* 分组设置 */}
          <div>
            <label className={`block text-sm font-semibold text-foreground mb-3 flex items-center gap-2`}>
              <Folder size={16} className={accent.text} />
              分组
            </label>
            
            {showNewGroupInput ? (
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      handleAddGroup()
                    } else if (e.key === 'Escape') {
                      setShowNewGroupInput(false)
                      setNewGroupName('')
                    }
                  }}
                  placeholder="输入新分组名..."
                  autoFocus
                  className={`flex-1 px-4 py-2.5 border-2 rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 transition-all outline-none`}
                />
                <button 
                  type="button" 
                  onClick={handleAddGroup} 
                  disabled={!newGroupName.trim()}
                  className={`px-4 py-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} disabled:opacity-50 transition-all cursor-pointer`}
                >
                  <Check size={18} />
                </button>
                <button 
                  type="button" 
                  onClick={() => { setShowNewGroupInput(false); setNewGroupName('') }}
                  className={`px-4 py-2.5 rounded-xl hover:bg-muted/50 transition-all cursor-pointer`}
                >
                  <X size={18} />
                </button>
              </div>
            ) : (
              <div className="flex gap-2">
                <select
                  value={selectedGroupId}
                  onChange={(e) => setSelectedGroupId(e.target.value)}
                  className={`flex-1 px-4 py-2.5 border-2 rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 transition-all outline-none cursor-pointer`}
                >
                  <option value="">无分组</option>
                  {groups.map(g => (
                    <option key={g.id} value={g.id}>{g.name}</option>
                  ))}
                </select>
                <button 
                  type="button" 
                  onClick={() => setShowNewGroupInput(true)}
                  className={`px-4 py-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} transition-all cursor-pointer`}
                  title="创建新分组"
                >
                  <Plus size={18} />
                </button>
              </div>
            )}
            
            {selectedGroupId && selectedGroup && (
              <div className="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
                <div 
                  className="w-3 h-3 rounded-full" 
                  style={{ backgroundColor: selectedGroup.color || '#8b5cf6' }}
                />
                已选择：{selectedGroup.name}
              </div>
            )}
          </div>

          {/* 标签设置 */}
          <div>
            <label className={`block text-sm font-semibold text-foreground mb-3 flex items-center gap-2`}>
              <Tag size={16} className={accent.text} />
              标签
            </label>
            
            {/* 已选标签 */}
            <div className="flex flex-wrap gap-2 min-h-[36px] mb-3">
              {selectedTagIds.length === 0 ? (
                <span className={`text-sm text-muted-foreground`}>未选择标签</span>
              ) : (
                selectedTagIds.map(tagId => {
                  const tag = tags.find(t => t.id === tagId)
                  if (!tag) return null
                  return (
                    <span key={tagId} className="inline-flex items-center gap-1 text-xs px-2.5 py-1.5 rounded-full text-white cursor-pointer hover:opacity-80"
                      style={{ backgroundColor: tag.color || '#8b5cf6' }}
                      onClick={() => handleToggleTag(tagId)}
                      title="点击移除"
                    >
                      {tag.name}
                      <X size={12} />
                    </span>
                  )
                })
              )}
            </div>

            {/* 搜索/添加标签 */}
            <div className="flex gap-2">
              <div className="flex-1 relative" ref={tagInputContainerRef}>
                <input
                  type="text"
                  value={newTagName}
                  onChange={(e) => setNewTagName(e.target.value)}
                  onFocus={() => setShowTagDropdown(true)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      if (newTagName.trim()) {
                        handleAddTag()
                      }
                    }
                  }}
                  placeholder="搜索或输入新标签..."
                  className={`w-full px-4 py-2.5 border-2 rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 transition-all outline-none`}
                />
                {/* 搜索建议下拉 */}
                {showTagDropdown && availableTags.length > 0 && (
                  <div className={`absolute top-full left-0 right-0 mt-1 glass-card border border-border rounded-xl shadow-lg z-10 max-h-48 overflow-y-auto`}>
                    {filteredTags.map(tag => (
                      <button key={tag.id} onClick={() => { setSelectedTagIds([...selectedTagIds, tag.id]); setNewTagName(''); setShowTagDropdown(false) }}
                        className={`w-full px-3 py-2 text-left text-sm text-foreground hover:opacity-80 flex items-center gap-2 transition-colors hover:bg-muted/50 rounded-lg cursor-pointer`}
                      >
                        <span className="w-3 h-3 rounded-full" style={{ backgroundColor: tag.color }} />
                        {tag.name}
                      </button>
                    ))}
                    {filteredTags.length === 0 && newTagName.trim() && (
                      <div className={`px-3 py-2 text-sm text-muted-foreground`}>
                        按回车创建 "{newTagName.trim()}"
                      </div>
                    )}
                  </div>
                )}
              </div>
              <button 
                type="button" 
                onClick={handleAddTag} 
                disabled={!newTagName.trim()}
                className={`px-4 py-2.5 ${accent.solidBg} text-white rounded-xl text-sm ${accent.solidHoverBg} disabled:opacity-50 transition-all cursor-pointer`}
                title="添加标签"
              >
                <Plus size={18} />
              </button>
            </div>
            <p className={`text-xs text-muted-foreground mt-2 leading-relaxed`}>
              输入搜索已有标签，或直接输入创建新标签
            </p>
          </div>
        </DialogBody>

        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={loading}
            loading={loading}
          >
            {loading ? t('common.saving') : t('common.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}

export default BatchEditModal
