import { RefreshCw, Eye, Trash2, Copy, Check, Clock, Repeat, Edit2 } from 'lucide-react'
import { useTheme } from '../../contexts/ThemeContext'
import { useI18n } from '../../i18n.jsx'
import { getUsagePercent, getProgressBarColor } from './hooks/useAccountStats'
import { getQuota, getUsed, getSubType, getSubPlan } from '../../utils/accountStats'

function AccountCard({
  account,
  isSelected,
  onSelect,
  copiedId,
  onCopy,
  onSwitch,
  onRefresh,
  onEdit,
  onEditLabel,
  onDelete,
  refreshingId,
  switchingId,
  isCurrentAccount,
}) {
  const { theme, colors } = useTheme()
  const { t } = useI18n()
  const isDark = theme === 'dark'
  
  const quota = getQuota(account)
  const used = getUsed(account)
  const subType = getSubType(account)
  const subPlan = getSubPlan(account)
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  const percent = getUsagePercent(used, quota)
  const isExpired = account.expiresAt && new Date(account.expiresAt.replace(/\//g, '-')) < new Date()
  const isBanned = account.status === '封禁' || account.status === '已封禁' || account.status === 'banned'
  const isNormal = account.status === '正常' || account.status === '有效' || account.status === 'active'

  // 状态光环颜色
  const glowColor = isCurrentAccount
    ? 'shadow-green-500/30 hover:shadow-green-500/50'
    : isBanned
      ? 'shadow-red-500/30 hover:shadow-red-500/50'
      : isNormal
        ? ''
        : 'shadow-orange-500/30 hover:shadow-orange-500/50'

  // 状态显示文本
  const statusText = isNormal ? t('accounts.active') : isBanned ? t('accounts.banned') : account.status

  return (
    <div className={`relative rounded-2xl border transition-all duration-200 hover:shadow-lg flex flex-col ${glowColor} ${
      isSelected 
        ? (isDark ? 'border-purple-500 bg-purple-500/10' : 'border-purple-400 bg-purple-50') 
        : isCurrentAccount
          ? (isDark ? 'border-green-500/50 bg-green-500/5' : 'border-green-400 bg-green-50/50')
          : isBanned
            ? (isDark ? 'border-red-500/50 bg-red-500/5' : 'border-red-300 bg-red-50/50')
            : !isNormal
              ? (isDark ? 'border-orange-500/50 bg-orange-500/5' : 'border-orange-300 bg-orange-50/50')
              : (isDark ? 'border-gray-700 bg-gray-800/50 hover:border-gray-600' : 'border-gray-200 bg-white hover:border-gray-300')
    }`}>
      {/* 选择框和当前使用标记 */}
      <div className="absolute top-3 left-3 flex items-center gap-2">
        <input 
          type="checkbox" 
          checked={isSelected} 
          onChange={(e) => onSelect(e.target.checked)} 
          className="w-4 h-4 rounded transition-transform hover:scale-110 cursor-pointer" 
        />
      </div>
      
      {/* 状态标签 */}
      <div className="absolute top-3 right-3 flex items-center gap-2">
        <span className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
          isNormal
            ? (isDark ? 'bg-green-500/20 text-green-400' : 'bg-green-100 text-green-700')
            : isBanned
              ? (isDark ? 'bg-red-500/20 text-red-400' : 'bg-red-100 text-red-600')
              : (isDark ? 'bg-orange-500/20 text-orange-400' : 'bg-orange-100 text-orange-600')
        }`}>{statusText}</span>
      </div>

      <div className="p-4 pt-10 flex-1 flex flex-col">
        {/* 头像和邮箱 */}
        <div className="flex items-start gap-3 mb-3">
          <div className={`w-10 h-10 rounded-xl flex items-center justify-center text-sm font-bold shadow-sm ${
            account.provider === 'Google' ? (isDark ? 'bg-red-500/20 text-red-400' : 'bg-red-100 text-red-600') :
            account.provider === 'Github' ? (isDark ? 'bg-gray-600 text-gray-200' : 'bg-gray-200 text-gray-700') :
            (isDark ? 'bg-blue-500/20 text-blue-400' : 'bg-blue-100 text-blue-600')
          }`}>
            {account.email[0].toUpperCase()}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1">
              <span className={`font-medium ${colors.text} text-sm truncate`}>{account.email}</span>
              <button 
                onClick={() => onCopy(account.email, account.id)} 
                className="btn-icon p-1 rounded hover:bg-gray-100 dark:hover:bg-white/10 flex-shrink-0"
              >
                {copiedId === account.id ? <Check size={12} className="text-green-500" /> : <Copy size={12} className="text-gray-400" />}
              </button>
            </div>
            <div className={`text-xs ${colors.textMuted}`}>{account.label || account.provider || t('common.noLabel')}</div>
          </div>
        </div>

        {/* 订阅类型和登录方式 */}
        <div className="flex items-center gap-2 mb-3">
          <span className={`inline-flex px-2 py-1 rounded-lg text-xs font-medium ${
            (subType.includes('PRO+') || subPlan.includes('PRO+'))
              ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white shadow-sm'
              : (subType.includes('PRO') || subPlan.includes('PRO'))
                ? 'bg-gradient-to-r from-blue-500 to-indigo-500 text-white shadow-sm'
                : isDark ? 'bg-gray-700 text-gray-300' : 'bg-gray-100 text-gray-600'
          }`}>
            {subPlan || 'Free'}
          </span>
          <span className={`text-xs px-2 py-1 rounded-lg ${isDark ? 'bg-gray-700 text-gray-400' : 'bg-gray-100 text-gray-500'}`}>
            {account.provider || t('common.unknown')}
          </span>
          {isCurrentAccount && (
            <span className="text-xs px-2 py-1 rounded-lg bg-gradient-to-r from-green-500 to-emerald-500 text-white font-medium">
              {t('common.currentlyUsing')}
            </span>
          )}
        </div>

        {/* 配额进度 */}
        <div className={`p-3 rounded-xl mb-3 ${isDark ? 'bg-white/5' : 'bg-gray-50'}`}>
          <div className="flex items-center justify-between text-xs mb-2">
            <span className={colors.textMuted}>{t('common.usage')}</span>
            <span className={`font-semibold ${percent > 80 ? 'text-red-500' : percent > 50 ? 'text-yellow-500' : 'text-green-500'}`}>
              {Math.round(percent)}%
            </span>
          </div>
          <div className={`h-2 ${isDark ? 'bg-white/10' : 'bg-gray-200'} rounded-full overflow-hidden mb-2`}>
            <div 
              className={`h-full rounded-full transition-all duration-500 ${getProgressBarColor(percent)}`} 
              style={{ width: `${Math.min(percent, 100)}%` }} 
            />
          </div>
          <div className="flex items-center justify-between text-xs">
            <span className={`font-medium ${isDark ? 'text-gray-300' : 'text-gray-700'}`}>{Math.round(used * 100) / 100} / {quota}</span>
            <span className={colors.textMuted}>{t('common.remaining')} {Math.round((quota - used) * 100) / 100}</span>
          </div>
          {breakdown?.nextDateReset && (
            <div className={`text-xs ${colors.textMuted} mt-1 flex items-center gap-1`}>
              <Clock size={10} />
              {new Date(breakdown.nextDateReset * 1000).toLocaleDateString()} {t('common.reset')}
            </div>
          )}
        </div>

        {/* Token 过期时间 */}
        {account.expiresAt && (
          <div className={`text-xs ${isExpired ? 'text-red-500' : colors.textMuted} mb-3 flex items-center gap-1`}>
            <Clock size={12} />
            Token: {account.expiresAt}
            {isExpired && <span className="text-red-500 font-medium ml-1">{t('accountCard.tokenExpired')}</span>}
          </div>
        )}

        {/* 操作按钮 */}
        <div className="flex items-center justify-center gap-2 pt-2 mt-auto border-t border-gray-200 dark:border-gray-700">
          <button 
            onClick={() => onSwitch(account)} 
            disabled={switchingId === account.id} 
            className={`p-2 rounded-lg transition-all ${isDark ? 'hover:bg-white/10' : 'hover:bg-gray-100'} disabled:opacity-50`}
            title={t('accountCard.switchAccount')}
          >
            <Repeat size={14} className={`text-blue-500 ${switchingId === account.id ? 'animate-spin' : ''}`} />
          </button>
          <button 
            onClick={() => onRefresh(account.id)} 
            disabled={refreshingId === account.id} 
            className={`p-2 rounded-lg transition-all ${isDark ? 'hover:bg-white/10' : 'hover:bg-gray-100'}`} 
            title={t('accountCard.refresh')}
          >
            <RefreshCw size={14} className={`${colors.textMuted} ${refreshingId === account.id ? 'animate-spin' : ''}`} />
          </button>
          <button 
            onClick={() => onEdit(account)} 
            className={`p-2 rounded-lg transition-all ${isDark ? 'hover:bg-white/10' : 'hover:bg-gray-100'}`} 
            title={t('accountCard.viewDetails')}
          >
            <Eye size={14} className={colors.textMuted} />
          </button>
          <button 
            onClick={() => onEditLabel(account)} 
            className={`p-2 rounded-lg transition-all ${isDark ? 'hover:bg-white/10' : 'hover:bg-gray-100'}`} 
            title={t('accountCard.editRemark')}
          >
            <Edit2 size={14} className={colors.textMuted} />
          </button>
          <button 
            onClick={() => onDelete(account.id)} 
            className={`p-2 rounded-lg transition-all ${isDark ? 'hover:bg-red-500/20' : 'hover:bg-red-50'}`} 
            title={t('accountCard.delete')}
          >
            <Trash2 size={14} className="text-red-400" />
          </button>
        </div>
      </div>
    </div>
  )
}

export default AccountCard
