import { useState, useRef, useEffect, useMemo } from 'react'
import { X, ChevronDown, Tag } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { isPointerInsideContainer } from './utils/pointerInside'
import { getThemeAccent } from '../KiroConfig/themeAccent'
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
 * Searchable tag select dropdown.
 */
function SearchableTagSelect({
  tags = [],
  value,
  onChange,
  placeholder = 'Search tags...',
  showAllOption = false,
  showNoneOption = false,
  allLabel = 'All',
  noneLabel = 'No tags',
  hasLabel = 'Has tags',
  className = ''}: SearchableTagSelectProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const activeOptionClass = `${accent.bgSoft} ${accent.text} font-medium`
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const containerRef = useRef<HTMLDivElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // Close on outside click.
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

  // Focus the input when opened.
  useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus()
    }
  }, [open])

  // Filter tags.
  const filteredTags = tags.filter(tag => 
    tag.name.toLowerCase().includes(search.toLowerCase())
  )

  // Get the selected tag.
  const selectedTag = tags.find(t => t.id === value)

  // Select a tag.
  const handleSelect = (tagId: string | null) => {
    onChange(tagId)
    setOpen(false)
    setSearch('')
  }

  // Display text.
  const displayText = value === '__none__' ? noneLabel : value === '__has__' ? hasLabel : (selectedTag?.name || '')

  return (
    <div className={`relative ${className}`} ref={containerRef}>
      {/* Searchable input */}
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
            title={t('settings.clear')}
          >
            <X size={14} className="text-red-500" strokeWidth={2.5} />
          </button>
        )}
        <button type="button" onClick={() => setOpen(!open)} className="pr-4">
          <ChevronDown size={16} className={`text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} strokeWidth={2.5} />
        </button>
      </div>

      {/* Dropdown panel */}
      {open && (
        <div
          ref={panelRef}
          className={`absolute left-0 right-0 top-full mt-2 glass-card border border-border rounded-xl shadow-xl z-50 overflow-hidden`}
        >
            <div className="max-h-56 overflow-y-auto">
            {/* All option */}
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
                {t('common.noMatches')}
              </div>
            ) : null}
          </div>
        </div>
      )}
    </div>
  )
}

export default SearchableTagSelect
