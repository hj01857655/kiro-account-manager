
import { useState, useEffect } from 'react'
import { Coffee, X, Users, ExternalLink, Github } from 'lucide-react'
import { useApp } from '../../hooks/useApp'
import alipayQR from '../../assets/donate/alipay.jpg'
import wechatQR from '../../assets/donate/wechat.jpg'

const TelegramIcon = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.562 8.161c-.18 1.897-.962 6.502-1.359 8.627-.168.9-.5 1.201-.82 1.23-.697.064-1.226-.461-1.901-.903-1.056-.692-1.653-1.123-2.678-1.799-1.185-.781-.417-1.21.258-1.911.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.139-5.062 3.345-.479.329-.913.489-1.302.481-.428-.009-1.252-.242-1.865-.442-.752-.244-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.831-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635.099-.002.321.023.465.141.121.099.155.232.171.326.016.094.036.308.02.475z" />
  </svg>
)

function WelcomeModal() {
  const { t } = useApp()
  const [open, setOpen] = useState(false)
  const [previewImg, setPreviewImg] = useState(null)

  useEffect(() => {
    // 检查今天是否已显示过
    const lastShown = localStorage.getItem('welcome_last_shown')
    const today = new Date().toDateString()

    if (lastShown !== today) {
      // 今天还没显示过，显示弹窗
      setOpen(true)
    }
  }, [])

  const handleClose = () => {
    setOpen(false)
    // 用户关闭弹窗时才记录今天已显示
    const today = new Date().toDateString()
    localStorage.setItem('welcome_last_shown', today)
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
      <div className={`glass-card rounded-2xl w-[760px] max-w-full shadow-2xl border border-border relative overflow-hidden flex flex-col`}>
        {/* 背景装饰 */}
        <div className="absolute top-0 right-0 w-64 h-64 bg-gradient-to-br from-blue-500/10 to-purple-500/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2" />

        {/* 关闭按钮 */}
        <button
          onClick={handleClose}
          className={`absolute right-4 top-4 w-8 h-8 rounded-full flex items-center justify-center transition-colors duration-200 z-10 cursor-pointer focus:ring-2 focus:ring-blue-500/30 hover:bg-muted/50`}
        >
          <X size={18} className={"text-foreground"} />
        </button>

        {/* 头部 */}
        <div className="px-6 pt-6 pb-2 relative">
          <div className="flex items-center gap-4">
            {/* Logo */}
            <div className="relative flex-shrink-0">
              <div className="w-14 h-14 bg-gradient-to-br from-[#4361ee] to-[#7c3aed] rounded-2xl flex items-center justify-center shadow-lg">
                <svg width="28" height="28" viewBox="0 0 40 40" fill="none">
                  <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white" />
                </svg>
              </div>
            </div>

            <div className="flex-1">
              <h2 className={`text-lg font-bold text-foreground mb-0.5`}>
                {t('welcome.appName')}
              </h2>
              <p className={`text-xs text-muted-foreground`}>
                {t('welcome.permanentlyFree')}
              </p>
            </div>
          </div>
        </div>

        {/* 内容 - 左右分栏 */}
        <div className="px-6 py-4 relative grid grid-cols-[1fr_1fr] gap-5">
          {/* 左侧：免费声明 + 联系链接 */}
          <div className="flex flex-col gap-3">
            <div className={`border-l-4 border-amber-500 bg-muted/30 rounded-r-xl p-4`}>
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
                  <span className="text-lg">⚠️</span>
                </div>
                <div className="flex-1">
                  <p className={`text-sm font-semibold text-foreground mb-2`}>
                    {t('welcome.permanentlyFree')}
                  </p>
                  <div className={`text-xs text-muted-foreground space-y-1.5 leading-relaxed`}>
                    <p>{t('welcome.openSourceFree')}</p>
                    <p>{t('welcome.allFeaturesOpen')}</p>
                    <p>{t('welcome.reportIfCharged')}</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 链接区域 */}
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground">{t('welcome.joinUs')}</p>

              <a
                href="https://github.com/hj01857655/kiro-account-manager"
                target="_blank"
                rel="noopener noreferrer"
                className="group flex items-center gap-3 px-3 py-2 rounded-lg border border-border bg-card hover:bg-muted/40 transition-colors"
              >
                <div className="w-8 h-8 rounded-md flex items-center justify-center text-foreground bg-foreground/5">
                  <Github size={15} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium text-foreground truncate">GitHub</div>
                  <div className="text-[11px] text-muted-foreground truncate">kiro-account-manager</div>
                </div>
                <ExternalLink size={13} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
              </a>

              <div className="grid grid-cols-2 gap-2">
                <a
                  href="https://t.me/kiro520"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="group flex items-center gap-2 px-3 py-2 rounded-lg border border-border bg-card hover:bg-muted/40 transition-colors"
                >
                  <div className="w-7 h-7 rounded-md flex items-center justify-center text-blue-500 bg-blue-500/10">
                    <TelegramIcon size={14} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-foreground truncate">{t('welcome.tgChannel')}</div>
                  </div>
                  <ExternalLink size={13} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
                </a>
                <a
                  href="https://t.me/ide520"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="group flex items-center gap-2 px-3 py-2 rounded-lg border border-border bg-card hover:bg-muted/40 transition-colors"
                >
                  <div className="w-7 h-7 rounded-md flex items-center justify-center text-blue-500 bg-blue-500/10">
                    <TelegramIcon size={14} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-foreground truncate">{t('welcome.tgGroup')}</div>
                  </div>
                  <ExternalLink size={13} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
                </a>
              </div>
            </div>
          </div>

          {/* 右侧：赞助信息 */}
          <div className="flex flex-col gap-3">
            <div className="bg-muted/30 rounded-xl p-4 flex flex-col">
              <div className="flex items-center gap-2 mb-2">
                <Coffee size={16} className="text-amber-500" />
                <p className={`text-sm font-semibold text-foreground`}>
                  {t('welcome.buyMeCoffee')}
                </p>
              </div>

              <p className={`text-xs text-muted-foreground mb-3`}>
                {t('welcome.supportMessage')}
              </p>

              {/* 二维码 */}
              <div className="flex justify-center gap-3 mb-2">
                <button
                  onClick={() => setPreviewImg(alipayQR)}
                  className="flex flex-col items-center gap-1.5 p-2 rounded-lg border border-border bg-card hover:bg-muted/40 cursor-pointer transition-colors focus:outline-none focus:ring-2 focus:ring-primary/30"
                >
                  <img
                    src={alipayQR}
                    alt="支付宝"
                    className="w-20 h-20 rounded-md"
                  />
                  <span className={`text-xs font-medium text-foreground`}>{t('welcome.alipay')}</span>
                </button>
                <button
                  onClick={() => setPreviewImg(wechatQR)}
                  className="flex flex-col items-center gap-1.5 p-2 rounded-lg border border-border bg-card hover:bg-muted/40 cursor-pointer transition-colors focus:outline-none focus:ring-2 focus:ring-primary/30"
                >
                  <img
                    src={wechatQR}
                    alt="WeChat Pay"
                    className="w-20 h-20 rounded-md"
                  />
                  <span className={`text-xs font-medium text-foreground`}>{t('welcome.wechatPay')}</span>
                </button>
              </div>
              <p className={`text-[11px] text-muted-foreground text-center`}>{t('welcome.clickToEnlarge')}</p>
            </div>

            {/* 赞助用户群 */}
            <a
              href="https://qm.qq.com/q/xzWxJsSUD0"
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-start gap-3 px-3 py-2.5 rounded-lg border border-border bg-card hover:bg-muted/40 transition-colors"
            >
              <div className="w-8 h-8 rounded-md flex items-center justify-center text-purple-500 bg-purple-500/10">
                <Users size={15} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-foreground mb-1">{t('welcome.sponsorBenefits')}</div>
                <div className="text-[11px] text-muted-foreground leading-relaxed space-y-0.5">
                  <p>{t('welcome.priorityFeedback')}</p>
                  <p>{t('welcome.priorityFeatures')}</p>
                  <p>{t('welcome.oneOnOneSupport')}</p>
                </div>
                <p className="text-[11px] text-muted-foreground mt-1.5 leading-relaxed">{t('welcome.sponsorHint')}</p>
              </div>
              <ExternalLink size={13} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0 mt-1" />
            </a>
          </div>
        </div>

        {/* 底部按钮 */}
        <div className="px-6 py-3 border-t border-border/50 flex justify-end relative">
          <button
            onClick={handleClose}
            className="px-5 py-2 text-sm font-semibold rounded-xl text-white bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 shadow-lg shadow-blue-500/30 hover:shadow-blue-500/40 transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30 active:scale-[0.98]"
          >
            {t('welcome.iUnderstand')}
          </button>
        </div>
      </div>

      {/* 图片预览弹窗 */}
      {previewImg && (
        <div
          className="fixed inset-0 bg-black/80 flex items-center justify-center z-[60] p-4"
          onClick={() => setPreviewImg(null)}
        >
          <div className="relative" onClick={(e) => e.stopPropagation()}>
            <img
              src={previewImg}
              alt={t('common.preview')}
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
