import { useState, useRef } from 'react'
import { relaunch } from '@tauri-apps/plugin-process'
import { X, Download, RefreshCw, Sparkles, CheckCircle2, FileText, AlertCircle } from 'lucide-react'
import { DialogRoot, DialogContent } from './dialog'
import { Button } from '../ui/button'
import { Progress } from '../ui/progress'
import { useApp } from '../../hooks/useApp'

function UpdateDialog({ updateInfo, update, onClose }) {
  const { t } = useApp()
  const [installing, setInstalling] = useState(false)
  const [downloadComplete, setDownloadComplete] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState(null)
  const [downloadSpeed, setDownloadSpeed] = useState(0)
  const [error, setError] = useState('')
  const lastDownloadedRef = useRef(0)
  const lastTimeRef = useRef(Date.now())

  const formatBytes = (bytes) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
  }

  const formatSpeed = (bytesPerSecond) => formatBytes(bytesPerSecond) + '/s'

  const doUpdate = async () => {
    if (!update) return
    setInstalling(true)
    setError('')
    setDownloadProgress({ percent: 0, downloaded: 0, total: 0 })
    lastDownloadedRef.current = 0
    lastTimeRef.current = Date.now()

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          setDownloadProgress({ percent: 0, downloaded: 0, total: event.data.contentLength || 0 })
        } else if (event.event === 'Progress') {
          setDownloadProgress(prev => {
            const downloaded = (prev?.downloaded || 0) + event.data.chunkLength
            const total = prev?.total || 0
            const percent = total > 0 ? Math.round((downloaded / total) * 100) : 0
            const now = Date.now()
            const timeDiff = (now - lastTimeRef.current) / 1000
            if (timeDiff >= 0.5) {
              const speed = (downloaded - lastDownloadedRef.current) / timeDiff
              setDownloadSpeed(speed)
              lastDownloadedRef.current = downloaded
              lastTimeRef.current = now
            }
            return { percent, downloaded, total }
          })
        } else if (event.event === 'Finished') {
          setDownloadProgress(prev => ({ ...prev, percent: 100 }))
          setDownloadComplete(true)
          setInstalling(false)
        }
      })
    } catch (e) {
      setError(t('update.installFailed') + ': ' + e)
      setInstalling(false)
      setDownloadProgress(null)
    }
  }

  const doRelaunch = async () => {
    try {
      await relaunch()
    } catch (e) {
      setError(e.toString())
    }
  }

  const handleClose = () => {
    if (!installing) onClose()
  }

  return (
    <DialogRoot open={true} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent maxWidth="512px" showClose={false} className="gap-0 overflow-hidden">
        {/* 顶部横幅 */}
        <div className="bg-gradient-to-r from-blue-500 to-indigo-600 px-6 py-5 relative">
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 bg-white/20 rounded-2xl flex items-center justify-center">
              <Sparkles size={28} className="text-white" />
            </div>
            <div className="flex flex-col gap-0.5">
              <h3 className="text-xl font-bold text-white">{t('update.newVersionAvailable')}</h3>
              <p className="text-sm text-white/80">
                v{updateInfo?.version} {t('update.readyToInstall')}
              </p>
            </div>
          </div>
          {!installing && !downloadComplete && (
            <button
              onClick={handleClose}
              className="absolute top-4 right-4 p-1.5 rounded-lg hover:bg-black/10 transition-colors"
            >
              <X size={20} className="text-white/80" />
            </button>
          )}
        </div>

        <div className="p-6">
          {downloadComplete ? (
            <div className="flex flex-col gap-4">
              <div className="flex items-center gap-2">
                <CheckCircle2 size={24} className="text-emerald-500" />
                <span className="text-lg font-medium text-emerald-600">{t('update.downloadComplete')}</span>
              </div>
              <p className="text-sm text-muted-foreground">{t('update.restartToInstall')}</p>
              <div className="flex gap-2">
                <Button onClick={onClose} variant="outline" className="flex-1">
                  {t('update.installLater')}
                </Button>
                <Button onClick={doRelaunch} className="flex-1">
                  <RefreshCw size={16} className="mr-2" />
                  {t('update.restartNow')}
                </Button>
              </div>
            </div>
          ) : installing && downloadProgress ? (
            <div className="flex flex-col gap-4">
              <div className="flex justify-between items-center">
                <span className="text-sm">{t('update.downloading')}...</span>
                <span className="text-sm font-medium text-blue-600">{downloadProgress.percent}%</span>
              </div>
              <Progress value={downloadProgress.percent} className="h-2" />
              <div className="flex justify-between items-center">
                <span className="text-xs text-muted-foreground">
                  {formatBytes(downloadProgress.downloaded)} / {formatBytes(downloadProgress.total)}
                </span>
                <span className="text-xs text-muted-foreground">{formatSpeed(downloadSpeed)}</span>
              </div>
              <div className="flex items-center gap-2">
                <RefreshCw size={12} className="animate-spin text-blue-500" />
                <span className="text-xs text-muted-foreground">{t('update.doNotClose')}</span>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              {updateInfo?.body && (
                <div className="flex flex-col gap-2">
                  <div className="flex items-center gap-2">
                    <FileText size={16} />
                    <span className="text-sm font-medium">{t('update.releaseNotes')}</span>
                  </div>
                  <div className="text-sm max-h-40 overflow-y-auto p-3 rounded-lg whitespace-pre-wrap leading-relaxed border bg-muted/50">
                    {updateInfo.body}
                  </div>
                </div>
              )}
              <div className="flex gap-2">
                <Button onClick={onClose} variant="outline" className="flex-1">
                  {t('update.later')}
                </Button>
                <Button onClick={doUpdate} className="flex-1">
                  <Download size={16} className="mr-2" />
                  {t('update.updateNow')}
                </Button>
              </div>
            </div>
          )}

          {error && (
            <div className="mt-4 p-3 rounded-lg bg-destructive/10 border border-destructive/20 flex items-start gap-2">
              <AlertCircle size={16} className="text-destructive mt-0.5 flex-shrink-0" />
              <span className="text-sm text-destructive">{error}</span>
            </div>
          )}
        </div>
      </DialogContent>
    </DialogRoot>
  )
}

export default UpdateDialog
