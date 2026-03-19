import { useState, useEffect, useMemo, useCallback } from 'react'
import { Github, Heart, Coffee, ExternalLink, Sparkles, Code2, Palette, Cpu, RefreshCw, X, BookOpen, MessageCircle } from 'lucide-react'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { check } from '@tauri-apps/plugin-updater'
import { Card, Stack, Group, Text, Badge, Image, Button, Modal, Transition } from '@mantine/core'
import { useApp } from '../../hooks/useApp'
import { useDialog } from '../../contexts/DialogContext'
import alipayQR from '../../assets/donate/alipay.jpg'
import wechatQR from '../../assets/donate/wechat.jpg'
import { getThemeAccent } from './KiroConfig/themeAccent'
import { isLightTheme as checkIsLightTheme, getMantinePrimaryColor } from '../../utils/themeMode'

// 常量定义
const QQ_NUMBER = '1292548381'
const CURRENT_YEAR = new Date().getFullYear()

const LINKS = {
  website: 'https://kiro-website-six.vercel.app',
  github: 'https://github.com/hj01857655/kiro-account-manager',
  tutorial: 'https://xcn46cm1l4ir.feishu.cn/wiki/YfaAw3qnoixFJgkzTSmcgtPfntc',
  qqGroup: 'https://qm.qq.com/q/xi0AglEqGs',
  gateway: 'https://github.com/hj01857655/kiro-gateway'
}

// QQ 图标组件
const QQIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="white">
    <path d="M12.003 2c-2.265 0-6.29 1.364-6.29 7.325v1.195S3.55 14.96 3.55 17.474c0 .665.17 1.025.281 1.025.114 0 .902-.484 1.748-2.072 0 0-.18 2.197 1.904 3.967 0 0-1.77.495-1.77 1.182 0 .686 4.078.43 6.29.43 2.213 0 6.29.256 6.29-.43 0-.687-1.77-1.182-1.77-1.182 2.085-1.77 1.905-3.967 1.905-3.967.845 1.588 1.634 2.072 1.746 2.072.111 0 .283-.36.283-1.025 0-2.514-2.166-6.954-2.166-6.954V9.325C18.29 3.364 14.268 2 12.003 2z"/>
  </svg>
)

// Logo 组件
const AppLogo = ({ accent, iconColors }) => (
  <div className="relative group">
    <div className={`absolute inset-0 bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} rounded-3xl blur-xl opacity-40 group-hover:opacity-60 transition-opacity duration-300`} />
    <div className={`relative w-20 h-20 bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} rounded-3xl flex items-center justify-center shadow-lg transition-shadow duration-300 group-hover:shadow-xl animate-float`}>
      <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
        <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white"/>
      </svg>
    </div>
    <div className={`absolute -bottom-1 -right-1 w-7 h-7 bg-gradient-to-br ${iconColors.sparkles} rounded-lg flex items-center justify-center shadow-md animate-pulse`}>
      <Sparkles size={14} className="text-white" />
    </div>
  </div>
)

// 技术栈徽章组件
const TechBadge = ({ icon: Icon, value, color }) => (
  <Badge
    leftSection={<Icon size={14} />}
    color={color}
    variant="light"
    size="lg"
    radius="xl"
  className="transition-colors duration-200 hover:shadow-md cursor-default"
  >
    {value}
  </Badge>
)

// 链接按钮组件
const LinkButton = ({ href, icon: Icon, children, color, variant = 'filled', fullWidth = false }) => (
  <Button
    component="a"
    href={href}
    target="_blank"
    rel="noopener noreferrer"
    leftSection={<Icon size={18} />}
    variant={variant}
    color={color}
    radius="md"
    fullWidth={fullWidth}
    className="transition-colors duration-200 hover:shadow-lg cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30"
  >
    {children}
  </Button>
)

// QR 码图片组件
const QRCodeImage = ({ src, alt, onClick, accent, colors, onKeyDown }) => (
  <Stack
    gap="xs"
    align="center"
    style={{ cursor: 'pointer' }}
    onClick={onClick}
    role="button"
    tabIndex={0}
    onKeyDown={onKeyDown}
    aria-label={alt}
    className="transition-colors duration-200 hover:shadow-sm cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30"
  >
    <div className="relative group">
      <div className={`absolute inset-0 ${accent.bg} rounded-lg blur-md opacity-0 group-hover:opacity-40 transition-opacity duration-200`} />
      <Image
        src={src}
        alt={alt}
        w={120}
        h={120}
        radius="md"
        className="relative transition-transform duration-200 shadow-md"
      />
    </div>
    <Text size="sm" fw={500} className={colors.text}>{alt}</Text>
  </Stack>
)

// 信息卡片组件
const InfoCard = ({ title, items, colors }) => (
  <div className={`${colors.cardSecondary} rounded-xl p-4 transition-colors duration-200 hover:shadow-md`}>
    {title && (
      <Text size="sm" fw={500} className={colors.text} mb="xs">
        {title}
      </Text>
    )}
    <Stack gap="xs" className={colors.text}>
      {items.map((item, index) => (
        <Text key={index} size="sm" style={{ lineHeight: 1.6 }}>
          {item}
        </Text>
      ))}
    </Stack>
  </div>
)

