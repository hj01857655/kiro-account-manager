import { useState, useEffect } from 'react'
import { AlertTriangle, ExternalLink, MessageCircle } from 'lucide-react'
import { Checkbox } from '@/components/ui/checkbox'
import { useApp } from '../../hooks/useApp'

// 公告 API
const ANNOUNCEMENT_API = 'https://kiro-website-six.vercel.app/api/announcement'

// 固定链接
const WEBSITE_URL = 'https://kiro-website-six.vercel.app'
const GITHUB_URL = 'https://github.com/hj01857655/kiro-account-manager'
const TUTORIAL_URL = 'https://xcn46cm1l4ir.feishu.cn/wiki/YfaAw3qnoixFJgkzTSmcgtPfntc'
const TG_CHANNEL = 'https://t.me/kiro520'
const TG_GROUP = 'https://t.me/ide520'

export default function AnnouncementModal() {
  const { t} = useApp()
  const [show, setShow] = useState(false)
  const [announcement, setAnnouncement] = useState(null)
  const [agreed, setAgreed] = useState(false)
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10 ring-1 ring-primary/15'
  }
  useEffect(() => {
    fetchAnnouncement()
  }, [])

  const fetchAnnouncement = async () => {
    try {
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), 5000) // 5秒超时
      
      // 从公告 API 获取公告列表
      const res = await fetch(ANNOUNCEMENT_API, {
        signal: controller.signal,
        cache: 'no-cache'
      })
      
      clearTimeout(timeoutId)
      
      if (!res.ok) {
        return
      }
      
      const data = await res.json()
      
      // 获取第一个启用的公告
      const announcement = data.announcements?.find(a => a.enabled)
      
      if (!announcement) {
        return
      }
      
      // 检查是否已读过此公告
      const readAnnouncements = JSON.parse(localStorage.getItem('readAnnouncements') || '[]')
      if (readAnnouncements.includes(announcement.id)) {
        return
      }
      
      // 补充固定链接（如果公告中没有）
      setAnnouncement({
        ...announcement,
        websiteUrl: announcement.websiteUrl || WEBSITE_URL,
        officialUrl: announcement.officialUrl || GITHUB_URL,
        tutorialUrl: announcement.tutorialUrl || TUTORIAL_URL,
        tgChannel: announcement.tgChannel || TG_CHANNEL,
        tgGroup: announcement.tgGroup || TG_GROUP
      })
      setShow(true)
    } catch (e) {
      // 静默失败，不影响应用使用
      console.error('[Announcement] 获取失败:', e)
    }
  }

  const handleClose = (dontRemind = false) => {
    if (dontRemind && announcement) {
      // 保存已读状态
      const readAnnouncements = JSON.parse(localStorage.getItem('readAnnouncements') || '[]')
      if (!readAnnouncements.includes(announcement.id)) {
        readAnnouncements.push(announcement.id)
        localStorage.setItem('readAnnouncements', JSON.stringify(readAnnouncements))
      }
    }
    setShow(false)
  }

  if (!show) return null

  // 普通公告需要有内容才显示
  if (!announcement) return null

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60">
      <div className={`relative w-[480px] max-w-[90vw] rounded-lg shadow-2xl border glass-card border-border overflow-hidden animate-fade-in-up`}>
        {/* 顶部警告条 */}
        <div className="bg-gradient-to-r from-red-500 to-orange-500 px-6 py-4 flex items-center gap-3">
          <AlertTriangle size={24} className="text-white" />
          <span className="text-white font-bold text-lg">{announcement.title || t('announcement.title')}</span>
        </div>

        {/* 内容 */}
        <div className="p-6" style={{ padding: 'var(--app-space-md)' }}>
          <div className={`text-base leading-relaxed text-foreground`}>
            {announcement.content?.map((text, i) => (
              <p key={i} className={i === 0 ? 'mb-4 font-medium text-red-500' : 'mb-3'}>
                {text}
              </p>
            ))}
          </div>

          {/* 官方开源信息 */}
          {(announcement.websiteUrl || announcement.officialUrl || announcement.tgChannel || announcement.tgGroup) && (
            <div className={`mt-5 p-4 rounded-xl bg-muted/30`}>
              <p className={`text-sm font-medium mb-3 text-foreground`}>Related links</p>
              <div className="flex flex-wrap gap-2">
                {announcement.websiteUrl && (
                  <a
                    href={announcement.websiteUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gradient-to-r from-cyan-500 to-blue-500 text-white text-sm hover:opacity-90 transition"
                  >
                    <span>🌐</span>
                    <span>Website</span>
                  </a>
                )}
                {announcement.officialUrl && (
                  <a
                    href={announcement.officialUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition hover:bg-muted/50 text-foreground`}
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
                    <span>GitHub</span>
                  </a>
                )}
                {announcement.tutorialUrl && (
                  <a
                    href={announcement.tutorialUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition hover:bg-muted/50 text-primary`}
                  >
                    <span>📖</span>
                    <span>Tutorial</span>
                  </a>
                )}
                {announcement.tgChannel && (
                  <a
                    href={announcement.tgChannel}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition hover:bg-muted/50 text-blue-500`}
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.562 8.161c-.18 1.897-.962 6.502-1.359 8.627-.168.9-.5 1.201-.82 1.23-.697.064-1.226-.461-1.901-.903-1.056-.692-1.653-1.123-2.678-1.799-1.185-.781-.417-1.21.258-1.911.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.139-5.062 3.345-.479.329-.913.489-1.302.481-.428-.009-1.252-.242-1.865-.442-.752-.244-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.831-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635.099-.002.321.023.465.141.121.099.155.232.171.326.016.094.036.308.02.475z"/></svg>
                    <span>Channel</span>
                  </a>
                )}
                {announcement.tgGroup && (
                  <a
                    href={announcement.tgGroup}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition hover:bg-muted/50 text-blue-500`}
                  >
                    <MessageCircle size={14} />
                    <span>Community</span>
                  </a>
                )}
              </div>
            </div>
          )}

          {/* 续杯交流群 */}
          {announcement.buyGroup && (
            <div className={`mt-3 p-4 rounded-xl bg-muted/30`}>
              <p className={`text-sm font-medium mb-3 text-foreground`}>Account purchase needs:</p>
              <div className="space-y-2">
                {announcement.buyUrl && (
                  <a 
                    href={announcement.buyUrl}
                    target="_blank" 
                    rel="noopener noreferrer"
                    className="flex items-center gap-2 text-blue-500 hover:text-blue-400 text-sm"
                  >
                    <ExternalLink size={14} className="shrink-0" />
                    <span>Buy online</span>
                  </a>
                )}
                {announcement.refillTutorialUrl && (
                  <a 
                    href={announcement.refillTutorialUrl}
                    target="_blank" 
                    rel="noopener noreferrer"
                    className="flex items-center gap-2 text-blue-500 hover:text-blue-400 text-sm"
                  >
                    <ExternalLink size={14} className="shrink-0" />
                    <span>{t('announcement.refillTutorial')}</span>
                  </a>
                )}
                <a 
                  href={announcement.buyGroupUrl || '#'}
                  target="_blank" 
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-blue-500 hover:text-blue-400 text-sm"
                >
                  <MessageCircle size={14} className="shrink-0" />
                  <span>{announcement.buyGroup}</span>
                </a>
              </div>
            </div>
          )}
        </div>

        {/* 底部按钮 */}
        <div className="px-6 pb-6" style={{ padding: 'var(--app-space-md)', paddingTop: 0 }}>
          <label className={`flex items-center gap-3 mb-4 cursor-pointer select-none text-foreground`}>
            <Checkbox
              checked={agreed}
              onCheckedChange={setAgreed}
            />
            <span className="text-sm">I have read and understood the above</span>
          </label>
          <div className="flex gap-3">
            <button
              onClick={() => handleClose(false)}
              disabled={!agreed}
              className={`flex-1 py-3 rounded-xl font-medium transition-all ${
                agreed 
                  ? 'bg-gradient-to-r from-blue-500 to-purple-500 text-white hover:opacity-90' 
                  : `${colors.btnDisabled} cursor-not-allowed`
              }`}
            >
              {t('announcement.understand')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
