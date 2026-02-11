import React from 'react'
import i18n from 'i18next'
import { initReactI18next, I18nextProvider, useTranslation } from 'react-i18next'

// 从 JSON 文件导入翻译
import zhCN from '../locales/zh-CN.json'
import enUS from '../locales/en-US.json'
import ruRU from '../locales/ru-RU.json'
import trTR from '../locales/tr-TR.json'

// 支持的语言
export const locales = {
  'zh-CN': '简体中文',
  'en-US': 'English',
  'ru-RU': 'Русский',
  'tr-TR': 'Türkçe',
}

i18n
  .use(initReactI18next)
  .init({
    lng: localStorage.getItem('locale') || 'zh-CN',
    fallbackLng: 'zh-CN',
    supportedLngs: ['zh-CN', 'en-US', 'ru-RU', 'tr-TR'],
    
    // 从 JSON 文件加载翻译
    resources: {
      'zh-CN': { translation: zhCN },
      'en-US': { translation: enUS },
      'ru-RU': { translation: ruRU },
      'tr-TR': { translation: trTR },
    },
    
    interpolation: {
      escapeValue: false,
    },
    
    react: {
      useSuspense: false,
    },
  })

// 切换语言
export const changeLanguage = async (lng) => {
  localStorage.setItem('locale', lng)
  await i18n.changeLanguage(lng)
}

// 兼容旧的 useI18n hook
export function useI18n() {
  const { t, i18n: i18nInstance } = useTranslation()
  const [loading, setLoading] = React.useState(false)
  const [, forceUpdate] = React.useReducer(x => x + 1, 0)
  
  const setLocale = async (lng) => {
    setLoading(true)
    try {
      await i18nInstance.changeLanguage(lng)
      localStorage.setItem('locale', lng)
      forceUpdate()
    } finally {
      setLoading(false)
    }
  }
  
  return {
    t,
    locale: i18nInstance.language,
    setLocale,
    loading,
  }
}

// I18nProvider 组件
export function I18nProvider({ children }) {
  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
}

export default i18n
