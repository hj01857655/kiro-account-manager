import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { Zap, RefreshCw, Trash2, Server, FileText, Tag, ChevronDown, ChevronRight, ExternalLink, Download, Check, Globe, Loader2 } from 'lucide-react'
import { getThemeAccent, getSolidAccentButton, getGradientAccentButton, getThemeSurfaceStyles } from './themeAccent'
import { handleUiError } from '../../../utils/errorLogger'

// 格式化文件大小
const formatSize = (bytes) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

function PowersPanel({ onCountChange }) {
  const { t, theme, colors } = useApp()
  const { showConfirm, showSuccess } = useDialog()
  const surface = getThemeSurfaceStyles(theme)
  const accent = getThemeAccent(theme)
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const accentGradientButtonClass = getGradientAccentButton(accent)

  const [tab, setTab] = useState('recommended') // 'installed' | 'recommended'
  const [powers, setPowers] = useState([])
  const [recommended, setRecommended] = useState([])
  const [registries, setRegistries] = useState([])
  const [loading, setLoading] = useState(true)
  const [recLoading, setRecLoading] = useState(false)
  const [selectedPower, setSelectedPower] = useState(null)
  const [selectedRec, setSelectedRec] = useState(null)
  const [expandedSections, setExpandedSections] = useState({ mcp: true, steering: false, md: false })
  const [installing, setInstalling] = useState(null) // 正在安装的 power name

  const loadPowers = useCallback(async () => {
    setLoading(true)
    try {
      const [data, regs] = await Promise.all([
        invoke('get_powers'),
        invoke('get_power_registries')
      ])
      setPowers(data)
      setRegistries(regs)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('加载 Powers 失败', e, { userMessage: t('powers.loadFailed') || '加载 Powers 失败' })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, t])

  const loadRecommended = useCallback(async () => {
    setRecLoading(true)
    try {
      const data = await invoke('get_recommended_powers')
      setRecommended(data)
    } catch (e) {
      handleUiError('加载推荐 Powers 失败', e, { userMessage: t('powers.loadRecommendedFailed') || '加载推荐 Powers 失败' })
    } finally {
      setRecLoading(false)
    }
  }, [t])

  useEffect(() => { loadPowers() }, [loadPowers])
  useEffect(() => { loadRecommended() }, [loadRecommended])

  const handleUninstall = async (power) => {
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
      handleUiError('卸载 Power 失败', e, { userMessage: t('powers.uninstallFailed') || '卸载失败' })
    }
  }

  const handleOpenRepo = (url) => {
    if (url) openUrl(url).catch(() => window.open(url, '_blank'))
  }

  const handleInstall = async (rec) => {
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
      const data = await invoke('get_powers')
      setPowers(data)
      onCountChange?.(data?.length || 0)
      showSuccess(t('powers.installSuccess'), rec.displayName || rec.name)
    } catch (e) {
      handleUiError('安装 Power 失败', e, { userMessage: t('powers.installFailed') || '安装失败' })
    } finally {
      setInstalling(null)
    }
  }

  const toggleSection = (key) => {
    setExpandedSections(prev => ({ ...prev, [key]: !prev[key] }))
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
      </div>
    )
  }

  return (
    <div className="h-full flex gap-4 p-4 max-w-full overflow-x-hidden">
      {/* 左侧列表 */}
    <div className={`w-80 flex flex-col ${colors.card} border ${colors.cardBorder} rounded-2xl overflow-hidden shadow-lg max-w-full`}>

        <div className={`p-3 border-b ${colors.cardBorder}`}>
          <div className={`flex items-center gap-1 p-1 rounded-xl ${colors.cardSecondary}`}>
            <button
              onClick={() => { setTab('recommended'); setSelectedPower(null) }}
              className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                tab === 'recommended'
                  ? accent.tabActive
                  : `${colors.textMuted} hover:opacity-80`
              }`}
            >
              <Globe size={13} />
              {t('powers.recommended')} ({recommended.length})
            </button>
            <button
              onClick={() => { setTab('installed'); setSelectedRec(null) }}
              className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                tab === 'installed'
                  ? accent.tabActive
                  : `${colors.textMuted} hover:opacity-80`
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
              <div className={`text-center py-16 ${colors.textMuted}`}>
                <Zap size={48} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">{t('powers.noPowers')}</p>
                <p className={`text-xs mt-2 ${colors.textMuted}`}>{t('powers.noPowersHint')}</p>
                <button
                  onClick={() => setTab('recommended')}
                  className={`mt-4 px-4 py-2 rounded-lg text-sm transition-colors ${accentSolidButtonClass}`}
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
                          : `${colors.card} border ${colors.cardBorder} ${colors.cardHover} hover:shadow-md`
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex items-center gap-2.5 flex-1 min-w-0">
                          <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 ${
                            isSelected ? accent.bg : colors.cardSecondary
                          }`}>
                            <Zap size={15} className={isSelected ? accent.text : colors.textMuted} />
                          </div>
                          <div className="flex-1 min-w-0">
                            <span className={`font-semibold text-sm ${isSelected ? accent.text : colors.text} truncate block leading-tight`}>
                              {power.displayName || power.name}
                            </span>
                            {power.description && (
                              <span className={`text-xs ${colors.textMuted} truncate block mt-0.5 leading-tight`}>{power.description}</span>
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
                      <div className={`flex items-center gap-2 text-xs ${colors.textMuted} mt-2 flex-wrap`} style={{ marginLeft: '2.375rem' }}>
                        <span className={`px-1.5 py-0.5 rounded ${colors.cardSecondary} text-[10px] font-medium`}>{formatSize(power.size)}</span>
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
              <div className={`text-center py-16 ${colors.textMuted}`}>
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
                          : `${colors.card} border ${colors.cardBorder} ${colors.cardHover} hover:shadow-md`
                      }`}
                    >
                      <div className="flex items-start gap-2.5">
                        {rec.iconUrl ? (
                          <img src={rec.iconUrl} alt="" className="w-7 h-7 rounded-lg flex-shrink-0 object-contain" onError={(e) => { e.target.style.display='none' }} />
                        ) : (
                          <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 ${
                            isSelected ? accent.bg : colors.cardSecondary
                          }`}>
                            <Zap size={15} className={isSelected ? accent.text : colors.textMuted} />
                          </div>
                        )}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className={`font-semibold text-sm ${isSelected ? accent.text : colors.text} truncate leading-tight`}>
                              {rec.displayName || rec.name}
                            </span>
                            {rec.installed && (
                              <span className={`flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium flex-shrink-0 ${accent.bgSoft} ${accent.text}`}>
                                <Check size={10} />{t('powers.installedBadge')}
                              </span>
                            )}
                          </div>
                          {rec.description && (
                            <span className={`text-xs ${colors.textMuted} line-clamp-2 mt-0.5 leading-tight`}>{rec.description}</span>
                          )}
                          {rec.author && (
                            <span className={`text-[10px] ${colors.textMuted} mt-1 block opacity-70`}>by {rec.author}</span>
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
      <div className={`flex-1 flex flex-col ${colors.card} border ${colors.cardBorder} rounded-2xl overflow-hidden shadow-lg`}>
        {selectedPower ? (
          /* 已安装 Power 详情 */
          <>
            <div className={`p-4 border-b ${colors.cardBorder}`}>
              <div className="flex items-center justify-between">
                <div>
                  <h3 className={`font-semibold text-lg ${colors.text}`}>{selectedPower.displayName || selectedPower.name}</h3>
                  {selectedPower.description && (
                    <p className={`text-sm mt-1 ${colors.textMuted}`}>{selectedPower.description}</p>
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
              <div className={`flex items-center gap-4 mt-3 text-xs ${colors.textMuted} flex-wrap`}>
                {selectedPower.author && <span>{t('powers.author')}: {selectedPower.author}</span>}
                {selectedPower.license && <span>{t('powers.license')}: {selectedPower.license}</span>}
                {selectedPower.registryId && (
                  <span className={`px-2 py-0.5 rounded ${colors.cardSecondary}`}>{selectedPower.registryId}</span>
                )}
              </div>
              {selectedPower.keywords.length > 0 && (
                <div className="flex items-center gap-2 mt-2 flex-wrap">
                  <Tag size={12} className={colors.textMuted} />
                  {selectedPower.keywords.map(kw => (
                    <span key={kw} className={`text-xs px-2 py-0.5 rounded-full ${colors.cardSecondary} ${colors.textMuted}`}>{kw}</span>
                  ))}
                </div>
              )}
            </div>
            <div className="flex-1 overflow-auto">
              {selectedPower.mcpServers.length > 0 && (
                <CollapsibleSection title={`MCP Servers (${selectedPower.mcpServers.length})`} icon={<Server size={14} className={accent.text} />} expanded={expandedSections.mcp} onToggle={() => toggleSection('mcp')} colors={colors}>
                  <div className="space-y-2">
                    {selectedPower.mcpServers.map(name => (
                      <div key={name} className={`flex items-center gap-2 px-3 py-2 rounded-lg ${colors.cardSecondary}`}>
                        <Server size={14} className={accent.textSoft} />
                        <code className={`text-sm ${colors.text}`}>{name}</code>
                      </div>
                    ))}
                  </div>
                </CollapsibleSection>
              )}
              {selectedPower.steeringFiles.length > 0 && (
                <CollapsibleSection title={`Steering Files (${selectedPower.steeringFiles.length})`} icon={<FileText size={14} className={accent.text} />} expanded={expandedSections.steering} onToggle={() => toggleSection('steering')} colors={colors}>
                  <div className="space-y-2">
                    {selectedPower.steeringFiles.map(name => (
                      <div key={name} className={`flex items-center gap-2 px-3 py-2 rounded-lg ${colors.cardSecondary}`}>
                        <FileText size={14} className={accent.textSoft} />
                        <code className={`text-sm ${colors.text}`}>{name}</code>
                      </div>
                    ))}
                  </div>
                </CollapsibleSection>
              )}
              {selectedPower.powerMd && (
                <CollapsibleSection title="POWER.md" icon={<FileText size={14} className={accent.text} />} expanded={expandedSections.md} onToggle={() => toggleSection('md')} colors={colors}>
                  <pre className={`text-xs leading-relaxed whitespace-pre-wrap font-mono p-3 rounded-lg ${colors.textMuted}`}
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
            <div className={`p-6 border-b ${colors.cardBorder}`}>
              <div className="flex items-start gap-4">
                {selectedRec.iconUrl ? (
                  <img src={selectedRec.iconUrl} alt="" className="w-14 h-14 rounded-xl flex-shrink-0 object-contain shadow-lg" onError={(e) => { e.target.style.display='none' }} />
                ) : (
                  <div className={`w-14 h-14 rounded-xl ${accent.iconBadgeBg} flex items-center justify-center flex-shrink-0`}>
                    <Zap size={28} className={accent.text} />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <h3 className={`font-bold text-xl ${colors.text}`}>{selectedRec.displayName || selectedRec.name}</h3>
                  {selectedRec.author && (
                    <p className={`text-sm ${colors.textMuted} mt-1`}>by {selectedRec.author}</p>
                  )}
                  {selectedRec.description && (
                    <p className={`text-sm mt-3 ${colors.textMuted} leading-relaxed`}>{selectedRec.description}</p>
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
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-all active:scale-[0.98] cursor-pointer disabled:opacity-60 disabled:cursor-not-allowed ${accentGradientButtonClass}`}
                  >
                    {installing === selectedRec.name ? (
                      <><RefreshCw size={16} className="animate-spin" />{t('powers.installing')}</>
                    ) : (
                      <><Download size={16} />{t('powers.install')}</>
                    )}
                  </button>
                )}
                {selectedRec.repositoryUrl && (
                  <a
                    href={selectedRec.repositoryUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium border ${colors.cardBorder} ${colors.text} ${colors.cardHover} transition-all cursor-pointer`}
                  >
                    <ExternalLink size={14} /> GitHub
                  </a>
                )}
              </div>
            </div>

            {/* 详细信息 */}
            <div className="p-6 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                {selectedRec.author && (
                  <div>
                    <span className={`text-xs ${colors.textMuted} block mb-1`}>{t('powers.author')}</span>
                    <span className={`text-sm font-medium ${colors.text}`}>{selectedRec.author}</span>
                  </div>
                )}
                {selectedRec.license && (
                  <div>
                    <span className={`text-xs ${colors.textMuted} block mb-1`}>{t('powers.license')}</span>
                    <span className={`text-sm font-medium ${colors.text}`}>{selectedRec.license}</span>
                  </div>
                )}
                <div>
                  <span className={`text-xs ${colors.textMuted} block mb-1`}>{t('powers.name')}</span>
                  <code className={`text-sm ${colors.text}`}>{selectedRec.name}</code>
                </div>
              </div>

              {/* 第三方免责 */}
              <div className={`mt-6 p-4 rounded-xl border ${colors.cardBorder} ${colors.cardSecondary}`}>
                <p className={`text-xs ${colors.textMuted} leading-relaxed`}>
                  {t('powers.thirdPartyDisclaimer')}
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div className={`flex-1 flex items-center justify-center ${colors.textMuted}`}>
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
function CollapsibleSection({ title, icon, expanded, onToggle, colors, children }) {
  return (
    <div className={`border-b ${colors.cardBorder}`}>
      <button
        onClick={onToggle}
        className={`w-full flex items-center gap-2 px-4 py-3 ${colors.cardHover} transition-colors`}
      >
        {expanded ? <ChevronDown size={14} className={colors.textMuted} /> : <ChevronRight size={14} className={colors.textMuted} />}
        {icon}
        <span className={`text-sm font-medium ${colors.text}`}>{title}</span>
      </button>
      {expanded && <div className="px-4 pb-3">{children}</div>}
    </div>
  )
}

export default PowersPanel