function About() {
  const { t, theme, colors } = useApp()
  const { showUpdate, showInfo, showSuccess } = useDialog()
  const [version, setVersion] = useState('')
  const [checking, setChecking] = useState(false)
  const [previewImg, setPreviewImg] = useState(null)
  const [mounted, setMounted] = useState(false)

  // 使用 useMemo 缓存主题相关计算
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const mantineAccentColor = useMemo(() => getMantinePrimaryColor(theme), [theme])

  // 统一的卡片样式
  const cardStyles = useMemo(() => ({
    className: `${colors.card} border ${colors.cardBorder}`,
    shadow: 'lg',
    radius: 'xl',
    styles: { root: { backgroundColor: 'transparent' } }
  }), [colors])

  // 图标颜色（适配主题）
  const iconColors = useMemo(() => {
    const isLight = checkIsLightTheme(theme)
    return {
      coffee: isLight ? 'text-amber-500' : 'text-amber-400',
      heart: isLight ? 'text-red-500 fill-red-500' : 'text-red-400 fill-red-400',
      sparkles: isLight
        ? 'from-amber-400 to-orange-500'
        : 'from-amber-300 to-orange-400'
    }
  }, [theme])

  // 技术栈配置
  const techStack = useMemo(() => [
    { icon: Code2, label: t('about.frontend'), value: 'React + Vite', color: 'cyan' },
    { icon: Palette, label: t('about.ui'), value: 'TailwindCSS', color: 'pink' },
    { icon: Cpu, label: t('about.backend'), value: 'Tauri + Rust', color: 'orange' },
  ], [t])

  // 赞助福利列表
  const sponsorBenefits = useMemo(() => [
    t('about.benefit1'),
    t('about.benefit2'),
    t('about.benefit3')
  ], [t])

  // 付费服务列表
  const paidServices = useMemo(() => [
    t('about.service1'),
    t('about.service2'),
    t('about.service3')
  ], [t])

  useEffect(() => {
    getVersion().then(setVersion)
    setMounted(true)
  }, [])

  const checkUpdate = useCallback(async () => {
    setChecking(true)
    try {
      const result = await invoke('check_update')

      if (result.has_update && result.latest_version) {
        const updateResult = await check()
        if (updateResult) {
          showUpdate(
            { version: result.latest_version, body: result.notes },
            updateResult
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

  const handlePreviewKeyDown = useCallback((event, imageSrc) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      setPreviewImg(imageSrc)
    }
  }, [])

  const handleContactQQ = useCallback(async () => {
    try {
      await openUrl(`tencent://message/?uin=${QQ_NUMBER}&Site=&Menu=yes`)
    } catch {
      try {
        await navigator.clipboard.writeText(QQ_NUMBER)
        showSuccess(t('common.copied'), `QQ: ${QQ_NUMBER}`)
      } catch {
        showInfo(t('about.contactQQ'), `QQ: ${QQ_NUMBER}`)
      }
    }
  }, [showSuccess, showInfo, t])

  const closePreview = useCallback(() => setPreviewImg(null), [])

  return (
    <div className={`h-full ${colors.main} overflow-auto`}>
      <div className="bg-glow bg-glow-1" />
      <div className="bg-glow bg-glow-2" />

      <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-6 sm:py-8 space-y-6">
        {/* 头部卡片 */}
        <Transition mounted={mounted} transition="fade-up" duration={400} timingFunction="ease">
          {(styles) => (
            <Card {...cardStyles} p="xl" style={styles}>
              <Stack gap="lg" align="center">
                <AppLogo accent={accent} iconColors={iconColors} />

                <Text size="xl" fw={700} className={colors.text}>{t('about.appName')}</Text>

                <Group gap="sm">
                  <Badge color="blue" size="lg" radius="xl" variant="dot">
                    v{version || '...'}
                  </Badge>
                  <Button
                    onClick={checkUpdate}
                    loading={checking}
                    leftSection={<RefreshCw size={12} />}
                    variant="light"
                    color="green"
                    size="compact-sm"
                    radius="xl"
                  >
                    {checking ? t('about.checking') : t('about.checkUpdate')}
                  </Button>
                </Group>

                <Text size="sm" className={colors.textMuted} ta="center" maw={500}>
                  {t('about.appDesc')}
                </Text>

                <Group gap="sm" justify="center" wrap="wrap">
                  {techStack.map(({ icon, label, value, color }) => (
                    <TechBadge key={label} icon={icon} value={value} color={color} />
                  ))}
                </Group>
              </Stack>
            </Card>
          )}
        </Transition>

        {/* 链接 */}
        <Transition mounted={mounted} transition="fade-up" duration={400} delay={100} timingFunction="ease">
          {(styles) => (
            <Card {...cardStyles} p="lg" style={styles}>
              <Text size="sm" fw={500} className={colors.text} ta="center" mb="md">{t('about.links')}</Text>
              <Stack gap="sm">
                <Group gap="sm" grow>
                  <LinkButton href={LINKS.website} icon={ExternalLink} color="cyan">
                    {t('about.website')}
                  </LinkButton>
                  <LinkButton href={LINKS.github} icon={Github} color="dark">
                    GitHub
                  </LinkButton>
                </Group>

                <Group gap="sm" grow>
                  <LinkButton href={LINKS.tutorial} icon={BookOpen} color="blue">
                    {t('about.tutorial')}
                  </LinkButton>
                  <LinkButton href={LINKS.qqGroup} icon={QQIcon} color="cyan">
                    {t('about.qqGroup')}
                  </LinkButton>
                </Group>

                <LinkButton href={LINKS.gateway} icon={Github} color="grape" variant="light" fullWidth>
                  {t('about.shop')}
                </LinkButton>
              </Stack>
            </Card>
          )}
        </Transition>

        {/* 赞赏 */}
        <Transition mounted={mounted} transition="fade-up" duration={400} delay={200} timingFunction="ease">
          {(styles) => (
            <Card {...cardStyles} p="lg" style={styles}>
              <Stack gap="md">
                <Group gap="xs" justify="center">
                  <Coffee size={20} className={`${iconColors.coffee} animate-bounce`} />
                  <Text size="lg" fw={600} className={colors.text}>{t('about.donate')}</Text>
                </Group>

                <Text size="sm" className={colors.text} ta="center" style={{ lineHeight: 1.6 }}>
                  {t('about.donateDesc')}
                </Text>

                <InfoCard
                  title={t('about.sponsorBenefits')}
                  items={sponsorBenefits}
                  colors={colors}
                />

                <Group justify="center" gap="xl" mt="sm">
                  <QRCodeImage
                    src={alipayQR}
                    alt={t('about.alipay')}
                    onClick={() => setPreviewImg(alipayQR)}
                    accent={accent}
                    colors={colors}
                    onKeyDown={(e) => handlePreviewKeyDown(e, alipayQR)}
                  />
                  <QRCodeImage
                    src={wechatQR}
                    alt={t('about.wechat')}
                    onClick={() => setPreviewImg(wechatQR)}
                    accent={accent}
                    colors={colors}
                    onKeyDown={(e) => handlePreviewKeyDown(e, wechatQR)}
                  />
                </Group>

                <Text size="xs" className={colors.textMuted} ta="center" mt="xs">
                  {t('about.clickToEnlarge')}
                </Text>
              </Stack>
            </Card>
          )}
        </Transition>

        {/* 付费服务 */}
        <Transition mounted={mounted} transition="fade-up" duration={400} delay={300} timingFunction="ease">
          {(styles) => (
            <Card {...cardStyles} p="xl" style={styles}>
              <Stack gap="md">
                <Group gap="xs" justify="center">
                  <MessageCircle size={20} className={accent.text} />
                  <Text size="lg" fw={600} className={colors.text}>{t('about.paidServices')}</Text>
                </Group>

                <Text size="sm" className={colors.text} ta="center" style={{ lineHeight: 1.6 }}>
                  {t('about.paidServicesDesc')}
                </Text>

                <InfoCard items={paidServices} colors={colors} />

                <Group justify="center" gap="md" mt="sm">
                  <Button
                    onClick={handleContactQQ}
                    leftSection={<MessageCircle size={16} />}
                    variant="light"
                    color={mantineAccentColor}
                    radius="xl"
                    className="transition-colors duration-200 hover:shadow-lg cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30"
                  >
                    {t('about.contactQQ')}
                  </Button>
                </Group>
              </Stack>
            </Card>
          )}
        </Transition>

        {/* 底部 */}
        <Transition mounted={mounted} transition="fade" duration={400} delay={400} timingFunction="ease">
          {(styles) => (
            <Group gap="xs" justify="center" className="py-4" style={styles}>
              <Text size="sm" className={colors.textMuted}>{t('about.madeWith')}</Text>
              <Heart size={14} className={`${iconColors.heart} animate-pulse`} />
              <Text size="sm" className={colors.textMuted}>{t('about.by')} hj01857655</Text>
              <Text size="sm" className={colors.textMuted}>·</Text>
              <Text size="sm" className={colors.textMuted}>© {CURRENT_YEAR}</Text>
            </Group>
          )}
        </Transition>
      </div>

      {/* 图片预览弹窗 */}
      <Modal
        opened={!!previewImg}
        onClose={closePreview}
        centered
        withCloseButton={false}
        size="auto"
        padding={0}
        styles={{
          content: { background: 'transparent' },
          body: { padding: 0 }
        }}
        transitionProps={{ transition: 'fade', duration: 200 }}
      >
        <div className="relative">
          <Image src={previewImg} alt="预览" maw={320} mah={320} radius="xl" />
          <button
            className={`absolute -top-3 -right-3 w-8 h-8 rounded-full flex items-center justify-center shadow-lg transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30 ${colors.card}`}
            onClick={closePreview}
            aria-label="关闭预览"
          >
            <X size={16} className={colors.text} />
          </button>
        </div>
      </Modal>
    </div>
  )
}

export default About
