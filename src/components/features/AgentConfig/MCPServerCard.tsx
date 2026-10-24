import { Server, Edit2, Trash2, Terminal } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useMemo } from 'react'
import { getThemeAccent } from './themeAccent'
import React from 'react'

function MCPServerCard({ name, config, onToggle, onEdit, onDelete }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  
  const isDisabled = config.disabled
  const commandStr = [config.command, ...(config.args || [])].join(' ')
  const autoApproveCount = config.autoApprove?.length || 0
  const envCount = Object.keys(config.env || {}).length

  // 定义本地色彩系统
  const colors = {
    badgeActive: 'bg-primary/20 text-primary border border-primary/30',
    badgeDisabled: 'bg-muted/50 text-muted-foreground border border-border/50',
    toggleOn: 'bg-primary',
    toggleOff: 'bg-muted',
    toggleThumb: 'bg-white'
  }

  return (
    <div className={`glass-card border border-border rounded-xl p-4 transition-all hover:shadow-lg`}>
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3 flex-1 min-w-0">
          {/* 状态指示器 */}
          <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${
            isDisabled ? "bg-muted/30" : colors.badgeActive
          }`}>
            <Server size={20} className={isDisabled ? "text-muted-foreground" : accent.text} />
          </div>
          
          <div className="flex-1 min-w-0">
            {/* 名称 */}
            <div className="flex items-center gap-2">
              <h3 className={`font-semibold text-foreground ${isDisabled ? 'opacity-50' : ''}`}>{name}</h3>
              {isDisabled && (
                <span className={`text-xs px-2 py-0.5 rounded ${colors.badgeDisabled}`}>
                  {t('mcpManager.disabled')}
                </span>
              )}
            </div>
            
            {/* 命令 */}
            <div className={`flex items-center gap-1.5 mt-1 text-muted-foreground ${isDisabled ? 'opacity-50' : ''}`}>
              <Terminal size={14} />
              <code className="text-sm truncate">{commandStr}</code>
            </div>
            
            {/* 标签 */}
            <div className="flex items-center gap-2 mt-2">
              {autoApproveCount > 0 && (
                <span className={`text-xs px-2 py-0.5 rounded info-badge`}>
                  {t('mcpManager.autoApprove')}: {autoApproveCount} {t('mcpManager.tools')}
                </span>
              )}
              {envCount > 0 && (
                <span className={`text-xs px-2 py-0.5 rounded bg-purple-500/10 text-purple-500`}>
                  {t('mcpManager.envVars')}: {envCount}
                </span>
              )}
            </div>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2 ml-4">
          {/* 开关 */}
          <button
            onClick={() => onToggle(!isDisabled)}
            className={`cursor-pointer relative w-11 h-6 rounded-full transition-colors ${
              isDisabled ? colors.toggleOff : colors.toggleOn
            }`}
          >
            <div className={`absolute top-1 w-4 h-4 rounded-full transition-transform ${
              isDisabled ? 'left-1' : 'left-6'
            } ${colors.toggleThumb}`} />
          </button>
          
          <button
            onClick={onEdit}
            className={`cursor-pointer p-2 rounded-lg hover:bg-muted/50 transition-colors duration-200 focus:outline-none focus:ring-2 ${accent.ring}`}
            title={t('common.edit')}
          >
            <Edit2 size={16} className={"text-muted-foreground"} />
          </button>
          
          <button
            onClick={onDelete}
            className={`cursor-pointer p-2 rounded-lg hover:bg-red-500/10 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60`}
            title={t('common.delete')}
          >
            <Trash2 size={16} className="text-red-500" />
          </button>
        </div>
      </div>
    </div>
  )
}

export default MCPServerCard
