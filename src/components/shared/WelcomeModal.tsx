import { useState, useEffect } from 'react'
import { Coffee, X } from 'lucide-react'
import { useApp } from '../../hooks/useApp'
import alipayQR from '../../assets/donate/alipay.jpg'
import wechatQR from '../../assets/donate/wechat.jpg'

function WelcomeModal() {
  const { t } = useApp()
  const [open, setOpen] = useState(false)
  const [previewImg, setPreviewImg] = useState(null)

  useEffect(() => {
    // Show the notice once per day.
    const lastShown = localStorage.getItem('welcome_last_shown')
    const today = new Date().toDateString()
    
    if (lastShown !== today) {
      setOpen(true)
    }
  }, [])

  const handleClose = () => {
    setOpen(false)
    // Record the day only after the user closes the modal.
    const today = new Date().toDateString()
    localStorage.setItem('welcome_last_shown', today)
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
      <div className={`glass-card rounded-2xl w-full max-w-[520px] shadow-2xl border border-border relative overflow-hidden`}>
        {/* Background accent */}
        <div className="absolute top-0 right-0 w-64 h-64 bg-gradient-to-br from-blue-500/10 to-purple-500/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2" />
        
        {/* Close button */}
        <button
          onClick={handleClose}
          className={`absolute right-4 top-4 w-8 h-8 rounded-full flex items-center justify-center transition-colors duration-200 z-10 cursor-pointer focus:ring-2 focus:ring-blue-500/30 hover:bg-muted/50`}
        >
          <X size={18} className={"text-foreground"} />
        </button>

        {/* Header */}
        <div className="px-6 pt-6 pb-4 relative">
          <div className="flex items-center gap-4">
            {/* Logo */}
            <div className="relative flex-shrink-0">
              <div className="w-16 h-16 bg-gradient-to-br from-[#4361ee] to-[#7c3aed] rounded-2xl flex items-center justify-center shadow-lg">
                <svg width="32" height="32" viewBox="0 0 40 40" fill="none">
                  <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white"/>
                </svg>
              </div>
            </div>

            <div className="flex-1">
              <h2 className={`text-xl font-bold text-foreground mb-1`}>
                {t('welcome.importantNotice')}
              </h2>
              <p className={`text-sm text-muted-foreground`}>
                {t('welcome.permanentlyFree')}
              </p>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="px-6 py-4 space-y-4 relative">
          {/* Free software notice */}
          <div className={`border-l-4 border-amber-500 bg-muted/30 rounded-r-xl p-4`}>
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
                <span className="text-lg">⚠️</span>
              </div>
              <div className="flex-1">
                <p className={`text-sm font-semibold text-foreground mb-2`}>
                  {t('welcome.importantNoticeSoftwareFree')}
                </p>
                <div className={`text-xs text-muted-foreground space-y-1.5 leading-relaxed`}>
                  <p>{t('welcome.openSourceFree')}</p>
                  <p>{t('welcome.allFeaturesOpen')}</p>
                  <p>{t('welcome.reportIfCharged')}</p>
                </div>
              </div>
            </div>
          </div>

          {/* Sponsorship */}
          <div>
              <div className="flex items-center gap-2 mb-3">
                <Coffee size={18} className="text-amber-500" />
                <p className={`text-base font-semibold text-foreground`}>
                  {t('welcome.buyMeCoffee')}
                </p>
              </div>
              
              <div className={`bg-muted/30 rounded-xl p-4 space-y-3`}>
                <p className={`text-sm text-muted-foreground`}>
                  {t('welcome.supportMessage')}
                </p>
                
                {/* Sponsor benefits */}
                <div className={`bg-gradient-to-r from-blue-500/10 to-purple-500/10 rounded-lg p-3 space-y-2`}>
                  <p className={`text-xs font-medium text-foreground`}>{t('welcome.sponsorBenefits')}</p>
                  <div className="space-y-1.5">
                    <div className="flex items-center gap-2">
                      <span className="text-base">⚡</span>
                      <p className={`text-sm text-foreground`}>{t('welcome.priorityFeedback')}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-base">🎯</span>
                      <p className={`text-sm text-foreground`}>{t('welcome.priorityFeatures')}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-base">🔧</span>
                      <p className={`text-sm text-foreground`}>{t('welcome.oneOnOneSupport')}</p>
                    </div>
                  </div>
                  <p className={`text-xs text-muted-foreground pt-1 border-t border-border/50`}>
                    {t('welcome.sponsorHint')}
                  </p>
                </div>
                
                {/* QR codes */}
                <div className="flex justify-center gap-8 pt-2">
                  <div 
                    className="flex flex-col items-center gap-2 cursor-pointer group"
                    onClick={() => setPreviewImg(alipayQR)}
                  >
                    <div className="bg-white p-2 rounded-xl shadow-lg group-hover:shadow-xl transition-shadow">
                      <img 
                        src={alipayQR} 
                        alt={t('welcome.alipay')}
                        className="w-24 h-24 rounded-lg"
                      />
                    </div>
                    <p className={`text-sm font-medium text-foreground`}>{t('welcome.alipay')}</p>
                  </div>
                  <div 
                    className="flex flex-col items-center gap-2 cursor-pointer group"
                    onClick={() => setPreviewImg(wechatQR)}
                  >
                    <div className="bg-white p-2 rounded-xl shadow-lg group-hover:shadow-xl transition-shadow">
                      <img 
                        src={wechatQR} 
                        alt={t('welcome.wechatPay')}
                        className="w-24 h-24 rounded-lg"
                      />
                    </div>
                    <p className={`text-sm font-medium text-foreground`}>{t('welcome.wechatPay')}</p>
                  </div>
                </div>
                <p className={`text-xs text-muted-foreground text-center`}>{t('welcome.clickToEnlarge')}</p>
              </div>
            </div>
          
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-border/50 flex justify-end relative">
          <button
            onClick={handleClose}
            className="px-6 py-3 text-sm font-semibold rounded-xl text-white bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 shadow-lg shadow-blue-500/30 hover:shadow-blue-500/40 transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30 active:scale-[0.98]"
          >
            {t('welcome.iUnderstand')}
          </button>
        </div>
      </div>

      {/* Image preview */}
      {previewImg && (
        <div 
          className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-[60] p-4"
          onClick={() => setPreviewImg(null)}
        >
          <div className="relative" onClick={(e) => e.stopPropagation()}>
            <img 
              src={previewImg} 
              alt={t('common.preview', { defaultValue: 'Preview' })}
              className="max-w-[360px] max-h-[360px] rounded-2xl shadow-2xl"
            />
            <button 
              className={`absolute -top-3 -right-3 w-10 h-10 rounded-full flex items-center justify-center shadow-lg transition-colors duration-200 cursor-pointer focus:ring-2 focus:ring-blue-500/30 glass-card border border-border`}
              onClick={() => setPreviewImg(null)}
            >
              <X size={18} className={"text-foreground"} />
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

export default WelcomeModal
