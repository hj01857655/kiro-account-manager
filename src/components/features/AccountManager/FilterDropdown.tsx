import { useState, useRef, useEffect, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { Filter, X } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useTranslation } from 'react-i18next'
import SearchableTagSelect from './SearchableTagSelect'
import { getThemeAccent } from '../KiroConfig/themeAccent'

import { buildFilterSummaryItems, countActiveFilters } from './utils/filterDropdownState'
import { isPointerInsideContainer } from './utils/pointerInside'
import React from 'react'

const SUBSCRIPTION_OPTIONS = [
  { value: '', label: 'All' },
  { value: 'FREE', label: 'FREE' },
  { value: 'KIRO FREE', label: 'KIRO FREE' },
  { value: 'KIRO PRO', label: 'KIRO PRO' },
  { value: 'KIRO PRO+', label: 'KIRO PRO+' },
  { value: 'KIRO POWER', label: 'KIRO POWER' },
]
const STATUS_OPTIONS = [
  { value: '', label: 'All' },
  { value: 'normal', label: 'Normal' },
  { value: 'capped', label: 'Capped' },
  { value: 'banned', label: 'Banned' },
  { value: 'invalid', label: 'Invalid' },
  { value: 'expired', label: 'Expired' },
]
const PROVIDER_OPTIONS = [
  { value: '', label: 'All' },
  { value: 'Google', label: 'Google' },
  { value: 'Github', label: 'Github' },
  { value: 'BuilderId', label: 'BuilderId' },
  { value: 'Enterprise', label: 'Enterprise' },
]
const USAGE_RANGE_OPTIONS = [
  { value: '', label: 'All' },
  { value: '0-500', label: '0-500' },
  { value: '500-1000', label: '500-1000' },
  { value: '1000-2000', label: '1000-2000' },
  { value: '2000-+', label: '2000+' },
]

interface FilterDropdownProps {
filters: any;
onFiltersChange: (filters: any) => void;
allGroups?: any[];
selectedGroup?: any;
onGroupFilter?: (group: any) => void;
allTags?: any[];
selectedTag?: any;
onTagFilter: (tag: any) => void;
selectedStatus?: any;
onStatusFilter?: (status: any) => void;
defaultGroupCollapsed?: boolean;
}

function SectionCard({ title, subtitle, children }: { title: string; subtitle?: string; children: React.ReactNode }) {
  return (
    <section className={`rounded-xl border border-border bg-muted/30 p-3 space-y-3`}>
      <div>
        <h4 className={`text-sm font-semibold text-foreground`}>{title}</h4>
        {subtitle && <p className={`mt-1 text-[11px] text-muted-foreground`}>{subtitle}</p>}
      </div>
      {children}
    </section>
  )
}

