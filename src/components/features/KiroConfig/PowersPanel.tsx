import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Zap, RefreshCw, Trash2, Server, FileText, Tag, ChevronDown, ChevronRight, ExternalLink, Download, Check, Globe } from 'lucide-react'
import { handleUiError } from '../../../utils/errorLogger'
import { getThemeAccent, getSolidAccentButton, getGradientAccentButton, getThemeSurfaceStyles } from './themeAccent'
import React from 'react'

// 格式化文件大小
const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

function PowersPanel({ onCountChange }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm, showSuccess } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const accentGradientButtonClass = getGradientAccentButton(accent)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10'
  }

  const [tab, setTab] = useState('recommended') // 'installed' | 'recommended'
  const [powers, setPowers] = useState<any[]>([])
  const [recommended, setRecommended] = useState<any[]>([])
  const [registries, setRegistries] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [recLoading, setRecLoading] = useState(false)
  const [selectedPower, setSelectedPower] = useState<any>(null)
  const [selectedRec, setSelectedRec] = useState<any>(null)
  const [expandedSections, setExpandedSections] = useState({ mcp: true, steering: false, md: false })
  const [installing, setInstalling] = useState<string | null>(null) // 正在安装的 power name

  const loadPowers = useCallback(async () => {
    setLoading(true)
    try {
      const [data, regs] = await Promise.all([
        invoke<any[]>('get_powers'),
        invoke<any[]>('get_power_registries')
      ])
      setPowers(data)
      setRegistries(regs)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('load Powers failed', e, { userMessage: t('powers.loadFailed') })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, t])

  const loadRecommended = useCallback(async () => {
    setRecLoading(true)
    try {
      const data = await invoke<any[]>('get_recommended_powers')
      setRecommended(data)
    } catch (e) {
      handleUiError('load recommended Powers failed', e, { userMessage: t('powers.loadRecommendedFailed') })
    } finally {
      setRecLoading(false)
    }
  }, [t])

  useEffect(() => { loadPowers() }, [loadPowers])
  useEffect(() => { loadRecommended() }, [loadRecommended])

  const handleUninstall = async (power: any) => {
    if (!await showConfirm(t('powers.confirmUninstall'), t('powers.confirmUninstallPower', { name: power.name }))) return
    try {
      await invoke('uninstall_power', { name: power.name })
      const newPowers = powers.filter(p => p.name !== power.name)
      setPowers(newPowers)
      onCountChange?.(newPowers.length)
      if (selectedPower?.name === power.name) setSelectedPower(null)
      // 更新推荐列表中的安装状态
      setRecommended(prev => prev.map(r => r.name === power.name ? { ...r, installed: false } : r))
    } catch (e) {
      handleUiError('uninstall Power failed', e, { userMessage: t('powers.uninstallFailed') })
    }
  }

  const handleOpenRepo = (url: string) => {
    if (url) openUrl(url).catch(() => window.open(url, '_blank'))
  }

  const handleInstall = async (rec: any) => {
    if (installing) return
    setInstalling(rec.name)
    try {
      await invoke('install_power', {
        name: rec.name,
        cloneUrl: rec.repositoryCloneUrl || rec.repositoryUrl,
        pathInRepo: rec.pathInRepo || '',
        branch: rec.repositoryBranch || 'main'
      })
      // 更新推荐列表状态
      setRecommended(prev => prev.map(r => r.name === rec.name ? { ...r, installed: true } : r))
      if (selectedRec?.name === rec.name) setSelectedRec({ ...selectedRec, installed: true })
      // 重新加载已安装列表
      const data = await invoke<any[]>('get_powers')
      setPowers(data)
      onCountChange?.(data?.length || 0)
      showSuccess(t('powers.installSuccess'), rec.displayName || rec.name)
    } catch (e) {
      handleUiError('install Power failed', e, { userMessage: t('powers.installFailed') })
    } finally {
      setInstalling(null)
    }
  }

  const toggleSection = (key: string) => {
    setExpandedSections((prev: any) => ({ ...prev, [key]: !prev[key] }))
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
      </div>
    )
  }

  return (
    <div className="h-full flex gap-3 p-4 max-w-full overflow-x-hidden">
      {/* 左侧列表 */}
    <div className={`w-72 flex flex-col glass-card border border-border rounded-xl overflow-hidden max-w-full`}>

        <div className={`p-3 border-b border-border`}>
          <div className={`flex items-center gap-1 p-1 rounded-xl bg-muted/30`}>
            <button
              onClick={() => { setTab('recommended'); setSelectedPower(null) }}
              className={`cursor-pointer flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                tab === 'recommended'
                  ? accent.tabActive
                  : `text-muted-foreground hover:opacity-80`
              }`}
            >
              <Globe size={13} />
              {t('powers.recommended')} ({recommended.length})
            </button>
            <button
              onClick={() => { setTab('installed'); setSelectedRec(null) }}
              className={`cursor-pointer flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                tab === 'installed'
                  ? accent.tabActive
                  : `text-muted-foreground hover:opacity-80`
              }`}
            >
              <Zap size={13} />
              {t('powers.installed')} ({powers.length})
            </button>
          </div>
        </div>

        {/* 列表内容 */}
        <div className="flex-1 overflow-auto p-3">
          {tab === 'installed' ? (
            /* 已安装列表 */
            powers.length === 0 ? (
              <div className={`text-center py-16 text-muted-foreground`}>
                <Zap size={48} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">{t('powers.noPowers')}</p>
                <p className={`text-xs mt-2 text-muted-foreground`}>{t('powers.noPowersHint')}</p>
                <button
                  onClick={() => setTab('recommended')}
                  className={`cursor-pointer mt-4 px-4 py-2 rounded-lg text-sm transition-colors ${accentSolidButtonClass}`}
                >
                  {t('powers.browseRecommended')}
                </button>
              </div>
            ) : (
              <div className="space-y-2">
                {powers.map(power => {
                  const isSelected = selectedPower?.name === power.name
                  return (
                    <div
                      key={power.name}
                      onClick={() => { setSelectedPower(power); setSelectedRec(null) }}
                      className={`p-3 rounded-xl cursor-pointer group transition-all duration-200 ${
                        isSelected
                          ? `${accent.bg} ring-2 ${accent.ring} shadow-lg border ${accent.border}`
                          : `glass-card border border-border hover:bg-muted/50 hover:shadow-md`
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex items-center gap-2.5 flex-1 min-w-0">
                          <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 ${
                            isSelected ? accent.bg : "bg-muted/30"
                          }`}>
                            <Zap size={15} className={isSelected ? accent.text : "text-muted-foreground"} />
                          </div>
                          <div className="flex-1 min-w-0">
                            <span className={`font-semibold text-sm ${isSelected ? accent.text : "text-foreground"} truncate block leading-tight`}>
                              {power.displayName || power.name}
                            </span>
                            {power.description && (
                              <span className={`text-xs text-muted-foreground truncate block mt-0.5 leading-tight`}>{power.description}</span>
                            )}
                          </div>
                        </div>
                        <button
                          onClick={(e) => { e.stopPropagation(); handleUninstall(power) }}
                          className="cursor-pointer opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                          title={t('powers.uninstall')}
                        >
                          <Trash2 size={14} className="text-red-500" />
                        </button>
                      </div>
                      <div className={`flex items-center gap-2 text-xs text-muted-foreground mt-2 flex-wrap`} style={{ marginLeft: '2.375rem' }}>
                        <span className={`px-1.5 py-0.5 rounded bg-muted/30 text-[10px] font-medium`}>{formatSize(power.size)}</span>
                        {power.mcpServers.length > 0 && (
                          <span className="flex items-center gap-1 text-[10px]">
                            <Server size={10} /> {power.mcpServers.length} MCP
                          </span>
                        )}
                        {power.steeringFiles.length > 0 && (
                          <span className="flex items-center gap-1 text-[10px]">
                            <FileText size={10} /> {power.steeringFiles.length} steering
                          </span>
                        )}
                        {power.autoInstalled && (
                          <span className={`px-1.5 py-0.5 rounded text-[10px] ${accent.bgSoft} ${accent.textSoft}`}>{t('powers.autoInstalled')}</span>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            )
          ) : (
            /* 推荐列表 */
            recLoading ? (
              <div className="flex items-center justify-center py-16">
                <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
              </div>
            ) : recommended.length === 0 ? (
              <div className={`text-center py-16 text-muted-foreground`}>
                <Globe size={48} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">{t('powers.noRecommended')}</p>
              </div>
            ) : (
              <div className="space-y-2">
                {recommended.map(rec => {
                  const isSelected = selectedRec?.name === rec.name
                  return (
                    <div
                      key={rec.name}
                      onClick={() => { setSelectedRec(rec); setSelectedPower(null) }}
                      className={`p-3 rounded-xl cursor-pointer group transition-all duration-200 ${
                        isSelected
                          ? `${accent.bg} ring-2 ${accent.ring} shadow-lg border ${accent.border}`
                          : `glass-card border border-border hover:bg-muted/50 hover:shadow-md`
                      }`}
                    >
                      <div className="flex items-start gap-2.5">
                        {rec.iconUrl ? (
                          <img src={rec.iconUrl} alt="" className="w-7 h-7 rounded-lg flex-shrink-0 object-contain" onError={(e: any) => { e.target.style.display='none' }} />
                        ) : (
                          <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 ${
                            isSelected ? accent.bg : "bg-muted/30"
                          }`}>
                            <Zap size={15} className={isSelected ? accent.text : "text-muted-foreground"} />
                          </div>
                        )}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className={`font-semibold text-sm ${isSelected ? accent.text : "text-foreground"} truncate leading-tight`}>
                              {rec.displayName || rec.name}
                            </span>
                            {rec.installed && (
                              <span className={`flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium flex-shrink-0 ${accent.bgSoft} ${accent.text}`}>
                                <Check size={10} />{t('powers.installedBadge')}
                              </span>
                            )}
                          </div>
                          {rec.description && (
                            <span className={`text-xs text-muted-foreground line-clamp-2 mt-0.5 leading-tight`}>{rec.description}</span>
                          )}
                          {rec.author && (
                            <span className={`text-[10px] text-muted-foreground mt-1 block opacity-70`}>by {rec.author}</span>
                          )}
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            )
          )}
        </div>
      </div>

      {/* 右侧详情 */}
      <div className={`flex-1 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        {selectedPower ? (
          /* 已安装 Power 详情 */
          <>
            <div className={`p-4 border-b border-border`}>
              <div className="flex items-center justify-between">
                <div>
                  <h3 className={`font-semibold text-lg text-foreground`}>{selectedPower.displayName || selectedPower.name}</h3>
                  {selectedPower.description && (
                    <p className={`text-sm mt-1 text-muted-foreground`}>{selectedPower.description}</p>
                  )}
                </div>
                <button
                  onClick={() => handleUninstall(selectedPower)}
                  className="cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium bg-red-500/10 text-red-500 hover:bg-red-500/20 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                >
                  <Trash2 size={14} />
                  {t('powers.uninstall')}
                </button>
              </div>
              <div className={`flex items-center gap-4 mt-3 text-xs text-muted-foreground flex-wrap`}>
                {selectedPower.author && <span>{t('powers.author')}: {selectedPower.author}</span>}
                {selectedPower.license && <span>{t('powers.license')}: {selectedPower.license}</span>}
                {selectedPower.registryId && (
                  <span className={`px-2 py-0.5 rounded bg-muted/30`}>{selectedPower.registryId}</span>
                )}
              </div>
              {selectedPower.keywords.length > 0 && (
                <div className="flex items-center gap-2 mt-2 flex-wrap">
                  <Tag size={12} className={"text-muted-foreground"} />
                  {selectedPower.keywords.map((kw: string) => (
                    <span key={kw} className={`text-xs px-2 py-0.5 rounded-full bg-muted/30 text-muted-foreground`}>{kw}</span>
                  ))}
                </div>
              )}
            </div>
            <div className="flex-1 overflow-auto">
              {selectedPower.mcpServers.length > 0 && (
                <CollapsibleSection title={`MCP Servers (${selectedPower.mcpServers.length})`} icon={<Server size={14} className={accent.text} />} expanded={expandedSections.mcp} onToggle={() => toggleSection('mcp')} colors={colors}>
                  <div className="space-y-2">
                    {selectedPower.mcpServers.map((name: string) => (
                      <div key={name} className={`flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/30`}>
                        <Server size={14} className={accent.textSoft} />
                        <code className={`text-sm text-foreground`}>{name}</code>
                      </div>
                    ))}
                  </div>
                </CollapsibleSection>
              )}
              {selectedPower.steeringFiles.length > 0 && (
                <CollapsibleSection title={`Steering Files (${selectedPower.steeringFiles.length})`} icon={<FileText size={14} className={accent.text} />} expanded={expandedSections.steering} onToggle={() => toggleSection('steering')} colors={colors}>
                  <div className="space-y-2">
                    {selectedPower.steeringFiles.map((name: string) => (
                      <div key={name} className={`flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/30`}>
                        <FileText size={14} className={accent.textSoft} />
                        <code className={`text-sm text-foreground`}>{name}</code>
                      </div>
                    ))}
                  </div>
                </CollapsibleSection>
              )}
              {selectedPower.powerMd && (
                <CollapsibleSection title="POWER.md" icon={<FileText size={14} className={accent.text} />} expanded={expandedSections.md} onToggle={() => toggleSection('md')} colors={colors}>
                  <pre className={`text-xs leading-relaxed whitespace-pre-wrap font-mono p-3 rounded-lg text-muted-foreground`}
                    style={{ backgroundColor: surface.editorBg }}
                  >
                    {selectedPower.powerMd}
                  </pre>
                </CollapsibleSection>
              )}
            </div>
          </>
        ) : selectedRec ? (
          /* 推荐 Power 详情 */
          <div className="flex-1 overflow-auto">
            <div className={`p-6 border-b border-border`}>
              <div className="flex items-start gap-4">
                {selectedRec.iconUrl ? (
                  <img src={selectedRec.iconUrl} alt="" className="w-14 h-14 rounded-xl flex-shrink-0 object-contain shadow-lg" onError={(e: any) => { e.target.style.display='none' }} />
                ) : (
                  <div className={`w-14 h-14 rounded-xl bg-primary/10 flex items-center justify-center flex-shrink-0`}>
                    <Zap size={28} className={accent.text} />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <h3 className={`font-bold text-xl text-foreground`}>{selectedRec.displayName || selectedRec.name}</h3>
                  {selectedRec.author && (
                    <p className={`text-sm text-muted-foreground mt-1`}>by {selectedRec.author}</p>
                  )}
                  {selectedRec.description && (
                    <p className={`text-sm mt-3 text-muted-foreground leading-relaxed`}>{selectedRec.description}</p>
                  )}
                </div>
              </div>

              <div className="flex items-center gap-3 mt-5">
                {selectedRec.installed ? (
                  <span className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium border ${accent.bgSoft} ${accent.text} ${accent.borderSoft}`}>
                    <Check size={16} />{t('powers.alreadyInstalled')}
                  </span>
                ) : (
                  <button
                    onClick={() => handleInstall(selectedRec)}
                    disabled={installing === selectedRec.name}
                    className={`cursor-pointer flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-all active:scale-[0.98] disabled:opacity-60 disabled:cursor-not-allowed ${accentGradientButtonClass}`}
                  >
                    {installing === selectedRec.name ? (
                      <><RefreshCw size={16} className="animate-spin" />{t('powers.installing')}</>
                    ) : (
                      <><Download size={16} />{t('powers.install')}</>
                    )}
                  </button>
                )}
                {selectedRec.repositoryUrl && (
                  <button
                    onClick={() => handleOpenRepo(selectedRec.repositoryUrl)}
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium border border-border text-foreground hover:bg-muted/50 transition-all cursor-pointer`}
                  >
                    <ExternalLink size={14} /> GitHub
                  </button>
                )}
              </div>
            </div>

            {/* 详细信息 */}
            <div className="p-6 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                {selectedRec.author && (
                  <div>
                    <span className={`text-xs text-muted-foreground block mb-1`}>{t('powers.author')}</span>
                    <span className={`text-sm font-medium text-foreground`}>{selectedRec.author}</span>
                  </div>
                )}
                {selectedRec.license && (
                  <div>
                    <span className={`text-xs text-muted-foreground block mb-1`}>{t('powers.license')}</span>
                    <span className={`text-sm font-medium text-foreground`}>{selectedRec.license}</span>
                  </div>
                )}
                <div>
                  <span className={`text-xs text-muted-foreground block mb-1`}>{t('powers.name')}</span>
                  <code className={`text-sm text-foreground`}>{selectedRec.name}</code>
                </div>
              </div>

              {/* 第三方免责 */}
              <div className={`mt-6 p-4 rounded-xl border border-border bg-muted/30`}>
                <p className={`text-xs text-muted-foreground leading-relaxed`}>
                  {t('powers.thirdPartyDisclaimer')}
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <Zap size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('powers.selectToView')}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

// 可折叠区段
function CollapsibleSection({ title, icon, expanded, onToggle, colors, children }: any) {
  return (
    <div className={`border-b border-border`}>
      <button
        onClick={onToggle}
        className={`w-full flex items-center gap-2 px-4 py-3 hover:bg-muted/50 transition-colors cursor-pointer`}
      >
        {expanded ? <ChevronDown size={14} className={"text-muted-foreground"} /> : <ChevronRight size={14} className={"text-muted-foreground"} />}
        {icon}
        <span className={`text-sm font-medium text-foreground`}>{title}</span>
      </button>
      {expanded && <div className="px-4 pb-3">{children}</div>}
    </div>
  )
}

export default PowersPanel
