import { useState, useEffect, useRef } from 'react'
import { X, Tag, Plus } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { getTags, setAccountTags } from '../../../api/groupTag'
import { invoke } from '@tauri-apps/api/core'
import { getThemeAccent, getGradientAccentButton } from '../KiroConfig/themeAccent'
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
  '#8b5cf6', '#3b82f6', '#10b981', '#f59e0b', 
  '#ef4444', '#ec4899', '#06b6d4', '#84cc16'
]

function BatchTagModal({ accountIds, accounts = [], onClose, onSuccess }) {
  const { t, colors, theme } = useApp()
  const accent = getThemeAccent(theme)
  const accentGradientButtonClass = getGradientAccentButton(accent)
  const { showError } = useDialog()
  
  const [tags, setTags] = useState([])
  const [selectedTagIds, setSelectedTagIds] = useState([])
  const [newTagName, setNewTagName] = useState('')
  const [loading, setLoading] = useState(false)
  const [showDropdown, setShowDropdown] = useState(false)
  const inputContainerRef = useRef(null)

  useEffect(() => {
    getTags().then(setTags).catch(() => {})
  }, [])

  // 点击外部关闭下拉
  useEffect(() => {
    const handleClickOutside = (e) => {
      if (inputContainerRef.current && !inputContainerRef.current.contains(e.target)) {
        setShowDropdown(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // 计算选中账号的共同标签（交集）
  useEffect(() => {
    if (accounts.length === 0 || accountIds.length === 0) return
    
    const selectedAccounts = accounts.filter(a => accountIds.includes(a.id))
    if (selectedAccounts.length === 0) return
    
    // 获取所有选中账号的标签交集（从 tagLinks 中提取 tagId）
    const firstTags = new Set((selectedAccounts[0]?.tagLinks || []).map(link => link.tagId))
    const commonTags = selectedAccounts.slice(1).reduce((common, account) => {
      const accountTags = new Set((account.tagLinks || []).map(link => link.tagId))
      return new Set([...common].filter(t => accountTags.has(t)))
    }, firstTags)
    
    setSelectedTagIds([...commonTags])
  }, [accounts, accountIds])

  const handleToggleTag = (tagId) => {
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
      const newTag = await invoke('add_tag', { name: safeName, color })
      setTags([...tags, newTag])
      setSelectedTagIds([...selectedTagIds, newTag.id])
      setNewTagName('')
    } catch (e) {
      await showError(t('common.error'), e.toString())
    }
  }

  const handleSubmit = async () => {
    setLoading(true)
    try {
      await Promise.all(accountIds.map(id => setAccountTags(id, selectedTagIds)))
      onSuccess?.({ accountIds, selectedTagIds })

    } catch (e) {
      await showError(t('common.error'), e.toString())
    } finally {
      setLoading(false)
    }
  }

  const availableTags = tags.filter(t => !selectedTagIds.includes(t.id))
  
  // 过滤：有输入时过滤，没输入时显示全部未选中的
  const filteredTags = newTagName.trim()
    ? availableTags.filter(t => t.name.toLowerCase().includes(newTagName.toLowerCase()))
    : availableTags

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="480px">
        <DialogHeader icon={Tag} iconColor={accent.text} iconBg={accent.iconBadgeBg}>
          <DialogTitle>{t('tags.batchSet')}</DialogTitle>
          <DialogDescription>{accountIds.length} 个账号</DialogDescription>
        </DialogHeader>

        <DialogBody gap="md">
          {/* 已选标签 - 点击 ❌ 取消 */}
          <div>
            <label className={`block text-sm font-semibold ${colors.text} mb-3`}>{t('tags.selected')}</label>
            <div className="flex flex-wrap gap-2 min-h-[36px]">
              {selectedTagIds.length === 0 ? (
                <span className={`text-sm ${colors.text} opacity-60`}>{t('tags.noTags')}</span>
              ) : (
                selectedTagIds.map(tagId => {
                  const tag = tags.find(t => t.id === tagId)
                  if (!tag) return null
                  return (
                    <span key={tagId} className="inline-flex items-center gap-1 text-xs px-2.5 py-1.5 rounded-full text-white cursor-pointer hover:opacity-80"
                      style={{ backgroundColor: tag.color || '#8b5cf6' }}
                      onClick={() => handleToggleTag(tagId)}
                      title={t('common.delete')}
                    >
                      {tag.name}
                      <X size={12} />
                    </span>
                  )
                })
              )}
            </div>
          </div>

          {/* 搜索/添加标签 - 合并输入框 */}
          <div>
            <label className={`block text-sm font-semibold ${colors.text} mb-3`}>{t('tags.addOrSelect')}</label>
            <div className="flex gap-3">
              <div className="flex-1 relative" ref={inputContainerRef}>
                <input
                  type="text"
                  value={newTagName}
                  onChange={(e) => setNewTagName(e.target.value)}
                  onFocus={() => setShowDropdown(true)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      if (newTagName.trim()) {
                        handleAddTag()
                      } else {
                        handleSubmit()
                      }
                    }
                  }}
                  placeholder={t('tags.searchOrCreate') || '搜索或输入新标签...'}
                  className={`w-full px-4 py-3 border-2 rounded-xl ${colors.text} ${colors.input} ${colors.inputFocus} focus:ring-2 transition-all`}
                />
                {/* 搜索建议下拉 - 聚焦就显示 */}
                {showDropdown && availableTags.length > 0 && (
                  <div className={`absolute top-full left-0 right-0 mt-1 ${colors.card} border ${colors.cardBorder} rounded-xl shadow-lg z-10 max-h-48 overflow-y-auto`}>
                    {filteredTags.map(tag => (
                      <button key={tag.id} onClick={() => { setSelectedTagIds([...selectedTagIds, tag.id]); setNewTagName(''); setShowDropdown(false) }}
                        className={`w-full px-3 py-2 text-left text-sm ${colors.text} hover:opacity-80 flex items-center gap-2 transition-colors ${colors.cardHover} rounded-lg`}
                      >
                        <span className="w-3 h-3 rounded-full" style={{ backgroundColor: tag.color }} />
                        {tag.name}
                      </button>
                    ))}
                    {filteredTags.length === 0 && newTagName.trim() && (
                      <div className={`px-3 py-2 text-sm ${colors.textMuted}`}>
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
                className={`px-4 py-3 ${accent.solidBg} text-white rounded-xl text-sm ${accent.solidHoverBg} disabled:opacity-50 transition-all shadow-lg ${accent.shadow} hover:shadow-xl disabled:shadow-none`}
                title={t('tags.addTag')}
              >
                <Plus size={18} />
              </button>
            </div>
            <p className={`text-xs ${colors.text} opacity-60 mt-2 leading-relaxed`}>{t('tags.hint') || '输入搜索已有标签，或直接输入创建新标签'}</p>
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
            className={accentGradientButtonClass}
          >
            {loading ? t('common.saving') : t('common.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}

export default BatchTagModal
