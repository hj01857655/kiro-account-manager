import { useState, useEffect, useMemo } from 'react'
import { X, Terminal, AlertCircle, Wand2 } from 'lucide-react'
import { saveMcpServer } from '../../../api/agentConfigApi'
import { Textarea } from '@/components/ui/textarea'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent, getGradientAccentButton } from './themeAccent'
import React from 'react'

function EditMCPModal({ name, config, source, onClose, onSuccess, projectDir, readOnly }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const accentGradientButtonClass = getGradientAccentButton(accent)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }

  const [jsonConfig, setJsonConfig] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [parseError, setParseError] = useState('')

  // 初始化 JSON
  useEffect(() => {
    const configObj = {
      command: config.command || '',
      args: config.args || [],
      env: config.env || {},
      disabled: config.disabled || false,
      autoApprove: config.autoApprove || []
    }
    setJsonConfig(JSON.stringify(configObj, null, 2))
  }, [config])

  // 实时校验 JSON
  useEffect(() => {
    if (!jsonConfig.trim()) {
      setParseError('')
      return
    }
    try {
      const parsed = JSON.parse(jsonConfig)
      if (!parsed.command) {
        setParseError('缺少 command 字段')
        return
      }
      setParseError('')
    } catch (e) {
      setParseError('JSON 格式错误')
    }
  }, [jsonConfig])

  // 格式化 JSON
  const formatJson = () => {
    try {
      const parsed = JSON.parse(jsonConfig)
      setJsonConfig(JSON.stringify(parsed, null, 2))
    } catch {}
  }

  // 保存
  const handleSave = async () => {
    if (readOnly) return
    let parsed: any
    try {
      parsed = JSON.parse(jsonConfig)
    } catch (e: any) {
      setError('JSON 格式错误: ' + e.message)
      return
    }

    if (!parsed.command) {
      setError(t('mcpManager.errorNoCommand'))
      return
    }

    setSaving(true)
    setError('')

    try {
      const newConfig = {
        command: parsed.command,
        args: parsed.args || [],
        env: parsed.env || {},
        disabled: parsed.disabled ?? config.disabled ?? false,
        autoApprove: parsed.autoApprove || []
      }

      // 写入层级跟随来源：user/power 来源写回用户级，避免把用户级配置
      // 复制到项目级制造冗余覆盖
      const targetDir = source === 'user' || source === 'power' ? null : (projectDir || null)
      await saveMcpServer(name, newConfig, targetDir)
      onSuccess()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div 
        className={`relative overflow-hidden glass-card border border-border rounded-lg shadow-2xl w-[520px] max-h-[85vh] flex flex-col`}
        onClick={e => e.stopPropagation()}
      >
        {/* 顶部渐变装饰 */}
        <div className={`absolute top-0 left-0 right-0 h-24 ${accent.bgSoft} pointer-events-none`} />
        
        {/* 标题 */}
        <div className={`relative flex items-center justify-between px-6 py-4 border-b border-border`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-lg bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-lg ${accent.shadow}`}>
              <Terminal size={20} className="text-white" />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('common.edit')}: {name}</h2>
          </div>
          <button onClick={onClose} className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        {/* 内容 */}
        <div className="relative flex-1 overflow-auto p-6 space-y-4">
          {/* JSON 配置 */}
          <div className="flex-1">
            <div className="flex items-center justify-between mb-1.5">
              <div className="flex items-center gap-2">
                <label className={`text-xs text-muted-foreground`}>配置</label>
                {parseError && (
                  <span className="text-xs text-red-500 flex items-center gap-1">
                    <AlertCircle size={12} />
                    {parseError}
                  </span>
                )}
              </div>
              <button
                onClick={formatJson}
                className={`cursor-pointer text-xs text-muted-foreground ${accent.textHover} flex items-center gap-1 transition-colors`}
              >
                <Wand2 size={12} />
                格式化
              </button>
            </div>
            <Textarea
              value={jsonConfig}
              onChange={e => setJsonConfig(e.target.value)}
              rows={16}
              spellCheck={false}
              className={`font-mono text-sm ${parseError ? 'border-red-500/50' : ''} text-foreground bg-background border-input ${colors.inputFocus}`}
            />
            <p className={`text-xs text-muted-foreground mt-2 flex items-start gap-1.5`}>
              <span className={`${accent.text} font-medium`}>💡</span>
              <span>autoApprove 支持通配符 <code className={`px-1.5 py-0.5 ${accent.bgSoft} ${accent.text} rounded`}>["*"]</code> 自动批准该服务器的所有工具</span>
            </p>
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="text-red-500 text-xs bg-red-500/10 px-3 py-2 rounded-lg">{error}</div>
          )}
        </div>

        {/* 底部按钮 */}
        <div className={`relative flex justify-end gap-3 px-6 py-4 border-t border-border`}>
          <button
            onClick={onClose}
            className={`cursor-pointer px-5 py-2.5 rounded-lg text-sm text-foreground hover:bg-muted/50`}
          >
            {t('common.cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={saving || !!parseError || readOnly}
            className={`cursor-pointer px-6 py-2.5 ${accentGradientButtonClass} rounded-lg text-sm font-medium disabled:opacity-50`}
          >
            {saving ? t('common.saving') : t('common.save')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default EditMCPModal
