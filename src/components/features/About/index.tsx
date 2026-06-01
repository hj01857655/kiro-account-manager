import { useState, useEffect, useMemo, useCallback } from 'react'
import { Github, Heart, Coffee, ExternalLink, Code2, Palette, Cpu, RefreshCw, X, Link2, Gift, Sparkles, Info } from 'lucide-react'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { check } from '@tauri-apps/plugin-updater'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import alipayQR from '../../../assets/donate/alipay.jpg'
import wechatQR from '../../../assets/donate/wechat.jpg'
import { isLightTheme as checkIsLightTheme } from '../../../utils/themeMode'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import SectionCard from '../Settings/SectionCard'

const CURRENT_YEAR = new Date().getFullYear()

const LINKS = {
  website: 'https://kiro-website-six.vercel.app',
  github: 'https://github.com/hj01857655/kiro-account-manager',
  kiroGo: 'https://github.com/hj01857655/Kiro-Go',
  tgChannel: 'https://t.me/kiro520',
  tgGroup: 'https://t.me/ide520',
}

// Telegram icon.
const TelegramIcon = ({ size = 16 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.562 8.161c-.18 1.897-.962 6.502-1.359 8.627-.168.9-.5 1.201-.82 1.23-.697.064-1.226-.461-1.901-.903-1.056-.692-1.653-1.123-2.678-1.799-1.185-.781-.417-1.21.258-1.911.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.139-5.062 3.345-.479.329-.913.489-1.302.481-.428-.009-1.252-.242-1.865-.442-.752-.244-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.831-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635.099-.002.321.023.465.141.121.099.155.232.171.326.016.094.036.308.02.475z"/>
  </svg>
)

// Compact logo.
const AppLogo = ({ accent }: { accent: any }) => (
  <div className="relative">
    <div className={`absolute inset-0 bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} rounded-2xl blur-md opacity-50`} />
    <div className={`relative w-14 h-14 bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} rounded-2xl flex items-center justify-center shadow-md`}>
      <svg width="28" height="28" viewBox="0 0 40 40" fill="none">
        <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white" />
      </svg>
    </div>
  </div>
)

// Link row: icon, title, subtitle, and external-link affordance.
interface LinkRowProps {
  href: string
  icon: React.ReactNode
  label: string
  desc?: string
  accent: 'primary' | 'github' | 'telegram'
}

function LinkRow({ href, icon, label, desc, accent }: LinkRowProps) {
  const accentClass = accent === 'github'
    ? 'text-foreground bg-foreground/5'
    : accent === 'telegram'
      ? 'text-blue-500 bg-blue-500/10'
      : 'text-primary bg-primary/10'
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="group flex items-center gap-3 px-3 py-2 rounded-lg border border-border bg-card hover:bg-muted/40 transition-colors"
    >
      <div className={`w-8 h-8 rounded-md flex items-center justify-center ${accentClass}`}>
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-foreground truncate">{label}</div>
        {desc && <div className="text-[11px] text-muted-foreground truncate">{desc}</div>}
      </div>
      <ExternalLink size={13} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
    </a>
  )
}

// QR code card.
const QRCodeCard = ({ src, label, onClick }: { src: string; label: string; onClick: () => void }) => (
  <button
    onClick={onClick}
    className="group flex flex-col items-center gap-1.5 p-2 rounded-lg border border-border bg-card hover:bg-muted/40 hover:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-primary/30"
    aria-label={label}
  >
    <img src={src} alt={label} className="w-[120px] h-[120px] rounded-md transition-transform duration-200 group-hover:scale-[1.02]" />
    <span className="text-xs font-medium text-foreground">{label}</span>
  </button>
)

