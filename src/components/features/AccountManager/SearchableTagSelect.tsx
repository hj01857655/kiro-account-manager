import { useState, useRef, useEffect, useMemo } from 'react'
import { X, ChevronDown, Tag } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { isPointerInsideContainer } from './utils/pointerInside'
import { getThemeAccent } from '../AgentConfig/themeAccent'
import React from 'react'

interface TagItem {
  id: string;
  name: string;
  color?: string;
}

interface SearchableTagSelectProps {
  tags?: TagItem[];
  value?: string | null;
  onChange: (value: string | null) => void;
  placeholder?: string;
  showAllOption?: boolean;
  showNoneOption?: boolean;
  allLabel?: string;
  noneLabel?: string;
  hasLabel?: string;
  className?: string;
}

/**
 * 可搜索的标签选择下拉框
 */
function SearchableTagSelect({
  tags = [],
  value,
  onChange,
  placeholder = '搜索标签...',
  showAllOption = false,
  showNoneOption = false,
  allLabel = '全部',
  noneLabel = '无标签',
  hasLabel = '有标签',
  className = ''}: SearchableTagSelectProps) {
  const { theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const activeOptionClass = `${accent.bgSoft} ${accent.text} font-medium`
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const containerRef = useRef<HTMLDivElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // 点击外部关闭
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !isPointerInsideContainer(e, [containerRef.current, panelRef.current])) {
        setOpen(false)
        setSearch('')
      }
    }
    document.addEventListener('click', handleClickOutside)
    return () => document.removeEventListener('click', handleClickOutside)
  }, [])

  // 打开时聚焦输入框
  useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus()
    }
  }, [open])

  // 过滤标签
  const filteredTags = tags.filter(tag => 
    tag.name.toLowerCase().includes(search.toLowerCase())
  )

  // 获取当前选中的标签
  const selectedTag = tags.find(t => t.id === value)

  // 选择标签
  const handleSelect = (tagId: string | null) => {
    onChange(tagId)
    setOpen(false)
    setSearch('')
  }

  // 显示文本
  const displayText = value === '__none__' ? noneLabel : value === '__has__' ? hasLabel : (selectedTag?.name || '')

  return (
    <div className={`relative ${className}`} ref={containerRef}>
      {/* 输入框（可搜索） */}
      <div className={`w-full flex items-center border rounded-xl text-sm bg-background border-input ${open ? `ring-2 ${accent.ring} ${accent.border}` : ''} transition-all cursor-pointer shadow-sm`}>
        {selectedTag && (
          <span className="ml-4 w-3 h-3 rounded-full flex-shrink-0" style={{ backgroundColor: selectedTag.color }} />
        )}
        <input
          ref={inputRef}
          type="text"
          value={open ? search : displayText}
          onChange={(e) => { setSearch(e.target.value); if (!open) setOpen(true) }}
          onFocus={() => setOpen(true)}
          placeholder={placeholder}
          className={`flex-1 px-4 py-3 bg-transparent text-sm text-foreground focus:outline-none cursor-pointer`}
        />
        {value && (
          <button 
            type="button" 
            onClick={(e) => { e.stopPropagation(); onChange(null); setSearch('') }} 
            className={`p-1.5 mr-1 rounded-lg hover:bg-muted/50 hover:bg-red-500/10 transition-all hover:scale-110 active:scale-95`}
            title="清空"
          >
            <X size={14} className="text-red-500" strokeWidth={2.5} />
          </button>
        )}
        <button type="button" onClick={() => setOpen(!open)} className="pr-4">
          <ChevronDown size={16} className={`text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} strokeWidth={2.5} />
        </button>
      </div>

      {/* 下拉面板 */}
      {open && (
        <div
          ref={panelRef}
          className={`absolute left-0 right-0 top-full mt-2 glass-card border border-border rounded-xl shadow-xl z-50 overflow-hidden`}
        >
            <div className="max-h-56 overflow-y-auto">
            {/* 全部选项 */}
            {showAllOption && (
              <button
                type="button"
                onClick={() => handleSelect(null)}
                className={`w-full px-4 py-3 text-left text-sm flex items-center gap-2.5 transition-all ${
                  !value ? activeOptionClass : `text-foreground hover:bg-muted/50`
                }`}
              >
                <Tag size={16} className={"text-muted-foreground"} strokeWidth={2.5} />
                {allLabel}
              </button>
            )}

            {/* 有标签选项 */}
            {showNoneOption && (
              <button
                type="button"
                onClick={() => handleSelect('__has__')}
                className={`w-full px-4 py-3 text-left text-sm flex items-center gap-2.5 transition-all ${
                  value === '__has__' ? activeOptionClass : `text-foreground hover:bg-muted/50`
                }`}
              >
                <span className={`w-3 h-3 rounded-full ${accent.solidBg}`} />
                {hasLabel}
              </button>
            )}

            {/* 无标签选项 */}
            {showNoneOption && (
              <button
                type="button"
                onClick={() => handleSelect('__none__')}
                className={`w-full px-4 py-3 text-left text-sm flex items-center gap-2.5 transition-all ${
                  value === '__none__' ? activeOptionClass : `text-foreground hover:bg-muted/50`
                }`}
              >
                <span className={`w-3 h-3 rounded-full border-2 border-dashed border-border`} />
                {noneLabel}
              </button>
            )}

            {/* 标签列表 */}
            {filteredTags.length > 0 ? (
              filteredTags.map(tag => (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => handleSelect(tag.id)}
                  className={`w-full px-4 py-3 text-left text-sm flex items-center gap-2.5 transition-all ${
                    value === tag.id ? activeOptionClass : `text-foreground hover:bg-muted/50`
                  }`}
                >
                  <span className="w-3 h-3 rounded-full flex-shrink-0" style={{ backgroundColor: tag.color }} />
                  {tag.name}
                </button>
              ))
            ) : search ? (
              <div className={`px-4 py-6 text-center text-sm text-muted-foreground`}>
                未找到匹配的标签
              </div>
            ) : null}
          </div>
        </div>
      )}
    </div>
  )
}

export default SearchableTagSelect
