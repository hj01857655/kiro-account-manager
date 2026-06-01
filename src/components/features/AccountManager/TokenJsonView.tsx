// Token 凭证 JSON 视图组件
import { useState, useRef, useEffect, useMemo } from 'react'
import { Copy, Check, ChevronDown, Key } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../KiroConfig/themeAccent'

// 构建凭证 JSON 对象（直接使用整个账号对象）
function buildCredentialsJson(account) {
  // 直接返回整个账号对象，让后端的序列化逻辑处理
  return account
}

// 可折叠的字符串值
function CollapsibleValue({ value, colors, t, threshold = 50 }) {
  const [expanded, setExpanded] = useState(false)
  const isLong = value.length > threshold
  
  if (!isLong) {
    return <span className="text-emerald-500 font-medium">"{value}"</span>
  }
  
  const displayValue = expanded ? value : `${value.slice(0, threshold)}...`
  
  return (
    <span className="inline">
      <span className="text-emerald-500 font-medium">"{displayValue}"</span>
      <button
        type="button"
        onClick={(e) => { e.stopPropagation(); setExpanded(!expanded) }}
        className={`
          ml-2 text-xs px-2 py-0.5 rounded-md 
          bg-muted/30 text-muted-foreground hover:bg-muted/50
          transition-all duration-200 font-medium
        `}
      >
        {expanded ? t('common.collapse') : t('common.expandMore', { count: value.length - threshold })}
      </button>
    </span>
  )
}

// JSON 渲染（带折叠，支持嵌套对象和数组）
function JsonRenderer({ json, colors, accent, t, indent = 0 }) {
  const entries = Object.entries(json).filter(([_, value]) => value !== undefined)
  const pad = '  '.repeat(indent)
  const padInner = '  '.repeat(indent + 1)

  return (
    <div className="text-sm font-mono leading-relaxed">
      <span className={"text-muted-foreground"}>{'{'}</span>
      {entries.map(([key, value], i) => (
        <div key={key} className="py-0.5">
          <span className={"text-muted-foreground"}>{padInner}</span>
          <span className={`${accent.text} font-semibold`}>"{key}"</span>
          <span className={"text-muted-foreground"}>: </span>
          {typeof value === 'string' ? (
            <CollapsibleValue value={value} colors={colors} t={t} />
          ) : value === null || value === undefined ? (
            <span className="text-orange-500 font-medium">null</span>
          ) : typeof value === 'boolean' ? (
            <span className={`${accent.text} font-medium`}>{String(value)}</span>
          ) : typeof value === 'number' ? (
            <span className="text-amber-500 font-medium">{value}</span>
          ) : Array.isArray(value) ? (
            <span className="text-emerald-500">[{value.length > 0 ? '...' : ''}]</span>
          ) : typeof value === 'object' ? (
            <span className="text-emerald-500">{'{...}'}</span>
          ) : (
            <span className="text-emerald-500">{JSON.stringify(value)}</span>
          )}
          {i < entries.length - 1 && <span className={"text-muted-foreground"}>,</span>}
        </div>
      ))}
      <span className={"text-muted-foreground"}>{pad}{'}'}</span>
    </div>
  )
}

// Token JSON 视图（只读）
export function TokenJsonView({ account, defaultExpanded = false }) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])
  const [expanded, setExpanded] = useState(defaultExpanded)
  const [copied, setCopied] = useState(false)
  const copiedTimerRef = useRef(null)
  
  const credentialsJson = useMemo(() => buildCredentialsJson(account), [account])
  const jsonStr = useMemo(() => JSON.stringify(credentialsJson, null, 2), [credentialsJson])
  
  useEffect(() => () => copiedTimerRef.current && clearTimeout(copiedTimerRef.current), [])
  
  const handleCopy = () => {
    navigator.clipboard.writeText(jsonStr).catch(e => console.error('Copy failed:', e))
    setCopied(true)
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current)
    copiedTimerRef.current = setTimeout(() => setCopied(false), 1500)
  }
  
  return (
    <div className={`border-b border-border`} style={{ margin: 0 }}>
      <div 
        className={`flex items-center justify-between cursor-pointer hover:bg-muted/30 transition-all duration-200 px-6 py-3`}
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <Key size={16} className={"text-muted-foreground"} />
          <span className={`text-sm font-medium text-foreground`}>{t('detail.tokenCredentials')}</span>
          <span className={`text-xs px-1.5 py-0.5 rounded bg-muted/50 text-muted-foreground font-mono`}>
            {Object.keys(credentialsJson).length} {t('common.fields')}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button 
            type="button" 
            onClick={(e) => { e.stopPropagation(); handleCopy() }}
            className="text-xs text-muted-foreground hover:text-foreground px-2 py-1 rounded hover:bg-muted/50 transition-colors"
          >
            {copied ? <Check size={13} className="text-green-500" /> : <Copy size={13} />}
          </button>
          <ChevronDown size={14} className={`text-muted-foreground transition-transform duration-200 ${expanded ? '' : '-rotate-90'}`} />
        </div>
      </div>
      
      {expanded && (
        <div className="px-6 pb-4">
          <div className="p-3 rounded-lg bg-muted/20 border border-border max-h-64 overflow-auto font-mono text-xs leading-relaxed">
            <JsonRenderer json={credentialsJson} colors={colors} accent={accent} t={t} />
          </div>
        </div>
      )}
    </div>
  )
}

export default TokenJsonView
