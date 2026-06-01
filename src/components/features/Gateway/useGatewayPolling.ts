import { useEffect } from 'react'
import { fetchGatewayRequestLogs, fetchGatewayStatus } from './gatewayPageState'
import { formatGatewayTimestamp } from './gatewayPageUtils'

interface UseGatewayPollingOptions {
  activeTab: string
  fallbackConfig: any
  onStatus: (data: { status: any; fallbackConfig: any; syncedAt: string }) => void
  onRequestLogs?: (data: { logs: any[]; syncedAt: string }) => void
  statusInterval?: number
  logsInterval?: number
}

export function useGatewayPolling({
  activeTab,
  fallbackConfig,
  onStatus,
  onRequestLogs,
  statusInterval = 2000,
  logsInterval = 5000
}: UseGatewayPollingOptions) {
  // Status polling.
  useEffect(() => {
    let timer: NodeJS.Timeout | null = null
    let isActive = true

    const poll = () => {
      if (!isActive || document.hidden) {
        return
      }

      fetchGatewayStatus()
        .then((status) => {
          if (isActive) {
            onStatus({
              status,
              fallbackConfig,
              syncedAt: formatGatewayTimestamp()
            })
          }
        })
        .catch((error) => {
          console.error('[Gateway] Failed to fetch status:', error)
        })
    }

    // Run once immediately.
    poll()

    // Schedule polling.
    timer = setInterval(poll, statusInterval)

    // Pause polling while the page is hidden.
    const handleVisibilityChange = () => {
      if (document.hidden) {
        // Clear the timer while the page is hidden.
        if (timer) {
          clearInterval(timer)
          timer = null
        }
      } else {
        // Restart polling when the page becomes visible.
        if (!timer && isActive) {
          poll()
          timer = setInterval(poll, statusInterval)
        }
      }
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      isActive = false
      if (timer) {
        clearInterval(timer)
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [fallbackConfig, onStatus, statusInterval])

  // Request log polling.
  useEffect(() => {
    if (activeTab !== 'observability') {
      return undefined
    }

    let timer: NodeJS.Timeout | null = null
    let isActive = true

    const poll = () => {
      if (!isActive || document.hidden) {
        return
      }

      fetchGatewayRequestLogs()
        .then((logs) => {
          if (isActive && onRequestLogs) {
            onRequestLogs({
              logs,
              syncedAt: formatGatewayTimestamp()
            })
          }
        })
        .catch((error) => {
          console.error('[Gateway] Failed to fetch request logs:', error)
        })
    }

    // Run once immediately.
    poll()

    // Schedule polling.
    timer = setInterval(poll, logsInterval)

    // Pause polling while the page is hidden.
    const handleVisibilityChange = () => {
      if (document.hidden) {
        // Clear the timer while the page is hidden.
        if (timer) {
          clearInterval(timer)
          timer = null
        }
      } else {
        // Restart polling when the page becomes visible.
        if (!timer && isActive) {
          poll()
          timer = setInterval(poll, logsInterval)
        }
      }
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      isActive = false
      if (timer) {
        clearInterval(timer)
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [activeTab, onRequestLogs, logsInterval])
}
