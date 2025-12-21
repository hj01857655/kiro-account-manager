import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { X, Copy, Check, RefreshCw, User, CreditCard, Key, Clock, ChevronDown, ChevronUp, Shield } from 'lucide-react'
import { useTheme } from '../contexts/ThemeContext'
import { useDialog } from '../contexts/DialogContext'
import { useI18n } from '../i18n'

function AccountDetailModal({ account, onClose }) {
  const { theme, colors } = useTheme()
  const { showError } = useDialog()
  const { t } = useI18n()
  const isDark = theme === 'dark'
  const initQuota = account.usageData?.usageBreakdownList?.[0]?.usageLimit ?? account.quota ?? 50
  const initUsed = account.usageData?.usageBreakdownList?.[0]?.currentUsage ?? account.used ?? 0
  const [form, setForm] = useState({
    email: account.email,
    label: account.label || '',
    quota: initQuota,
    used: initUsed,
    status: account.status,
    accessToken: account.accessToken || '',
    refreshToken: account.refreshToken || '',
  })

  const [refreshing, setRefreshing] = useState(false)
  const [copied, setCopied] = useState(null)
  const [showTokens, setShowTokens] = useState(true)



  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const updated = await invoke('sync_account', { id: account.id })
      const quota = updated.usageData?.usageBreakdownList?.[0]?.usageLimit ?? 50
      const used = updated.usageData?.usageBreakdownList?.[0]?.currentUsage ?? 0
      setForm(prev => ({ ...prev, quota, used, status: updated.status }))
    } catch (e) {
      await showError(t('detail.refreshFailed'), e.toString())
    } finally {
      setRefreshing(false)
    }
  }

  const handleCopy = (text, field) => {
    navigator.clipboard.writeText(text)
    setCopied(field)
    setTimeout(() => setCopied(null), 1500)
  }

  // 从 usageData 读取免费试用和奖励信息
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  const freeTrialInfo = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const freeTrialQuota = freeTrialInfo?.usageLimit || 0
  const freeTrialUsed = freeTrialInfo?.currentUsage || 0
  const bonusQuota = bonuses.reduce((sum, b) => sum + (b.usageLimit || 0), 0)
  const bonusUsed = bonuses.reduce((sum, b) => sum + (b.currentUsage || 0), 0)
  
  const totalQuota = form.quota + freeTrialQuota + bonusQuota
  const totalUsed = form.used + freeTrialUsed + bonusUsed
  const totalPercent = totalQuota > 0 ? Math.min(100, (totalUsed / totalQuota) * 100) : 0

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 animate-fade-in" onClick={onClose}>
      <div 
        className={`${isDark ? 'bg-[#1a1a2e]' : 'bg-gray-50'} rounded-2xl w-full max-w-2xl shadow-2xl max-h-[90vh] overflow-hidden flex flex-col`} 
        onClick={e => e.stopPropagation()}
        style={{ animation: 'modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1)' }}
      >
        {/* Header */}
        <div className={`flex items-center justify-between px-6 py-4 ${colors.card} border-b ${colors.cardBorder}`}>
          <div className="flex items-center gap-4">
            <div className={`w-12 h-12 rounded-xl flex items-center justify-center ${account.provider === 'Google' ? (isDark ? 'bg-red-500/20' : 'bg-red-100') : account.provider === 'Github' ? (isDark ? 'bg-gray-600' : 'bg-gray-200') : (isDark ? 'bg-blue-500/20' : 'bg-blue-100')}`}>
              <User size={24} className={account.provider === 'Google' ? (isDark ? 'text-red-400' : 'text-red-600') : account.provider === 'Github' ? (isDark ? 'text-gray-300' : 'text-gray-700') : (isDark ? 'text-blue-400' : 'text-blue-600')} />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className={`text-lg font-semibold ${colors.text}`}>{account.email}</h2>
                <span className={`px-2 py-0.5 rounded text-xs font-medium ${(account.usageData?.subscriptionInfo?.type?.includes('PRO+') || account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO+')) ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white' : (account.usageData?.subscriptionInfo?.type?.includes('PRO') || account.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO')) ? 'bg-blue-500 text-white' : (isDark ? 'bg-gray-700 text-gray-300' : 'bg-gray-200 text-gray-600')}`}>
                  {account.usageData?.subscriptionInfo?.subscriptionTitle || 'Free'}
                </span>
              </div>
              <p className={`text-sm ${colors.textMuted}`}>{account.provider || t('common.unknown')} · {t('detail.addedAt')} {account.addedAt?.split(' ')[0]}</p>
            </div>
          </div>
          <button onClick={onClose} className={`p-2 ${isDark ? 'hover:bg-white/10' : 'hover:bg-gray-100'} rounded-xl transition-all`}>
            <X size={20} className={colors.textMuted} />
          </button>
        </div>
        
        <div className="flex-1 overflow-y-auto">
          <div className="p-6 space-y-4">
            {/* 配额总览 */}
            <div className={`${colors.card} rounded-xl p-5 shadow-sm`}>
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2">
                  <CreditCard size={18} className={colors.textMuted} />
                  <span className={`font-medium ${colors.text}`}>{t('detail.quotaOverview')}</span>
                </div>
                <button type="button" onClick={handleRefresh} disabled={refreshing} className={`p-2 ${isDark ? 'bg-blue-500/20 hover:bg-blue-500/30' : 'bg-blue-50 hover:bg-blue-100'} rounded-lg transition-colors disabled:opacity-50`} title={t('detail.syncQuota')}>
                  <RefreshCw size={16} className={`text-blue-500 ${refreshing ? 'animate-spin' : ''}`} />
                </button>
              </div>
              
              <div className="mb-4">
                <div className="flex items-baseline justify-between mb-2">
                  <div>
                    <span className={`text-3xl font-bold ${colors.text}`}>{totalUsed}</span>
                    <span className={`${colors.textMuted} ml-1`}>/ {totalQuota}</span>
                  </div>
                  <span className={`text-sm font-medium ${totalPercent > 80 ? 'text-red-500' : totalPercent > 50 ? 'text-yellow-600' : 'text-green-600'}`}>
                    {totalPercent.toFixed(0)}% {t('detail.used')}
                  </span>
                </div>
                <div className={`h-3 ${isDark ? 'bg-white/10' : 'bg-gray-100'} rounded-full overflow-hidden`}>
                  <div className={`h-full rounded-full transition-all duration-500 ${totalPercent > 80 ? 'bg-gradient-to-r from-red-400 to-red-500' : totalPercent > 50 ? 'bg-gradient-to-r from-yellow-400 to-orange-500' : 'bg-gradient-to-r from-green-400 to-emerald-500'}`} style={{ width: `${totalPercent}%` }} />
                </div>
              </div>
              
              <div className="grid grid-cols-3 gap-3">
                <div className={`${isDark ? 'bg-white/5' : 'bg-gray-50'} rounded-lg p-3`}>
                  <div className="flex items-center gap-1.5 mb-1">
                    <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
                    <span className={`text-xs ${colors.textMuted}`}>{t('detail.mainQuota')}</span>
                  </div>
                  <div className={`text-lg font-semibold ${colors.text}`} title={breakdown?.currentUsageWithPrecision != null ? `${t('detail.precise')}: ${breakdown.currentUsageWithPrecision} / ${breakdown.usageLimitWithPrecision}` : undefined}>{form.used} / {form.quota}</div>
                  {breakdown?.nextDateReset && <div className={`text-xs ${colors.textMuted} mt-1`}>{new Date(breakdown.nextDateReset * 1000).toLocaleDateString()} {t('detail.reset')}</div>}
                </div>
                
                <div className={`rounded-lg p-3 ${freeTrialQuota && freeTrialInfo?.freeTrialStatus === 'ACTIVE' ? (isDark ? 'bg-cyan-500/20' : 'bg-cyan-50') : (isDark ? 'bg-white/5' : 'bg-gray-50')}`}>
                  <div className="flex items-center gap-1.5 mb-1">
                    <div className={`w-2 h-2 rounded-full ${freeTrialInfo?.freeTrialStatus === 'ACTIVE' ? 'bg-cyan-500' : 'bg-gray-300'}`}></div>
                    <span className={`text-xs ${colors.textMuted}`}>{t('detail.freeTrial')}</span>
                    {freeTrialInfo?.freeTrialStatus && <span className={`text-xs ${freeTrialInfo.freeTrialStatus === 'ACTIVE' ? 'text-cyan-500' : colors.textMuted}`}>({freeTrialInfo.freeTrialStatus})</span>}
                  </div>
                  <div className={`text-lg font-semibold ${colors.text}`} title={freeTrialInfo?.currentUsageWithPrecision != null ? `${t('detail.precise')}: ${freeTrialInfo.currentUsageWithPrecision} / ${freeTrialInfo.usageLimitWithPrecision}` : undefined}>{freeTrialQuota ? `${freeTrialUsed} / ${freeTrialQuota}` : '-'}</div>
                  {freeTrialInfo?.freeTrialExpiry && <div className={`text-xs ${colors.textMuted} mt-1`}>{new Date(freeTrialInfo.freeTrialExpiry * 1000).toLocaleDateString()} {t('detail.expires')}</div>}
                </div>
                
                <div className={`rounded-lg p-3 ${bonusQuota ? (isDark ? 'bg-purple-500/20' : 'bg-purple-50') : (isDark ? 'bg-white/5' : 'bg-gray-50')}`}>
                  <div className="flex items-center gap-1.5 mb-1">
                    <div className={`w-2 h-2 rounded-full ${bonusQuota ? 'bg-purple-500' : 'bg-gray-300'}`}></div>
                    <span className={`text-xs ${colors.textMuted}`}>{t('detail.bonusTotal')}</span>
                  </div>
                  <div className={`text-lg font-semibold ${colors.text}`}>{bonusQuota ? `${Math.round(bonusUsed)} / ${Math.round(bonusQuota)}` : '-'}</div>
                  {bonuses.length > 0 && <div className={`text-xs ${colors.textMuted} mt-1`}>{bonuses.length} {t('detail.bonusCount')}</div>}
                </div>
              </div>
              
              {/* Bonuses 列表 */}
              {bonuses.length > 0 && (
                <div className={`mt-4 pt-4 border-t ${colors.cardBorder}`}>
                  <div className={`text-xs font-medium ${colors.textMuted} mb-2`}>{t('detail.bonusDetails')}</div>
                  <div className="space-y-2">
                    {bonuses.map((bonus, idx) => (
                      <div key={idx} className={`flex items-center justify-between p-2.5 rounded-lg ${bonus.status === 'ACTIVE' ? (isDark ? 'bg-purple-500/10' : 'bg-purple-50') : bonus.status === 'EXHAUSTED' ? (isDark ? 'bg-gray-500/10' : 'bg-gray-100') : (isDark ? 'bg-white/5' : 'bg-gray-50')}`}>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className={`text-sm font-medium ${colors.text}`}>{bonus.displayName || bonus.bonusCode}</span>
                            <span className={`text-xs px-1.5 py-0.5 rounded ${bonus.status === 'ACTIVE' ? 'bg-green-500/20 text-green-500' : bonus.status === 'EXHAUSTED' ? 'bg-gray-500/20 text-gray-500' : 'bg-yellow-500/20 text-yellow-600'}`}>{bonus.status}</span>
                          </div>
                          <div className={`text-xs ${colors.textMuted} mt-0.5`}>
                            {bonus.description && <span>{bonus.description} · </span>}
                            {bonus.redeemedAt && <span>{t('detail.redeemed')}: {new Date(bonus.redeemedAt * 1000).toLocaleDateString()} · </span>}
                            {bonus.expiresAt && <span>{t('detail.expires')}: {new Date(bonus.expiresAt * 1000).toLocaleDateString()}</span>}
                          </div>
                        </div>
                        <div className="text-right ml-3">
                          <div className={`text-sm font-semibold ${colors.text}`}>{Math.round(bonus.currentUsage || 0)} / {Math.round(bonus.usageLimit || 0)}</div>
                          <div className={`text-xs ${colors.textMuted}`}>{bonus.bonusCode}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              
              {/* 订阅信息 */}
              <div className={`mt-4 pt-4 border-t ${colors.cardBorder} grid grid-cols-2 gap-x-6 gap-y-2 text-sm`}>
                <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.userId')}</span><span className={`${colors.text} font-mono text-xs truncate max-w-[150px]`} title={account.usageData?.userInfo?.userId}>{account.usageData?.userInfo?.userId?.slice(-12) || '-'}</span></div>
                <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.email')}</span><span className={`${colors.text} text-xs`}>{account.usageData?.userInfo?.email || account.email}</span></div>
                <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.subscriptionType')}</span><span className={`${colors.text} font-mono text-xs truncate max-w-[150px]`} title={account.usageData?.subscriptionInfo?.type}>{account.usageData?.subscriptionInfo?.type || '-'}</span></div>
                <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.upgradeable')}</span><span className={colors.text}>{account.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? t('common.yes') : t('common.no')}</span></div>
                {breakdown?.overageRate && (
                  <>
                    <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.overageRate')}</span><span className={colors.text}>${breakdown.overageRate}/{breakdown.unit || t('detail.perUnit')}</span></div>
                    <div className="flex justify-between"><span className={colors.textMuted}>{t('detail.overageCap')}</span><span className={colors.text}>{breakdown.overageCap}</span></div>
                  </>
                )}
              </div>
            </div>

            {/* 基本信息 */}
            <div className={`${colors.card} rounded-xl p-5 shadow-sm`}>
              <div className="flex items-center gap-2 mb-4">
                <User size={18} className={colors.textMuted} />
                <span className={`font-medium ${colors.text}`}>{t('detail.basicInfo')}</span>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('detail.emailAddress')}</label>
                  <input type="email" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} className={`w-full px-3 py-2 border ${colors.cardBorder} rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 ${colors.input} ${colors.text}`} required />
                </div>
                <div>
                  <label className={`block text-xs font-medium ${colors.textMuted} mb-1.5`}>{t('detail.remarkLabel')}</label>
                  <input 
                    type="text" 
                    value={form.label} 
                    readOnly
                    placeholder={t('common.none')} 
                    className={`w-full px-3 py-2 border ${colors.cardBorder} rounded-lg text-sm ${colors.input} ${colors.text} opacity-60`} 
                  />
                </div>
              </div>
              

            </div>

            {/* account */}
            <div className={`${colors.card} rounded-xl shadow-sm overflow-hidden`}>
              <div className={`flex items-center justify-between px-5 py-4 cursor-pointer ${isDark ? 'hover:bg-white/5' : 'hover:bg-gray-50'} transition-colors`} onClick={() => setShowTokens(!showTokens)}>
                <div className="flex items-center gap-2">
                  <Key size={18} className={colors.textMuted} />
                  <span className={`font-medium ${colors.text}`}>{t('detail.tokenCredentials')}</span>
                </div>
                <div className="flex items-center gap-3">
                  {account.expiresAt && <span className={`text-xs ${colors.textMuted} flex items-center gap-1`}><Clock size={12} />{account.expiresAt}</span>}
                  {showTokens ? <ChevronUp size={16} className={colors.textMuted} /> : <ChevronDown size={16} className={colors.textMuted} />}
                </div>
              </div>
              
              {showTokens && (
                <div className={`px-5 pb-5 space-y-3 border-t ${colors.cardBorder} pt-4`}>
                    <div>
                      <div className="flex items-center justify-between mb-1.5">
                        <span className={`text-xs font-medium ${colors.textMuted}`}>Access Token</span>
                        <button type="button" onClick={() => handleCopy(form.accessToken, 'access')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                          {copied === 'access' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                          {copied === 'access' ? t('common.copied') : t('common.copy')}
                        </button>
                      </div>
                      <textarea value={form.accessToken} onChange={(e) => setForm({ ...form, accessToken: e.target.value })} placeholder={account.provider === 'BuilderId' ? t('detail.aoaPrefix') : t('detail.eyjPrefix')} className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg resize-none h-14 focus:outline-none focus:ring-2 focus:ring-blue-500/20 ${colors.text}`} />
                    </div>
                    <div>
                      <div className="flex items-center justify-between mb-1.5">
                        <span className={`text-xs font-medium ${colors.textMuted}`}>Refresh Token</span>
                        <button type="button" onClick={() => handleCopy(form.refreshToken, 'refresh')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                          {copied === 'refresh' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                          {copied === 'refresh' ? t('common.copied') : t('common.copy')}
                        </button>
                      </div>
                      <textarea value={form.refreshToken} onChange={(e) => setForm({ ...form, refreshToken: e.target.value })} placeholder={account.provider === 'BuilderId' ? t('detail.aorPrefix') : 'refresh token'} className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg resize-none h-14 focus:outline-none focus:ring-2 focus:ring-blue-500/20 ${colors.text}`} />
                    </div>
                    
                    {/* IdC (BuilderId) 专用字段 */}
                    {account.provider === 'BuilderId' && (
                      <div className={`pt-3 border-t ${colors.cardBorder} space-y-3`}>
                        <div className={`text-xs font-medium ${colors.textMuted} flex items-center gap-1`}>
                          <Shield size={12} />
                          {t('detail.ssoCredentials')}
                        </div>
                        <div>
                          <div className="flex items-center justify-between mb-1">
                            <label className={`text-xs ${colors.textMuted}`}>Client ID Hash</label>
                            <button type="button" onClick={() => handleCopy(account.clientIdHash, 'clientIdHash')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                              {copied === 'clientIdHash' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                            </button>
                          </div>
                          <input type="text" value={account.clientIdHash || '-'} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                        </div>
                        <div className="grid grid-cols-2 gap-3">
                          <div>
                            <label className={`block text-xs ${colors.textMuted} mb-1`}>Region</label>
                            <input type="text" value={account.region || 'us-east-1'} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                          </div>
                          <div>
                            <label className={`block text-xs ${colors.textMuted} mb-1`}>Session ID</label>
                            <input type="text" value={account.ssoSessionId || '-'} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60 truncate`} />
                          </div>
                        </div>
                        <div>
                          <div className="flex items-center justify-between mb-1">
                            <label className={`text-xs ${colors.textMuted}`}>Client ID</label>
                            <button type="button" onClick={() => handleCopy(account.clientId, 'clientId')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                              {copied === 'clientId' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                            </button>
                          </div>
                          <input type="text" value={account.clientId || ''} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                        </div>
                        <div>
                          <div className="flex items-center justify-between mb-1">
                            <label className={`text-xs ${colors.textMuted}`}>Client Secret</label>
                            <button type="button" onClick={() => handleCopy(account.clientSecret, 'clientSecret')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                              {copied === 'clientSecret' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                            </button>
                          </div>
                          <textarea value={account.clientSecret || ''} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg resize-none h-14 ${colors.text} opacity-60`} />
                        </div>
                      </div>
                    )}
                    
                    {/* Social 专用字段 */}
                    {(account.provider === 'Google' || account.provider === 'Github') && (
                      <div className={`pt-3 border-t ${colors.cardBorder} space-y-3`}>
                        {account.profileArn && (
                          <div>
                            <div className="flex items-center justify-between mb-1">
                              <label className={`text-xs ${colors.textMuted}`}>Profile ARN</label>
                              <button type="button" onClick={() => handleCopy(account.profileArn, 'profileArn')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                                {copied === 'profileArn' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                              </button>
                            </div>
                            <input type="text" value={account.profileArn} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                          </div>
                        )}
                        {account.csrfToken && (
                          <div>
                            <div className="flex items-center justify-between mb-1">
                              <label className={`text-xs ${colors.textMuted}`}>CSRF Token</label>
                              <button type="button" onClick={() => handleCopy(account.csrfToken, 'csrfToken')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                                {copied === 'csrfToken' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                              </button>
                            </div>
                            <input type="text" value={account.csrfToken} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                          </div>
                        )}
                        {account.sessionToken && (
                          <div>
                            <div className="flex items-center justify-between mb-1">
                              <label className={`text-xs ${colors.textMuted}`}>Session Token</label>
                              <button type="button" onClick={() => handleCopy(account.sessionToken, 'sessionToken')} className={`text-xs ${colors.textMuted} hover:text-blue-500 flex items-center gap-1`}>
                                {copied === 'sessionToken' ? <Check size={12} className="text-green-500" /> : <Copy size={12} />}
                              </button>
                            </div>
                            <input type="text" value={account.sessionToken} readOnly className={`w-full px-3 py-2 text-xs font-mono ${isDark ? 'bg-white/5' : 'bg-gray-50'} border ${colors.cardBorder} rounded-lg ${colors.text} opacity-60`} />
                          </div>
                        )}
                      </div>
                    )}
                    

                </div>
              )}
            </div>
          </div>

          {/* Footer */}
          <div className={`flex justify-between items-center px-6 py-4 ${colors.card} border-t ${colors.cardBorder}`}>
            <div className={`text-xs ${colors.textMuted}`}>
              {account.status === '正常' || account.status === '有效' || account.status === 'active'
                ? <span className="flex items-center gap-1 text-green-500"><Shield size={12} />{t('detail.accountNormal')}</span> 
                : account.status === '封禁' || account.status === '已封禁' || account.status === 'banned'
                  ? <span className="flex items-center gap-1 text-red-500"><Shield size={12} />{t('detail.accountBanned')}</span>
                  : <span className="flex items-center gap-1 text-orange-500"><Shield size={12} />{account.status}</span>}
            </div>
            <button type="button" onClick={onClose} className="px-5 py-2 bg-blue-500 text-white rounded-lg text-sm font-medium hover:bg-blue-600">
              {t('common.close')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

export default AccountDetailModal

// 添加动画样式
const style = document.createElement('style')
style.textContent = `
  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(-20px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  .animate-fade-in {
    animation: fade-in 0.2s ease-out;
  }
`
if (!document.querySelector('#edit-modal-styles')) {
  style.id = 'edit-modal-styles'
  document.head.appendChild(style)
}