function About() {
  const { t, theme } = useApp()
  const { showUpdate, showInfo } = useDialog()
  const [version, setVersion] = useState('')
  const [checking, setChecking] = useState(false)
  const [previewImg, setPreviewImg] = useState<string | null>(null)

  const accent = useMemo(() => getThemeAccent(theme), [theme])

  const heartClass = useMemo(() => {
    const isLight = checkIsLightTheme(theme)
    return isLight ? 'text-red-500 fill-red-500' : 'text-red-400 fill-red-400'
  }, [theme])

  const techStack = useMemo(() => [
    { icon: Code2, value: 'React + Vite' },
    { icon: Palette, value: 'TailwindCSS' },
    { icon: Cpu, value: 'Tauri + Rust' },
  ], [])

  const sponsorBenefits = useMemo(() => [
    t('about.benefit1'),
    t('about.benefit2'),
    t('about.benefit3'),
  ], [t])

  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(''))
  }, [])

  const checkUpdate = useCallback(async () => {
    setChecking(true)
    try {
      const result = await invoke<any>('check_update')
      if (result.has_update && result.latest_version) {
        const updateResult = await check()
        if (updateResult) {
          showUpdate(
            { version: result.latest_version, body: result.notes },
            updateResult,
          )
        } else {
          showInfo(t('about.checkUpdate'), t('about.updateFailed'))
        }
      } else {
        showInfo(t('about.checkUpdate'), t('about.upToDate'))
      }
    } catch (e) {
      console.error('Check update failed:', e)
      showInfo(t('about.checkUpdate'), t('about.updateFailed'))
    } finally {
      setChecking(false)
    }
  }, [showUpdate, showInfo, t])

  return (
    <div className="h-full glass-main overflow-auto p-6">
      <div className="space-y-3">
        {/* App intro card */}
        <Card className="card-glow">
          <CardContent className="p-5">
            <div className="flex items-start gap-4">
              <AppLogo accent={accent} />
              <div className="flex-1 min-w-0 space-y-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <h1 className="text-base font-semibold text-foreground">{t('about.appName')}</h1>
                  <Badge variant="default" className="px-2 py-0 h-5 text-[11px] font-mono">
                    v{version || '...'}
                  </Badge>
                  <Button
                    onClick={checkUpdate}
                    disabled={checking}
                    variant="outline"
                    size="sm"
                    className="ml-auto h-7 text-xs gap-1"
                  >
                    <RefreshCw size={12} className={checking ? 'animate-spin' : ''} />
                    {checking ? t('about.checking') : t('about.checkUpdate')}
                  </Button>
                </div>

                <p className="text-xs text-muted-foreground leading-relaxed">
                  {t('about.appDesc')}
                </p>

                <div className="flex items-center gap-1.5 flex-wrap pt-1">
                  {techStack.map(({ icon: Icon, value }) => (
                    <span
                      key={value}
                      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] text-muted-foreground border border-border bg-muted/30"
                    >
                      <Icon size={11} />
                      {value}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* === 2. 链接卡 === */}
        <SectionCard
          title={t('about.links')}
          accent="blue"
          icon={<Link2 size={14} className="text-blue-500" />}
        >
          <div className="space-y-2">
            <LinkRow
              href={LINKS.website}
              icon={<ExternalLink size={15} />}
              label={t('about.website')}
              desc="kiro-website-six.vercel.app"
              accent="primary"
            />
            <LinkRow
              href={LINKS.github}
              icon={<Github size={15} />}
              label="GitHub"
              desc="hj01857655/kiro-account-manager"
              accent="github"
            />
            <LinkRow
              href={LINKS.kiroGo}
              icon={<Github size={15} />}
              label="Kiro-Go"
              desc={t('about.kiroGoDesc', { defaultValue: 'Kiro API reverse proxy - standard OpenAI/Anthropic interfaces' })}
              accent="github"
            />
            <div className="grid grid-cols-2 gap-2">
              <LinkRow
                href={LINKS.tgChannel}
                icon={<TelegramIcon size={15} />}
                label={t('about.tgChannel')}
                accent="telegram"
              />
              <LinkRow
                href={LINKS.tgGroup}
                icon={<TelegramIcon size={15} />}
                label={t('about.tgGroup')}
                accent="telegram"
              />
            </div>
          </div>
        </SectionCard>

        {/* Donation card */}
        <SectionCard
          title={t('about.donate')}
          accent="amber"
          icon={<Coffee size={14} className="text-amber-500" />}
          desc={t('about.donateDesc')}
        >
          {/* Benefit list */}
          <div className="rounded-lg border border-border bg-muted/20 p-3">
            <div className="flex items-center gap-1.5 mb-2">
              <Gift size={13} className="text-amber-500" />
              <span className="text-xs font-medium text-foreground">{t('about.sponsorBenefits')}</span>
            </div>
            <ul className="space-y-1">
              {sponsorBenefits.map((b, i) => (
                <li key={i} className="text-xs text-muted-foreground leading-relaxed">{b}</li>
              ))}
            </ul>
          </div>

          {/* Hint */}
          <div className="flex items-start gap-2 px-3 py-2 rounded-lg border border-blue-500/20 bg-blue-500/5">
            <Info size={13} className="text-blue-500 flex-shrink-0 mt-0.5" />
            <p className="text-[11px] text-foreground leading-relaxed">{t('about.sponsorNote')}</p>
          </div>

          {/* QR code columns */}
          <div className="grid grid-cols-2 gap-2 pt-1">
            <QRCodeCard src={alipayQR} label={t('about.alipay')} onClick={() => setPreviewImg(alipayQR)} />
            <QRCodeCard src={wechatQR} label={t('about.wechat')} onClick={() => setPreviewImg(wechatQR)} />
          </div>
          <p className="text-[11px] text-center text-muted-foreground">{t('about.clickToEnlarge')}</p>
        </SectionCard>

        {/* Footer signature */}
        <div className="flex items-center justify-center gap-1.5 py-3 text-xs text-muted-foreground">
          <Sparkles size={12} className="text-primary/70" />
          <span>{t('about.madeWith')}</span>
          <Heart size={12} className={heartClass} />
          <span>{t('about.by')} hj01857655</span>
          <span className="opacity-50">·</span>
          <span>© {CURRENT_YEAR}</span>
        </div>
      </div>

      {/* QR code preview dialog */}
      <Dialog open={!!previewImg} onOpenChange={(open) => !open && setPreviewImg(null)}>
        <DialogContent className="max-w-fit p-0 bg-transparent border-none shadow-none">
          <div className="relative">
            {previewImg && <img src={previewImg} alt={t('common.preview')} className="max-w-[320px] max-h-[320px] rounded-xl shadow-xl" />}
            <button
              className="absolute -top-3 -right-3 w-8 h-8 rounded-full glass-card flex items-center justify-center shadow-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-primary/30"
              onClick={() => setPreviewImg(null)}
              aria-label={t('about.closePreview', { defaultValue: 'Close preview' })}
            >
              <X size={16} className="text-foreground" />
            </button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default About
