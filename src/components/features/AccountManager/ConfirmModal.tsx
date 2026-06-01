import { useState, useMemo } from 'react'
import { AlertTriangle, CheckCircle, XCircle, Info, ChevronDown, ChevronUp, Copy, Check } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogBody,
  DialogFooter} from '../../shared/dialog'
import { Button } from '../../shared/button'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import React from 'react'

interface ConfirmModalProps {
  type?: 'confirm' | 'success' | 'error' | 'info';
  title: string;
  message: string;
  rawData?: any;
  onConfirm: () => void;
  onCancel: () => void;
  confirmText?: string;
  cancelText?: string;
  loading?: boolean;
  customContent?: React.ReactNode;
}

/**
 * Shared confirmation / notification modal.
 */
function ConfirmModal({
  type = 'confirm',
  title,
  message,
  rawData,
  onConfirm,
  onCancel,
  confirmText,
  cancelText,
  loading = false,
  customContent}: ConfirmModalProps) {
  const { t, theme } = useApp()
  const [expanded, setExpanded] = useState(false)
  const [copied, setCopied] = useState(false)
  
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    codeBlock: 'bg-muted/30 border border-border text-foreground',
  }), [])

  // Use i18n defaults if not provided
  const finalConfirmText = confirmText || t('common.ok')
  const finalCancelText = cancelText || t('common.cancel')

  const config = {
    confirm: {
      icon: AlertTriangle,
      iconColor: 'text-amber-400',
      iconBg: 'bg-gradient-to-br from-amber-500/20 to-orange-500/10',
      btnVariant: 'primary' as const},
    success: {
      icon: CheckCircle,
      iconColor: 'text-emerald-400',
      iconBg: 'bg-gradient-to-br from-emerald-500/20 to-green-500/10',
      btnVariant: 'success' as const},
    error: {
      icon: XCircle,
      iconColor: 'text-red-400',
      iconBg: 'bg-gradient-to-br from-red-500/20 to-rose-500/10',
      btnVariant: 'danger' as const},
    info: {
      icon: Info,
      iconColor: accent.text,
      iconBg: accent.iconBadgeBg,
      btnVariant: 'primary' as const}}

  const { icon: Icon, iconColor, iconBg, btnVariant } = config[type]

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent maxWidth="400px">
        <DialogHeader icon={Icon} iconColor={iconColor} iconBg={iconBg}>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <DialogBody>
          <p className="text-sm leading-relaxed whitespace-pre-line text-foreground">
            {message}
          </p>
          
          {/* Custom content */}
          {customContent}
          
          {/* Raw response disclosure */}
          {rawData && (
            <div className="mt-4">
              <button
                onClick={() => setExpanded(!expanded)}
                className={`flex items-center gap-1.5 text-xs text-muted-foreground hover:text-primary transition-colors cursor-pointer`}
              >
                {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                {expanded ? t('common.hideRawResponse', { defaultValue: 'Hide raw response' }) : t('common.viewRawResponse', { defaultValue: 'View raw response' })}
              </button>
              {expanded && (
                <div className="mt-2 relative">
                  <button
                    onClick={() => {
                      navigator.clipboard.writeText(JSON.stringify(rawData, null, 2))
                      setCopied(true)
                      setTimeout(() => setCopied(false), 2000)
                    }}
                    className={`absolute top-2 right-2 p-1.5 rounded hover:bg-muted/50 transition-colors cursor-pointer`}
                    title={t('common.copy')}
                  >
                    {copied ? <Check size={14} className="text-green-500" /> : <Copy size={14} className={"text-muted-foreground"} />}
                  </button>
                  <pre className={`text-xs p-3 rounded-lg overflow-auto max-h-48 ${colors.codeBlock}`}>
                    {JSON.stringify(rawData, null, 2)}
                  </pre>
                </div>
              )}
            </div>
          )}
        </DialogBody>

        <DialogFooter>
          {type === 'confirm' && (
            <Button 
              variant="secondary" 
              size="lg"
              onClick={onCancel}
            >
              {finalCancelText}
            </Button>
          )}
          <Button
            variant={btnVariant}
            size="lg"
            onClick={onConfirm}
            disabled={loading}
            loading={loading}
          >
            {finalConfirmText}
          </Button>
        </DialogFooter>
      </DialogContent>
    </DialogRoot>
  )
}

export default ConfirmModal