function FilterField({ label, hint, active, accent, children, fullWidth = false, activeLabel = 'Set' }: any) {
  return (
    <div
      className={`
        rounded-xl border p-3 space-y-2.5 transition-all duration-200
        ${active ? `${accent.border} ${accent.bgSoft} shadow-sm ${accent.shadow}` : `border-border glass-card`}
        ${fullWidth ? 'sm:col-span-2' : ''}
      `}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <label className={`block text-xs font-medium text-muted-foreground`}>
            {label}
          </label>
          {hint && (
            <p className={`mt-1 text-[11px] leading-5 text-muted-foreground`}>
              {hint}
            </p>
          )}
        </div>
        {active && (
          <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold ${accent.bgSoft} ${accent.text}`}>
            {activeLabel}
          </span>
        )}
      </div>
      {children}
    </div>
  )
}

function FilterSelect({ label, hint, value, options, onChange, onClear, accent, activeLabel, clearLabel = 'Clear' }: any) {
  const displayValue = Array.isArray(value) ? (value[0] || '') : (value || '')
  const hasValue = displayValue !== ''

  return (
    <FilterField
      label={label}
      hint={hint}
      active={hasValue}
      accent={accent}
      activeLabel={activeLabel}
    >
      <div className="relative">
        <select
          value={displayValue}
          onChange={(e) => {
            const nextValue = e.target.value
            if (nextValue === '') {
              onClear?.()
            } else {
              onChange(nextValue)
            }
          }}
          className={`w-full px-3 py-2.5 ${hasValue ? 'pr-9' : 'pr-3'} text-sm rounded-lg border bg-background border-input text-foreground transition-all duration-200 cursor-pointer shadow-sm`}
        >
          {options.map((opt: any) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        {hasValue && (
          <button
            onClick={(e) => {
              e.stopPropagation()
              onClear?.()
            }}
            className={`cursor-pointer absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted/50 hover:bg-red-500/10 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60`}
            title={clearLabel}
          >
            <X size={12} className="text-red-500" strokeWidth={2.5} />
          </button>
        )}
      </div>
    </FilterField>
  )
}

function FilterDropdown({
  filters,
  onFiltersChange,
  allGroups = [],
  selectedGroup,
  onGroupFilter,
  allTags = [],
  selectedTag,
  onTagFilter}: FilterDropdownProps) {
  const { theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [buttonRect, setButtonRect] = useState<DOMRect | null>(null)
  const dropdownRef = useRef<HTMLDivElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !isPointerInsideContainer(e, [dropdownRef.current, panelRef.current])) {
        setOpen(false)
      }
    }
    document.addEventListener('click', handleClickOutside)
    return () => document.removeEventListener('click', handleClickOutside)
  }, [])

  const activeCount = countActiveFilters({ filters, selectedGroup, selectedTag })
  const summaryItems = buildFilterSummaryItems({
    filters,
    selectedGroup,
    selectedTag,
    allGroups,
    allTags,
    t})

  const clearAll = () => {
    onFiltersChange({ subscriptions: [], statuses: [], providers: [], usageRange: null })
    onGroupFilter?.(null)
    onTagFilter(null)
  }

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={(e) => {
          const rect = e.currentTarget.getBoundingClientRect()
          setButtonRect(rect)
          setOpen(!open)
        }}
        className={`relative h-8 w-8 rounded-md inline-flex items-center justify-center transition-colors cursor-pointer focus:outline-none focus:ring-2 ${accent.ring} ${
          activeCount > 0
            ? `${accent.solidBg} text-white shadow-sm`
            : 'glass-card border border-border hover:bg-muted/50 text-muted-foreground'
        } ${open ? `ring-2 ${accent.ring}` : ''}`}
        title={t('filter.title')}
        aria-label={t('filter.title')}
      >
        <Filter size={14} strokeWidth={2.5} />
        {activeCount > 0 && (
          <span className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-primary text-white text-[9px] font-bold flex items-center justify-center ring-2 ring-background">
            {activeCount}
          </span>
        )}
      </button>

      {open && buttonRect && createPortal(
        <div
          ref={panelRef}
          className="fixed w-[380px] max-w-[calc(100vw-32px)] glass-card border border-border rounded-xl shadow-xl z-[9999] overflow-hidden"
          style={{
            top: `${buttonRect.bottom + 8}px`,
            right: `${window.innerWidth - buttonRect.right}px`,
            animation: 'slideDown 0.15s ease-out',
          }}
        >
            <div className={`px-4 py-3 border-b border-border`}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Filter size={14} className={accent.text} strokeWidth={2.5} />
                <span className="text-sm font-semibold text-foreground">{t('filter.title')}</span>
                {activeCount > 0 && (
                  <span className={`rounded-full px-1.5 py-0.5 text-[10px] font-bold ${accent.bgSoft} ${accent.text}`}>
                    {activeCount}
                  </span>
                )}
              </div>
              <div className="flex items-center gap-1.5">
                {activeCount > 0 && (
                  <button
                    onClick={clearAll}
                    className="cursor-pointer text-[11px] text-red-500 hover:bg-red-500/10 px-2 py-1 rounded-md transition-colors"
                  >
                    {t('settings.clear')}
                  </button>
                )}
                <button
                  onClick={() => setOpen(false)}
                  className="cursor-pointer h-7 w-7 rounded-md inline-flex items-center justify-center hover:bg-muted/50 text-muted-foreground transition-colors"
                  aria-label={t('settings.close')}
                >
                  <X size={14} />
                </button>
              </div>
            </div>
            {summaryItems.length > 0 && (
              <div className="flex flex-wrap gap-1.5 mt-2">
                {summaryItems.map(item => (
                  <span
                    key={item.key}
                    className="inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-[10px] border-border bg-muted/30 text-foreground"
                  >
                    <span className="text-muted-foreground">{item.label}</span>
                    <span className="font-medium">{item.value}</span>
                  </span>
                ))}
              </div>
            )}
          </div>

          <div className="p-3 space-y-3 max-h-[420px] overflow-y-auto custom-scrollbar max-w-full">
            {allTags.length > 0 && (
              <SectionCard title={t('tags.title')}>
                <div>
                  <SearchableTagSelect
                    tags={allTags}
                    value={selectedTag}
                    onChange={onTagFilter}
                    placeholder={t('tags.searchPlaceholder')}
                    showAllOption={true}
                    showNoneOption={true}
                    allLabel={t('tags.all')}
                    noneLabel={t('tags.noTags')}
                    hasLabel={t('tags.hasTags')}
                  />
                </div>
              </SectionCard>
            )}

            <SectionCard title={t('filter.advanced')}>
              <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
                <FilterSelect
                  label={t('filter.subscription')}
                  value={filters.subscriptions?.length > 0 ? filters.subscriptions[0] : ''}
                  options={SUBSCRIPTION_OPTIONS}
                  onChange={(value: string) => onFiltersChange({ ...filters, subscriptions: [value] })}
                  onClear={() => onFiltersChange({ ...filters, subscriptions: [] })}
                  accent={accent}
                  activeLabel={t('filter.set')}
                  clearLabel={t('settings.clear')}
                />

                <FilterSelect
                  label={t('filter.status')}
                  value={filters.statuses?.length > 0 ? filters.statuses[0] : ''}
                  options={STATUS_OPTIONS}
                  onChange={(value: string) => onFiltersChange({ ...filters, statuses: [value] })}
                  onClear={() => onFiltersChange({ ...filters, statuses: [] })}
                  accent={accent}
                  activeLabel={t('filter.set')}
                  clearLabel={t('settings.clear')}
                />

                <FilterSelect
                  label={t('filter.provider')}
                  value={filters.providers?.length > 0 ? filters.providers[0] : ''}
                  options={PROVIDER_OPTIONS}
                  onChange={(value: string) => onFiltersChange({ ...filters, providers: [value] })}
                  onClear={() => onFiltersChange({ ...filters, providers: [] })}
                  accent={accent}
                  activeLabel={t('filter.set')}
                  clearLabel={t('settings.clear')}
                />

                <FilterSelect
                  label={t('filter.usage')}
                  value={filters.usageRange || ''}
                  options={USAGE_RANGE_OPTIONS}
                  onChange={(value: string) => onFiltersChange({ ...filters, usageRange: value })}
                  onClear={() => onFiltersChange({ ...filters, usageRange: null })}
                  accent={accent}
                  activeLabel={t('filter.set')}
                  clearLabel={t('settings.clear')}
                />

                {allGroups.length > 0 && (
                  <FilterField
                    label={t('groups.title')}
                    active={Boolean(selectedGroup)}
                    accent={accent}
                    activeLabel={t('filter.set')}
                    fullWidth
                  >
                    <SearchableTagSelect
                      tags={allGroups}
                      value={selectedGroup}
                      onChange={onGroupFilter}
                      placeholder={t('groups.searchPlaceholder')}
                      showAllOption={true}
                      showNoneOption={true}
                      allLabel={t('groups.all')}
                      noneLabel={t('groups.noGroup')}
                      hasLabel={t('groups.hasGroup')}
                    />
                  </FilterField>
                )}
              </div>
            </SectionCard>
          </div>

          <div className="flex items-center justify-end border-t border-border px-3 py-2">
            <button
              onClick={() => setOpen(false)}
              className={`cursor-pointer rounded-md px-3 h-8 text-xs font-medium ${accent.text} ${accent.bgSoft} hover:opacity-90 transition-colors`}
            >
              {t('common.finish')}
            </button>
          </div>

          <style>{`
            .custom-scrollbar::-webkit-scrollbar {
              width: 6px;
            }
            .custom-scrollbar::-webkit-scrollbar-track {
              background: transparent;
              margin: 4px 0;
            }
            .custom-scrollbar::-webkit-scrollbar-thumb {
              background: var(--app-primary-solid);
              border-radius: 3px;
            }
            .custom-scrollbar::-webkit-scrollbar-thumb:hover {
              background: var(--app-primary-solid-hover);
            }
          `}</style>
        </div>,
        document.body
      )}
    </div>
  )
}

export default FilterDropdown
