import { RefreshCw } from 'lucide-react'
import { useMemo } from 'react'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../AgentConfig/themeAccent'

function RefreshProgressModal({ refreshProgress }) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  if (!refreshProgress || refreshProgress.total === 0) return null

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div 
        className={`surface-card rounded-2xl w-[400px] shadow-2xl overflow-hidden`}
        style={{ animation: 'modalBounceIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)' }}
      >
        <div className={`px-5 py-4 border-b border-border bg-muted/30 flex items-center gap-2`}>
          <RefreshCw size={18} className={`${accent.text} animate-spin`} />
          <h2 className={`font-semibold text-foreground`}>{t('refresh.title')}</h2>
        </div>
        <div className="p-5 space-y-4">
          <div>
            <div className={`flex justify-between text-sm mb-2`}>
              <span className={"text-muted-foreground"}>{t('refresh.progress')}</span>
              <span className={`${accent.text} font-medium`}>{refreshProgress.current}/{refreshProgress.total}</span>
            </div>
            <div className={`h-2 bg-muted/30 rounded-full overflow-hidden`}>
              <div className={`h-full ${accent.solidBg} rounded-full`} style={{ width: `${(refreshProgress.current / refreshProgress.total) * 100}%` }} />
            </div>
          </div>
          {refreshProgress.currentEmail && (
            <div className={`text-sm text-muted-foreground`}>
              {t('refresh.refreshing')}: <span className={"text-foreground"}>{refreshProgress.currentEmail}</span>
            </div>
          )}
          {refreshProgress.results.length > 0 && (
            <div className="max-h-48 overflow-y-auto space-y-1">
              {refreshProgress.results.map((r, i) => (
                <div key={i} className={`text-xs px-3 py-2 rounded-xl flex justify-between ${r.success ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}`}>
                  <span className="truncate">{r.email}</span>
                  <span>{r.success ? `✓ ${r.message}` : `✗ ${r.message}`}</span>
                </div>
              ))}
            </div>
          )}
          {refreshProgress.current === refreshProgress.total && (
            <div className="text-center text-green-500 font-medium">{t('refresh.complete')}</div>
          )}
        </div>
      </div>
    </div>
  )
}

export default RefreshProgressModal

// 添加动画样式
const style = document.createElement('style')
style.textContent = `
  @keyframes modalBounceIn {
    0% {
      opacity: 0;
      transform: scale(0.8);
    }
    50% {
      transform: scale(1.02);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
`
if (!document.querySelector('#refresh-modal-styles')) {
  style.id = 'refresh-modal-styles'
  document.head.appendChild(style)
}
