import { createContext, useContext, useState, useCallback, useMemo, ReactNode } from 'react'
import {
  GatewayConfig,
  DEFAULT_GATEWAY_CONFIG,
  buildGatewayConfigSnapshot,
  buildGatewayRuntimeSnapshot
} from '../gatewayPageState'
import { createGatewayFieldErrors } from '../gatewayPageUtils'

// Generate API keys consistently with index.tsx.
function generateApiKey(): string {
  const random = crypto?.randomUUID?.().replace(/-/g, '') || `${Date.now()}${Math.random().toString(36).slice(2)}`
  return `sk-${random}`
}

interface GatewayConfigContextValue {
  // State
  config: GatewayConfig
  savedConfigSnapshot: string
  hasUnsavedChanges: boolean
  hasFieldErrors: boolean
  fieldErrors: Record<string, string>

  // Computed
  configSnapshot: string
  runtimeSnapshot: string

  // Actions
  setConfig: (config: GatewayConfig) => void
  setField: (key: string, value: any) => void
  setSavedConfigSnapshot: (snapshot: string) => void
  handleGenerateApiKey: () => void
}

export const GatewayConfigContext = createContext<GatewayConfigContextValue | null>(null)

export function useGatewayConfig() {
  const context = useContext(GatewayConfigContext)
  if (!context) {
    throw new Error('useGatewayConfig must be used within GatewayConfigProvider')
  }
  return context
}

export function GatewayConfigProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<GatewayConfig>(DEFAULT_GATEWAY_CONFIG)
  const [savedConfigSnapshot, setSavedConfigSnapshot] = useState(() =>
    buildGatewayConfigSnapshot(DEFAULT_GATEWAY_CONFIG)
  )

  const fieldErrors = useMemo(() => createGatewayFieldErrors(config), [config])
  const hasFieldErrors = Object.keys(fieldErrors).length > 0
  const configSnapshot = useMemo(() => buildGatewayConfigSnapshot(config), [config])
  const runtimeSnapshot = useMemo(() => buildGatewayRuntimeSnapshot(config), [config])
  const hasUnsavedChanges = configSnapshot !== savedConfigSnapshot

  const setField = useCallback((key: string, value: any) => {
    setConfig(prev => ({ ...prev, [key]: value }))
  }, [])

  const handleGenerateApiKey = useCallback(() => {
    setConfig(prev => {
      const generatedKey = generateApiKey()
      const existingKeys = String(prev.clientApiKeysText || prev.apiKey || '').trim()
      const clientApiKeysText = existingKeys ? `${existingKeys}\n${generatedKey}` : generatedKey
      return { ...prev, apiKey: generatedKey, clientApiKeysText }
    })
  }, [])

  const value = useMemo(() => ({
    config,
    savedConfigSnapshot,
    hasUnsavedChanges,
    hasFieldErrors,
    fieldErrors,
    configSnapshot,
    runtimeSnapshot,
    setConfig,
    setField,
    setSavedConfigSnapshot,
    handleGenerateApiKey,
  }), [
    config,
    savedConfigSnapshot,
    hasUnsavedChanges,
    hasFieldErrors,
    fieldErrors,
    configSnapshot,
    runtimeSnapshot,
    setField,
    handleGenerateApiKey,
  ])

  return (
    <GatewayConfigContext.Provider value={value}>
      {children}
    </GatewayConfigContext.Provider>
  )
}
